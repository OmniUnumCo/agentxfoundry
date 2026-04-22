//! `DeterministicLLMClient` — a trait-object wrapper that makes any
//! `LLMClient` behave deterministically on repeated calls.
//!
//! Guarantees:
//!
//! 1. `temperature` is forced to `0.0` before the inner call.
//! 2. A content-addressed cache is consulted first. On hit, the cached
//!    response is returned and the inner provider is never invoked.
//! 3. On miss, the inner provider is called, the response is cached, then
//!    returned.
//! 4. If `enforce_local_only` is set, non-local providers are refused at the
//!    wrapper boundary (see `ProviderKind::is_local`).
//!
//! The wrapper does NOT inject a seed by default — that is a per-provider
//! concern (Ollama honors `options.seed`, OpenAI honors `seed`, Anthropic's
//! messages API does not expose it). Providers that accept a seed should
//! store it on their own struct (see `OllamaClient::with_seed`).

use crate::llm::cache::ResponseCache;
use crate::llm::ProviderKind;
use crate::traits::LLMClient;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct DeterministicLLMClient {
    inner: Arc<dyn LLMClient>,
    cache: Arc<ResponseCache>,
    /// Recorded for cache-key uniqueness.
    provider_label: String,
    /// Recorded for cache-key uniqueness.
    model_label: String,
    /// `ProviderKind` of `inner`; used by `enforce_local_only`.
    kind: ProviderKind,
    /// When true, any non-local `kind` triggers a wrapper-level refusal.
    enforce_local_only: bool,
}

impl DeterministicLLMClient {
    pub fn new(
        inner: Arc<dyn LLMClient>,
        cache: Arc<ResponseCache>,
        provider_label: impl Into<String>,
        model_label: impl Into<String>,
        kind: ProviderKind,
    ) -> Self {
        Self {
            inner,
            cache,
            provider_label: provider_label.into(),
            model_label: model_label.into(),
            kind,
            enforce_local_only: false,
        }
    }

    pub fn with_enforce_local_only(mut self, enforce: bool) -> Self {
        self.enforce_local_only = enforce;
        self
    }

    pub fn provider_kind(&self) -> ProviderKind {
        self.kind
    }

    /// Normalize the params bag so cache keys are stable across callers that
    /// supply `None` vs. `Some(0.0)` etc.
    fn normalize_params(max_tokens: Option<usize>, temperature: Option<f64>) -> serde_json::Value {
        // Temperature is always 0.0 because we're forcing determinism; record
        // that explicitly so the cache key reflects reality.
        let mut obj = serde_json::Map::new();
        obj.insert("temperature".into(), json!(0.0_f64));
        if let Some(t) = max_tokens {
            obj.insert("max_tokens".into(), json!(t));
        } else {
            // Explicit null keeps the key stable.
            obj.insert("max_tokens".into(), json!(null));
        }
        // `temperature` came in as `Option<f64>`; ignore the caller-supplied
        // value because we override it, but keep the field present.
        let _ = temperature;
        serde_json::Value::Object(obj)
    }
}

#[async_trait]
impl LLMClient for DeterministicLLMClient {
    async fn complete(
        &self,
        prompt: &str,
        tier: &str,
        max_tokens: Option<usize>,
        _temperature: Option<f64>,
    ) -> Result<String, BoxError> {
        if self.enforce_local_only && !self.kind.is_local() {
            return Err(format!(
                "DeterministicLLMClient: enforce_local_only=true refuses non-local provider {:?}",
                self.kind
            )
            .into());
        }

        let normalized = Self::normalize_params(max_tokens, Some(0.0));

        // Cache lookup first.
        if let Some(hit) =
            self.cache
                .get(&self.provider_label, &self.model_label, tier, prompt, &normalized)?
        {
            return Ok(hit);
        }

        // Miss -> forward to inner, always with temperature=0.
        let response = self
            .inner
            .complete(prompt, tier, max_tokens, Some(0.0))
            .await?;

        // Store for replay. Persist-or-error is intentionally non-fatal:
        // a cache write failure should not prevent returning a valid response
        // to the caller. We log a warning instead (TODO: wire EventLogger).
        if let Err(e) =
            self.cache
                .put(&self.provider_label, &self.model_label, tier, prompt, &normalized, &response)
        {
            eprintln!("warning: response cache write failed: {}", e);
        }

        Ok(response)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Test fixture: counts calls and returns a fixed response.
    struct CountingClient {
        response: String,
        calls: AtomicUsize,
        /// If `true`, second call panics (proves cache replay avoided it).
        panic_on_second: bool,
    }

    #[async_trait]
    impl LLMClient for CountingClient {
        async fn complete(
            &self,
            _prompt: &str,
            _tier: &str,
            _max_tokens: Option<usize>,
            _temperature: Option<f64>,
        ) -> Result<String, BoxError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if self.panic_on_second && n >= 1 {
                panic!("second call should have been served from cache");
            }
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn cache_hit_on_repeat_call() {
        let cache = Arc::new(ResponseCache::in_memory().unwrap());
        let inner: Arc<dyn LLMClient> = Arc::new(CountingClient {
            response: "hello".to_string(),
            calls: AtomicUsize::new(0),
            panic_on_second: true, // ensure we never reach the inner twice
        });
        let wrapped = DeterministicLLMClient::new(
            inner.clone(),
            cache.clone(),
            "ollama",
            "llama3.2:3b",
            ProviderKind::Ollama,
        );

        let a = wrapped
            .complete("prompt", "execution", Some(64), None)
            .await
            .unwrap();
        let b = wrapped
            .complete("prompt", "execution", Some(64), None)
            .await
            .unwrap();
        assert_eq!(a, "hello");
        assert_eq!(b, "hello");
        assert_eq!(cache.count().unwrap(), 1);
    }

    #[tokio::test]
    async fn enforce_local_only_rejects_cloud_provider() {
        let cache = Arc::new(ResponseCache::in_memory().unwrap());
        let inner: Arc<dyn LLMClient> = Arc::new(CountingClient {
            response: "x".into(),
            calls: AtomicUsize::new(0),
            panic_on_second: false,
        });
        let wrapped = DeterministicLLMClient::new(
            inner,
            cache,
            "anthropic",
            "claude-sonnet-4",
            ProviderKind::Anthropic,
        )
        .with_enforce_local_only(true);

        let err = wrapped
            .complete("p", "strategic", None, None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("enforce_local_only"));
    }

    #[tokio::test]
    async fn different_prompts_are_distinct_cache_entries() {
        let cache = Arc::new(ResponseCache::in_memory().unwrap());
        let inner: Arc<dyn LLMClient> = Arc::new(CountingClient {
            response: "ok".into(),
            calls: AtomicUsize::new(0),
            panic_on_second: false,
        });
        let wrapped = DeterministicLLMClient::new(
            inner,
            cache.clone(),
            "ollama",
            "llama3.2:3b",
            ProviderKind::Ollama,
        );

        wrapped.complete("one", "execution", None, None).await.unwrap();
        wrapped.complete("two", "execution", None, None).await.unwrap();
        assert_eq!(cache.count().unwrap(), 2);
    }
}
