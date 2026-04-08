use tauri::{AppHandle, Manager};
use serde::{Deserialize, Serialize};

pub mod commands;
pub mod services;
pub mod models;

use commands::{
    github::{start_github_oauth, get_github_token, clear_github_token, get_github_user, handle_oauth_callback},
    repo::{list_github_repos, list_indexed_repos, add_repo, remove_repo, get_repo},
    index::{start_index, cancel_index, get_index_status},
    search::{search, get_context, get_impact},
    settings::{get_settings, update_settings},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            services::state::AppState::init(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // GitHub auth
            start_github_oauth,
            get_github_token,
            clear_github_token,
            get_github_user,
            handle_oauth_callback,
            // Repo management
            list_github_repos,
            list_indexed_repos,
            add_repo,
            remove_repo,
            get_repo,
            // Indexing
            start_index,
            cancel_index,
            get_index_status,
            // Search & Query
            search,
            get_context,
            get_impact,
            // Settings
            get_settings,
            update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
