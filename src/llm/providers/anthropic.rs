//! Anthropic Claude `/v1/messages` client.
//!
//! Ports the Python client's `self.client.messages.create(model=..., max_tokens=4096, ...)`
//! call. Anthropic's messages API does NOT expose `seed`; determinism relies
//! on `temperature=0` plus the DeterministicLLMClient cache for exact replay.

use crate::traits::LLMClient;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct AnthropicClient {
    base_url: String,
    model: String,
    api_key: String,
    /// Anthropic's `anthropic-version` header. Pinning this to a known-good
    /// version is part of the determinism contract (a bumped API version may
    /// change default behavior).
    anthropic_version: String,
    default_max_tokens: usize,
    http: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: "https://api.anthropic.com".to_string(),
            model: model.into(),
            api_key: api_key.into(),
            anthropic_version: "2023-06-01".to_string(),
            default_max_tokens: 4096,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("build reqwest client"),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    pub fn with_anthropic_version(mut self, version: impl Into<String>) -> Self {
        self.anthropic_version = version.into();
        self
    }

    pub fn with_default_max_tokens(mut self, n: usize) -> Self {
        self.default_max_tokens = n;
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    messages: Vec<MessageIn<'a>>,
    max_tokens: usize,
    temperature: f64,
}

#[derive(Serialize)]
struct MessageIn<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    kind: String,
    #[serde(default)]
    text: String,
}

#[async_trait]
impl LLMClient for AnthropicClient {
    async fn complete(
        &self,
        prompt: &str,
        _tier: &str,
        max_tokens: Option<usize>,
        temperature: Option<f64>,
    ) -> Result<String, BoxError> {
        let body = MessagesRequest {
            model: &self.model,
            messages: vec![MessageIn {
                role: "user",
                content: prompt,
            }],
            max_tokens: max_tokens.unwrap_or(self.default_max_tokens),
            temperature: temperature.unwrap_or(0.0),
        };

        let url = format!("{}/v1/messages", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.anthropic_version)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("anthropic request failed ({}): {}", url, e))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("anthropic {} returned {}: {}", url, status, text).into());
        }
        let parsed: MessagesResponse = serde_json::from_str(&text)
            .map_err(|e| format!("anthropic returned non-JSON body: {} (body: {})", e, text))?;

        // Concatenate all text blocks (Anthropic may split into multiple).
        let out: String = parsed.content.into_iter().map(|b| b.text).collect();
        if out.is_empty() {
            return Err("anthropic response had no text content".into());
        }
        Ok(out)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use httptest::{matchers::*, responders::*, Expectation, Server};
    use serde_json::json;

    #[tokio::test]
    async fn sends_expected_messages_request() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/v1/messages")).respond_with(
                json_encoded(json!({
                    "id": "msg_01",
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {"type": "text", "text": "hello "},
                        {"type": "text", "text": "world"}
                    ],
                    "model": "claude-sonnet-4-20250514",
                    "stop_reason": "end_turn"
                })),
            ),
        );
        let client =
            AnthropicClient::new("claude-sonnet-4-20250514", "sk-ant-test").with_base_url(
                server.url("").to_string().trim_end_matches('/').to_string(),
            );
        let out = client.complete("hi", "strategic", Some(4096), Some(0.0)).await.unwrap();
        assert_eq!(out, "hello world");
    }

    #[tokio::test]
    async fn errors_on_401() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/v1/messages"))
                .respond_with(status_code(401).body("unauthorized")),
        );
        let client = AnthropicClient::new("claude", "bad-key")
            .with_base_url(server.url("").to_string().trim_end_matches('/').to_string());
        let err = client.complete("p", "strategic", None, None).await.unwrap_err();
        assert!(err.to_string().contains("401"));
    }

    #[tokio::test]
    async fn errors_when_no_content_text() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/v1/messages"))
                .respond_with(json_encoded(json!({"content":[]}))),
        );
        let client = AnthropicClient::new("claude", "k")
            .with_base_url(server.url("").to_string().trim_end_matches('/').to_string());
        let err = client.complete("p", "strategic", None, None).await.unwrap_err();
        assert!(err.to_string().contains("no text content"));
    }
}
