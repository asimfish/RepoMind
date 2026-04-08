use tauri::State;
use crate::models::{GitHubUser, Repo};
use crate::services::state::AppState;

const GITHUB_CLIENT_ID: &str = "Ov23li8DFpDJQHMXvfge"; // 需要用户配置自己的

#[tauri::command]
pub async fn start_github_oauth() -> Result<String, String> {
    let redirect_uri = "http://localhost:7890/callback";
    let scope = "repo,read:user,user:email";
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}",
        GITHUB_CLIENT_ID, redirect_uri, scope
    );
    Ok(url)
}

#[tauri::command]
pub async fn handle_oauth_callback(
    code: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Exchange code for token via GitHub API
    let client = reqwest::Client::new();
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": GITHUB_CLIENT_ID,
            "client_secret": std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default(),
            "code": code,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let token = body["access_token"]
        .as_str()
        .ok_or("No access token in response")?
        .to_string();

    let mut t = state.github_token.write().await;
    *t = Some(token.clone());

    // Also persist in settings
    let mut settings = state.settings.write().await;
    settings.github_token = Some(token);

    Ok(())
}

#[tauri::command]
pub async fn get_github_token(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.github_token.read().await.clone())
}

#[tauri::command]
pub async fn clear_github_token(state: State<'_, AppState>) -> Result<(), String> {
    let mut t = state.github_token.write().await;
    *t = None;
    let mut settings = state.settings.write().await;
    settings.github_token = None;
    Ok(())
}

#[tauri::command]
pub async fn get_github_user(state: State<'_, AppState>) -> Result<GitHubUser, String> {
    let token = state.github_token.read().await.clone();
    let token = token.ok_or("Not authenticated")?;

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "RepoMind/0.1.0")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    Ok(GitHubUser {
        login: body["login"].as_str().unwrap_or("").to_string(),
        name: body["name"].as_str().map(|s| s.to_string()),
        avatar_url: body["avatar_url"].as_str().unwrap_or("").to_string(),
        email: body["email"].as_str().map(|s| s.to_string()),
    })
}
