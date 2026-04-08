use tauri::{Manager, State};
use tokio::process::Command as AsyncCommand;
use crate::models::{SearchResult, GraphNode, ImpactResult, ImpactNode};
use crate::services::state::AppState;

fn repo_name_for(local_path: &str) -> String {
    std::path::Path::new(local_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}

#[tauri::command]
pub async fn search(
    repo_id: String,
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let repos = state.indexed_repos.read().await;
    let repo = repos.get(&repo_id).ok_or("Repo not found")?;
    let repo_name = repo.name.clone();
    let local_path = repo.local_path.clone().ok_or("Repo not cloned")?;
    let is_indexed = repo.index_status == crate::models::IndexStatus::Indexed;
    drop(repos);

    // ── BM25 via gitnexus query ─────────────────────────────────────────────
    let mut results = if is_indexed {
        gitnexus_query(&local_path, &query, &repo_name).await
            .unwrap_or_else(|_| grep_search(&local_path, &query, &repo_name).unwrap_or_default())
    } else {
        grep_search(&local_path, &query, &repo_name).unwrap_or_default()
    };

    // ── Semantic vector search (if available) ─────────────────────────────
    use crate::services::vector::{VectorStore, get_embedding, is_ollama_available};
    if is_indexed && is_ollama_available().await {
        let data_dir = dirs::data_dir()
            .unwrap_or_default()
            .join("com.liyufeng.repomind")
            .join("vectors")
            .join(&repo_id);

        if data_dir.exists() {
            let store = VectorStore::new(&data_dir);
            if store.count() > 0 {
                if let Ok(query_embedding) = get_embedding(&query).await {
                    if !query_embedding.is_empty() {
                        let vector_results = store.search(&query_embedding, 15);
                        let mut fused = crate::services::vector::rrf_fuse(
                            &results, &vector_results, 60.0
                        );
                        for r in &mut fused { r.repo_name = repo_name.clone(); }
                        return Ok(fused.into_iter().take(25).collect());
                    }
                }
            }
        }
    }

    Ok(results.into_iter().take(25).collect())
}

/// gitnexus query returns { processes, process_symbols }
async fn gitnexus_query(local_path: &str, query: &str, repo_name: &str) -> Result<Vec<SearchResult>, String> {
    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();
    let rname = repo_name_for(local_path);

    let output = AsyncCommand::new(&gitnexus)
        .args(["query", query, "-r", &rname])
        .current_dir(local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let body: serde_json::Value = serde_json::from_str(
        &String::from_utf8_lossy(&output.stdout)
    ).map_err(|e| e.to_string())?;

    let symbols = body["process_symbols"].as_array().cloned().unwrap_or_default();

    // Deduplicate by id
    let mut seen = std::collections::HashSet::new();
    let results = symbols.into_iter()
        .filter_map(|s| {
            let id = s["id"].as_str()?.to_string();
            if !seen.insert(id.clone()) { return None; }

            let file_path = s["filePath"].as_str().unwrap_or("").to_string();
            let start_line = s["startLine"].as_u64().unwrap_or(0) as u32;
            let end_line = s["endLine"].as_u64().unwrap_or(start_line as u64 + 10) as u32;

            // Read actual code snippet from file
            let snippet = read_snippet(&format!("{}/{}", local_path, file_path), start_line, end_line);

            Some(SearchResult {
                symbol: s["name"].as_str().unwrap_or("").to_string(),
                file: file_path,
                line: start_line,
                snippet,
                result_type: infer_type_from_id(&id),
                score: 1.0 / (s["step_index"].as_f64().unwrap_or(0.0) + 1.0) as f32,
                repo_name: repo_name.to_string(),
            })
        })
        .collect();

    Ok(results)
}

/// Read lines from a file between start_line and end_line (1-indexed)
fn read_snippet(file_path: &str, start: u32, end: u32) -> String {
    let content = std::fs::read_to_string(file_path).unwrap_or_default();
    content.lines()
        .enumerate()
        .filter(|(i, _)| {
            let line = *i as u32 + 1;
            line >= start && line <= end.min(start + 30) // max 30 lines
        })
        .map(|(_, l)| l)
        .collect::<Vec<_>>()
        .join("\n")
}

fn infer_type_from_id(id: &str) -> String {
    if id.starts_with("Function:") { "function" }
    else if id.starts_with("Class:") { "class" }
    else if id.starts_with("Method:") { "method" }
    else if id.starts_with("Interface:") { "interface" }
    else if id.starts_with("Variable:") { "variable" }
    else { "function" }
    .to_string()
}

fn grep_search(local_path: &str, query: &str, repo_name: &str) -> Result<Vec<SearchResult>, String> {
    let query = if query.len() > 200 { &query[..200] } else { query };
    let output = std::process::Command::new("grep")
        .args(["-rn", "--include=*.ts", "--include=*.tsx", "--include=*.js",
               "--include=*.py", "--include=*.rs", "--include=*.go",
               "--include=*.java", "--include=*.swift",
               "-F", query, local_path])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .take(20)
        .filter_map(|line| parse_grep_line(line, local_path, repo_name))
        .collect())
}

fn parse_grep_line(line: &str, base_path: &str, repo_name: &str) -> Option<SearchResult> {
    let parts: Vec<&str> = line.splitn(3, ':').collect();
    if parts.len() < 3 { return None; }
    let file = parts[0].replace(base_path, "").trim_start_matches('/').to_string();
    let line_num: u32 = parts[1].parse().ok()?;
    let snippet = parts[2].trim().to_string();
    Some(SearchResult {
        symbol: snippet.split_whitespace().nth(1).unwrap_or(&snippet).trim_end_matches('(').to_string(),
        file,
        line: line_num,
        snippet,
        result_type: "function".to_string(),
        score: 0.3,
        repo_name: repo_name.to_string(),
    })
}

#[tauri::command]
pub async fn get_graph(
    repo_id: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let repos = state.indexed_repos.read().await;
    let repo = repos.get(&repo_id).ok_or("Repo not found")?;
    let local_path = repo.local_path.clone().ok_or("Not cloned")?;
    drop(repos);

    let rname = repo_name_for(&local_path);
    let limit = limit.unwrap_or(300);
    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();

    // Get symbol nodes (functions, classes, interfaces, etc.)
    let node_query = format!(
        "MATCH (n) WHERE n.startLine IS NOT NULL RETURN n.id, n.name, n.filePath, n.startLine LIMIT {}",
        limit
    );
    let nodes_out = AsyncCommand::new(&gitnexus)
        .args(["cypher", &node_query, "-r", &rname])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    // Get edges (any relationship between symbols)
    let edge_query = format!(
        "MATCH (a)-[r]->(b) WHERE a.startLine IS NOT NULL AND b.startLine IS NOT NULL RETURN a.id, b.id LIMIT {}",
        limit * 2
    );
    let edges_out = AsyncCommand::new(&gitnexus)
        .args(["cypher", &edge_query, "-r", &rname])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let nodes = parse_cypher_table(&String::from_utf8_lossy(&nodes_out.stdout), |cells| {
        let id = cells.first()?.trim().to_string();
        let label = cells.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
        let file = cells.get(2).map(|s| s.trim().to_string());
        let line = cells.get(3).and_then(|s| s.trim().parse::<u64>().ok());
        let node_type = infer_type_from_id(&id);
        Some(serde_json::json!({
            "id": id, "label": label, "type": node_type,
            "file": file, "line": line,
        }))
    });

    let edges = parse_cypher_table(&String::from_utf8_lossy(&edges_out.stdout), |cells| {
        let source = cells.first()?.trim().to_string();
        let target = cells.get(1)?.trim().to_string();
        Some(serde_json::json!({
            "id": format!("{}->{}", &source[..source.len().min(20)], &target[..target.len().min(20)]),
            "source": source,
            "target": target,
            "type": "calls",
            "confidence": 1.0,
        }))
    });

    Ok(serde_json::json!({ "nodes": nodes, "edges": edges }))
}

/// Generic cypher table parser
/// Format: { "markdown": "| col1 | col2 |\n| --- |\n| val1 | val2 |" }
fn parse_cypher_table<F>(output: &str, mapper: F) -> Vec<serde_json::Value>
where
    F: Fn(&[&str]) -> Option<serde_json::Value>,
{
    let body: serde_json::Value = serde_json::from_str(output).unwrap_or_default();
    let markdown = body["markdown"].as_str().unwrap_or("");

    markdown.lines()
        .filter(|l| l.starts_with('|') && !l.contains("---") && !l.contains("| n.id |") && !l.contains("| a.id |"))
        .filter_map(|row| {
            let cells: Vec<&str> = row.split('|')
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .collect();
            mapper(&cells)
        })
        .collect()
}

#[tauri::command]
pub async fn get_context(
    repo_id: String,
    symbol: String,
    state: State<'_, AppState>,
) -> Result<Vec<GraphNode>, String> {
    let repos = state.indexed_repos.read().await;
    let repo = repos.get(&repo_id).ok_or("Repo not found")?;
    let local_path = repo.local_path.clone().ok_or("Not cloned")?;
    drop(repos);

    let rname = repo_name_for(&local_path);
    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();

    let output = AsyncCommand::new(&gitnexus)
        .args(["context", &symbol, "-r", &rname])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = serde_json::from_str(
        &String::from_utf8_lossy(&output.stdout)
    ).unwrap_or_default();

    let mut nodes = vec![];

    // The target symbol itself
    if let Some(sym) = body["symbol"].as_object() {
        nodes.push(GraphNode {
            id: sym["uid"].as_str().unwrap_or("").to_string(),
            label: sym["name"].as_str().unwrap_or("").to_string(),
            node_type: "function".to_string(),
            file: sym["filePath"].as_str().map(|s| s.to_string()),
            line: sym["startLine"].as_u64().map(|l| l as u32),
            community: None,
        });
    }

    // Callers (incoming calls)
    if let Some(calls) = body["incoming"]["calls"].as_array() {
        for c in calls {
            nodes.push(GraphNode {
                id: c["uid"].as_str().unwrap_or("").to_string(),
                label: c["name"].as_str().unwrap_or("").to_string(),
                node_type: infer_type_from_id(c["uid"].as_str().unwrap_or("")),
                file: c["filePath"].as_str().map(|s| s.to_string()),
                line: None,
                community: Some("caller".to_string()),
            });
        }
    }

    // Callees (outgoing calls)
    if let Some(calls) = body["outgoing"]["calls"].as_array() {
        for c in calls {
            nodes.push(GraphNode {
                id: c["uid"].as_str().unwrap_or("").to_string(),
                label: c["name"].as_str().unwrap_or("").to_string(),
                node_type: infer_type_from_id(c["uid"].as_str().unwrap_or("")),
                file: c["filePath"].as_str().map(|s| s.to_string()),
                line: None,
                community: Some("callee".to_string()),
            });
        }
    }

    Ok(nodes)
}

#[tauri::command]
pub async fn get_impact(
    repo_id: String,
    symbol: String,
    state: State<'_, AppState>,
) -> Result<ImpactResult, String> {
    let repos = state.indexed_repos.read().await;
    let repo = repos.get(&repo_id).ok_or("Repo not found")?;
    let local_path = repo.local_path.clone().ok_or("Not cloned")?;
    drop(repos);

    let rname = repo_name_for(&local_path);
    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();

    let output = AsyncCommand::new(&gitnexus)
        .args(["impact", &symbol, "-r", &rname])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = serde_json::from_str(
        &String::from_utf8_lossy(&output.stdout)
    ).unwrap_or_default();

    // byDepth: { "1": [...], "2": [...], "3": [...] }
    let by_depth = body["byDepth"].as_object().cloned().unwrap_or_default();

    let mut directly = vec![];
    let mut indirectly = vec![];

    for (depth_str, nodes) in &by_depth {
        let depth: u32 = depth_str.parse().unwrap_or(1);
        if let Some(arr) = nodes.as_array() {
            for n in arr {
                let node = ImpactNode {
                    symbol: n["name"].as_str().unwrap_or("").to_string(),
                    file: n["filePath"].as_str().unwrap_or("").to_string(),
                    confidence: n["confidence"].as_f64().unwrap_or(1.0) as f32,
                    depth,
                };
                if depth == 1 { directly.push(node); }
                else { indirectly.push(node); }
            }
        }
    }

    // Affected processes
    let processes = body["affected_processes"].as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|p| p["name"].as_str().map(|s| s.to_string()))
        .collect();

    Ok(ImpactResult {
        symbol,
        directly_affected: directly,
        indirectly_affected: indirectly,
        processes,
    })
}

#[tauri::command]
pub async fn get_ai_summary(
    repo_id: String,
    symbol: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let claude_key = state.settings.read().await.claude_api_key.clone();
    let claude_key = claude_key.ok_or("Claude API key not configured")?;

    let repos = state.indexed_repos.read().await;
    let repo = repos.get(&repo_id).ok_or("Repo not found")?;
    let local_path = repo.local_path.clone().ok_or("Not cloned")?;
    drop(repos);

    let rname = repo_name_for(&local_path);
    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();

    let context_out = AsyncCommand::new(&gitnexus)
        .args(["context", &symbol, "-r", &rname, "--content"])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let context = String::from_utf8_lossy(&context_out.stdout).to_string();

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &claude_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 300,
            "messages": [{
                "role": "user",
                "content": format!(
                    "用中文简洁描述以下代码符号的功能（2-3句话，重点说明：做什么、接受什么参数、返回什么、有什么副作用）：\n\n符号名：{}\n\n上下文：\n{}",
                    symbol, &context[..context.len().min(2000)]
                )
            }]
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    Ok(body["content"][0]["text"].as_str().unwrap_or("无法生成摘要").to_string())
}

#[tauri::command]
pub async fn validate_claude_key(api_key: String) -> Result<bool, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.status().is_success())
}

#[tauri::command]
pub async fn get_mcp_status() -> Result<serde_json::Value, String> {
    let home = dirs::home_dir().unwrap_or_default();
    let candidates = vec![
        home.join("Desktop/RepoMind/src-tauri/target/release/repomind-mcp"),
        home.join("Desktop/RepoMind/src-tauri/target/debug/repomind-mcp"),
        std::path::PathBuf::from("/usr/local/bin/repomind-mcp"),
    ];
    let found = candidates.iter().find(|p| p.exists());
    let installed = found.is_some();
    let path = found.map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

    let claude_settings = home.join(".claude/settings.json");
    let registered_claude = std::fs::read_to_string(&claude_settings)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["mcpServers"]["repomind"].as_object().map(|_| true))
        .unwrap_or(false);

    Ok(serde_json::json!({
        "installed": installed,
        "path": path,
        "registeredClaude": registered_claude,
    }))
}
