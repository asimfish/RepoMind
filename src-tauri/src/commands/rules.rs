use std::path::PathBuf;

use tauri::State;
use uuid::Uuid;

use crate::models::rules::{BehaviorRule, ExtractionBatch, RuleStats};
use crate::services::rules_extractor::{collect_rule_files_under, infer_source_type, RulesExtractor};
use crate::services::rules_store::RulesStore;
use crate::services::state::AppState;

fn empty_as_none(s: Option<String>) -> Option<String> {
    s.filter(|x| !x.trim().is_empty())
}

#[tauri::command]
pub async fn scan_rule_sources(
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<ExtractionBatch, String> {
    let data_dir = state.data_dir.clone();
    let store = RulesStore::open(&data_dir)?;

    let mut source_files: Vec<String> = Vec::new();
    let mut to_scan: Vec<PathBuf> = Vec::new();

    for p in &paths {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            to_scan.push(pb);
        } else {
            collect_rule_files_under(&pb, &mut to_scan)?;
        }
    }

    to_scan.sort();
    to_scan.dedup();

    let mut upserted = 0u32;
    for path in &to_scan {
        let path_str = path.to_string_lossy().to_string();
        let st = infer_source_type(path);
        let candidates = match RulesExtractor::extract_from_file(&path_str, st) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("rules scan skip {:?}: {}", path, e);
                continue;
            }
        };
        source_files.push(path_str);
        for rule in candidates {
            store.upsert_rule(&rule)?;
            upserted += 1;
        }
    }

    source_files.sort();
    source_files.dedup();

    let stats = store.get_stats()?;
    let extracted_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    Ok(ExtractionBatch {
        id: Uuid::new_v4().to_string(),
        source_files,
        extracted_at,
        total_candidates: upserted,
        approved: stats.approved,
        rejected: stats.rejected,
        pending: stats.candidate,
    })
}

#[tauri::command]
pub async fn list_rules(
    status: Option<String>,
    category: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<BehaviorRule>, String> {
    let store = RulesStore::open(&state.data_dir)?;
    store.list_rules(
        empty_as_none(status).as_deref(),
        empty_as_none(category).as_deref(),
        page.unwrap_or(1),
        page_size.unwrap_or(50),
    )
}

#[tauri::command]
pub async fn approve_rule(rule_id: String, state: State<'_, AppState>) -> Result<BehaviorRule, String> {
    let store = RulesStore::open(&state.data_dir)?;
    store.update_rule_status(&rule_id, "approved")?;
    store
        .get_rule(&rule_id)?
        .ok_or_else(|| format!("rule not found: {}", rule_id))
}

#[tauri::command]
pub async fn reject_rule(rule_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let store = RulesStore::open(&state.data_dir)?;
    store.update_rule_status(&rule_id, "rejected")
}

#[tauri::command]
pub async fn create_rule(mut rule: BehaviorRule, state: State<'_, AppState>) -> Result<BehaviorRule, String> {
    if rule.id.trim().is_empty() {
        rule.id = Uuid::new_v4().to_string();
    }
    if rule.source_type.trim().is_empty() {
        rule.source_type = "user_created".to_string();
    }
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if rule.created_at.trim().is_empty() {
        rule.created_at = now.clone();
    }
    rule.updated_at = now;
    if rule.status.trim().is_empty() {
        rule.status = "candidate".to_string();
    }
    if rule.category.trim().is_empty() {
        rule.category = "custom".to_string();
    }
    if rule.scope.trim().is_empty() {
        rule.scope = "global".to_string();
    }
    if rule.priority == 0 {
        rule.priority = 3;
    }

    let store = RulesStore::open(&state.data_dir)?;
    store.upsert_rule(&rule)?;
    store
        .get_rule(&rule.id)?
        .ok_or_else(|| format!("rule not found after create: {}", rule.id))
}

#[tauri::command]
pub async fn get_rule_stats(state: State<'_, AppState>) -> Result<RuleStats, String> {
    let store = RulesStore::open(&state.data_dir)?;
    store.get_stats()
}
