//! Vector store for doctrine embeddings, contract-brief retrieval, and
//! retrieval-augmented prompt assembly.
//!
//! Slice E.1 ships the [`BruteForceStore`] — a pure-Rust in-memory
//! cosine-similarity search over a `Vec<(id, vector, metadata)>`. It is
//! correct, deterministic, and has no external dependencies. It's intended
//! for unit tests and small corpora (doctrine files are ~10-50 entries total).
//!
//! LanceDB-backed `LanceDbStore` lands in Slice E.3 behind the `lancedb`
//! Cargo feature — adding the heavy `arrow` dependency tree now is not
//! justified by the current volume.

use async_trait::async_trait;

pub mod embeddings;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Abstract vector store. Impls are expected to be concurrent-safe.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Upsert an embedding under `id`. Replaces any existing entry with the
    /// same id.
    async fn upsert(
        &self,
        id: &str,
        vec: &[f32],
        metadata: &serde_json::Value,
    ) -> Result<(), BoxError>;

    /// Return the `k` nearest entries by **cosine similarity** (higher score
    /// = more similar). Each result is `(id, cosine_similarity, metadata)`.
    async fn query(
        &self,
        vec: &[f32],
        k: usize,
    ) -> Result<Vec<(String, f32, serde_json::Value)>, BoxError>;

    /// Remove an entry by id. No-op if the id is not present.
    async fn delete(&self, id: &str) -> Result<(), BoxError>;

    /// Number of stored entries.
    async fn len(&self) -> Result<usize, BoxError>;
}

// ============================================================================
// BruteForceStore
// ============================================================================

use std::sync::Mutex;

struct Entry {
    id: String,
    vector: Vec<f32>,
    metadata: serde_json::Value,
}

/// In-memory brute-force cosine-similarity store. Deterministic, suitable for
/// small corpora and tests.
pub struct BruteForceStore {
    entries: Mutex<Vec<Entry>>,
}

impl BruteForceStore {
    pub fn new() -> Self {
        Self { entries: Mutex::new(Vec::new()) }
    }
}

impl Default for BruteForceStore {
    fn default() -> Self {
        Self::new()
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..len {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

#[async_trait]
impl VectorStore for BruteForceStore {
    async fn upsert(
        &self,
        id: &str,
        vec: &[f32],
        metadata: &serde_json::Value,
    ) -> Result<(), BoxError> {
        let mut entries = self.entries.lock().unwrap();
        if let Some(existing) = entries.iter_mut().find(|e| e.id == id) {
            existing.vector = vec.to_vec();
            existing.metadata = metadata.clone();
        } else {
            entries.push(Entry {
                id: id.to_string(),
                vector: vec.to_vec(),
                metadata: metadata.clone(),
            });
        }
        Ok(())
    }

    async fn query(
        &self,
        vec: &[f32],
        k: usize,
    ) -> Result<Vec<(String, f32, serde_json::Value)>, BoxError> {
        let entries = self.entries.lock().unwrap();
        let mut scored: Vec<(String, f32, serde_json::Value)> = entries
            .iter()
            .map(|e| (e.id.clone(), cosine(vec, &e.vector), e.metadata.clone()))
            .collect();
        // Sort by score descending. Ties break by id ascending for stability.
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        scored.truncate(k);
        Ok(scored)
    }

    async fn delete(&self, id: &str) -> Result<(), BoxError> {
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|e| e.id != id);
        Ok(())
    }

    async fn len(&self) -> Result<usize, BoxError> {
        Ok(self.entries.lock().unwrap().len())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn upsert_and_query_exact_match() {
        let store = BruteForceStore::new();
        store
            .upsert("a", &[1.0, 0.0, 0.0], &json!({"label": "a"}))
            .await
            .unwrap();
        store
            .upsert("b", &[0.0, 1.0, 0.0], &json!({"label": "b"}))
            .await
            .unwrap();
        store
            .upsert("c", &[0.0, 0.0, 1.0], &json!({"label": "c"}))
            .await
            .unwrap();

        let results = store.query(&[1.0, 0.0, 0.0], 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "a");
        assert!((results[0].1 - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn upsert_overwrites_existing() {
        let store = BruteForceStore::new();
        store
            .upsert("x", &[1.0, 0.0], &json!({"v": 1}))
            .await
            .unwrap();
        store
            .upsert("x", &[0.0, 1.0], &json!({"v": 2}))
            .await
            .unwrap();
        assert_eq!(store.len().await.unwrap(), 1);
        let results = store.query(&[0.0, 1.0], 1).await.unwrap();
        assert_eq!(results[0].0, "x");
        assert!((results[0].1 - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn delete_removes_entry() {
        let store = BruteForceStore::new();
        store.upsert("a", &[1.0], &json!(null)).await.unwrap();
        store.upsert("b", &[1.0], &json!(null)).await.unwrap();
        assert_eq!(store.len().await.unwrap(), 2);
        store.delete("a").await.unwrap();
        assert_eq!(store.len().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn query_ranking_is_deterministic_on_ties() {
        let store = BruteForceStore::new();
        // Two vectors identically close to the query. Tie-break by id ASC.
        store
            .upsert("zebra", &[1.0, 0.0], &json!({}))
            .await
            .unwrap();
        store
            .upsert("alpha", &[1.0, 0.0], &json!({}))
            .await
            .unwrap();
        let r = store.query(&[1.0, 0.0], 2).await.unwrap();
        assert_eq!(r[0].0, "alpha");
        assert_eq!(r[1].0, "zebra");
    }

    #[tokio::test]
    async fn query_tolerates_zero_vector() {
        let store = BruteForceStore::new();
        store
            .upsert("a", &[0.0, 0.0, 0.0], &json!({}))
            .await
            .unwrap();
        let r = store.query(&[1.0, 0.0, 0.0], 1).await.unwrap();
        assert_eq!(r.len(), 1);
        // Cosine with zero-norm returns 0.0 per our guard.
        assert!((r[0].1 - 0.0).abs() < 1e-6);
    }
}
