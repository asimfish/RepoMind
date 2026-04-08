use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::{AppHandle, Manager};
use crate::models::{AppSettings, IndexStatus, Repo};

pub struct AppState {
    pub settings: Arc<RwLock<AppSettings>>,
    pub indexed_repos: Arc<RwLock<HashMap<String, Repo>>>,
    pub github_token: Arc<RwLock<Option<String>>>,
}

impl AppState {
    pub fn init(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        let data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;

        let settings = AppSettings {
            github_token: None,
            index_storage_path: data_dir.join("repos").to_string_lossy().to_string(),
            claude_api_key: None,
            mcp_enabled: true,
            auto_index_on_commit: true,
            search_language: "zh".to_string(),
        };

        let state = AppState {
            settings: Arc::new(RwLock::new(settings)),
            indexed_repos: Arc::new(RwLock::new(HashMap::new())),
            github_token: Arc::new(RwLock::new(None)),
        };

        app.manage(state);
        Ok(())
    }
}
