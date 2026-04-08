/// Vector search service using SQLite for storage and Ollama for embeddings
/// Falls back gracefully if Ollama is not available

use rusqlite::{Connection, Result as SqlResult, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const EMBEDDING_DIM: usize = 768; // nomic-embed-text dimension
const OLLAMA_URL: &str = "http://localhost:11434";
const EMBED_MODEL: &str = "nomic-embed-text";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: String,
    pub symbol: String,
    pub file: String,
    pub line: u32,
    pub snippet: String,
    pub symbol_type: String,
    pub embedding: Vec<f32>,
}

pub struct VectorStore {
    db_path: PathBuf,
}

impl VectorStore {
    pub fn new(repo_data_dir: &Path) -> Self {
        VectorStore {
            db_path: repo_data_dir.join("vectors.db"),
        }
    }

    fn open(&self) -> SqlResult<Connection> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS vectors (
                id TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                file TEXT NOT NULL,
                line INTEGER NOT NULL,
                snippet TEXT NOT NULL,
                symbol_type TEXT NOT NULL,
                embedding BLOB NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_symbol ON vectors(symbol);
        ")?;
        Ok(conn)
    }

    pub fn insert(&self, entry: &VectorEntry) -> SqlResult<()> {
        let conn = self.open()?;
        let blob = embedding_to_blob(&entry.embedding);
        conn.execute(
            "INSERT OR REPLACE INTO vectors (id, symbol, file, line, snippet, symbol_type, embedding) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![entry.id, entry.symbol, entry.file, entry.line, entry.snippet, entry.symbol_type, blob],
        )?;
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.open()
            .and_then(|c| c.query_row("SELECT COUNT(*) FROM vectors", [], |r| r.get::<_, i64>(0)))
            .unwrap_or(0) as usize
    }

    /// Cosine similarity search — returns top-k results
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Vec<(VectorEntry, f32)> {
        let conn = match self.open() {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        let mut stmt = match conn.prepare(
            "SELECT id, symbol, file, line, snippet, symbol_type, embedding FROM vectors"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let rows = match stmt.query_map([], |row| {
            let blob: Vec<u8> = row.get(6)?;
            Ok(VectorEntry {
                id: row.get(0)?,
                symbol: row.get(1)?,
                file: row.get(2)?,
                line: row.get::<_, i64>(3)? as u32,
                snippet: row.get(4)?,
                symbol_type: row.get(5)?,
                embedding: blob_to_embedding(&blob),
            })
        }) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        let mut results: Vec<(VectorEntry, f32)> = rows
            .flatten()
            .map(|entry| {
                let score = cosine_similarity(query_embedding, &entry.embedding);
                (entry, score)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    pub fn clear(&self) -> SqlResult<()> {
        let conn = self.open()?;
        conn.execute("DELETE FROM vectors", [])?;
        Ok(())
    }
}

// ── Embedding generation (Ollama) ──────────────────────────────────────────

pub async fn get_embedding(text: &str) -> Result<Vec<f32>, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/embed", OLLAMA_URL))
        .json(&serde_json::json!({
            "model": EMBED_MODEL,
            "input": text
        }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Ollama connection failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Ollama error: {}", response.status()));
    }

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    // Ollama /api/embed returns { "embeddings": [[...]] }
    let embedding = body["embeddings"][0]
        .as_array()
        .ok_or("No embeddings in response")?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();

    Ok(embedding)
}

pub async fn is_ollama_available() -> bool {
    reqwest::Client::new()
        .get(format!("{}/api/tags", OLLAMA_URL))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub async fn has_embed_model() -> bool {
    let client = reqwest::Client::new();
    let Ok(resp) = client
        .get(format!("{}/api/tags", OLLAMA_URL))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await else { return false };

    let Ok(body) = resp.json::<serde_json::Value>().await else { return false };
    body["models"]
        .as_array()
        .map(|models| models.iter().any(|m| {
            m["name"].as_str().unwrap_or("").contains("nomic-embed")
        }))
        .unwrap_or(false)
}

// ── RRF fusion: blend BM25 + vector results ────────────────────────────────

pub fn rrf_fuse(
    bm25_results: &[crate::models::SearchResult],
    vector_results: &[(VectorEntry, f32)],
    k: f32,
) -> Vec<crate::models::SearchResult> {
    use std::collections::HashMap;

    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut entries: HashMap<String, crate::models::SearchResult> = HashMap::new();

    // BM25 results: rank 1..n
    for (rank, r) in bm25_results.iter().enumerate() {
        let key = format!("{}:{}", r.file, r.line);
        *scores.entry(key.clone()).or_default() += 1.0 / (k as f64 + rank as f64 + 1.0);
        entries.insert(key, r.clone());
    }

    // Vector results: rank 1..n
    for (rank, (entry, score)) in vector_results.iter().enumerate() {
        if *score < 0.3 { continue; } // ignore low-confidence matches
        let key = format!("{}:{}", entry.file, entry.line);
        *scores.entry(key.clone()).or_default() += 1.0 / (k as f64 + rank as f64 + 1.0);
        entries.entry(key.clone()).or_insert_with(|| crate::models::SearchResult {
            symbol: entry.symbol.clone(),
            file: entry.file.clone(),
            line: entry.line,
            snippet: entry.snippet.clone(),
            result_type: entry.symbol_type.clone(),
            score: *score,
            repo_name: String::new(),
        });
    }

    let mut fused: Vec<_> = scores.into_iter()
        .filter_map(|(key, rrf_score)| {
            entries.get(&key).map(|r| {
                let mut r = r.clone();
                r.score = rrf_score as f32;
                r
            })
        })
        .collect();

    fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    fused
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    // Return 0 for mismatched or empty dimensions rather than panicking
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}
