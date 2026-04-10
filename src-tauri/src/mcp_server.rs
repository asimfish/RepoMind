/// RepoMind MCP Server
/// Implements the Model Context Protocol (MCP) over stdio
/// Compatible with Claude ​Code, Cursor, Codex, and other MCP clients
///
/// Usage: repomind-mcp
/// Configure in ~/.claude/mcp.json or ~/.cursor/mcp.json

use std::io::{BufRead, Write};
use std::path::PathBuf;

use once_cell::sync::Lazy;
use repomindapp_lib::models::agent::McpInvocation;
use repomindapp_lib::models::skill::Skill;
use repomindapp_lib::services::behavior_store::BehaviorStore;
use repomindapp_lib::services::skill_store::SkillStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

static SESSION_ID: Lazy<String> = Lazy::new(|| uuid::Uuid::new_v4().to_string());

#[derive(Debug, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
}

impl McpResponse {
    fn ok(id: Option<Value>, result: Value) -> Self {
        McpResponse { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    fn err(id: Option<Value>, code: i32, message: &str) -> Self {
        McpResponse { jsonrpc: "2.0".into(), id, result: None, error: Some(McpError { code, message: message.into() }) }
    }
}

fn send(resp: &McpResponse) {
    let s = serde_json::to_string(resp).unwrap_or_default();
    println!("{}", s);
    std::io::stdout().flush().ok();
}

fn gitnexus_bin() -> String {
    let home = dirs::home_dir().unwrap_or_default();
    let mut candidates: Vec<String> = vec![];

    for var in &["NVM_BIN", "PNPM_HOME"] {
        if let Ok(p) = std::env::var(var) {
            candidates.push(format!("{}/gitnexus", p));
        }
    }

    let nvm_versions = home.join(".nvm/versions/node");
    if nvm_versions.exists() {
        if let Ok(entries) = std::fs::read_dir(&nvm_versions) {
            let mut versions: Vec<_> = entries.flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .collect();
            versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
            for v in versions {
                candidates.push(v.path().join("bin/gitnexus").to_string_lossy().to_string());
            }
        }
    }

    candidates.push(home.join("Library/pnpm/gitnexus").to_string_lossy().to_string());
    candidates.extend(["/usr/local/bin/gitnexus".to_string(), "/opt/homebrew/bin/gitnexus".to_string()]);

    for c in &candidates {
        if std::path::Path::new(c).exists() { return c.clone(); }
    }

    if let Ok(out) = std::process::Command::new("which").arg("gitnexus").output() {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() && std::path::Path::new(&path).exists() { return path; }
    }

    "gitnexus".to_string()
}

fn state_file() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("com.liyufeng.repomind")
        .join("state.json")
}

fn load_repos() -> Vec<Value> {
    let path = state_file();
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let state: Value = serde_json::from_str(&content).unwrap_or_default();
    state["indexed_repos"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r["index_status"].as_str() == Some("Indexed"))
        .collect()
}

fn find_repo(repos: &[Value], name: &str) -> Option<(String, String)> {
    repos.iter().find_map(|r| {
        let full_name = r["full_name"].as_str()?;
        let local_path = r["local_path"].as_str()?;
        if full_name.contains(name) || r["name"].as_str()? == name {
            Some((full_name.to_string(), local_path.to_string()))
        } else {
            None
        }
    })
}

fn run_gitnexus(args: &[&str], cwd: &str) -> Result<String, String> {
    let bin = gitnexus_bin();
    let out = std::process::Command::new(&bin)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// App data directory (parent of `state.json`), same layout as Tauri `AppState`.
fn app_data_dir() -> PathBuf {
    state_file()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn open_skill_store() -> Result<SkillStore, String> {
    SkillStore::open(&app_data_dir())
}

/// Tools whose `tools/call` is persisted for Meta-Agent behavior learning (`agent_profile` excluded).
const BEHAVIOR_RECORDED_TOOLS: &[&str] = &[
    "list_repos",
    "search",
    "context",
    "impact",
    "cypher",
    "list_skills",
    "search_skills",
    "get_workflow",
];

fn record_tool_invocation(tool_name: &str, params: &Value) {
    if !BEHAVIOR_RECORDED_TOOLS.contains(&tool_name) {
        return;
    }
    if let Ok(store) = BehaviorStore::open(&app_data_dir()) {
        let inv = McpInvocation {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: SESSION_ID.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            tool_name: tool_name.to_string(),
            arguments_json: params.to_string(),
            duration_ms: None,
            repo_id: None,
        };
        let _ = store.record_invocation(&inv);
    }
}

/// Omit `raw_content` in MCP responses to keep payloads small.
fn skills_json_for_mcp(skills: &[Skill]) -> Value {
    let items: Vec<Value> = skills
        .iter()
        .map(|s| {
            let mut v = serde_json::to_value(s).unwrap_or(json!({}));
            if let Some(obj) = v.as_object_mut() {
                obj.remove("raw_content");
            }
            v
        })
        .collect();
    json!(items)
}

fn handle(req: McpRequest) -> McpResponse {
    let id = req.id.clone();
    let params = req.params.unwrap_or(json!({}));

    match req.method.as_str() {
        // ── MCP lifecycle ─────────────────────────────────────────────────
        "initialize" => McpResponse::ok(id, json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "repomind", "version": "0.1.0" }
        })),

        "notifications/initialized" => McpResponse::ok(id, json!(null)),

        // ── Tool listing ──────────────────────────────────────────────────
        "tools/list" => McpResponse::ok(id, json!({
            "tools": [
                {
                    "name": "list_repos",
                    "description": "List all indexed code repositories in RepoMind",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "search",
                    "description": "Search for symbols (functions, classes, variables) in a repository knowledge graph. Returns ranked results with file/line info.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "repo": { "type": "string", "description": "Repository name or full_name (owner/repo)" },
                            "query": { "type": "string", "description": "Search query — function name, concept, or keyword" }
                        },
                        "required": ["repo", "query"]
                    }
                },
                {
                    "name": "context",
                    "description": "Get 360-degree context for a symbol: callers, callees, community membership, and execution flows it participates in.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "repo": { "type": "string", "description": "Repository name" },
                            "symbol": { "type": "string", "description": "Symbol name to inspect" }
                        },
                        "required": ["repo", "symbol"]
                    }
                },
                {
                    "name": "impact",
                    "description": "Blast radius analysis — what breaks if you change this symbol? Returns direct callers (WILL BREAK), indirect deps (LIKELY AFFECTED), and involved processes.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "repo": { "type": "string", "description": "Repository name" },
                            "symbol": { "type": "string", "description": "Symbol to analyze" }
                        },
                        "required": ["repo", "symbol"]
                    }
                },
                {
                    "name": "cypher",
                    "description": "Run a raw Cypher query against the repository knowledge graph (LadybugDB).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "repo": { "type": "string", "description": "Repository name" },
                            "query": { "type": "string", "description": "Cypher query string" }
                        },
                        "required": ["repo", "query"]
                    }
                },
                {
                    "name": "list_skills",
                    "description": "List all indexed skills across Cursor/Claude/Codex platforms. Filter by platform, category, or search query.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "platform": { "type": "string", "description": "Filter: cursor, claude, codex" },
                            "category": { "type": "string", "description": "Filter: research, coding, ml, etc." },
                            "query": { "type": "string", "description": "Search by name or description" }
                        }
                    }
                },
                {
                    "name": "search_skills",
                    "description": "Find skills by natural language query across all platforms",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Natural language query" }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "get_workflow",
                    "description": "Get a discovered workflow template - a chain of skills that are frequently used together",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "Workflow name or keyword" }
                        },
                        "required": ["name"]
                    }
                },
                {
                    "name": "agent_profile",
                    "description": "View Meta-Agent's learned user profile: tool usage stats, patterns, preferences",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "section": { "type": "string", "description": "all, affinity, or patterns" }
                        }
                    }
                }
            ]
        })),

        // ── Tool calls ────────────────────────────────────────────────────
        "tools/call" => {
            let tool = params["name"].as_str().unwrap_or("");
            let args = &params["arguments"];

            let response = match tool {
                "list_repos" => {
                    let repos = load_repos();
                    let list: Vec<Value> = repos.iter().map(|r| json!({
                        "name": r["name"],
                        "full_name": r["full_name"],
                        "language": r["language"],
                        "status": r["index_status"],
                        "last_indexed": r["last_indexed_at"]
                    })).collect();
                    McpResponse::ok(id, json!({
                        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&list).unwrap_or_default() }]
                    }))
                }

                "search" => {
                    let repo_name = args["repo"].as_str().unwrap_or("");
                    let query = args["query"].as_str().unwrap_or("");
                    let repos = load_repos();

                    match find_repo(&repos, repo_name) {
                        None => McpResponse::ok(id, json!({
                            "content": [{ "type": "text", "text": format!("Repository '{}' not found or not indexed. Run RepoMind and index it first.", repo_name) }],
                            "isError": true
                        })),
                        Some((_, path)) => {
                            let out = run_gitnexus(&["query", query], &path)
                                .unwrap_or_else(|e| format!("Error: {}", e));
                            McpResponse::ok(id, json!({
                                "content": [{ "type": "text", "text": out }]
                            }))
                        }
                    }
                }

                "context" => {
                    let repo_name = args["repo"].as_str().unwrap_or("");
                    let symbol = args["symbol"].as_str().unwrap_or("");
                    let repos = load_repos();

                    match find_repo(&repos, repo_name) {
                        None => McpResponse::ok(id, json!({
                            "content": [{ "type": "text", "text": format!("Repository '{}' not found.", repo_name) }],
                            "isError": true
                        })),
                        Some((_, path)) => {
                            let out = run_gitnexus(&["context", symbol], &path)
                                .unwrap_or_else(|e| format!("Error: {}", e));
                            McpResponse::ok(id, json!({
                                "content": [{ "type": "text", "text": out }]
                            }))
                        }
                    }
                }

                "impact" => {
                    let repo_name = args["repo"].as_str().unwrap_or("");
                    let symbol = args["symbol"].as_str().unwrap_or("");
                    let repos = load_repos();

                    match find_repo(&repos, repo_name) {
                        None => McpResponse::ok(id, json!({
                            "content": [{ "type": "text", "text": format!("Repository '{}' not found.", repo_name) }],
                            "isError": true
                        })),
                        Some((_, path)) => {
                            let out = run_gitnexus(&["impact", symbol], &path)
                                .unwrap_or_else(|e| format!("Error: {}", e));
                            McpResponse::ok(id, json!({
                                "content": [{ "type": "text", "text": out }]
                            }))
                        }
                    }
                }

                "cypher" => {
                    let repo_name = args["repo"].as_str().unwrap_or("");
                    let query = args["query"].as_str().unwrap_or("");
                    let repos = load_repos();

                    match find_repo(&repos, repo_name) {
                        None => McpResponse::ok(id, json!({
                            "content": [{ "type": "text", "text": format!("Repository '{}' not found.", repo_name) }],
                            "isError": true
                        })),
                        Some((_, path)) => {
                            let out = run_gitnexus(&["cypher", query], &path)
                                .unwrap_or_else(|e| format!("Error: {}", e));
                            McpResponse::ok(id, json!({
                                "content": [{ "type": "text", "text": out }]
                            }))
                        }
                    }
                }

                "list_skills" => match open_skill_store() {
                    Err(e) => McpResponse::ok(id, json!({
                        "content": [{ "type": "text", "text": format!("SkillStore error: {}", e) }],
                        "isError": true
                    })),
                    Ok(store) => {
                        let platform = args["platform"].as_str();
                        let category = args["category"].as_str();
                        let query = args["query"].as_str();
                        match store.list_skills(platform, category, query) {
                            Err(e) => McpResponse::ok(id, json!({
                                "content": [{ "type": "text", "text": format!("list_skills failed: {}", e) }],
                                "isError": true
                            })),
                            Ok(skills) => {
                                let text = serde_json::to_string_pretty(&skills_json_for_mcp(&skills))
                                    .unwrap_or_default();
                                McpResponse::ok(id, json!({
                                    "content": [{ "type": "text", "text": text }]
                                }))
                            }
                        }
                    }
                }

                "search_skills" => {
                    let query = args["query"].as_str().unwrap_or("");
                    if query.is_empty() {
                        McpResponse::ok(id, json!({
                            "content": [{ "type": "text", "text": "search_skills requires non-empty \"query\"." }],
                            "isError": true
                        }))
                    } else {
                        match open_skill_store() {
                            Err(e) => McpResponse::ok(id, json!({
                                "content": [{ "type": "text", "text": format!("SkillStore error: {}", e) }],
                                "isError": true
                            })),
                            Ok(store) => match store.search_skills(query) {
                                Err(e) => McpResponse::ok(id, json!({
                                    "content": [{ "type": "text", "text": format!("search_skills failed: {}", e) }],
                                    "isError": true
                                })),
                                Ok(skills) => {
                                    let text = serde_json::to_string_pretty(&skills_json_for_mcp(&skills))
                                        .unwrap_or_default();
                                    McpResponse::ok(id, json!({
                                        "content": [{ "type": "text", "text": text }]
                                    }))
                                }
                            },
                        }
                    }
                }

                "get_workflow" => {
                    let name = args["name"].as_str().unwrap_or("");
                    if name.is_empty() {
                        McpResponse::ok(id.clone(), json!({
                            "content": [{ "type": "text", "text": "get_workflow requires non-empty \"name\"." }],
                            "isError": true
                        }))
                    } else {
                        match open_skill_store() {
                            Err(e) => McpResponse::ok(id.clone(), json!({
                                "content": [{ "type": "text", "text": format!("SkillStore error: {}", e) }],
                                "isError": true
                            })),
                            Ok(store) => match store.find_workflows_by_name(name) {
                                Err(e) => McpResponse::ok(id.clone(), json!({
                                    "content": [{ "type": "text", "text": format!("get_workflow failed: {}", e) }],
                                    "isError": true
                                })),
                                Ok(wfs) => {
                                    let text = serde_json::to_string_pretty(&wfs).unwrap_or_default();
                                    McpResponse::ok(id.clone(), json!({
                                        "content": [{ "type": "text", "text": text }]
                                    }))
                                }
                            },
                        }
                    }
                }

                "agent_profile" => {
                    let section = args.get("section").and_then(|v| v.as_str()).unwrap_or("all");
                    match BehaviorStore::open(&app_data_dir()) {
                        Err(e) => McpResponse::ok(id.clone(), json!({
                            "content": [{ "type": "text", "text": format!("BehaviorStore error: {}", e) }],
                            "isError": true
                        })),
                        Ok(store) => match store.get_user_profile() {
                            Err(e) => McpResponse::ok(id.clone(), json!({
                                "content": [{ "type": "text", "text": format!("get_user_profile failed: {}", e) }],
                                "isError": true
                            })),
                            Ok(profile) => {
                                let body = match section {
                                    "affinity" => json!({ "tool_affinity": profile.tool_affinity }),
                                    "patterns" => json!({ "top_patterns": profile.top_patterns }),
                                    _ => serde_json::to_value(&profile).unwrap_or(json!({})),
                                };
                                let text = serde_json::to_string_pretty(&body).unwrap_or_default();
                                McpResponse::ok(id.clone(), json!({
                                    "content": [{ "type": "text", "text": text }]
                                }))
                            }
                        },
                    }
                }

                _ => McpResponse::err(id.clone(), -32601, &format!("Unknown tool: {}", tool))
            };

            record_tool_invocation(tool, &params);
            response
        }

        _ => McpResponse::err(id, -32601, &format!("Method not found: {}", req.method))
    }
}

fn main() {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };
        match serde_json::from_str::<McpRequest>(&line) {
            Ok(req) => send(&handle(req)),
            Err(e) => send(&McpResponse::err(None, -32700, &format!("Parse error: {}", e))),
        }
    }
}
