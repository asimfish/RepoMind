use regex::Regex;

use crate::models::rules::BehaviorRule;
use crate::services::rules_store::hash_rule_content;

pub struct RulesExtractor;

impl RulesExtractor {
    pub fn extract_from_file(path: &str, source_type: &str) -> Result<Vec<BehaviorRule>, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let body = strip_frontmatter(&content);
        Self::extract_from_content(&body, source_type, Some(path))
    }

    pub fn extract_from_content(
        content: &str,
        source_type: &str,
        file_path: Option<&str>,
    ) -> Result<Vec<BehaviorRule>, String> {
        let sections = split_into_sections(content);
        let mut out = Vec::new();
        let mut seen_hashes = std::collections::HashSet::<String>::new();

        for (title, body) in sections {
            let candidates = Self::extract_from_section(&title, &body, source_type, file_path);
            for r in candidates {
                let h = hash_rule_content(&r.content);
                if seen_hashes.contains(&h) {
                    continue;
                }
                seen_hashes.insert(h);
                out.push(r);
            }
        }

        Ok(out)
    }

    fn extract_from_section(
        section_title: &str,
        body: &str,
        source_type: &str,
        file_path: Option<&str>,
    ) -> Vec<BehaviorRule> {
        let mut rules: Vec<BehaviorRule> = Vec::new();

        for line in body.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if is_list_line(t) {
                let text = strip_list_prefix(t);
                if text.len() < 5 {
                    continue;
                }
                let (cat, pri, conf) = classify_text(&text);
                rules.push(make_rule(
                    &text,
                    section_title,
                    source_type,
                    file_path,
                    cat,
                    pri,
                    conf,
                ));
            }
        }

        for para in split_paragraphs(body) {
            let p = para.trim();
            if p.len() < 12 {
                continue;
            }
            let u = p.to_uppercase();
            let l = p.to_lowercase();
            let has_directive = u.contains("MUST")
                || u.contains("NEVER")
                || u.contains("ALWAYS")
                || l.contains("prefer")
                || l.contains("should");
            if !has_directive {
                continue;
            }
            // Skip if this paragraph is only a list of items (already handled line-by-line)
            let non_empty_lines: Vec<&str> = p.lines().map(|x| x.trim()).filter(|x| !x.is_empty()).collect();
            if !non_empty_lines.is_empty() && non_empty_lines.iter().all(|l| is_list_line(l)) {
                continue;
            }
            let (cat, pri, conf) = classify_text(p);
            rules.push(make_rule(
                p,
                section_title,
                source_type,
                file_path,
                cat,
                pri,
                conf,
            ));
        }

        rules
    }
}

fn strip_frontmatter(content: &str) -> String {
    if !content.starts_with("---") {
        return content.to_string();
    }
    if let Some(rest) = content.get(3..) {
        if let Some(end) = rest.find("\n---") {
            let after = rest[end + 4..].trim_start_matches('\n');
            return after.to_string();
        }
    }
    content.to_string()
}

fn split_into_sections(content: &str) -> Vec<(String, String)> {
    let re = match Regex::new(r"(?m)^##\s+(.+)$") {
        Ok(r) => r,
        Err(_) => return vec![("Overview".to_string(), content.trim().to_string())],
    };

    let matches: Vec<(usize, usize, String)> = re
        .captures_iter(content)
        .filter_map(|c| {
            let full = c.get(0)?;
            let title = c.get(1)?.as_str().trim().to_string();
            Some((full.start(), full.end(), title))
        })
        .collect();

    if matches.is_empty() {
        let t = content.trim();
        if t.is_empty() {
            return vec![];
        }
        return vec![("Overview".to_string(), t.to_string())];
    }

    let mut out = Vec::new();
    let first_start = matches[0].0;
    if first_start > 0 {
        let pre = content[0..first_start].trim();
        if !pre.is_empty() {
            out.push(("Overview".to_string(), pre.to_string()));
        }
    }

    for i in 0..matches.len() {
        let (_, end_h, ref title) = matches[i];
        let body_end = if i + 1 < matches.len() {
            matches[i + 1].0
        } else {
            content.len()
        };
        let body = content[end_h..body_end].trim();
        if !body.is_empty() {
            out.push((title.clone(), body.to_string()));
        }
    }

    out
}

fn split_paragraphs(body: &str) -> Vec<String> {
    let re = Regex::new(r"\n\s*\n").unwrap();
    re.split(body).map(|s| s.to_string()).collect()
}

fn is_list_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("- ")
        || t.starts_with("* ")
        || t.starts_with("+ ")
        || (t
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && t.contains('.'))
}

fn strip_list_prefix(line: &str) -> String {
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix("- ") {
        return rest.trim().to_string();
    }
    if let Some(rest) = t.strip_prefix("* ") {
        return rest.trim().to_string();
    }
    if let Some(rest) = t.strip_prefix("+ ") {
        return rest.trim().to_string();
    }
    if let Some((_, after)) = t.split_once('.') {
        if t.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            return after.trim().to_string();
        }
    }
    t.to_string()
}

fn classify_text(text: &str) -> (&'static str, u8, f32) {
    let upper = text.to_uppercase();
    if upper.contains("MUST") || upper.contains("NEVER") || upper.contains("ALWAYS") {
        return ("safety", 5, 0.85);
    }
    let lower = text.to_lowercase();
    if lower.contains("prefer")
        || lower.contains("should not")
        || lower.contains("should ")
        || lower.contains("should,")
    {
        return ("coding", 3, 0.70);
    }
    ("workflow", 3, 0.55)
}

fn slug_tag(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    s.trim_matches('-').chars().take(48).collect()
}

fn make_rule(
    content: &str,
    section_title: &str,
    source_type: &str,
    file_path: Option<&str>,
    category: &str,
    priority: u8,
    confidence: f32,
) -> BehaviorRule {
    let id = format!("rule_{}", hash_rule_content(content));
    let title = if content.len() > 100 {
        let head: String = content.chars().take(80).collect();
        format!("{} — {}…", section_title, head)
    } else {
        format!("{} — {}", section_title, content)
    };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let excerpt: String = content.chars().take(200).collect();
    let tag = slug_tag(section_title);
    let mut tags = vec![tag];
    if tags[0].is_empty() {
        tags = vec!["section".to_string()];
    }

    BehaviorRule {
        id,
        title,
        content: content.to_string(),
        category: category.to_string(),
        status: "candidate".to_string(),
        confidence,
        source_type: source_type.to_string(),
        source_file: file_path.map(String::from),
        source_excerpt: Some(excerpt),
        tags,
        scope: "project".to_string(),
        priority,
        created_at: now.clone(),
        updated_at: now,
        version: 1,
    }
}

/// Infer `source_type` from path (basename / `.cursor/rules`).
pub fn infer_source_type(path: &std::path::Path) -> &'static str {
    let lossy = path.to_string_lossy();
    if lossy.contains(".cursor/rules") && path.extension().map(|e| e == "mdc").unwrap_or(false) {
        return "cursor_rule";
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if name == "agents.md" {
        return "agents_md";
    }
    if name == "claude.md" {
        return "claude_md";
    }
    if name == "skill.md" {
        return "skill_file";
    }
    "user_created"
}

/// Recursively collect files under `root` that look like rule sources.
pub fn collect_rule_files_under(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
    if root.is_file() {
        if is_rule_source_file(root) {
            out.push(root.to_path_buf());
        }
        return Ok(());
    }
    if !root.is_dir() {
        return Ok(());
    }
    let read = std::fs::read_dir(root).map_err(|e| e.to_string())?;
    for e in read {
        let e = e.map_err(|e| e.to_string())?;
        let p = e.path();
        if p.is_dir() {
            collect_rule_files_under(&p, out)?;
        } else if is_rule_source_file(&p) {
            out.push(p);
        }
    }
    Ok(())
}

fn is_rule_source_file(path: &std::path::Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if name == "agents.md" || name == "claude.md" || name == "skill.md" {
        return true;
    }
    let lossy = path.to_string_lossy();
    lossy.contains(".cursor/rules") && path.extension().map(|e| e == "mdc").unwrap_or(false)
}
