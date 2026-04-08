use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::{AppHandle, Manager};
use serde::{Deserialize, Serialize};
use crate::models::{AppSettings, Repo};

#[derive(Serialize, Deserialize, Default)]
struct PersistedState {
    github_token: Option<String>,
    settings: Option<PersistedSettings>,
    indexed_repos: Option<Vec<Repo>>,
}

#[derive(Serialize, Deserialize, Default)]
struct PersistedSettings {
    github_client_id: Option<String>,
    claude_api_key: Option<String>,
    mcp_enabled: bool,
    auto_index_on_commit: bool,
    search_language: String,
}

pub struct AppState {
    pub settings: Arc<RwLock<AppSettings>>,
    pub indexed_repos: Arc<RwLock<HashMap<String, Repo>>>,
    pub github_token: Arc<RwLock<Option<String>>>,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn init(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        let data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(data_dir.join("repos"))?;

        // Load persisted state
        let persisted = load_persisted_state(&data_dir);
        let ps = persisted.settings.unwrap_or_default();

        let settings = AppSettings {
            github_token: persisted.github_token.clone(),
            github_client_id: ps.github_client_id,
            index_storage_path: data_dir.join("repos").to_string_lossy().to_string(),
            claude_api_key: ps.claude_api_key,
            mcp_enabled: ps.mcp_enabled,
            auto_index_on_commit: ps.auto_index_on_commit,
            search_language: if ps.search_language.is_empty() { "zh".to_string() } else { ps.search_language },
        };

        let repos: HashMap<String, Repo> = persisted.indexed_repos
            .unwrap_or_default()
            .into_iter()
            .map(|r| (r.id.clone(), r))
            .collect();

        let state = AppState {
            github_token: Arc::new(RwLock::new(persisted.github_token)),
            settings: Arc::new(RwLock::new(settings)),
            indexed_repos: Arc::new(RwLock::new(repos)),
            data_dir,
        };

        app.manage(state);
        Ok(())
    }

    pub async fn persist(&self) {
        let token = self.github_token.read().await.clone();
        let settings = self.settings.read().await;
        let repos: Vec<Repo> = self.indexed_repos.read().await.values().cloned().collect();

        let persisted = PersistedState {
            github_token: token,
            settings: Some(PersistedSettings {
                github_client_id: settings.github_client_id.clone(),
                claude_api_key: settings.claude_api_key.clone(),
                mcp_enabled: settings.mcp_enabled,
                auto_index_on_commit: settings.auto_index_on_commit,
                search_language: settings.search_language.clone(),
            }),
            indexed_repos: Some(repos),
        };
        drop(settings);

        let path = self.data_dir.join("state.json");
        if let Ok(json) = serde_json::to_string_pretty(&persisted) {
            // Atomic write: write to temp file then rename to avoid corruption
            let tmp_path = self.data_dir.join("state.json.tmp");
            if std::fs::write(&tmp_path, &json).is_ok() {
                let _ = std::fs::rename(&tmp_path, &path);
            }
        }
    }
}

fn load_persisted_state(data_dir: &PathBuf) -> PersistedState {
    let path = data_dir.join("state.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
