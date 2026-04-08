use tauri::{AppHandle, Emitter, State};
use crate::models::GitHubUser;
use crate::services::state::AppState;

const OAUTH_CALLBACK_PORT: u16 = 7890;

fn get_client_id(settings: &crate::models::AppSettings) -> Option<String> {
    // Priority: user-saved > compile-time env var
    settings.github_client_id.clone()
        .or_else(|| std::env::var("GITHUB_CLIENT_ID").ok())
        .filter(|s| !s.is_empty() && s != "YOUR_CLIENT_ID")
}

#[tauri::command]
pub async fn is_oauth_configured(state: State<'_, AppState>) -> Result<bool, String> {
    let settings = state.settings.read().await;
    Ok(get_client_id(&*settings).is_some())
}

#[tauri::command]
pub async fn set_github_client_id(
    client_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut settings = state.settings.write().await;
        settings.github_client_id = Some(client_id);
    }
    state.persist().await;
    Ok(())
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct DeviceCodeInfo {
    #[serde(rename = "userCode")]
    pub user_code: String,
    #[serde(rename = "verificationUri")]
    pub verification_uri: String,
    #[serde(rename = "expiresIn")]
    pub expires_in: u64,
}

#[tauri::command]
pub async fn start_github_oauth(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DeviceCodeInfo, String> {
    let client_id = {
        let settings = state.settings.read().await;
        get_client_id(&*settings).ok_or("GitHub OAuth App 未配置，请先完成初始设置")?
    };

    let client = reqwest::Client::new();

    // Step 1: Request device code
    let resp = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": client_id,
            "scope": "repo,read:user,user:email"
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if let Some(err) = body["error"].as_str() {
        return Err(format!("GitHub error: {} — {}", err,
            body["error_description"].as_str().unwrap_or("")));
    }

    let device_code = body["device_code"].as_str()
        .ok_or("No device_code in response")?.to_string();
    let user_code = body["user_code"].as_str()
        .ok_or("No user_code in response")?.to_string();
    let verification_uri = body["verification_uri"].as_str()
        .unwrap_or("https://github.com/login/device").to_string();
    let interval = body["interval"].as_u64().unwrap_or(5);
    let expires_in = body["expires_in"].as_u64().unwrap_or(900);

    // Open browser to verification page
    let opener = tauri_plugin_opener::OpenerExt::opener(&app);
    let _ = opener.open_url(&verification_uri, None::<&str>);

    // Step 2: Poll for token in background
    let token_store = state.github_token.clone();
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        poll_for_token(device_code, client_id, interval, expires_in, token_store, app_clone).await;
    });

    Ok(DeviceCodeInfo { user_code, verification_uri, expires_in })
}

async fn poll_for_token(
    device_code: String,
    client_id: String,
    interval_secs: u64,
    expires_in: u64,
    token_store: std::sync::Arc<tokio::sync::RwLock<Option<String>>>,
    app: AppHandle,
) {
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now()
        + tokio::time::Duration::from_secs(expires_in);

    loop {
        if tokio::time::Instant::now() > deadline {
            let _ = app.emit("oauth-error", "授权超时，请重试");
            return;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;

        let resp = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .json(&serde_json::json!({
                "client_id": client_id,
                "device_code": device_code,
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
            }))
            .send()
            .await;

        let Ok(resp) = resp else { continue };
        let Ok(body) = resp.json::<serde_json::Value>().await else { continue };

        match body["error"].as_str() {
            Some("authorization_pending") => continue, // user hasn't approved yet
            Some("slow_down") => {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
            Some("expired_token") => {
                let _ = app.emit("oauth-error", "授权码已过期，请重新登录");
                return;
            }
            Some("access_denied") => {
                let _ = app.emit("oauth-error", "用户拒绝了授权");
                return;
            }
            Some(other) => {
                let _ = app.emit("oauth-error", format!("授权失败: {}", other));
                return;
            }
            None => {} // no error field — check for token
        }

        if let Some(token) = body["access_token"].as_str() {
            let mut t = token_store.write().await;
            *t = Some(token.to_string());
            drop(t);

            // Persist and notify frontend
            {
                use tauri::Manager;
                if let Some(state) = app.try_state::<AppState>() {
                    state.persist().await;
                }
            }
            let _ = app.emit("oauth-success", ());
            return;
        }
    }
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
    drop(settings);
    state.persist().await;
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
