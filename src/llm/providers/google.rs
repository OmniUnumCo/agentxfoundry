//! Google Gemini `generateContent` REST client.
//!
//! Ports the Python client's `genai.GenerativeModel(...).generate_content(prompt)`
//! call. Gemini accepts `temperature` in the `generationConfig` object. As of
//! current Gemini API releases, a `seed` field is also honored for
//! deterministic-ish generation.
//!
//! Auth: the API key is passed as a `?key=` query parameter (Google's
//! convention for REST calls), NOT a Bearer token.

use crate::traits::LLMClient;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct GoogleClient {
    base_url: String,
    model: String,
    api_key: String,
    seed: Option<i64>,
    http: reqwest::Client,
}

impl GoogleClient {
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: "https://generativelanguage.googleapis.com".to_string(),
            model: model.into(),
            api_key: api_key.into(),
            seed: None,
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

    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    contents: Vec<Content<'a>>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content<'a> {
    role: &'a str,
    parts: Vec<Part<'a>>,
}

#[derive(Serialize)]
struct Part<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct GenerationConfig {
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxOutputTokens")]
    max_output_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<i64>,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Vec<CandidatePart>,
}

#[derive(Deserialize)]
struct CandidatePart {
    #[serde(default)]
    text: String,
}

#[async_trait]
impl LLMClient for GoogleClient {
    async fn complete(
        &self,
        prompt: &str,
        _tier: &str,
        max_tokens: Option<usize>,
        temperature: Option<f64>,
    ) -> Result<String, BoxError> {
        let body = GenerateRequest {
            contents: vec![Content {
                role: "user",
                parts: vec![Part { text: prompt }],
            }],
            generation_config: GenerationConfig {
                temperature: temperature.unwrap_or(0.0),
                max_output_tokens: max_tokens,
                seed: self.seed,
            },
        };

        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        );
        let resp = self
            .http
            .post(&url)
            .query(&[("key", self.api_key.as_str())])
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("google request failed ({}): {}", url, e))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("google {} returned {}: {}", url, status, text).into());
        }
        let parsed: GenerateResponse = serde_json::from_str(&text)
            .map_err(|e| format!("google returned non-JSON body: {} (body: {})", e, text))?;

        let candidate = parsed
            .candidates
            .into_iter()
            .next()
            .ok_or_else(|| -> BoxError { "google response had no candidates".into() })?;
        let out: String = candidate.content.parts.into_iter().map(|p| p.text).collect();
        if out.is_empty() {
            return Err("google response had no text parts".into());
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
    async fn sends_expected_generate_content_request() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/v1beta/models/gemini-pro:generateContent",
            ))
            .respond_with(json_encoded(json!({
                "candidates":[{
                    "content":{"parts":[{"text":"bonjour"}], "role":"model"},
                    "finishReason":"STOP"
                }]
            }))),
        );
        let client = GoogleClient::new("gemini-pro", "k").with_base_url(
            server.url("").to_string().trim_end_matches('/').to_string(),
        );
        let out = client
            .complete("hello", "tactical", Some(256), Some(0.0))
            .await
            .unwrap();
        assert_eq!(out, "bonjour");
    }

    #[tokio::test]
    async fn errors_on_403() {
        let server = Server::run();
        server.expect(
            Expectation::matching(request::method_path(
                "POST",
                "/v1beta/models/gemini-pro:generateContent",
            ))
            .respond_with(status_code(403).body("forbidden")),
        );
        let client = GoogleClient::new("gemini-pro", "bad").with_base_url(
            server.url("").to_string().trim_end_matches('/').to_string(),
        );
        let err = client.complete("p", "tactical", None, None).await.unwrap_err();
        assert!(err.to_string().contains("403"));
    }
}
