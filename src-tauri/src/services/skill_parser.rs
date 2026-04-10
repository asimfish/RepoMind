use crate::models::skill::{Skill, SkillPlatform};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct SkillParser {
    known_skill_names: HashSet<String>,
}

impl SkillParser {
    pub fn new() -> Self {
        SkillParser {
            known_skill_names: HashSet::new(),
        }
    }

    pub fn set_known_names(&mut self, names: HashSet<String>) {
        self.known_skill_names = names;
    }

    /// 获取默认的扫描目录
    pub fn default_scan_dirs() -> Vec<(PathBuf, SkillPlatform)> {
        let home = dirs::home_dir().unwrap_or_default();
        vec![
            (home.join(".cursor/skills"), SkillPlatform::Cursor),
            (home.join(".cursor/skills-cursor"), SkillPlatform::Cursor),
            (home.join(".claude/skills"), SkillPlatform::Claude),
            (home.join(".codex/skills"), SkillPlatform::Codex),
        ]
    }

    /// 扫描目录，找到所有 SKILL.md 文件路径
    pub fn discover(root: &Path) -> Vec<PathBuf> {
        let mut results = vec![];
        if !root.exists() {
            return results;
        }
        Self::walk(root, &mut results);
        results
    }

    fn walk(dir: &Path, acc: &mut Vec<PathBuf>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase());
                if matches!(name.as_deref(), Some("node_modules" | ".git" | "target")) {
                    continue;
                }
                Self::walk(&path, acc);
            } else if path.file_name().map(|n| n == "SKILL.md").unwrap_or(false) {
                acc.push(path);
            }
        }
    }

    /// 解析单个 SKILL.md 文件
    pub fn parse(&self, path: &Path, platform: SkillPlatform) -> Result<Skill, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let (frontmatter, body) = Self::split_frontmatter(&content);
        let meta = Self::parse_yaml_frontmatter(&frontmatter);

        let name = meta
            .get("name")
            .cloned()
            .unwrap_or_else(|| Self::infer_name_from_path(path));

        let description = meta.get("description").cloned().unwrap_or_default();
        let tags = Self::parse_list_field(&meta, "tags");
        let mut trigger_patterns = Self::extract_triggers(&body);
        let fm_triggers = Self::parse_list_field(&meta, "triggers");
        for t in fm_triggers {
            if !trigger_patterns.contains(&t) {
                trigger_patterns.push(t);
            }
        }
        trigger_patterns.truncate(20);

        let depends_on = self.extract_dependencies(&body);
        let category = meta
            .get("category")
            .cloned()
            .or_else(|| Self::infer_category(path, &tags));

        let content_hash = Self::sha256_hex(body.as_bytes());
        let id = Self::sha256_hex(path.to_string_lossy().as_bytes());

        Ok(Skill {
            id,
            name,
            description,
            source_path: path.to_string_lossy().to_string(),
            source_platform: platform,
            author: meta.get("author").cloned(),
            version: meta.get("version").cloned(),
            tags,
            category,
            trigger_patterns,
            depends_on,
            raw_content: body,
            content_hash,
            parsed_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn split_frontmatter(content: &str) -> (String, String) {
        if content.starts_with("---") {
            if let Some(end) = content[3..].find("\n---") {
                let fm = content[3..end + 3].trim().to_string();
                let body = content[end + 7..].trim().to_string();
                return (fm, body);
            }
        }
        (String::new(), content.to_string())
    }

    fn parse_yaml_frontmatter(yaml_str: &str) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        for line in yaml_str.lines() {
            if let Some((key, val)) = line.split_once(':') {
                let key = key.trim().to_string();
                let val = val.trim().trim_matches('"').trim_matches('\'').to_string();
                if !key.is_empty() && !val.is_empty() {
                    map.insert(key, val);
                }
            }
        }
        map
    }

    fn parse_list_field(
        meta: &std::collections::HashMap<String, String>,
        key: &str,
    ) -> Vec<String> {
        meta.get(key)
            .map(|v| {
                v.trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn extract_triggers(body: &str) -> Vec<String> {
        let mut triggers = vec![];
        for line in body.lines() {
            let lower = line.to_lowercase();
            if lower.contains("use when")
                || lower.contains("触发")
                || lower.contains("activates when")
            {
                let mut start = 0;
                while let Some(pos) = line[start..].find('"') {
                    let abs_pos = start + pos + 1;
                    if let Some(end) = line[abs_pos..].find('"') {
                        let kw = &line[abs_pos..abs_pos + end];
                        if kw.len() > 1 && kw.len() < 50 {
                            triggers.push(kw.to_string());
                        }
                        start = abs_pos + end + 1;
                    } else {
                        break;
                    }
                }
            }
        }
        triggers.truncate(20);
        triggers
    }

    fn extract_dependencies(&self, body: &str) -> Vec<String> {
        let mut deps = vec![];
        for name in &self.known_skill_names {
            if body.contains(name.as_str()) {
                deps.push(name.clone());
            }
        }
        deps
    }

    fn infer_name_from_path(path: &Path) -> String {
        path.parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn infer_category(path: &Path, _tags: &[String]) -> Option<String> {
        let components: Vec<&std::ffi::OsStr> = path
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s),
                _ => None,
            })
            .collect();

        for (i, comp) in components.iter().enumerate() {
            if comp.to_string_lossy().contains("skills") && i + 2 < components.len() {
                let potential = components[i + 1].to_string_lossy().to_string();
                if potential != "SKILL.md" {
                    return Some(potential);
                }
            }
        }
        None
    }

    fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}
