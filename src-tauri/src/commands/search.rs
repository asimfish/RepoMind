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
        // Use gitnexus query for graph-aware search
        gitnexus_search(&local_path, &query, &repo_name).await
            .or_else(|_| grep_search(&local_path, &query, &repo_name))
    } else {
        // Fallback to grep if not indexed yet
        grep_search(&local_path, &query, &repo_name)
    }
}

async fn gitnexus_search(
    local_path: &str,
    query: &str,
    repo_name: &str,
) -> Result<Vec<SearchResult>, String> {
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
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&text)
        .unwrap_or_default();

    let results = parsed.into_iter().take(30).filter_map(|r| {
        Some(SearchResult {
            symbol: r["name"].as_str()?.to_string(),
            file: r["file"].as_str().unwrap_or("").to_string(),
            line: r["line"].as_u64().unwrap_or(0) as u32,
            snippet: r["snippet"].as_str().unwrap_or("").to_string(),
            result_type: r["type"].as_str().unwrap_or("function").to_string(),
            score: r["score"].as_f64().unwrap_or(1.0) as f32,
            repo_name: repo_name.to_string(),
        })
    }).collect();

    Ok(results)
}

fn grep_search(local_path: &str, query: &str, repo_name: &str) -> Result<Vec<SearchResult>, String> {
    let output = std::process::Command::new("grep")
        .args([
            "-rn",
            "--include=*.ts", "--include=*.tsx",
            "--include=*.js", "--include=*.py",
            "--include=*.rs", "--include=*.go",
            "--include=*.java", "--include=*.swift",
            "-E",
            &format!(r"(fn |function |def |class |const |let |interface |type )\s*{}", regex_escape(query)),
            local_path,
        ])
        .output()
        .map_err(|e| e.to_string())?;

    let text = String::from_utf8_lossy(&output.stdout);
    let results = text.lines()
        .take(20)
        .filter_map(|line| parse_grep_line(line, local_path, repo_name))
        .collect();

    Ok(results)
}

fn parse_grep_line(line: &str, base_path: &str, repo_name: &str) -> Option<SearchResult> {
    let parts: Vec<&str> = line.splitn(3, ':').collect();
    if parts.len() < 3 { return None; }

    let file = parts[0].replace(base_path, "").trim_start_matches('/').to_string();
    let line_num: u32 = parts[1].parse().ok()?;
    let snippet = parts[2].trim().to_string();
    let (symbol, result_type) = infer_symbol(&snippet)?;

    Some(SearchResult {
        symbol,
        file,
        line: line_num,
        snippet,
        result_type,
        score: 0.5,
        repo_name: repo_name.to_string(),
    })
}

fn infer_symbol(snippet: &str) -> Option<(String, String)> {
    let patterns = [
        (r"fn\s+(\w+)", "function"),
        (r"function\s+(\w+)", "function"),
        (r"def\s+(\w+)", "function"),
        (r"class\s+(\w+)", "class"),
        (r"interface\s+(\w+)", "interface"),
        (r"type\s+(\w+)", "class"),
        (r"const\s+(\w+)", "variable"),
        (r"let\s+(\w+)", "variable"),
    ];
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
    s.chars().map(|c| {
        if ".+*?^${}[]|()\\".contains(c) { format!("\\{}", c) }
        else { c.to_string() }
    }).collect()
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

    if !output.status.success() {
        return Ok(vec![]);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap_or_default();

    let nodes = parsed.into_iter().filter_map(|n| {
        Some(GraphNode {
            id: n["id"].as_str()?.to_string(),
            label: n["name"].as_str().unwrap_or("").to_string(),
            node_type: n["type"].as_str().unwrap_or("function").to_string(),
            file: n["file"].as_str().map(|s| s.to_string()),
            line: n["line"].as_u64().map(|l| l as u32),
            community: n["community"].as_str().map(|s| s.to_string()),
        })
    }).collect();

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

    let gitnexus = crate::commands::index::find_gitnexus_bin_pub();
    let output = AsyncCommand::new(&gitnexus)
        .args(["impact", &symbol, "--json"])
        .current_dir(&local_path)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Ok(ImpactResult {
            symbol: symbol.clone(),
            directly_affected: vec![],
            indirectly_affected: vec![],
            processes: vec![],
        });
    }

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
        processes: parsed["processes"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|p| p.as_str().map(|s| s.to_string()))
            .collect(),
    })
}
