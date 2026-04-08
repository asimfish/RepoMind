use tauri::{Manager, State};
use tokio::process::Command as AsyncCommand;
use crate::models::{SearchResult, GraphNode, ImpactResult, ImpactNode};
use crate::services::state::AppState;

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

    if is_indexed {
        gitnexus_search(&local_path, &query, &repo_name).await
            .or_else(|_| grep_search(&local_path, &query, &repo_name))
    } else {
        grep_search(&local_path, &query, &repo_name)
    }
}

async fn gitnexus_search(local_path: &str, query: &str, repo_name: &str) -> Result<Vec<SearchResult>, String> {
    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();
    let output = AsyncCommand::new(&gitnexus)
        .args(["query", query, "--json"])
        .current_dir(local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("gitnexus query failed".to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap_or_default();

    Ok(parsed.into_iter().take(30).filter_map(|r| {
        Some(SearchResult {
            symbol: r["name"].as_str()?.to_string(),
            file: r["file"].as_str().unwrap_or("").to_string(),
            line: r["line"].as_u64().unwrap_or(0) as u32,
            snippet: r["snippet"].as_str().unwrap_or("").to_string(),
            result_type: r["type"].as_str().unwrap_or("function").to_string(),
            score: r["score"].as_f64().unwrap_or(1.0) as f32,
            repo_name: repo_name.to_string(),
        })
    }).collect())
}

fn grep_search(local_path: &str, query: &str, repo_name: &str) -> Result<Vec<SearchResult>, String> {
    let output = std::process::Command::new("grep")
        .args(["-rn", "--include=*.ts", "--include=*.tsx", "--include=*.js",
               "--include=*.py", "--include=*.rs", "--include=*.go",
               "--include=*.java", "--include=*.swift", "-E",
               &format!(r"(fn |function |def |class |const |let |interface |type )\s*{}", regex_escape(query)),
               local_path])
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
    let (symbol, result_type) = infer_symbol(&snippet)?;
    Some(SearchResult { symbol, file, line: line_num, snippet, result_type, score: 0.5, repo_name: repo_name.to_string() })
}

fn infer_symbol(snippet: &str) -> Option<(String, String)> {
    let patterns = [("fn\\s+(\\w+)", "function"), ("function\\s+(\\w+)", "function"),
                    ("def\\s+(\\w+)", "function"), ("class\\s+(\\w+)", "class"),
                    ("interface\\s+(\\w+)", "interface"), ("const\\s+(\\w+)", "variable")];
    for (pattern, rtype) in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(caps) = re.captures(snippet) {
                if let Some(name) = caps.get(1) {
                    return Some((name.as_str().to_string(), rtype.to_string()));
                }
            }
        }
    }
    None
}

fn regex_escape(s: &str) -> String {
    s.chars().map(|c| if ".+*?^${}[]|()\\".contains(c) { format!("\\{}", c) } else { c.to_string() }).collect()
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

    let limit = limit.unwrap_or(500);
    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();

    // Use gitnexus cypher to get graph nodes and edges
    let nodes_out = AsyncCommand::new(&gitnexus)
        .args(["cypher",
               &format!("MATCH (n) WHERE n.type IN ['function','class','method','community'] RETURN n.id, n.name, n.type, n.file, n.line LIMIT {}", limit),
               "--json"])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let edges_out = AsyncCommand::new(&gitnexus)
        .args(["cypher",
               &format!("MATCH (a)-[r]->(b) RETURN a.id, type(r), b.id, r.confidence LIMIT {}", limit * 2),
               "--json"])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let nodes_raw: Vec<serde_json::Value> = serde_json::from_str(
        &String::from_utf8_lossy(&nodes_out.stdout)
    ).unwrap_or_default();

    let edges_raw: Vec<serde_json::Value> = serde_json::from_str(
        &String::from_utf8_lossy(&edges_out.stdout)
    ).unwrap_or_default();

    let nodes: Vec<serde_json::Value> = nodes_raw.into_iter().filter_map(|n| {
        Some(serde_json::json!({
            "id": n["n.id"].as_str()?,
            "label": n["n.name"].as_str().unwrap_or(""),
            "type": n["n.type"].as_str().unwrap_or("function"),
            "file": n["n.file"],
            "line": n["n.line"],
        }))
    }).collect();

    let edges: Vec<serde_json::Value> = edges_raw.into_iter().enumerate().filter_map(|(i, e)| {
        Some(serde_json::json!({
            "id": format!("e{}", i),
            "source": e["a.id"].as_str()?,
            "target": e["b.id"].as_str()?,
            "type": e["type(r)"].as_str().unwrap_or("calls").to_lowercase(),
            "confidence": e["r.confidence"].as_f64().unwrap_or(1.0),
        }))
    }).collect();

    Ok(serde_json::json!({ "nodes": nodes, "edges": edges }))
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

    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();
    let output = AsyncCommand::new(&gitnexus)
        .args(["context", &symbol, "--json"])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let text = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap_or_default();

    Ok(parsed.into_iter().filter_map(|n| {
        Some(GraphNode {
            id: n["id"].as_str()?.to_string(),
            label: n["name"].as_str().unwrap_or("").to_string(),
            node_type: n["type"].as_str().unwrap_or("function").to_string(),
            file: n["file"].as_str().map(|s| s.to_string()),
            line: n["line"].as_u64().map(|l| l as u32),
            community: n["community"].as_str().map(|s| s.to_string()),
        })
    }).collect())
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

    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();
    let output = AsyncCommand::new(&gitnexus)
        .args(["impact", &symbol, "--json"])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let text = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();

    let parse_nodes = |arr: &serde_json::Value, depth: u32| -> Vec<ImpactNode> {
        arr.as_array().unwrap_or(&vec![]).iter().filter_map(|n| {
            Some(ImpactNode {
                symbol: n["name"].as_str()?.to_string(),
                file: n["file"].as_str().unwrap_or("").to_string(),
                confidence: n["confidence"].as_f64().unwrap_or(1.0) as f32,
                depth,
            })
        }).collect()
    };

    Ok(ImpactResult {
        symbol,
        directly_affected: parse_nodes(&parsed["direct"], 1),
        indirectly_affected: parse_nodes(&parsed["indirect"], 2),
        processes: parsed["processes"].as_array().unwrap_or(&vec![])
            .iter().filter_map(|p| p.as_str().map(|s| s.to_string())).collect(),
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

    // Get code snippet via gitnexus context
    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();
    let context_out = AsyncCommand::new(&gitnexus)
        .args(["context", &symbol])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let context = String::from_utf8_lossy(&context_out.stdout).to_string();

    // Call Claude API
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
    let summary = body["content"][0]["text"].as_str()
        .unwrap_or("无法生成摘要")
        .to_string();

    Ok(summary)
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
    // Check if repomind-mcp binary exists
    let home = dirs::home_dir().unwrap_or_default();
    let candidates = vec![
        home.join("Desktop/RepoMind/src-tauri/target/release/repomind-mcp"),
        home.join("Desktop/RepoMind/src-tauri/target/debug/repomind-mcp"),
        std::path::PathBuf::from("/usr/local/bin/repomind-mcp"),
    ];

    let found = candidates.iter().find(|p| p.exists());
    let installed = found.is_some();
    let path = found.map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

    // Check if registered in Claude settings
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
