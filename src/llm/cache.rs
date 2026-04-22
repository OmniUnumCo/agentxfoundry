//! Content-addressed response cache.
//!
//! Keys are `blake3(provider || 0x1F || model || 0x1F || tier || 0x1F || prompt
//! || 0x1F || normalized_params_json)` hex-encoded. 0x1F is the ASCII unit
//! separator; it cannot appear inside any of the inputs, so the concatenation
//! is unambiguous.
//!
//! The cache is a tiny dedicated SQLite file (`llm_cache.db`) so replay is
//! isolated from RR's main event log. A fresh rational-reserve installation
//! can ship without a cache file; it will be created on first write.

use blake3::Hasher;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Field separator used inside cache keys. Chosen because 0x1F does not occur
/// in UTF-8 prompts or model identifiers in practice.
const FIELD_SEP: u8 = 0x1F;

pub struct ResponseCache {
    conn: Mutex<Connection>,
}

impl ResponseCache {
    /// Open or create the cache at `db_path`. Creates the parent directory if
    /// necessary. Uses SQLite WAL for concurrent read access.
    pub fn open(db_path: &Path) -> Result<Self, BoxError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS llm_cache (
                 key        TEXT PRIMARY KEY,
                 provider   TEXT NOT NULL,
                 model      TEXT NOT NULL,
                 tier       TEXT NOT NULL,
                 response   TEXT NOT NULL,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open an in-memory cache for tests.
    pub fn in_memory() -> Result<Self, BoxError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS llm_cache (
                 key        TEXT PRIMARY KEY,
                 provider   TEXT NOT NULL,
                 model      TEXT NOT NULL,
                 tier       TEXT NOT NULL,
                 response   TEXT NOT NULL,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Compute the cache key for a call. Exposed so tests can assert stability.
    pub fn key(
        provider: &str,
        model: &str,
        tier: &str,
        prompt: &str,
        normalized_params: &serde_json::Value,
    ) -> String {
        let mut hasher = Hasher::new();
        hasher.update(provider.as_bytes());
        hasher.update(&[FIELD_SEP]);
        hasher.update(model.as_bytes());
        hasher.update(&[FIELD_SEP]);
        hasher.update(tier.as_bytes());
        hasher.update(&[FIELD_SEP]);
        hasher.update(prompt.as_bytes());
        hasher.update(&[FIELD_SEP]);
        // Serialize params in a stable canonical form. serde_json's Value
        // uses BTreeMap for objects, so ordering is deterministic.
        let params_text = serde_json::to_string(normalized_params).unwrap_or_default();
        hasher.update(params_text.as_bytes());
        hasher.finalize().to_hex().to_string()
    }

    pub fn get(
        &self,
        provider: &str,
        model: &str,
        tier: &str,
        prompt: &str,
        normalized_params: &serde_json::Value,
    ) -> Result<Option<String>, BoxError> {
        let key = Self::key(provider, model, tier, prompt, normalized_params);
        let conn = self.conn.lock().unwrap();
        let response: Option<String> = conn
            .query_row(
                "SELECT response FROM llm_cache WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?;
        Ok(response)
    }

    pub fn put(
        &self,
        provider: &str,
        model: &str,
        tier: &str,
        prompt: &str,
        normalized_params: &serde_json::Value,
        response: &str,
    ) -> Result<(), BoxError> {
        let key = Self::key(provider, model, tier, prompt, normalized_params);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO llm_cache (key, provider, model, tier, response)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![key, provider, model, tier, response],
        )?;
        Ok(())
    }

    pub fn count(&self) -> Result<usize, BoxError> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM llm_cache", [], |r| r.get(0))?;
        Ok(n as usize)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn key_is_deterministic() {
        let a = ResponseCache::key("ollama", "llama3.2:3b", "execution", "hello", &json!({"t":0}));
        let b = ResponseCache::key("ollama", "llama3.2:3b", "execution", "hello", &json!({"t":0}));
        assert_eq!(a, b);
    }

    #[test]
    fn key_changes_when_any_field_changes() {
        let base = ResponseCache::key("ollama", "m", "execution", "p", &json!({}));
        assert_ne!(base, ResponseCache::key("openai", "m", "execution", "p", &json!({})));
        assert_ne!(base, ResponseCache::key("ollama", "m2", "execution", "p", &json!({})));
        assert_ne!(base, ResponseCache::key("ollama", "m", "strategic", "p", &json!({})));
        assert_ne!(base, ResponseCache::key("ollama", "m", "execution", "p2", &json!({})));
        assert_ne!(base, ResponseCache::key("ollama", "m", "execution", "p", &json!({"seed":1})));
    }

    #[test]
    fn put_get_roundtrip() {
        let cache = ResponseCache::in_memory().unwrap();
        assert_eq!(
            cache.get("ollama", "m", "execution", "p", &json!({})).unwrap(),
            None
        );
        cache
            .put("ollama", "m", "execution", "p", &json!({}), "answer")
            .unwrap();
        assert_eq!(
            cache.get("ollama", "m", "execution", "p", &json!({})).unwrap(),
            Some("answer".to_string())
        );
        assert_eq!(cache.count().unwrap(), 1);
    }

    #[test]
    fn put_is_idempotent_on_same_key() {
        let cache = ResponseCache::in_memory().unwrap();
        cache.put("ollama", "m", "execution", "p", &json!({}), "v1").unwrap();
        cache.put("ollama", "m", "execution", "p", &json!({}), "v2").unwrap();
        assert_eq!(cache.count().unwrap(), 1);
        // v2 overwrites v1 because INSERT OR REPLACE.
        assert_eq!(
            cache.get("ollama", "m", "execution", "p", &json!({})).unwrap(),
            Some("v2".to_string())
        );
    }
}
