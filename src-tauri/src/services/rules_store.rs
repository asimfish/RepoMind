/// Behavior rules persistence (SQLite), same style as `skill_store::SkillStore`.

use crate::models::rules::{BehaviorRule, RuleConflict, RuleStats};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS behavior_rules (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        content TEXT NOT NULL,
        category TEXT NOT NULL DEFAULT 'custom',
        status TEXT NOT NULL DEFAULT 'candidate',
        confidence REAL NOT NULL DEFAULT 0.0,
        source_type TEXT NOT NULL,
        source_file TEXT,
        source_excerpt TEXT,
        tags TEXT NOT NULL DEFAULT '[]',
        scope TEXT NOT NULL DEFAULT 'global',
        priority INTEGER NOT NULL DEFAULT 3,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        version INTEGER NOT NULL DEFAULT 1,
        content_hash TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_rules_status ON behavior_rules(status);
    CREATE INDEX IF NOT EXISTS idx_rules_category ON behavior_rules(category);

    CREATE TABLE IF NOT EXISTS rule_conflicts (
        id TEXT PRIMARY KEY,
        rule_a_id TEXT NOT NULL,
        rule_b_id TEXT NOT NULL,
        conflict_type TEXT NOT NULL,
        description TEXT NOT NULL,
        resolved INTEGER NOT NULL DEFAULT 0,
        detected_at TEXT NOT NULL
    );
"#;

const RULE_SELECT: &str = "SELECT id, title, content, category, status, confidence, source_type,
        source_file, source_excerpt, tags, scope, priority, created_at, updated_at, version
     FROM behavior_rules ";

pub fn hash_rule_content(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    hex_lower(&h.finalize())
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub struct RulesStore {
    db_path: PathBuf,
}

impl RulesStore {
    pub fn open(data_dir: &Path) -> Result<Self, String> {
        let db_path = data_dir.join("rules.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Self { db_path })
    }

    fn connect(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path).map_err(|e| e.to_string())
    }

    pub fn upsert_rule(&self, rule: &BehaviorRule) -> Result<(), String> {
        let content_hash = hash_rule_content(&rule.content);
        let tags_json = serde_json::to_string(&rule.tags).map_err(|e| e.to_string())?;
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO behavior_rules (id, title, content, category, status, confidence, source_type, source_file, source_excerpt, tags, scope, priority, created_at, updated_at, version, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(id) DO UPDATE SET
               title = excluded.title,
               content = excluded.content,
               category = excluded.category,
               status = excluded.status,
               confidence = excluded.confidence,
               source_type = excluded.source_type,
               source_file = excluded.source_file,
               source_excerpt = excluded.source_excerpt,
               tags = excluded.tags,
               scope = excluded.scope,
               priority = excluded.priority,
               created_at = behavior_rules.created_at,
               updated_at = excluded.updated_at,
               version = excluded.version,
               content_hash = excluded.content_hash",
            params![
                rule.id,
                rule.title,
                rule.content,
                rule.category,
                rule.status,
                rule.confidence as f64,
                rule.source_type,
                rule.source_file,
                rule.source_excerpt,
                tags_json,
                rule.scope,
                rule.priority as i64,
                rule.created_at,
                rule.updated_at,
                rule.version as i64,
                content_hash,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_rules(
        &self,
        status: Option<&str>,
        category: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<BehaviorRule>, String> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 500);
        let offset = (page - 1) * page_size;

        let conn = self.connect()?;
        let order = " ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2";

        let rules = match (status.filter(|s| !s.is_empty()), category.filter(|c| !c.is_empty())) {
            (None, None) => {
                let mut stmt = conn
                    .prepare(&(RULE_SELECT.to_string() + order))
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(params![page_size as i64, offset as i64], rule_from_row)
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
            (Some(st), None) => {
                let mut stmt = conn
                    .prepare(&(RULE_SELECT.to_string() + " WHERE status = ?1" + order))
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(params![st, page_size as i64, offset as i64], rule_from_row)
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
            (None, Some(cat)) => {
                let mut stmt = conn
                    .prepare(&(RULE_SELECT.to_string() + " WHERE category = ?1" + order))
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(params![cat, page_size as i64, offset as i64], rule_from_row)
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
            (Some(st), Some(cat)) => {
                let mut stmt = conn
                    .prepare(
                        &(RULE_SELECT.to_string() + " WHERE status = ?1 AND category = ?2" + order),
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(
                        params![st, cat, page_size as i64, offset as i64],
                        rule_from_row,
                    )
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
        };

        Ok(rules)
    }

    pub fn get_rule(&self, id: &str) -> Result<Option<BehaviorRule>, String> {
        let conn = self.connect()?;
        let sql = format!("{RULE_SELECT} WHERE id = ?1");
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(rule_from_row(&row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn update_rule_status(&self, id: &str, status: &str) -> Result<(), String> {
        let conn = self.connect()?;
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let n = conn
            .execute(
                "UPDATE behavior_rules SET status = ?1, updated_at = ?2, version = version + 1 WHERE id = ?3",
                params![status, now, id],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err(format!("rule not found: {}", id));
        }
        Ok(())
    }

    pub fn get_stats(&self) -> Result<RuleStats, String> {
        let conn = self.connect()?;
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM behavior_rules", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let approved: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM behavior_rules WHERE status = 'approved'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let candidate: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM behavior_rules WHERE status = 'candidate'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let rejected: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM behavior_rules WHERE status = 'rejected'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let mut by_category: HashMap<String, u32> = HashMap::new();
        let mut c_stmt = conn
            .prepare("SELECT category, COUNT(*) FROM behavior_rules GROUP BY category")
            .map_err(|e| e.to_string())?;
        let c_rows = c_stmt
            .query_map([], |row| {
                let cat: String = row.get(0)?;
                let c: i64 = row.get(1)?;
                Ok((cat, c as u32))
            })
            .map_err(|e| e.to_string())?;
        for r in c_rows {
            let (k, v) = r.map_err(|e| e.to_string())?;
            by_category.insert(k, v);
        }

        Ok(RuleStats {
            total: total as u32,
            approved: approved as u32,
            candidate: candidate as u32,
            rejected: rejected as u32,
            by_category,
        })
    }

    pub fn save_conflict(&self, conflict: &RuleConflict) -> Result<(), String> {
        let conn = self.connect()?;
        let detected_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let resolved_i: i64 = if conflict.resolved { 1 } else { 0 };
        conn.execute(
            "INSERT INTO rule_conflicts (id, rule_a_id, rule_b_id, conflict_type, description, resolved, detected_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               rule_a_id = excluded.rule_a_id,
               rule_b_id = excluded.rule_b_id,
               conflict_type = excluded.conflict_type,
               description = excluded.description,
               resolved = excluded.resolved",
            params![
                conflict.id,
                conflict.rule_a_id,
                conflict.rule_b_id,
                conflict.conflict_type,
                conflict.description,
                resolved_i,
                detected_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_conflicts(&self, rule_id: &str) -> Result<Vec<RuleConflict>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, rule_a_id, rule_b_id, conflict_type, description, resolved
                 FROM rule_conflicts
                 WHERE rule_a_id = ?1 OR rule_b_id = ?1
                 ORDER BY detected_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![rule_id], |row| {
                let resolved: i64 = row.get(5)?;
                Ok(RuleConflict {
                    id: row.get(0)?,
                    rule_a_id: row.get(1)?,
                    rule_b_id: row.get(2)?,
                    conflict_type: row.get(3)?,
                    description: row.get(4)?,
                    resolved: resolved != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }
}

fn rule_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BehaviorRule> {
    let tags_s: String = row.get(9)?;
    let tags: Vec<String> = serde_json::from_str(&tags_s).unwrap_or_default();
    Ok(BehaviorRule {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        category: row.get(3)?,
        status: row.get(4)?,
        confidence: row.get::<_, f64>(5)? as f32,
        source_type: row.get(6)?,
        source_file: row.get(7)?,
        source_excerpt: row.get(8)?,
        tags,
        scope: row.get(10)?,
        priority: row.get::<_, i64>(11)? as u8,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        version: row.get::<_, i64>(14)? as u32,
    })
}
