use tauri::{AppHandle, Manager};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use tauri::menu::{MenuBuilder, MenuItemBuilder};

pub mod commands;
pub mod services;
pub mod models;

use commands::{
    github::{start_github_oauth, get_github_token, clear_github_token, get_github_user,
             is_oauth_configured, set_github_client_id},
    repo::{list_github_repos, list_indexed_repos, add_repo, add_repo_by_url, remove_repo, get_repo},
    index::{start_index, cancel_index, get_index_status},
    search::{search, get_context, get_impact, get_graph, get_ai_summary, validate_claude_key, get_mcp_status},
    settings::{get_settings, update_settings},
    skill::{
        scan_skills, list_skills, get_skill, get_skill_stats, collect_invocations, mine_workflows,
        list_workflows, update_workflow_status, get_skill_graph, export_workflow,
        record_skill_usage, get_recommendations,
    },
    rules::{
        scan_rule_sources, list_rules, approve_rule, reject_rule, create_rule, get_rule_stats,
    },
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Inherit login shell PATH on macOS so NVM/PNPM tools are discoverable
            // (Tauri apps launched from Dock don't get the user's shell environment)
            #[cfg(target_os = "macos")]
            inject_shell_path();

            services::state::AppState::init(app.handle())?;
            setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_github_oauth,
            get_github_token,
            clear_github_token,
            get_github_user,
            is_oauth_configured,
            set_github_client_id,
            list_github_repos,
            list_indexed_repos,
            add_repo,
            add_repo_by_url,
            remove_repo,
            get_repo,
            start_index,
            cancel_index,
            get_index_status,
            search,
            get_context,
            get_impact,
            get_graph,
            get_ai_summary,
            validate_claude_key,
            get_mcp_status,
            get_settings,
            update_settings,
            scan_skills,
            list_skills,
            get_skill,
            get_skill_stats,
            collect_invocations,
            mine_workflows,
            list_workflows,
            update_workflow_status,
            get_skill_graph,
            export_workflow,
            record_skill_usage,
            get_recommendations,
            scan_rule_sources,
            list_rules,
            approve_rule,
            reject_rule,
            create_rule,
            get_rule_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItemBuilder::with_id("show", "打开 RepoMind").build(app)?;
    let search = MenuItemBuilder::with_id("search", "快速搜索...").build(app)?;
    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show)
        .item(&search)
        .item(&separator)
        .item(&quit)
        .build()?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("RepoMind — 代码知识管家")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "search" => show_search_spotlight(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_search_spotlight(app: &AppHandle) {
    use tauri::Emitter;
    let _ = app.emit("open-spotlight", ());
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// On macOS, apps launched from Dock/Spotlight don't inherit the user's shell PATH.
/// This reads PATH from the login shell so NVM, PNPM, Homebrew tools are findable.
#[cfg(target_os = "macos")]
fn inject_shell_path() {
    use std::process::Command;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let out = Command::new(&shell)
        .args(["-l", "-c", "echo $PATH"])
        .output();

    if let Ok(out) = out {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() {
            // Prepend login-shell PATH to current PATH
            let current = std::env::var("PATH").unwrap_or_default();
            let merged = if current.is_empty() {
                path
            } else {
                format!("{}:{}", path, current)
            };
            std::env::set_var("PATH", merged);
        }
    }
}
