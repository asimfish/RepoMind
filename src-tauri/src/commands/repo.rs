use tauri::State;
use crate::models::{Repo, IndexStatus};
use crate::services::state::AppState;
use uuid::Uuid;

#[tauri::command]
pub async fn list_github_repos(
    page: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<Repo>, String> {
    let token = state.github_token.read().await.clone();
    let token = token.ok_or("Not authenticated")?;
    let page = page.unwrap_or(1);

    let client = reqwest::Client::new();
    let response = client
        .get(format!("https://api.github.com/user/repos?per_page=50&page={}&sort=updated", page))
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "RepoMind/0.1.0")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let repos: Vec<serde_json::Value> = response.json().await.map_err(|e| e.to_string())?;

    let indexed = state.indexed_repos.read().await;

    let result = repos
        .into_iter()
        .map(|r| {
            let full_name = r["full_name"].as_str().unwrap_or("").to_string();
            let is_indexed = indexed.values().any(|ir| ir.full_name == full_name);
            let index_status = if is_indexed {
                IndexStatus::Indexed
            } else {
                IndexStatus::NotIndexed
            };

            Repo {
                id: r["id"].to_string(),
                name: r["name"].as_str().unwrap_or("").to_string(),
                full_name,
                description: r["description"].as_str().map(|s| s.to_string()),
                language: r["language"].as_str().map(|s| s.to_string()),
                stars: r["stargazers_count"].as_u64().unwrap_or(0),
                is_private: r["private"].as_bool().unwrap_or(false),
                clone_url: r["clone_url"].as_str().unwrap_or("").to_string(),
                html_url: r["html_url"].as_str().unwrap_or("").to_string(),
                updated_at: r["updated_at"].as_str().unwrap_or("").to_string(),
                local_path: None,
                index_status,
                last_indexed_at: None,
                last_commit: None,
            }
        })
        .collect();

    Ok(result)
}

#[tauri::command]
pub async fn list_indexed_repos(state: State<'_, AppState>) -> Result<Vec<Repo>, String> {
    let repos = state.indexed_repos.read().await;
    Ok(repos.values().cloned().collect())
}

#[tauri::command]
pub async fn add_repo(
    repo_full_name: String,
    state: State<'_, AppState>,
) -> Result<Repo, String> {
    let token = state.github_token.read().await.clone();
    let token = token.ok_or("Not authenticated")?;

    // Fetch repo info from GitHub
    let client = reqwest::Client::new();
    let response = client
        .get(format!("https://api.github.com/repos/{}", repo_full_name))
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "RepoMind/0.1.0")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let r: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let settings = state.settings.read().await;
    let local_path = format!("{}/{}", settings.index_storage_path, repo_full_name.replace('/', "_"));

    let repo = Repo {
        id: id.clone(),
        name: r["name"].as_str().unwrap_or("").to_string(),
        full_name: r["full_name"].as_str().unwrap_or("").to_string(),
        description: r["description"].as_str().map(|s| s.to_string()),
        language: r["language"].as_str().map(|s| s.to_string()),
        stars: r["stargazers_count"].as_u64().unwrap_or(0),
        is_private: r["private"].as_bool().unwrap_or(false),
        clone_url: r["clone_url"].as_str().unwrap_or("").to_string(),
        html_url: r["html_url"].as_str().unwrap_or("").to_string(),
        updated_at: r["updated_at"].as_str().unwrap_or("").to_string(),
        local_path: Some(local_path),
        index_status: IndexStatus::NotIndexed,
        last_indexed_at: None,
        last_commit: None,
    };

    let mut repos = state.indexed_repos.write().await;
    repos.insert(id, repo.clone());
    drop(repos);
    state.persist().await;

    Ok(repo)
}

/// Add a repo by GitHub URL (https://github.com/owner/repo or git@github.com:owner/repo)
#[tauri::command]
pub async fn add_repo_by_url(
    url: String,
    state: State<'_, AppState>,
) -> Result<Repo, String> {
    let full_name = parse_github_url(&url)
        .ok_or_else(|| format!("无法解析 GitHub 仓库链接: {}", url))?;
    add_repo(full_name, state).await
}

fn parse_github_url(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches('/').trim_end_matches(".git");

    // https://github.com/owner/repo  or  github.com/owner/repo
    if let Some(rest) = url.strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("github.com/"))
    {
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            return Some(format!("{}/{}", parts[0], parts[1]));
        }
    }

    // git@github.com:owner/repo
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Some(format!("{}/{}", parts[0], parts[1]));
        }
    }

    // Plain owner/repo
    let parts: Vec<&str> = url.splitn(2, '/').collect();
    if parts.len() == 2 && !parts[0].contains('.') && !parts[0].is_empty() {
        return Some(url.to_string());
    }

    None
}

#[tauri::command]
pub async fn remove_repo(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut repos = state.indexed_repos.write().await;
    repos.remove(&repo_id);
    Ok(())
}

#[tauri::command]
pub async fn get_repo(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<Repo, String> {
    let repos = state.indexed_repos.read().await;
    repos.get(&repo_id).cloned().ok_or_else(|| "Repo not found".to_string())
}
