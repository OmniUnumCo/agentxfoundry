//! Ollama HTTP provider.
//!
//! Targets the Ollama `/api/generate` endpoint (request/response, not
//! streaming). Ollama honors `seed` and `temperature` in its options map, so
//! this is our primary deterministic driver.
//!
//! # Example
//!
//! ```no_run
//! use rr::llm::providers::OllamaClient;
//! use rr::traits::LLMClient;
//!
//! # async fn demo() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let client = OllamaClient::new("http://localhost:11434", "llama3.2:3b")
//!     .with_seed(42);
//! let out = client.complete("hello", "execution", None, Some(0.0)).await?;
//! println!("{}", out);
//! # Ok(())
//! # }
//! ```
//!
//! # Determinism
//!
//! `temperature=0` + `seed=fixed` + `num_predict` capped makes the endpoint
//! deterministic in practice. Two calls with identical
//! `(model, prompt, options)` should produce byte-identical outputs.

use crate::traits::LLMClient;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Ollama HTTP client.
pub struct OllamaClient {
    base_url: String,
    model: String,
    seed: Option<i64>,
    /// Upper bound on generated tokens (`num_predict`). `None` lets Ollama
    /// pick its default (usually unlimited until EOS).
    num_predict: Option<u32>,
    /// Underlying HTTP client. Kept on the struct so connection pooling works
    /// across calls.
    http: reqwest::Client,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            seed: None,
            num_predict: None,
            http: reqwest::Client::builder()
                // A request that hangs past 5 minutes almost certainly means the
                // model is thrashing; bail rather than block the agent forever.
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("build reqwest client"),
        }
    }

    /// Pin a seed for deterministic generation. Two calls with the same
    /// (prompt, model, seed, temperature=0) return byte-identical output
    /// under Ollama's current implementation.
    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn with_num_predict(mut self, n: u32) -> Self {
        self.num_predict = Some(n);
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    options: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    #[allow(dead_code)]
    #[serde(default)]
    done: bool,
}

#[async_trait]
impl LLMClient for OllamaClient {
    async fn complete(
        &self,
        prompt: &str,
        _tier: &str,
        max_tokens: Option<usize>,
        temperature: Option<f64>,
    ) -> Result<String, BoxError> {
        // Build the options bag. We deliberately default temperature to 0.0
        // here; `DeterministicLLMClient` will also overwrite it to 0.0 when
        // wrapping a provider, but a raw call (e.g. in a test) still gets
        // determinism by default.
        let mut options = json!({
            "temperature": temperature.unwrap_or(0.0),
        });
        if let Some(seed) = self.seed {
            options["seed"] = json!(seed);
        }
        if let Some(n) = max_tokens.map(|n| n as u32).or(self.num_predict) {
            options["num_predict"] = json!(n);
        }

        let req = OllamaGenerateRequest {
            model: &self.model,
            prompt,
            stream: false,
            options,
        };

        let url = format!("{}/api/generate", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("ollama request failed ({}): {}", url, e))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("ollama body read failed: {}", e))?;
        if !status.is_success() {
            return Err(format!("ollama {} returned {}: {}", url, status, body).into());
        }

        let parsed: OllamaGenerateResponse = serde_json::from_str(&body)
            .map_err(|e| format!("ollama returned non-JSON body: {} (body: {})", e, body))?;
        Ok(parsed.response)
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
    async fn ollama_client_sends_expected_request_shape() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/api/generate")).respond_with(
                json_encoded(serde_json::json!({
                    "model": "llama3.2:3b",
                    "response": "hello world",
                    "done": true
                })),
            ),
        );

        let client = OllamaClient::new(server.url("").to_string().trim_end_matches('/'), "llama3.2:3b")
            .with_seed(42);
        let out = client.complete("prompt", "execution", Some(64), Some(0.0)).await.unwrap();
        assert_eq!(out, "hello world");
    }

    #[tokio::test]
    async fn ollama_client_propagates_http_errors() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/api/generate"))
                .respond_with(status_code(500).body("model not loaded")),
        );

        let client = OllamaClient::new(
            server.url("").to_string().trim_end_matches('/'),
            "missing-model",
        );
        let err = client
            .complete("prompt", "execution", None, None)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("500"), "expected status in error, got {}", msg);
        assert!(
            msg.contains("model not loaded"),
            "expected body in error, got {}",
            msg
        );
    }
}
