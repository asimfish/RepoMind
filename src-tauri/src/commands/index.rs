use tauri::{AppHandle, Emitter, State};
use crate::models::{IndexProgress, IndexStatus};
use crate::services::state::AppState;

#[tauri::command]
pub async fn start_index(
    repo_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Update status to indexing
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

    // Spawn background task
    tauri::async_runtime::spawn(async move {
        let _ = run_index(repo_id_clone, repo_clone_url, repo_local_path, token, app_clone).await;
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
    let emit_progress = |phase: &str, percent: u8, message: &str| {
        let _ = app.emit("index-progress", IndexProgress {
            repo_id: repo_id.clone(),
            phase: phase.to_string(),
            percent,
            message: message.to_string(),
        });
    };

    emit_progress("clone", 5, "正在克隆仓库...");

    // Clone or pull the repository
    std::fs::create_dir_all(&local_path).map_err(|e| e.to_string())?;

    let auth_url = if let Some(t) = &token {
        clone_url.replace("https://", &format!("https://oauth2:{}@", t))
    } else {
        clone_url.clone()
    };

    let git_dir = format!("{}/.git", local_path);
    let status = if std::path::Path::new(&git_dir).exists() {
        emit_progress("pull", 10, "更新仓库...");
        std::process::Command::new("git")
            .args(["-C", &local_path, "pull", "--ff-only"])
            .status()
    } else {
        std::process::Command::new("git")
            .args(["clone", "--depth", "1", &auth_url, &local_path])
            .status()
    };

    match status {
        Ok(s) if s.success() => {
            emit_progress("parse", 20, "解析代码结构...");
        }
        _ => {
            emit_progress("error", 0, "克隆失败");
            return Err("Git clone/pull failed".to_string());
        }
    }

    // TODO: Integrate actual GitNexus indexing engine here
    // For now, simulate indexing progress
    for (phase, pct, msg) in [
        ("scan", 30u8, "扫描文件..."),
        ("parse", 50, "解析 AST..."),
        ("resolve", 65, "解析符号依赖..."),
        ("community", 80, "构建知识社区..."),
        ("process", 90, "追踪执行流程..."),
        ("done", 100, "索引完成"),
    ] {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        emit_progress(phase, pct, msg);
    }

    Ok(())
}

#[tauri::command]
pub async fn cancel_index(
    repo_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut repos = state.indexed_repos.write().await;
    if let Some(repo) = repos.get_mut(&repo_id) {
        repo.index_status = IndexStatus::NotIndexed;
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
