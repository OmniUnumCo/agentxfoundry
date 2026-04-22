//! Embedding generation.
//!
//! The [`Embedder`] trait is intentionally minimal: one method, `embed(text)`.
//! Implementations are expected to be deterministic once the model is pinned.
//!
//! [`OllamaEmbedder`] calls Ollama's `/api/embeddings` endpoint with a fixed
//! model (default `nomic-embed-text`). Identical text + same model produces
//! identical vectors on the same Ollama version.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, BoxError>;
    fn model(&self) -> &str;
}

/// Ollama `/api/embeddings` adapter. Stateless other than the HTTP client.
pub struct OllamaEmbedder {
    base_url: String,
    model: String,
    http: reqwest::Client,
}

impl OllamaEmbedder {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("build reqwest client"),
        }
    }

    /// Default local Ollama + nomic-embed-text.
    pub fn local_default() -> Self {
        Self::new("http://localhost:11434", "nomic-embed-text")
    }
}

#[derive(Serialize)]
struct EmbeddingsRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingsResponse {
    embedding: Vec<f32>,
}

#[async_trait]
impl Embedder for OllamaEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, BoxError> {
        let url = format!("{}/api/embeddings", self.base_url);
        let req = EmbeddingsRequest {
            model: &self.model,
            prompt: text,
        };
        let resp = self
            .http
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("embedding request failed ({}): {}", url, e))?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("embedding {} returned {}: {}", url, status, body).into());
        }
        let parsed: EmbeddingsResponse = serde_json::from_str(&body)
            .map_err(|e| format!("embedding returned non-JSON body: {} (body: {})", e, body))?;
        Ok(parsed.embedding)
    }

    fn model(&self) -> &str {
        &self.model
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use httptest::{matchers::*, responders::*, Expectation, Server};

    #[tokio::test]
    async fn embed_returns_vector_from_server() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/api/embeddings")).respond_with(
                json_encoded(serde_json::json!({
                    "embedding": [0.1, 0.2, 0.3, 0.4]
                })),
            ),
        );

        let embedder = OllamaEmbedder::new(
            server.url("").to_string().trim_end_matches('/'),
            "nomic-embed-text",
        );
        let v = embedder.embed("hello").await.unwrap();
        assert_eq!(v, vec![0.1, 0.2, 0.3, 0.4]);
        assert_eq!(embedder.model(), "nomic-embed-text");
    }

    #[tokio::test]
    async fn embed_surfaces_http_error() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/api/embeddings"))
                .respond_with(status_code(500).body("model not pulled")),
        );
        let embedder = OllamaEmbedder::new(
            server.url("").to_string().trim_end_matches('/'),
            "nomic-embed-text",
        );
        let err = embedder.embed("hello").await.unwrap_err();
        assert!(err.to_string().contains("500"));
    }
}
