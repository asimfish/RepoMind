use std::collections::{HashMap, HashSet};

use tauri::State;

use crate::models::skill::*;
use crate::services::skill_parser::SkillParser;
use crate::services::skill_recommender::{Recommendation, SkillRecommender};
use crate::services::skill_store::SkillStore;
use crate::services::state::AppState;
use crate::services::transcript_collector::TranscriptCollector;
use crate::services::workflow_miner::WorkflowMiner;

#[tauri::command]
pub async fn scan_skills(state: State<'_, AppState>) -> Result<SkillScanResult, String> {
    let data_dir = state.data_dir.clone();
    let store = SkillStore::open(&data_dir)?;

    let mut parser = SkillParser::new();
    let scan_dirs = SkillParser::default_scan_dirs();

    let mut total_scanned = 0u32;
    let mut new_skills = 0u32;
    let mut updated_skills = 0u32;
    let mut by_platform: HashMap<String, u32> = HashMap::new();

    // First pass: collect all skill names for dependency detection
    let mut all_names = HashSet::new();
    for (dir, _) in &scan_dirs {
        for path in SkillParser::discover(dir) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let (fm, _) = split_frontmatter_quick(&content);
                if let Some(name) = fm.get("name") {
                    all_names.insert(name.clone());
                } else {
                    let name = path
                        .parent()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if !name.is_empty() {
                        all_names.insert(name);
                    }
                }
            }
        }
    }
    parser.set_known_names(all_names);

    // Second pass: parse and store
    for (dir, platform) in &scan_dirs {
        for path in SkillParser::discover(dir) {
            total_scanned += 1;
            let platform_str = format!("{:?}", platform).to_lowercase();
            *by_platform.entry(platform_str).or_default() += 1;

            match parser.parse(&path, platform.clone()) {
                Ok(skill) => {
                    let existing = store.get_skill(&skill.id)?;
                    if let Some(existing_skill) = existing {
                        if existing_skill.content_hash != skill.content_hash {
                            store.upsert_skill(&skill)?;
                            updated_skills += 1;
                        }
                    } else {
                        store.upsert_skill(&skill)?;
                        new_skills += 1;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(SkillScanResult {
        total_scanned,
        new_skills,
        updated_skills,
        by_platform,
    })
}

fn split_frontmatter_quick(content: &str) -> (HashMap<String, String>, String) {
    let mut map = HashMap::new();
    if content.starts_with("---") {
        if let Some(end) = content[3..].find("\n---") {
            let fm = &content[3..end + 3];
            for line in fm.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let k = k.trim().to_string();
                    let v = v.trim().trim_matches('"').to_string();
                    if !k.is_empty() {
                        map.insert(k, v);
                    }
                }
            }
            let body = content[end + 7..].trim().to_string();
            return (map, body);
        }
    }
    (map, content.to_string())
}

#[tauri::command]
pub async fn list_skills(
    platform: Option<String>,
    category: Option<String>,
    search: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Skill>, String> {
    let store = SkillStore::open(&state.data_dir)?;
    store.list_skills(
        platform.as_deref(),
        category.as_deref(),
        search.as_deref(),
    )
}

#[tauri::command]
pub async fn get_skill(
    skill_id: String,
    state: State<'_, AppState>,
) -> Result<Option<Skill>, String> {
    let store = SkillStore::open(&state.data_dir)?;
    store.get_skill(&skill_id)
}

#[tauri::command]
pub async fn get_skill_stats(state: State<'_, AppState>) -> Result<SkillStats, String> {
    let store = SkillStore::open(&state.data_dir)?;
    store.get_skill_stats()
}

#[tauri::command]
pub async fn collect_invocations(
    transcripts_dir: Option<String>,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let store = SkillStore::open(&state.data_dir)?;
    let skills = store.list_skills(None, None, None)?;

    let mut collector = TranscriptCollector::new();
    let known_map: HashMap<String, String> = skills
        .iter()
        .map(|s| (s.name.clone(), s.id.clone()))
        .collect();
    collector.set_known_skills(known_map);

    let dir = transcripts_dir
        .map(std::path::PathBuf::from)
        .or_else(TranscriptCollector::default_transcripts_dir)
        .ok_or("No transcripts directory found")?;

    let chains = collector.collect(&dir);
    let count = chains.len() as u32;

    for chain in &chains {
        store.save_chain(chain)?;
    }

    Ok(count)
}

#[tauri::command]
pub async fn mine_workflows(
    min_frequency: Option<u32>,
    min_length: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowTemplate>, String> {
    let store = SkillStore::open(&state.data_dir)?;
    let chains = store.list_chains(1000)?;

    let mut miner = WorkflowMiner::new();
    if let Some(f) = min_frequency {
        miner.min_support = f;
    }
    if let Some(l) = min_length {
        miner.min_length = l;
    }

    let workflows = miner.mine(&chains);

    for wf in &workflows {
        store.save_workflow(wf)?;
    }

    Ok(workflows)
}

#[tauri::command]
pub async fn list_workflows(
    status: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowTemplate>, String> {
    let store = SkillStore::open(&state.data_dir)?;
    store.list_workflows(status.as_deref())
}

#[tauri::command]
pub async fn update_workflow_status(
    workflow_id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let store = SkillStore::open(&state.data_dir)?;
    store.update_workflow_status(&workflow_id, &status)
}

#[tauri::command]
pub async fn get_skill_graph(state: State<'_, AppState>) -> Result<SkillGraphData, String> {
    let store = SkillStore::open(&state.data_dir)?;
    let (nodes_raw, edges_raw) = store.get_skill_graph()?;
    let skills = store.list_skills(None, None, None)?;
    let id_platform: HashMap<String, String> = skills
        .into_iter()
        .map(|s| (s.id, s.source_platform.as_str().to_string()))
        .collect();

    let nodes = nodes_raw
        .into_iter()
        .map(|(id, label, category, count)| SkillGraphNode {
            id: id.clone(),
            label,
            platform: id_platform.get(&id).cloned().unwrap_or_default(),
            category,
            invoke_count: count,
        })
        .collect();

    let edges = edges_raw
        .into_iter()
        .map(|(source, target, weight)| SkillGraphEdge {
            source,
            target,
            weight,
        })
        .collect();

    Ok(SkillGraphData { nodes, edges })
}

#[tauri::command]
pub async fn export_workflow(
    workflow_id: String,
    target_dir: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let store = SkillStore::open(&state.data_dir)?;
    let workflows = store.list_workflows(None)?;
    let wf = workflows
        .iter()
        .find(|w| w.id == workflow_id)
        .ok_or("Workflow not found")?;

    let dir = target_dir.map(std::path::PathBuf::from).unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".cursor/skills/workflows")
    });

    let safe_name = wf.name.replace(' ', "-").replace('/', "-");
    let wf_dir = dir.join(&safe_name);
    std::fs::create_dir_all(&wf_dir).map_err(|e| e.to_string())?;

    let steps_text: Vec<String> = wf
        .steps
        .iter()
        .map(|s| {
            format!(
                "{}. `{}`{}",
                s.order + 1,
                s.skill_name,
                if s.is_optional { " (可选)" } else { "" }
            )
        })
        .collect();

    let content = format!(
        "---\nname: {}\ndescription: \"{}\"\ntags: [workflow, auto-generated]\n---\n\n# {}\n\n{}\n\n## Steps\n\n{}\n\n## Metadata\n\n- Frequency: {}\n- Confidence: {:.0}%\n- Source: Auto-discovered by RepoMind\n",
        safe_name,
        wf.description,
        wf.name,
        wf.description,
        steps_text.join("\n"),
        wf.frequency,
        wf.confidence * 100.0
    );

    let file_path = wf_dir.join("SKILL.md");
    std::fs::write(&file_path, content).map_err(|e| e.to_string())?;

    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn record_skill_usage(
    skill_id: String,
    event_type: String,
    weight: Option<f32>,
    context: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let store = SkillStore::open(&state.data_dir)?;
    store.record_usage(
        &skill_id,
        &event_type,
        weight.unwrap_or(1.0),
        context.as_deref(),
    )?;
    Ok(())
}

#[tauri::command]
pub async fn get_recommendations(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<Recommendation>, String> {
    let store = SkillStore::open(&state.data_dir)?;
    let recommender = SkillRecommender::new(&store);
    recommender.recommend(limit.unwrap_or(10))
}
