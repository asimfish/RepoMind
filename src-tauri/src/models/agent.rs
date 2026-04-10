use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInvocation {
    pub id: String,
    pub session_id: String,
    pub timestamp: i64,
    pub tool_name: String,
    pub arguments_json: String,
    pub duration_ms: Option<i64>,
    pub repo_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSequence {
    pub id: String,
    pub session_id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub intent_label: Option<String>,
    pub tool_chain: Vec<String>,
    pub repo_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub tool_affinity: HashMap<String, f64>,
    pub total_invocations: u64,
    pub total_sessions: u64,
    pub avg_tools_per_session: f64,
    pub most_active_hour: u8,
    pub top_patterns: Vec<PatternSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSummary {
    pub pattern: Vec<String>,
    pub count: u32,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSuggestion {
    pub tool: String,
    pub arguments: serde_json::Value,
    pub confidence: f64,
    pub reason: String,
}
