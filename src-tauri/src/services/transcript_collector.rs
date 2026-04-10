use crate::models::skill::InvocationChain;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct TranscriptCollector {
    known_skills: HashMap<String, String>, // name → id
}

impl TranscriptCollector {
    pub fn new() -> Self {
        TranscriptCollector {
            known_skills: HashMap::new(),
        }
    }

    pub fn set_known_skills(&mut self, skills: HashMap<String, String>) {
        self.known_skills = skills;
    }

    /// 获取默认的 transcripts 目录
    pub fn default_transcripts_dir() -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        // Try Cursor's agent-transcripts location
        let cursor_dir = home.join(".cursor/projects");
        if cursor_dir.exists() {
            // Find the first project with agent-transcripts
            if let Ok(entries) = std::fs::read_dir(&cursor_dir) {
                for entry in entries.flatten() {
                    let transcripts = entry.path().join("agent-transcripts");
                    if transcripts.exists() {
                        return Some(transcripts);
                    }
                }
            }
        }
        None
    }

    /// 扫描 transcripts 目录，提取调用链
    pub fn collect(&self, transcripts_dir: &Path) -> Vec<InvocationChain> {
        let mut chains = vec![];
        let entries = match std::fs::read_dir(transcripts_dir) {
            Ok(e) => e,
            Err(_) => return chains,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                if let Some(chain) = self.parse_transcript(&path) {
                    if chain.skill_sequence.len() >= 2 {
                        chains.push(chain);
                    }
                }
            }
        }
        chains
    }

    fn parse_transcript(&self, path: &Path) -> Option<InvocationChain> {
        let session_id = path.file_stem()?.to_string_lossy().to_string();
        let content = std::fs::read_to_string(path).ok()?;

        let mut skill_seq = vec![];
        let mut started_at = String::new();

        for line in content.lines() {
            let v: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Extract timestamp
            if started_at.is_empty() {
                if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
                    started_at = ts.to_string();
                }
            }

            // Detect skill references:
            // 1. Read tool reading SKILL.md
            if let Some(tool_name) = v.get("tool").and_then(|t| t.as_str()) {
                if tool_name == "Read" {
                    if let Some(read_path) = v
                        .get("args")
                        .and_then(|a| a.get("path"))
                        .and_then(|p| p.as_str())
                    {
                        if read_path.contains("SKILL.md") {
                            let skill_name = Self::extract_skill_name_from_path(read_path);
                            if !skill_seq
                                .last()
                                .map(|s: &String| s == &skill_name)
                                .unwrap_or(false)
                            {
                                skill_seq.push(skill_name);
                            }
                        }
                    }
                }
            }

            // 2. Detect skill name mentions in content
            if let Some(text) = v.get("content").and_then(|c| c.as_str()) {
                for name in self.known_skills.keys() {
                    if text.contains(name.as_str())
                        && !skill_seq.last().map(|s| s == name).unwrap_or(false)
                    {
                        skill_seq.push(name.clone());
                    }
                }
            }
        }

        if skill_seq.is_empty() {
            return None;
        }

        Some(InvocationChain {
            session_id,
            skill_sequence: skill_seq,
            started_at,
            task_summary: None,
        })
    }

    fn extract_skill_name_from_path(path: &str) -> String {
        Path::new(path)
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}
