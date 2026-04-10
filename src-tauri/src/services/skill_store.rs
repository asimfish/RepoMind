/// Skill persistence using SQLite (rusqlite), same connection/schema style as `vector::VectorStore`.

use crate::models::skill::*;
use chrono::{Duration, Utc};
use rusqlite::{Connection, params};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS skills (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        source_path TEXT NOT NULL UNIQUE,
        source_platform TEXT,
        author TEXT,
        version TEXT,
        tags TEXT NOT NULL DEFAULT '[]',
        category TEXT,
        trigger_patterns TEXT NOT NULL DEFAULT '[]',
        depends_on TEXT NOT NULL DEFAULT '[]',
        content_hash TEXT,
        parsed_at TEXT
    );
    CREATE TABLE IF NOT EXISTS skill_content (
        skill_id TEXT PRIMARY KEY NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
        raw_content TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS invocations (
        id TEXT PRIMARY KEY NOT NULL,
        session_id TEXT NOT NULL,
        skill_name TEXT NOT NULL,
        skill_id TEXT,
        invoked_at TEXT NOT NULL,
        sequence_index INTEGER NOT NULL,
        context_snippet TEXT,
        trigger_type TEXT
    );
    CREATE TABLE IF NOT EXISTS chains (
        session_id TEXT PRIMARY KEY NOT NULL,
        skill_sequence TEXT NOT NULL,
        started_at TEXT NOT NULL,
        task_summary TEXT
    );
    CREATE TABLE IF NOT EXISTS workflows (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        steps TEXT NOT NULL,
        frequency REAL,
        confidence REAL,
        source_sessions TEXT NOT NULL DEFAULT '[]',
        category TEXT,
        created_at TEXT NOT NULL,
        status TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS skill_edges (
        source_skill TEXT NOT NULL,
        target_skill TEXT NOT NULL,
        co_occurrence INTEGER NOT NULL DEFAULT 0,
        avg_distance REAL,
        PRIMARY KEY (source_skill, target_skill)
    );
    CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);
    CREATE INDEX IF NOT EXISTS idx_skills_platform ON skills(source_platform);
    CREATE INDEX IF NOT EXISTS idx_inv_session ON invocations(session_id);
    CREATE INDEX IF NOT EXISTS idx_wf_status ON workflows(status);
    CREATE TABLE IF NOT EXISTS usage_logs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        skill_id TEXT NOT NULL,
        event_type TEXT NOT NULL,
        weight REAL NOT NULL DEFAULT 1.0,
        context TEXT,
        timestamp TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_usage_skill ON usage_logs(skill_id);
    CREATE INDEX IF NOT EXISTS idx_usage_time ON usage_logs(timestamp);
    CREATE TABLE IF NOT EXISTS co_occurrence (
        skill_a TEXT NOT NULL,
        skill_b TEXT NOT NULL,
        count INTEGER NOT NULL DEFAULT 0,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (skill_a, skill_b)
    );
"#;

const SKILL_SELECT: &str = "SELECT s.id, s.name, s.description, s.source_path, s.source_platform, s.author, s.version,
        s.tags, s.category, s.trigger_patterns, s.depends_on, s.content_hash, s.parsed_at,
        c.raw_content
     FROM skills s
     LEFT JOIN skill_content c ON c.skill_id = s.id ";

pub struct SkillStore {
    db_path: PathBuf,
}

impl SkillStore {
    pub fn open(data_dir: &Path) -> Result<Self, String> {
        let db_path = data_dir.join("skills.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Self { db_path })
    }

    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    fn connect(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path).map_err(|e| e.to_string())
    }

    pub fn upsert_skill(&self, skill: &Skill) -> Result<(), String> {
        let mut conn = self.connect()?;
        let tags_json = serde_json::to_string(&skill.tags).map_err(|e| e.to_string())?;
        let trigger_json = serde_json::to_string(&skill.trigger_patterns).map_err(|e| e.to_string())?;
        let depends_json = serde_json::to_string(&skill.depends_on).map_err(|e| e.to_string())?;
        let platform = skill.source_platform.as_str();

        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO skills (id, name, description, source_path, source_platform, author, version, tags, category, trigger_patterns, depends_on, content_hash, parsed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               description = excluded.description,
               source_path = excluded.source_path,
               source_platform = excluded.source_platform,
               author = excluded.author,
               version = excluded.version,
               tags = excluded.tags,
               category = excluded.category,
               trigger_patterns = excluded.trigger_patterns,
               depends_on = excluded.depends_on,
               content_hash = excluded.content_hash,
               parsed_at = excluded.parsed_at",
            params![
                skill.id,
                skill.name,
                skill.description,
                skill.source_path,
                platform,
                skill.author,
                skill.version,
                tags_json,
                skill.category,
                trigger_json,
                depends_json,
                skill.content_hash,
                skill.parsed_at,
            ],
        )
        .map_err(|e| e.to_string())?;

        if !skill.raw_content.is_empty() {
            tx.execute(
                "INSERT INTO skill_content (skill_id, raw_content) VALUES (?1, ?2)
                 ON CONFLICT(skill_id) DO UPDATE SET raw_content = excluded.raw_content",
                params![skill.id, skill.raw_content.as_str()],
            )
            .map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_skill(&self, id: &str) -> Result<Option<Skill>, String> {
        let conn = self.connect()?;
        let sql = format!("{SKILL_SELECT} WHERE s.id = ?1");
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(Some(skill_from_row(&row).map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn list_skills(
        &self,
        platform: Option<&str>,
        category: Option<&str>,
        search: Option<&str>,
    ) -> Result<Vec<Skill>, String> {
        let conn = self.connect()?;
        let order = " ORDER BY s.name";

        let search_pat = search.map(|q| format!("%{}%", q));

        let skills = match (platform, category, search_pat.as_ref()) {
            (None, None, None) => {
                let mut stmt = conn
                    .prepare(&(SKILL_SELECT.to_string() + order))
                    .map_err(|e| e.to_string())?;
                let rows = stmt.query_map([], |row| skill_from_row(row)).map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
            (Some(p), None, None) => {
                let mut stmt = conn
                    .prepare(&(SKILL_SELECT.to_string() + " WHERE s.source_platform = ?1" + order))
                    .map_err(|e| e.to_string())?;
                let rows = stmt.query_map(params![p], |row| skill_from_row(row)).map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
            (None, Some(c), None) => {
                let mut stmt = conn
                    .prepare(&(SKILL_SELECT.to_string() + " WHERE s.category = ?1" + order))
                    .map_err(|e| e.to_string())?;
                let rows = stmt.query_map(params![c], |row| skill_from_row(row)).map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
            (None, None, Some(pat)) => {
                let mut stmt = conn
                    .prepare(
                        &(SKILL_SELECT.to_string()
                            + " WHERE (s.name LIKE ?1 OR IFNULL(s.description, '') LIKE ?2 OR s.source_path LIKE ?3)"
                            + order),
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(params![pat.as_str(), pat.as_str(), pat.as_str()], |row| skill_from_row(row))
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
            (Some(p), Some(c), None) => {
                let mut stmt = conn
                    .prepare(
                        &(SKILL_SELECT.to_string()
                            + " WHERE s.source_platform = ?1 AND s.category = ?2"
                            + order),
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(params![p, c], |row| skill_from_row(row))
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
            (Some(p), None, Some(pat)) => {
                let mut stmt = conn
                    .prepare(
                        &(SKILL_SELECT.to_string()
                            + " WHERE s.source_platform = ?1 AND (s.name LIKE ?2 OR IFNULL(s.description, '') LIKE ?3 OR s.source_path LIKE ?4)"
                            + order),
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(params![p, pat.as_str(), pat.as_str(), pat.as_str()], |row| skill_from_row(row))
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
            (None, Some(c), Some(pat)) => {
                let mut stmt = conn
                    .prepare(
                        &(SKILL_SELECT.to_string()
                            + " WHERE s.category = ?1 AND (s.name LIKE ?2 OR IFNULL(s.description, '') LIKE ?3 OR s.source_path LIKE ?4)"
                            + order),
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(params![c, pat.as_str(), pat.as_str(), pat.as_str()], |row| skill_from_row(row))
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
            (Some(p), Some(c), Some(pat)) => {
                let mut stmt = conn
                    .prepare(
                        &(SKILL_SELECT.to_string()
                            + " WHERE s.source_platform = ?1 AND s.category = ?2 AND (s.name LIKE ?3 OR IFNULL(s.description, '') LIKE ?4 OR s.source_path LIKE ?5)"
                            + order),
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(
                        params![p, c, pat.as_str(), pat.as_str(), pat.as_str()],
                        |row| skill_from_row(row),
                    )
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
            }
        };

        Ok(skills)
    }

    pub fn get_skill_stats(&self) -> Result<SkillStats, String> {
        let conn = self.connect()?;
        let total_skills: i64 = conn
            .query_row("SELECT COUNT(*) FROM skills", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let total_chains: i64 = conn
            .query_row("SELECT COUNT(*) FROM chains", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let total_workflows: i64 = conn
            .query_row("SELECT COUNT(*) FROM workflows", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;

        let mut by_platform: HashMap<String, u32> = HashMap::new();
        let mut p_stmt = conn
            .prepare("SELECT source_platform, COUNT(*) FROM skills GROUP BY source_platform")
            .map_err(|e| e.to_string())?;
        let p_rows = p_stmt
            .query_map([], |row| {
                let plat: Option<String> = row.get(0)?;
                let c: i64 = row.get(1)?;
                Ok((plat.unwrap_or_else(|| "unknown".to_string()), c as u32))
            })
            .map_err(|e| e.to_string())?;
        for r in p_rows {
            let (k, v) = r.map_err(|e| e.to_string())?;
            by_platform.insert(k, v);
        }

        let mut by_category: HashMap<String, u32> = HashMap::new();
        let mut c_stmt = conn
            .prepare("SELECT category, COUNT(*) FROM skills WHERE category IS NOT NULL AND category != '' GROUP BY category")
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

        let mut top_invoked: Vec<SkillInvokeCount> = Vec::new();
        let mut t_stmt = conn
            .prepare(
                "SELECT skill_name, COUNT(*) AS c FROM invocations GROUP BY skill_name ORDER BY c DESC LIMIT 20",
            )
            .map_err(|e| e.to_string())?;
        let t_rows = t_stmt
            .query_map([], |row| {
                Ok(SkillInvokeCount {
                    name: row.get(0)?,
                    count: row.get::<_, i64>(1)? as u32,
                })
            })
            .map_err(|e| e.to_string())?;
        for r in t_rows {
            top_invoked.push(r.map_err(|e| e.to_string())?);
        }

        let mut recent_workflows: Vec<WorkflowTemplate> = Vec::new();
        let mut w_stmt = conn
            .prepare(
                "SELECT id, name, description, steps, frequency, confidence, source_sessions, category, created_at, status
                 FROM workflows ORDER BY created_at DESC LIMIT 10",
            )
            .map_err(|e| e.to_string())?;
        let w_rows = w_stmt
            .query_map([], |row| workflow_from_row(row))
            .map_err(|e| e.to_string())?;
        for r in w_rows {
            recent_workflows.push(r.map_err(|e| e.to_string())?);
        }

        Ok(SkillStats {
            total_skills: total_skills as u32,
            total_chains: total_chains as u32,
            total_workflows: total_workflows as u32,
            by_platform,
            by_category,
            top_invoked,
            recent_workflows,
        })
    }

    pub fn save_invocation(&self, inv: &SkillInvocation) -> Result<(), String> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT OR REPLACE INTO invocations (id, session_id, skill_name, skill_id, invoked_at, sequence_index, context_snippet, trigger_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                inv.id,
                inv.session_id,
                inv.skill_name,
                inv.skill_id,
                inv.invoked_at,
                inv.sequence_index,
                inv.context_snippet,
                inv.trigger_type,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save_chain(&self, chain: &InvocationChain) -> Result<(), String> {
        let conn = self.connect()?;
        let seq_json = serde_json::to_string(&chain.skill_sequence).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO chains (session_id, skill_sequence, started_at, task_summary)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id) DO UPDATE SET
               skill_sequence = excluded.skill_sequence,
               started_at = excluded.started_at,
               task_summary = excluded.task_summary",
            params![chain.session_id, seq_json, chain.started_at, chain.task_summary],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_chains(&self, limit: u32) -> Result<Vec<InvocationChain>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT session_id, skill_sequence, started_at, task_summary FROM chains
                 ORDER BY started_at DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |row| chain_from_row(row))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn save_workflow(&self, wf: &WorkflowTemplate) -> Result<(), String> {
        let conn = self.connect()?;
        let steps_json = serde_json::to_string(&wf.steps).map_err(|e| e.to_string())?;
        let sessions_json = serde_json::to_string(&wf.source_sessions).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO workflows (id, name, description, steps, frequency, confidence, source_sessions, category, created_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               description = excluded.description,
               steps = excluded.steps,
               frequency = excluded.frequency,
               confidence = excluded.confidence,
               source_sessions = excluded.source_sessions,
               category = excluded.category,
               created_at = excluded.created_at,
               status = excluded.status",
            params![
                wf.id,
                wf.name,
                wf.description,
                steps_json,
                wf.frequency as f64,
                wf.confidence as f64,
                sessions_json,
                wf.category,
                wf.created_at,
                wf.status.as_str(),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_workflows(&self, status: Option<&str>) -> Result<Vec<WorkflowTemplate>, String> {
        let conn = self.connect()?;
        if let Some(s) = status {
            let sn = s.to_lowercase().replace('-', "_");
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, description, steps, frequency, confidence, source_sessions, category, created_at, status
                     FROM workflows WHERE LOWER(REPLACE(status, '-', '_')) = ?1 ORDER BY created_at DESC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![sn], |row| workflow_from_row(row))
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, description, steps, frequency, confidence, source_sessions, category, created_at, status
                     FROM workflows ORDER BY created_at DESC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| workflow_from_row(row))
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        }
    }

    pub fn update_workflow_status(&self, id: &str, status: &str) -> Result<(), String> {
        let st = parse_workflow_status(status)?;
        let conn = self.connect()?;
        let n = conn
            .execute(
                "UPDATE workflows SET status = ?1 WHERE id = ?2",
                params![st.as_str(), id],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err(format!("workflow not found: {}", id));
        }
        Ok(())
    }

    pub fn save_edge(&self, source: &str, target: &str, count: u32) -> Result<(), String> {
        let conn = self.connect()?;
        let c = count as i64;
        conn.execute(
            "INSERT INTO skill_edges (source_skill, target_skill, co_occurrence, avg_distance)
             VALUES (?1, ?2, ?3, NULL)
             ON CONFLICT(source_skill, target_skill) DO UPDATE SET
               co_occurrence = co_occurrence + excluded.co_occurrence",
            params![source, target, c],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_skill_graph(
        &self,
    ) -> Result<(Vec<(String, String, Option<String>, u32)>, Vec<(String, String, u32)>), String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.name, s.category, COALESCE(inv.c, 0) AS cnt
                 FROM skills s
                 LEFT JOIN (
                   SELECT skill_id, COUNT(*) AS c FROM invocations WHERE skill_id IS NOT NULL AND skill_id != ''
                   GROUP BY skill_id
                 ) inv ON inv.skill_id = s.id
                 ORDER BY s.name",
            )
            .map_err(|e| e.to_string())?;
        let nodes: Vec<(String, String, Option<String>, u32)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)? as u32,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut estmt = conn
            .prepare(
                "SELECT source_skill, target_skill, co_occurrence FROM skill_edges ORDER BY co_occurrence DESC",
            )
            .map_err(|e| e.to_string())?;
        let edges: Vec<(String, String, u32)> = estmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? as u32,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok((nodes, edges))
    }

    /// Same as `list_skills` with only a text query (name, description, path).
    pub fn search_skills(&self, query: &str) -> Result<Vec<Skill>, String> {
        self.list_skills(None, None, Some(query))
    }

    /// Case-insensitive match on workflow name, description, or category.
    pub fn find_workflows_by_name(&self, keyword: &str) -> Result<Vec<WorkflowTemplate>, String> {
        let k = keyword.to_lowercase();
        Ok(self
            .list_workflows(None)?
            .into_iter()
            .filter(|w| {
                w.name.to_lowercase().contains(&k)
                    || w.description.to_lowercase().contains(&k)
                    || w
                        .category
                        .as_ref()
                        .map(|c| c.to_lowercase().contains(&k))
                        .unwrap_or(false)
            })
            .collect())
    }

    pub fn record_usage(
        &self,
        skill_id: &str,
        event_type: &str,
        weight: f32,
        context: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.connect()?;
        let ts = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO usage_logs (skill_id, event_type, weight, context, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                skill_id,
                event_type,
                weight as f64,
                context,
                ts,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_usage_stats(&self, skill_id: &str) -> Result<UsageStats, String> {
        let conn = self.connect()?;
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM usage_logs WHERE skill_id = ?1",
                params![skill_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let now = Utc::now();
        let cutoff_7 = (now - Duration::days(7)).to_rfc3339();
        let cutoff_30 = (now - Duration::days(30)).to_rfc3339();

        let last_7: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM usage_logs WHERE skill_id = ?1 AND timestamp >= ?2",
                params![skill_id, cutoff_7],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let last_30: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM usage_logs WHERE skill_id = ?1 AND timestamp >= ?2",
                params![skill_id, cutoff_30],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        Ok(UsageStats {
            total_count: total as u32,
            last_7_days: last_7 as u32,
            last_30_days: last_30 as u32,
        })
    }

    pub fn get_recent_used_skills(&self, days: u32, limit: u32) -> Result<Vec<(String, u32)>, String> {
        let conn = self.connect()?;
        let cutoff = (Utc::now() - Duration::days(days as i64)).to_rfc3339();
        let mut stmt = conn
            .prepare(
                "SELECT skill_id, COUNT(*) AS c FROM usage_logs
                 WHERE timestamp >= ?1
                 GROUP BY skill_id
                 ORDER BY c DESC
                 LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![cutoff, limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_all_usage_tags(&self) -> Result<Vec<String>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT DISTINCT skill_id FROM usage_logs")
            .map_err(|e| e.to_string())?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut tag_set: HashSet<String> = HashSet::new();
        for id in ids {
            if let Some(skill) = self.get_skill(&id)? {
                for t in skill.tags {
                    tag_set.insert(t);
                }
            }
        }

        let mut tags: Vec<String> = tag_set.into_iter().collect();
        tags.sort();
        Ok(tags)
    }

    pub fn update_co_occurrence(&self, skill_a: &str, skill_b: &str) -> Result<(), String> {
        if skill_a == skill_b {
            return Ok(());
        }
        let (a, b) = if skill_a <= skill_b {
            (skill_a, skill_b)
        } else {
            (skill_b, skill_a)
        };
        let conn = self.connect()?;
        let updated_at = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO co_occurrence (skill_a, skill_b, count, updated_at) VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(skill_a, skill_b) DO UPDATE SET
               count = count + 1,
               updated_at = excluded.updated_at",
            params![a, b, updated_at],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn skill_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Skill> {
    let tags_s: String = row.get(7)?;
    let trigger_s: String = row.get(9)?;
    let depends_s: String = row.get(10)?;
    let tags: Vec<String> = serde_json::from_str(&tags_s).unwrap_or_default();
    let trigger_patterns: Vec<String> = serde_json::from_str(&trigger_s).unwrap_or_default();
    let depends_on: Vec<String> = serde_json::from_str(&depends_s).unwrap_or_default();
    let platform_s: Option<String> = row.get(4)?;
    let source_platform = SkillPlatform::parse(platform_s.as_deref().unwrap_or("cursor"));

    let description: String = row
        .get::<_, Option<String>>(2)?
        .unwrap_or_default();
    let content_hash: String = row
        .get::<_, Option<String>>(11)?
        .unwrap_or_default();
    let parsed_at: String = row
        .get::<_, Option<String>>(12)?
        .unwrap_or_default();
    let raw_content: String = row
        .get::<_, Option<String>>(13)?
        .unwrap_or_default();

    Ok(Skill {
        id: row.get(0)?,
        name: row.get(1)?,
        description,
        source_path: row.get(3)?,
        source_platform,
        author: row.get(5)?,
        version: row.get(6)?,
        tags,
        category: row.get(8)?,
        trigger_patterns,
        depends_on,
        content_hash,
        parsed_at,
        raw_content,
    })
}

fn chain_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<InvocationChain> {
    let seq_s: String = row.get(1)?;
    let skill_sequence: Vec<String> = serde_json::from_str(&seq_s).unwrap_or_default();
    Ok(InvocationChain {
        session_id: row.get(0)?,
        skill_sequence,
        started_at: row.get(2)?,
        task_summary: row.get(3)?,
    })
}

fn workflow_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkflowTemplate> {
    let steps_s: String = row.get(3)?;
    let sessions_s: String = row.get(6)?;
    let steps: Vec<WorkflowStep> = serde_json::from_str(&steps_s).unwrap_or_default();
    let source_sessions: Vec<String> = serde_json::from_str(&sessions_s).unwrap_or_default();
    let frequency: Option<f64> = row.get(4)?;
    let confidence: Option<f64> = row.get(5)?;
    let status_s: String = row.get(9)?;
    Ok(WorkflowTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        steps,
        frequency: frequency.map(|v| v.round() as u32).unwrap_or(0),
        confidence: confidence.map(|v| v as f32).unwrap_or(0.0),
        source_sessions,
        category: row.get(7)?,
        created_at: row.get(8)?,
        status: parse_workflow_status_db(&status_s).unwrap_or(WorkflowStatus::Discovered),
    })
}

fn parse_workflow_status(s: &str) -> Result<WorkflowStatus, String> {
    let n = s.to_lowercase().replace('-', "_");
    match n.as_str() {
        "discovered" => Ok(WorkflowStatus::Discovered),
        "confirmed" => Ok(WorkflowStatus::Confirmed),
        "exported" => Ok(WorkflowStatus::Exported),
        "dismissed" => Ok(WorkflowStatus::Dismissed),
        _ => Err(format!("Unknown workflow status: {s}")),
    }
}

fn parse_workflow_status_db(s: &str) -> Option<WorkflowStatus> {
    parse_workflow_status(s).ok()
}
