//! MCP tool invocation and behavior sequence persistence (`behavior.db`).

use crate::models::agent::{BehaviorSequence, McpInvocation, PatternSummary, UserProfile};
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS mcp_invocations (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        timestamp INTEGER NOT NULL,
        tool_name TEXT NOT NULL,
        arguments_json TEXT NOT NULL,
        duration_ms INTEGER,
        repo_id TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_inv_session ON mcp_invocations(session_id);
    CREATE INDEX IF NOT EXISTS idx_inv_tool ON mcp_invocations(tool_name);
    CREATE INDEX IF NOT EXISTS idx_inv_time ON mcp_invocations(timestamp);

    CREATE TABLE IF NOT EXISTS behavior_sequences (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        started_at INTEGER NOT NULL,
        ended_at INTEGER,
        intent_label TEXT,
        tool_chain TEXT NOT NULL,
        repo_id TEXT
    );
"#;

pub struct BehaviorStore {
    db_path: PathBuf,
}

impl BehaviorStore {
    pub fn open(data_dir: &Path) -> Result<Self, String> {
        let db_path = data_dir.join("behavior.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Self { db_path })
    }

    fn connect(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path).map_err(|e| e.to_string())
    }

    pub fn record_invocation(&self, inv: &McpInvocation) -> Result<(), String> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO mcp_invocations (id, session_id, timestamp, tool_name, arguments_json, duration_ms, repo_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                inv.id,
                inv.session_id,
                inv.timestamp,
                inv.tool_name,
                inv.arguments_json,
                inv.duration_ms,
                inv.repo_id,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_invocations(&self, limit: u32, tool_filter: Option<&str>) -> Result<Vec<McpInvocation>, String> {
        let conn = self.connect()?;
        let limit = i64::from(limit);
        let mut out = Vec::new();
        if let Some(tf) = tool_filter.filter(|s| !s.is_empty()) {
            let mut stmt = conn
                .prepare(
                    "SELECT id, session_id, timestamp, tool_name, arguments_json, duration_ms, repo_id
                     FROM mcp_invocations WHERE tool_name = ?1 ORDER BY timestamp DESC LIMIT ?2",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![tf, limit], |r| {
                    Ok(McpInvocation {
                        id: r.get(0)?,
                        session_id: r.get(1)?,
                        timestamp: r.get(2)?,
                        tool_name: r.get(3)?,
                        arguments_json: r.get(4)?,
                        duration_ms: r.get(5)?,
                        repo_id: r.get(6)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                out.push(row.map_err(|e| e.to_string())?);
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, session_id, timestamp, tool_name, arguments_json, duration_ms, repo_id
                     FROM mcp_invocations ORDER BY timestamp DESC LIMIT ?1",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![limit], |r| {
                    Ok(McpInvocation {
                        id: r.get(0)?,
                        session_id: r.get(1)?,
                        timestamp: r.get(2)?,
                        tool_name: r.get(3)?,
                        arguments_json: r.get(4)?,
                        duration_ms: r.get(5)?,
                        repo_id: r.get(6)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                out.push(row.map_err(|e| e.to_string())?);
            }
        }
        Ok(out)
    }

    pub fn save_sequence(&self, seq: &BehaviorSequence) -> Result<(), String> {
        let conn = self.connect()?;
        let chain_json = serde_json::to_string(&seq.tool_chain).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO behavior_sequences (id, session_id, started_at, ended_at, intent_label, tool_chain, repo_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                seq.id,
                seq.session_id,
                seq.started_at,
                seq.ended_at,
                seq.intent_label,
                chain_json,
                seq.repo_id,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_tool_affinity(&self) -> Result<HashMap<String, f64>, String> {
        let conn = self.connect()?;
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM mcp_invocations", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if total == 0 {
            return Ok(HashMap::new());
        }
        let total_f = total as f64;
        let mut stmt = conn
            .prepare("SELECT tool_name, COUNT(*) FROM mcp_invocations GROUP BY tool_name")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map_err(|e| e.to_string())?;
        let mut map = HashMap::new();
        for row in rows {
            let (name, c): (String, i64) = row.map_err(|e| e.to_string())?;
            map.insert(name, (c as f64) / total_f);
        }
        Ok(map)
    }

    pub fn get_recent_patterns(&self, min_count: u32) -> Result<Vec<PatternSummary>, String> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, timestamp, tool_name FROM mcp_invocations ORDER BY session_id, timestamp ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut by_session: HashMap<String, Vec<(i64, String)>> = HashMap::new();
        for row in rows {
            let (_, sid, ts, tool): (String, String, i64, String) = row.map_err(|e| e.to_string())?;
            by_session.entry(sid).or_default().push((ts, tool));
        }

        let mut pair_counts: HashMap<(String, String), u32> = HashMap::new();
        for chain in by_session.values_mut() {
            chain.sort_by_key(|(t, _)| *t);
            let tools: Vec<String> = chain.iter().map(|(_, n)| n.clone()).collect();
            for w in tools.windows(2) {
                let key = (w[0].clone(), w[1].clone());
                *pair_counts.entry(key).or_insert(0) += 1;
            }
        }

        let max_count = pair_counts.values().copied().max().unwrap_or(0).max(1);
        let mut summaries: Vec<PatternSummary> = pair_counts
            .into_iter()
            .filter(|(_, c)| *c >= min_count)
            .map(|((a, b), count)| PatternSummary {
                pattern: vec![a, b],
                count,
                confidence: count as f64 / max_count as f64,
            })
            .collect();
        summaries.sort_by(|x, y| y.count.cmp(&x.count).then_with(|| x.pattern.cmp(&y.pattern)));
        Ok(summaries)
    }

    pub fn get_user_profile(&self) -> Result<UserProfile, String> {
        let conn = self.connect()?;
        let total_invocations: u64 = conn
            .query_row("SELECT COUNT(*) FROM mcp_invocations", [], |r| {
                Ok(r.get::<_, i64>(0)? as u64)
            })
            .unwrap_or(0);

        let total_sessions: u64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT session_id) FROM mcp_invocations",
                [],
                |r| Ok(r.get::<_, i64>(0)? as u64),
            )
            .unwrap_or(0);

        let avg_tools_per_session = if total_sessions > 0 {
            total_invocations as f64 / total_sessions as f64
        } else {
            0.0
        };

        let tool_affinity = self.get_tool_affinity()?;

        let mut hour_hist = [0u32; 24];
        let mut stmt = conn
            .prepare("SELECT timestamp FROM mcp_invocations")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .map_err(|e| e.to_string())?;
        for ts in rows {
            let ts: i64 = ts.map_err(|e| e.to_string())?;
            let sec = ts / 1000;
            let sec_of_day = sec.rem_euclid(86_400);
            let hour = (sec_of_day / 3600) as usize;
            if hour < 24 {
                hour_hist[hour] += 1;
            }
        }
        let most_active_hour = hour_hist
            .iter()
            .enumerate()
            .max_by_key(|(_, c)| *c)
            .map(|(h, _)| h as u8)
            .unwrap_or(0);

        let top_patterns = self.get_recent_patterns(2)?;

        Ok(UserProfile {
            tool_affinity,
            total_invocations,
            total_sessions,
            avg_tools_per_session,
            most_active_hour,
            top_patterns,
        })
    }
}
