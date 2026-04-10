use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorRule {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category: String, // coding/workflow/safety/tool_usage/architecture/testing/custom
    pub status: String,   // candidate/approved/rejected/archived
    pub confidence: f32,
    pub source_type: String, // agents_md/claude_md/cursor_rule/skill_file/user_created
    pub source_file: Option<String>,
    pub source_excerpt: Option<String>,
    pub tags: Vec<String>,
    pub scope: String, // global/project/language/tool
    pub priority: u8,  // 1-5
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConflict {
    pub id: String,
    pub rule_a_id: String,
    pub rule_b_id: String,
    pub conflict_type: String, // contradiction/overlap/superseded
    pub description: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionBatch {
    pub id: String,
    pub source_files: Vec<String>,
    pub extracted_at: String,
    pub total_candidates: u32,
    pub approved: u32,
    pub rejected: u32,
    pub pending: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleStats {
    pub total: u32,
    pub approved: u32,
    pub candidate: u32,
    pub rejected: u32,
    pub by_category: HashMap<String, u32>,
}
