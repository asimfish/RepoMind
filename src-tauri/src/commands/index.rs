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

    let git_dir = format!("{}/.git", local_path);
    if std::path::Path::new(&git_dir).exists() {
        emit("pull", 5, "更新仓库...");
        let mut cmd = AsyncCommand::new("git");
        cmd.args(["-C", &local_path, "pull", "--ff-only"]);
        if let Some(t) = &token {
            // Pass token via GIT_ASKPASS env — never in URL or process args
            cmd.env("GIT_ASKPASS", "echo").env("GIT_PASSWORD", t);
        }
        let status = cmd.status().await.map_err(|e| e.to_string())?;
        if !status.success() {
            emit("pull", 10, "已是最新，继续索引...");
        }
    } else {
        emit("clone", 5, "克隆仓库...");
        let mut cmd = AsyncCommand::new("git");
        cmd.args(["clone", "--depth", "1", &clone_url, &local_path]);
        if let Some(t) = &token {
            // Use credential helper via env — avoids token in process args
            cmd.env("GIT_TERMINAL_PROMPT", "0");
            // Inject via insteadOf + header approach (safest)
            let auth_header = format!("Authorization: Bearer {}", t);
            cmd.args([
                "--config",
                &format!("http.extraHeader={}", auth_header),
            ]);
        }
        let status = cmd.status().await.map_err(|e| e.to_string())?;
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

    emit("done", 95, "图谱索引完成，开始向量化...");

    // ── Step 3: Build vector index (if Ollama available) ────────────────────
    build_vector_index(&repo_id, &local_path, &app, emit).await;

    emit("done", 100, "索引完成 ✓");

    // ── Step 4: Start file watcher for incremental updates ──────────────────
    start_file_watcher(repo_id.clone(), local_path.clone(), app.clone());

    Ok(())
}

async fn build_vector_index<F>(repo_id: &str, local_path: &str, app: &AppHandle, emit: F)
where
    F: Fn(&str, u8, &str),
{
    use crate::services::vector::{VectorStore, VectorEntry, get_embedding, is_ollama_available, has_embed_model};

    if !is_ollama_available().await {
        emit("vector", 95, "Ollama 未运行，跳过向量索引");
        return;
    }
    if !has_embed_model().await {
        emit("vector", 95, "nomic-embed-text 未安装（运行 ollama pull nomic-embed-text）");
        return;
    }

    emit("vector", 95, "构建语义向量索引...");

    // Get data dir for this repo
    let data_dir = if let Some(app_data) = dirs::data_dir() {
        app_data.join("com.liyufeng.repomind").join("vectors").join(repo_id)
    } else {
        return;
    };
    let _ = std::fs::create_dir_all(&data_dir);

    let store = VectorStore::new(&data_dir);
    let _ = store.clear();

    // Use gitnexus to get all symbols with snippets
    let gitnexus = find_gitnexus_bin();
    let output = match tokio::process::Command::new(&gitnexus)
        .args(["cypher",
               "MATCH (n) WHERE n.type IN ['function','class','method'] AND n.snippet IS NOT NULL RETURN n.id, n.name, n.file, n.line, n.snippet, n.type LIMIT 500",
               "--json"])
        .current_dir(local_path)
        .output()
        .await
    {
        Ok(o) => o,
        Err(_) => return,
    };

    let symbols: Vec<serde_json::Value> = serde_json::from_str(
        &String::from_utf8_lossy(&output.stdout)
    ).unwrap_or_default();

    let total = symbols.len().max(1);
    for (i, sym) in symbols.into_iter().enumerate() {
        let id = sym["n.id"].as_str().unwrap_or("").to_string();
        let name = sym["n.name"].as_str().unwrap_or("").to_string();
        let file = sym["n.file"].as_str().unwrap_or("").to_string();
        let line = sym["n.line"].as_u64().unwrap_or(0) as u32;
        let snippet = sym["n.snippet"].as_str().unwrap_or("").to_string();
        let sym_type = sym["n.type"].as_str().unwrap_or("function").to_string();

        if snippet.is_empty() || id.is_empty() { continue; }

        // Embed: combine name + snippet for better retrieval
        let text = format!("{}\n{}", name, &snippet[..snippet.len().min(512)]);
        match get_embedding(&text).await {
            Ok(embedding) => {
                let entry = VectorEntry { id, symbol: name, file, line, snippet, symbol_type: sym_type, embedding };
                let _ = store.insert(&entry);
            }
            Err(_) => continue,
        }

        if i % 10 == 0 {
            let pct = 95 + (i * 4 / total) as u8;
            emit("vector", pct.min(99), &format!("向量化 {}/{}", i + 1, total));
        }
    }
}


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
    let home = dirs::home_dir().unwrap_or_default();

    // Build a comprehensive list of candidates, covering:
    // - NVM versions (Tauri apps don't inherit shell env, so NVM_BIN is often empty)
    // - PNPM global
    // - System locations
    let mut candidates: Vec<String> = vec![];

    // 1. Env vars (work when launched from terminal or if set in launchd)
    for var in &["NVM_BIN", "PNPM_HOME"] {
        if let Ok(p) = std::env::var(var) {
            candidates.push(format!("{}/gitnexus", p));
        }
    }

    // 2. Scan all NVM versions (covers app launched from Dock/Spotlight)
    let nvm_versions = home.join(".nvm/versions/node");
    if nvm_versions.exists() {
        if let Ok(entries) = std::fs::read_dir(&nvm_versions) {
            let mut versions: Vec<_> = entries
                .flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .collect();
            // Sort descending — latest first
            versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
            for v in versions {
                let bin = v.path().join("bin/gitnexus");
                candidates.push(bin.to_string_lossy().to_string());
            }
        }
    }

    // 3. PNPM global store
    candidates.push(home.join("Library/pnpm/gitnexus").to_string_lossy().to_string());

    // 4. System-wide
    candidates.extend([
        "/usr/local/bin/gitnexus".to_string(),
        "/opt/homebrew/bin/gitnexus".to_string(),
    ]);

    // First existing path wins
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return c.clone();
        }
    }

    // 5. Try `which` (works if Tauri inherits PATH from login shell)
    if let Ok(out) = std::process::Command::new("which").arg("gitnexus").output() {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() && std::path::Path::new(&path).exists() {
            return path;
        }
    }

    "gitnexus".to_string()
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
