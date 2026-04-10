pub mod agent;
pub mod skill;
pub mod rules;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stars: u64,
    pub is_private: bool,
    pub clone_url: String,
    pub html_url: String,
    pub updated_at: String,
    pub local_path: Option<String>,
    pub index_status: IndexStatus,
    pub last_indexed_at: Option<String>,
    pub last_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IndexStatus {
    NotIndexed,
    Indexing,
    Indexed,
    Stale,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub name: Option<String>,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexProgress {
    #[serde(rename = "repoId")]
    pub repo_id: String,
    pub phase: String,
    pub percent: u8,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub symbol: String,
    pub file: String,
    pub line: u32,
    pub snippet: String,
    #[serde(rename = "type")]
    pub result_type: String,
    pub score: f32,
    #[serde(rename = "repoName")]
    pub repo_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub community: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactResult {
    pub symbol: String,
    #[serde(rename = "directlyAffected")]
    pub directly_affected: Vec<ImpactNode>,
    #[serde(rename = "indirectlyAffected")]
    pub indirectly_affected: Vec<ImpactNode>,
    pub processes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactNode {
    pub symbol: String,
    pub file: String,
    pub confidence: f32,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub github_token: Option<String>,
    pub github_client_id: Option<String>,  // stored after first-run setup
    pub index_storage_path: String,
    pub claude_api_key: Option<String>,
    pub mcp_enabled: bool,
    pub auto_index_on_commit: bool,
    pub search_language: String,
}
