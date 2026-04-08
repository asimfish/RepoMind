use tauri::{AppHandle, Emitter, State};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::models::GitHubUser;
use crate::services::state::AppState;

// GitHub OAuth App credentials
// Users should register their own at: https://github.com/settings/developers
// Default is a shared dev app — replace with your own for production
const GITHUB_CLIENT_ID: &str = "Ov23li8DFpDJQHMXvfge";
const OAUTH_CALLBACK_PORT: u16 = 7890;

#[tauri::command]
pub async fn start_github_oauth(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let redirect_uri = format!("http://localhost:{}/callback", OAUTH_CALLBACK_PORT);
    let scope = "repo,read:user,user:email";
    let oauth_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}",
        GITHUB_CLIENT_ID, redirect_uri, scope
    );

    // Open browser
    let opener = tauri_plugin_opener::OpenerExt::opener(&app);
    opener.open_url(&oauth_url, None::<&str>).map_err(|e| e.to_string())?;

    // Start local callback server
    let state_clone = state.github_token.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) = listen_for_callback(state_clone, app_clone).await {
            eprintln!("OAuth callback error: {}", e);
        }
    });

    Ok(())
}

async fn listen_for_callback(
    token_store: std::sync::Arc<tokio::sync::RwLock<Option<String>>>,
    app: AppHandle,
) -> Result<(), String> {
    let addr = format!("127.0.0.1:{}", OAUTH_CALLBACK_PORT);
    let listener = TcpListener::bind(&addr).await.map_err(|e| e.to_string())?;

    // Accept one connection only
    let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await.map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the code from GET /callback?code=xxx
    let code = extract_query_param(&request, "code")
        .ok_or("No code in callback")?;

    // Exchange code for token
    let client_secret = std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default();
    let token = crate::services::github::exchange_code_for_token(
        &code,
        &client_secret,
        GITHUB_CLIENT_ID,
    )
    .await?;

    // Store token
    let mut t = token_store.write().await;
    *t = Some(token);
    drop(t);

    // Send success HTML response
    let html = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>RepoMind</title>
<style>body{font-family:system-ui;background:#0d1117;color:#e6edf3;display:flex;align-items:center;justify-content:center;height:100vh;margin:0}
.box{text-align:center}.icon{font-size:48px}.title{font-size:24px;margin:16px 0}.sub{color:#8b949e}</style></head>
<body><div class="box"><div class="icon">✓</div>
<div class="title">授权成功</div>
<div class="sub">请返回 RepoMind 应用</div></div></body></html>"#;

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(), html
    );
    stream.write_all(response.as_bytes()).await.ok();
    stream.flush().await.ok();

    // Notify frontend & persist state
    let _ = app.emit("oauth-success", ());
    {
        use tauri::Manager;
        if let Ok(state) = app.try_state::<crate::services::state::AppState>().ok_or("") {
            state.persist().await;
        }
    }

    Ok(())
}

fn extract_query_param<'a>(request: &'a str, key: &str) -> Option<String> {
    let line = request.lines().next()?;
    // GET /callback?code=xxx&state=yyy HTTP/1.1
    let query_start = line.find('?')?;
    let path_end = line.rfind(' ')?;
    let query_str = &line[query_start + 1..path_end];

    for pair in query_str.split('&') {
        let mut parts = pair.splitn(2, '=');
        if parts.next() == Some(key) {
            return parts.next().map(|v| urlencoding_decode(v));
        }
    }
    None
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().and_then(|c| c.to_digit(16));
            let h2 = chars.next().and_then(|c| c.to_digit(16));
            if let (Some(h1), Some(h2)) = (h1, h2) {
                result.push(char::from(((h1 * 16 + h2) as u8)));
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
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
