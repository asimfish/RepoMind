use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillPlatform {
    Cursor,
    Claude,
    Codex,
}

impl SkillPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillPlatform::Cursor => "cursor",
            SkillPlatform::Claude => "claude",
            SkillPlatform::Codex => "codex",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "claude" => SkillPlatform::Claude,
            "codex" => SkillPlatform::Codex,
            _ => SkillPlatform::Cursor,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_path: String,
    pub source_platform: SkillPlatform,
    pub author: Option<String>,
    pub version: Option<String>,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub trigger_patterns: Vec<String>,
    pub depends_on: Vec<String>,
    pub content_hash: String,
    pub parsed_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub raw_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInvokeCount {
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillStats {
    pub total_skills: u32,
    pub total_chains: u32,
    pub total_workflows: u32,
    pub by_platform: HashMap<String, u32>,
    pub by_category: HashMap<String, u32>,
    pub top_invoked: Vec<SkillInvokeCount>,
    pub recent_workflows: Vec<WorkflowTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillScanResult {
    pub total_scanned: u32,
    pub new_skills: u32,
    pub updated_skills: u32,
    pub by_platform: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillGraphData {
    pub nodes: Vec<SkillGraphNode>,
    pub edges: Vec<SkillGraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillGraphNode {
    pub id: String,
    pub label: String,
    pub platform: String,
    pub category: Option<String>,
    pub invoke_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillGraphEdge {
    pub source: String,
    pub target: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInvocation {
    pub id: String,
    pub session_id: String,
    pub skill_name: String,
    pub skill_id: Option<String>,
    pub invoked_at: String,
    pub sequence_index: i32,
    pub context_snippet: Option<String>,
    pub trigger_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationChain {
    pub session_id: String,
    pub skill_sequence: Vec<String>,
    pub started_at: String,
    pub task_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStep {
    pub order: u32,
    pub skill_name: String,
    pub skill_id: Option<String>,
    pub is_optional: bool,
    pub avg_position: f32,
    pub co_occurrence_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WorkflowStatus {
    Discovered,
    Confirmed,
    Exported,
    Dismissed,
}

impl WorkflowStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkflowStatus::Discovered => "discovered",
            WorkflowStatus::Confirmed => "confirmed",
            WorkflowStatus::Exported => "exported",
            WorkflowStatus::Dismissed => "dismissed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "confirmed" => WorkflowStatus::Confirmed,
            "exported" => WorkflowStatus::Exported,
            "dismissed" => WorkflowStatus::Dismissed,
            _ => WorkflowStatus::Discovered,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub total_count: u32,
    pub last_7_days: u32,
    pub last_30_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub frequency: u32,
    pub confidence: f32,
    pub source_sessions: Vec<String>,
    pub category: Option<String>,
    pub created_at: String,
    pub status: WorkflowStatus,
}
