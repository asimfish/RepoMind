use tauri::State;
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
    let local_path = repo.local_path.clone().ok_or("Repo not indexed")?;
    drop(repos);

    // TODO: Replace with actual LadybugDB query
    // For now, grep-based fallback search
    let results = grep_search(&local_path, &query, &repo_name);
    Ok(results)
}

fn grep_search(local_path: &str, query: &str, repo_name: &str) -> Vec<SearchResult> {
    let output = std::process::Command::new("grep")
        .args([
            "-rn",
            "--include=*.ts",
            "--include=*.js",
            "--include=*.py",
            "--include=*.rs",
            "--include=*.go",
            "-E",
            &format!(r"(fn|function|def|class|const|let|var|type|interface)\s+{}", regex_escape(query)),
            local_path,
        ])
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines()
                .take(20)
                .filter_map(|line| parse_grep_line(line, local_path, repo_name))
                .collect()
        }
        Err(_) => vec![],
    }
}

fn parse_grep_line(line: &str, base_path: &str, repo_name: &str) -> Option<SearchResult> {
    let parts: Vec<&str> = line.splitn(3, ':').collect();
    if parts.len() < 3 {
        return None;
    }

    let file = parts[0].replace(base_path, "").trim_start_matches('/').to_string();
    let line_num: u32 = parts[1].parse().ok()?;
    let snippet = parts[2].trim().to_string();

    // Infer symbol name and type from snippet
    let (symbol, result_type) = infer_symbol(&snippet)?;

    Some(SearchResult {
        symbol,
        file,
        line: line_num,
        snippet,
        result_type,
        score: 1.0,
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
    s.chars()
        .map(|c| {
            if ".+*?^${}[]|()\\".contains(c) {
                format!("\\{}", c)
            } else {
                c.to_string()
            }
        })
        .collect()
}

#[tauri::command]
pub async fn get_context(
    repo_id: String,
    symbol: String,
    state: State<'_, AppState>,
) -> Result<Vec<GraphNode>, String> {
    // TODO: Query LadybugDB for symbol context
    // Placeholder
    Ok(vec![
        GraphNode {
            id: "1".to_string(),
            label: symbol.clone(),
            node_type: "function".to_string(),
            file: Some("src/main.ts".to_string()),
            line: Some(42),
            community: Some("core".to_string()),
        },
    ])
}

#[tauri::command]
pub async fn get_impact(
    repo_id: String,
    symbol: String,
    state: State<'_, AppState>,
) -> Result<ImpactResult, String> {
    // TODO: Query LadybugDB for impact analysis
    // Placeholder
    Ok(ImpactResult {
        symbol: symbol.clone(),
        directly_affected: vec![],
        indirectly_affected: vec![],
        processes: vec![],
    })
}
