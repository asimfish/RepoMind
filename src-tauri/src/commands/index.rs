use tauri::{AppHandle, Emitter, Manager, State};
use tokio::process::Command as AsyncCommand;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::models::{IndexProgress, IndexStatus};
use crate::services::state::AppState;

#[tauri::command]
pub async fn start_index(
    repo_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut repos = state.indexed_repos.write().await;
        if let Some(repo) = repos.get_mut(&repo_id) {
            repo.index_status = IndexStatus::Indexing;
        }
    }

    let repo_clone_url;
    let repo_local_path;
    {
        let repos = state.indexed_repos.read().await;
        let repo = repos.get(&repo_id).ok_or("Repo not found")?;
        repo_clone_url = repo.clone_url.clone();
        repo_local_path = repo.local_path.clone().ok_or("No local path set")?;
    }

    let token = state.github_token.read().await.clone();
    let app_clone = app.clone();
    let repo_id_clone = repo_id.clone();

    tauri::async_runtime::spawn(async move {
        match run_index(repo_id_clone.clone(), repo_clone_url, repo_local_path, token, app_clone.clone()).await {
            Ok(_) => {
                // Update status to indexed and persist
                if let Ok(state) = app_clone.try_state::<AppState>().ok_or("") {
                    {
                        let mut repos = state.indexed_repos.write().await;
                        if let Some(repo) = repos.get_mut(&repo_id_clone) {
                            repo.index_status = IndexStatus::Indexed;
                            repo.last_indexed_at = Some(chrono_now());
                        }
                    }
                    state.persist().await;
                }
            }
            Err(e) => {
                eprintln!("Index failed for {}: {}", repo_id_clone, e);
                if let Ok(state) = app_clone.try_state::<AppState>().ok_or("") {
                    let mut repos = state.indexed_repos.write().await;
                    if let Some(repo) = repos.get_mut(&repo_id_clone) {
                        repo.index_status = IndexStatus::Error;
                    }
                }
            }
        }
    });

    Ok(())
}

async fn run_index(
    repo_id: String,
    clone_url: String,
    local_path: String,
    token: Option<String>,
    app: AppHandle,
) -> Result<(), String> {
    let emit = |phase: &str, percent: u8, message: &str| {
        let _ = app.emit("index-progress", IndexProgress {
            repo_id: repo_id.clone(),
            phase: phase.to_string(),
            percent,
            message: message.to_string(),
        });
    };

    // ── Step 1: Clone or pull ────────────────────────────────────────────────
    std::fs::create_dir_all(&local_path).map_err(|e| e.to_string())?;

    let auth_url = token.as_deref()
        .map(|t| clone_url.replace("https://", &format!("https://oauth2:{}@", t)))
        .unwrap_or_else(|| clone_url.clone());

    let git_dir = format!("{}/.git", local_path);
    if std::path::Path::new(&git_dir).exists() {
        emit("pull", 5, "更新仓库...");
        let status = AsyncCommand::new("git")
            .args(["-C", &local_path, "pull", "--ff-only"])
            .status()
            .await
            .map_err(|e| e.to_string())?;
        if !status.success() {
            // Not fatal — repo might have diverged; proceed with existing code
            emit("pull", 10, "已是最新，继续索引...");
        }
    } else {
        emit("clone", 5, "克隆仓库...");
        let status = AsyncCommand::new("git")
            .args(["clone", "--depth", "1", &auth_url, &local_path])
            .status()
            .await
            .map_err(|e| e.to_string())?;
        if !status.success() {
            emit("error", 0, "克隆失败");
            return Err("git clone failed".to_string());
        }
    }

    emit("analyze", 15, "开始知识图谱分析...");

    // ── Step 2: Run `gitnexus analyze` with stdout streaming ────────────────
    // Resolve gitnexus path (installed globally via npm/pnpm)
    let gitnexus_bin = find_gitnexus_bin();

    let mut child = AsyncCommand::new(&gitnexus_bin)
        .args(["analyze", "--no-color"])
        .current_dir(&local_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start gitnexus: {}", e))?;

    // Parse stdout for progress lines
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        let (phase, pct, msg) = parse_gitnexus_line(&line);
        emit(&phase, pct, &msg);
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if !status.success() {
        emit("error", 0, "索引失败");
        return Err("gitnexus analyze failed".to_string());
    }

    emit("done", 100, "索引完成 ✓");

    // ── Step 3: Start file watcher for incremental updates ──────────────────
    start_file_watcher(repo_id.clone(), local_path.clone(), app.clone());

    Ok(())
}

/// Parse gitnexus analyze stdout lines into (phase, percent, message)
fn parse_gitnexus_line(line: &str) -> (String, u8, String) {
    let line = line.trim();
    // GitNexus outputs lines like:
    //   [1/6] Scanning repository...
    //   [2/6] Building structure...
    //   [3/6] Parsing files... (234/1042)
    //   [4/6] Resolving symbols...
    //   [5/6] Detecting communities...
    //   [6/6] Tracing processes...
    if let Some(rest) = line.strip_prefix('[') {
        if let Some(slash_pos) = rest.find('/') {
            let step: u8 = rest[..slash_pos].parse().unwrap_or(1);
            let total_end = rest.find(']').unwrap_or(slash_pos + 2);
            let total: u8 = rest[slash_pos + 1..total_end].parse().unwrap_or(6);
            let msg_start = rest.find("] ").map(|p| p + 2).unwrap_or(total_end + 1);
            let msg = if msg_start < rest.len() { &rest[msg_start..] } else { line };
            let pct = ((step as u16 * 85 / total as u16) + 15).min(99) as u8;
            let phase = format!("step{}", step);
            return (phase, pct, msg.to_string());
        }
    }
    // Fallback
    ("analyze".to_string(), 50, line.to_string())
}

pub fn find_gitnexus_bin_pub() -> String {
    find_gitnexus_bin()
}

fn find_gitnexus_bin() -> String {
    // Check common install locations
    for candidate in &[
        "/Users/liyufeng/.nvm/versions/node/v24.14.0/bin/gitnexus",
        "/usr/local/bin/gitnexus",
        "/opt/homebrew/bin/gitnexus",
    ] {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }
    // Last resort: use npx
    "npx gitnexus".to_string()
}

fn start_file_watcher(repo_id: String, local_path: String, app: AppHandle) {
    std::thread::spawn(move || {
        use notify::{Watcher, RecursiveMode, Config};
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel();
        let mut watcher = match notify::RecommendedWatcher::new(tx, Config::default()) {
            Ok(w) => w,
            Err(e) => { eprintln!("Watcher error: {}", e); return; }
        };

        // Watch the repo directory, ignoring .git and .gitnexus
        if watcher.watch(std::path::Path::new(&local_path), RecursiveMode::Recursive).is_err() {
            return;
        }

        let mut last_trigger = std::time::Instant::now();
        let debounce = std::time::Duration::from_secs(5);

        for event in rx {
            if let Ok(e) = event {
                // Ignore .git and .gitnexus directory changes
                let relevant = e.paths.iter().any(|p| {
                    let s = p.to_string_lossy();
                    !s.contains("/.git/") && !s.contains("/.gitnexus/")
                });
                if relevant && last_trigger.elapsed() > debounce {
                    last_trigger = std::time::Instant::now();
                    let _ = app.emit("repo-changed", repo_id.clone());
                }
            }
        }
    });
}

fn chrono_now() -> String {
    // Simple ISO timestamp without chrono dependency
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", d.as_secs())
}

#[tauri::command]
pub async fn cancel_index(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut repos = state.indexed_repos.write().await;
    if let Some(repo) = repos.get_mut(&repo_id) {
        if repo.index_status == IndexStatus::Indexing {
            repo.index_status = IndexStatus::NotIndexed;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_index_status(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<IndexStatus, String> {
    let repos = state.indexed_repos.read().await;
    repos
        .get(&repo_id)
        .map(|r| r.index_status.clone())
        .ok_or_else(|| "Repo not found".to_string())
}
