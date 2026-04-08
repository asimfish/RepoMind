use tauri::State;
use crate::models::AppSettings;
use crate::services::state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
pub async fn update_settings(
    settings: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut current = state.settings.write().await;

    if let Some(v) = settings.get("claudeApiKey").and_then(|v| v.as_str()) {
        current.claude_api_key = Some(v.to_string());
    }
    if let Some(v) = settings.get("mcpEnabled").and_then(|v| v.as_bool()) {
        current.mcp_enabled = v;
    }
    if let Some(v) = settings.get("autoIndexOnCommit").and_then(|v| v.as_bool()) {
        current.auto_index_on_commit = v;
    }
    if let Some(v) = settings.get("searchLanguage").and_then(|v| v.as_str()) {
        current.search_language = v.to_string();
    }

    Ok(())
}
