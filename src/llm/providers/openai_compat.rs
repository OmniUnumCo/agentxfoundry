//! OpenAI-compatible Chat Completions client.
//!
//! Speaks the `/v1/chat/completions` protocol used by OpenAI and a long list
//! of "OpenAI-compatible" providers. One adapter covers all of them by
//! varying the base URL and model name:
//!
//! | Provider  | base_url                          | model examples             |
//! |-----------|-----------------------------------|----------------------------|
//! | OpenAI    | `https://api.openai.com/v1`       | `gpt-4`, `gpt-4o`          |
//! | Groq      | `https://api.groq.com/openai/v1`  | `llama-3.1-70b-versatile`  |
//! | Mistral   | `https://api.mistral.ai/v1`       | `mistral-large`, `devstral-small` |
//! | Kimi      | `https://api.moonshot.cn/v1`      | `kimi-k1-8k`, `moonshot-v1-128k` |
//! | Qwen      | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` | `qwen-coder-plus`, `qwen3-coder` |
//!
//! # Determinism
//!
//! The protocol exposes `temperature` and `seed`. The adapter threads both
//! through on every call; wrapping in [`crate::llm::DeterministicLLMClient`]
//! is the supported path for replay determinism.

use crate::traits::LLMClient;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct OpenAiCompatClient {
    base_url: String,
    model: String,
    api_key: String,
    seed: Option<i64>,
    http: reqwest::Client,
}

impl OpenAiCompatClient {
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            api_key: api_key.into(),
            seed: None,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("build reqwest client"),
        }
    }

    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    // Convenience constructors that capture the per-provider base URL.
    // `api_key` is supplied by the caller from env / config.

    pub fn openai(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::new("https://api.openai.com/v1", model, api_key)
    }

    pub fn groq(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::new("https://api.groq.com/openai/v1", model, api_key)
    }

    pub fn mistral(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::new("https://api.mistral.ai/v1", model, api_key)
    }

    pub fn kimi(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::new("https://api.moonshot.cn/v1", model, api_key)
    }

    pub fn qwen(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self::new(
            "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
            model,
            api_key,
        )
    }

    pub fn model(&self) -> &str {
        &self.model
    }
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<i64>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageOwned,
}

#[derive(Deserialize)]
struct ChatMessageOwned {
    content: String,
}

#[async_trait]
impl LLMClient for OpenAiCompatClient {
    async fn complete(
        &self,
        prompt: &str,
        _tier: &str,
        max_tokens: Option<usize>,
        temperature: Option<f64>,
    ) -> Result<String, BoxError> {
        let body = ChatRequest {
            model: &self.model,
            messages: vec![ChatMessage {
                role: "user",
                content: prompt,
            }],
            temperature: temperature.unwrap_or(0.0),
            max_tokens,
            seed: self.seed,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("openai-compat request failed ({}): {}", url, e))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("openai-compat {} returned {}: {}", url, status, text).into());
        }
        let parsed: ChatResponse = serde_json::from_str(&text)
            .map_err(|e| format!("openai-compat returned non-JSON body: {} (body: {})", e, text))?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| -> BoxError { "openai-compat response had no choices".into() })?;
        // An empty `content` string is treated as such -- do NOT substitute
        // "" for Err; the caller decides whether empty is acceptable.
        let _ = json!({}); // keep the serde_json import used if nothing else
        Ok(choice.message.content)
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
    async fn sends_expected_request_shape() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/chat/completions"))
                .respond_with(json_encoded(json!({
                    "choices":[
                        {"message":{"role":"assistant","content":"hi back"}}
                    ]
                }))),
        );
        let client = OpenAiCompatClient::new(
            server.url("").to_string().trim_end_matches('/'),
            "gpt-test",
            "fake-key",
        )
        .with_seed(7);
        let out = client
            .complete("hi", "execution", Some(64), Some(0.0))
            .await
            .unwrap();
        assert_eq!(out, "hi back");
    }

    #[tokio::test]
    async fn errors_on_non_200() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/chat/completions"))
                .respond_with(status_code(401).body("invalid_api_key")),
        );
        let client = OpenAiCompatClient::new(
            server.url("").to_string().trim_end_matches('/'),
            "gpt-test",
            "bad-key",
        );
        let err = client.complete("p", "execution", None, None).await.unwrap_err();
        assert!(err.to_string().contains("401"));
    }

    #[tokio::test]
    async fn errors_when_response_has_no_choices() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", "/chat/completions"))
                .respond_with(json_encoded(json!({"choices": []}))),
        );
        let client = OpenAiCompatClient::new(
            server.url("").to_string().trim_end_matches('/'),
            "gpt-test",
            "k",
        );
        let err = client.complete("p", "execution", None, None).await.unwrap_err();
        assert!(err.to_string().contains("no choices"));
    }

    #[test]
    fn convenience_constructors_set_base_urls() {
        let o = OpenAiCompatClient::openai("gpt-4", "k");
        assert_eq!(o.base_url(), "https://api.openai.com/v1");
        let g = OpenAiCompatClient::groq("llama-3.1-70b-versatile", "k");
        assert_eq!(g.base_url(), "https://api.groq.com/openai/v1");
        let m = OpenAiCompatClient::mistral("mistral-large", "k");
        assert_eq!(m.base_url(), "https://api.mistral.ai/v1");
        let k = OpenAiCompatClient::kimi("moonshot-v1-128k", "k");
        assert_eq!(k.base_url(), "https://api.moonshot.cn/v1");
        let q = OpenAiCompatClient::qwen("qwen-coder-plus", "k");
        assert!(q.base_url().contains("dashscope"));
    }
}
