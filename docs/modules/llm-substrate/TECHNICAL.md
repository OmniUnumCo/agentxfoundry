# Technical Specification — Deterministic LLM Substrate

## 1. Module map

| File | Responsibility |
|---|---|
| `src/traits.rs` | `LLMClient` async trait definition |
| `src/llm/mod.rs` | `LLMTier`, `ProviderKind` enums; re-exports |
| `src/llm/cache.rs` | `ResponseCache` (SQLite-backed, BLAKE3-keyed) |
| `src/llm/deterministic.rs` | `DeterministicLLMClient` wrapper |
| `src/llm/router.rs` | `LlmRouter`, `RouterConfig`, `TierRoute`, TOML loader |
| `src/llm/providers/mod.rs` | Re-exports |
| `src/llm/providers/ollama.rs` | `OllamaClient` — `/api/generate` |
| `src/llm/providers/anthropic.rs` | `AnthropicClient` — `/v1/messages` |
| `src/llm/providers/openai_compat.rs` | `OpenAiCompatClient` — `/v1/chat/completions` |
| `src/llm/providers/google.rs` | `GoogleClient` — `/v1beta/models/<m>:generateContent` |

## 2. Core trait

```rust
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn complete(
        &self,
        prompt: &str,
        tier: &str,
        max_tokens: Option<usize>,
        temperature: Option<f64>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}
```

Single method, string-typed tier (kept framework-free), fully async.

## 3. Provider constructors and request shapes

### Ollama
```rust
OllamaClient::new(base_url, model)
    .with_seed(42)
    .with_num_predict(2048)
```
Request body:
```json
{"model":"...", "prompt":"...", "stream":false,
 "options":{"temperature":0.0, "seed":42, "num_predict":2048}}
```

### Anthropic
```rust
AnthropicClient::new(model, api_key)
    .with_anthropic_version("2023-06-01")
    .with_default_max_tokens(4096)
```
Headers: `x-api-key`, `anthropic-version`, `content-type: application/json`
Request body: `{model, messages: [{role: "user", content: prompt}], max_tokens, temperature}`

### OpenAI-compat
```rust
OpenAiCompatClient::openai(model, api_key)    // → https://api.openai.com/v1
OpenAiCompatClient::groq(model, api_key)       // → https://api.groq.com/openai/v1
OpenAiCompatClient::mistral(model, api_key)    // → https://api.mistral.ai/v1
OpenAiCompatClient::kimi(model, api_key)       // → https://api.moonshot.cn/v1
OpenAiCompatClient::qwen(model, api_key)       // → dashscope-intl...
OpenAiCompatClient::new(base_url, model, api_key).with_seed(42)  // custom
```
Headers: `Authorization: Bearer <key>`
Request body: `{model, messages: [{role: "user", content: prompt}], temperature, max_tokens?, seed?}`

### Google
```rust
GoogleClient::new(model, api_key).with_seed(42)
```
Auth via query: `?key=<api_key>`
Request body: `{contents: [{role:"user", parts:[{text:prompt}]}], generationConfig: {temperature, maxOutputTokens?, seed?}}`

## 4. Cache key algorithm

```rust
pub fn key(
    provider: &str,
    model: &str,
    tier: &str,
    prompt: &str,
    normalized_params: &serde_json::Value,
) -> String {
    const FIELD_SEP: u8 = 0x1F;
    let mut hasher = Hasher::new();
    hasher.update(provider.as_bytes());
    hasher.update(&[FIELD_SEP]);
    hasher.update(model.as_bytes());
    hasher.update(&[FIELD_SEP]);
    hasher.update(tier.as_bytes());
    hasher.update(&[FIELD_SEP]);
    hasher.update(prompt.as_bytes());
    hasher.update(&[FIELD_SEP]);
    let params_text = serde_json::to_string(normalized_params).unwrap_or_default();
    hasher.update(params_text.as_bytes());
    hasher.finalize().to_hex().to_string()
}
```

32-byte BLAKE3 digest, hex-encoded → 64-character stable string.

## 5. Cache SQLite schema

```sql
CREATE TABLE llm_cache (
    key        TEXT PRIMARY KEY,
    provider   TEXT NOT NULL,
    model      TEXT NOT NULL,
    tier       TEXT NOT NULL,
    response   TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

No foreign keys. No indices beyond primary. Grows monotonically (TTL/eviction deferred).

## 6. DeterministicLLMClient API

```rust
pub struct DeterministicLLMClient {
    inner: Arc<dyn LLMClient>,
    cache: Arc<ResponseCache>,
    provider_label: String,
    model_label: String,
    kind: ProviderKind,
    enforce_local_only: bool,
}

impl DeterministicLLMClient {
    pub fn new(inner, cache, provider_label, model_label, kind) -> Self;
    pub fn with_enforce_local_only(self, enforce: bool) -> Self;
    pub fn provider_kind(&self) -> ProviderKind;
}
```

`impl LLMClient for DeterministicLLMClient` forces `temperature=0.0`, consults cache, calls inner on miss, stores response.

## 7. Router configuration format

TOML shape:
```toml
[tiers.<tier_name>]
provider = "ollama" | "anthropic" | "openai_compat" | "google" | "kimi" | "onnx"
model    = "<model-id>"
base_url = "<optional-url>"
seed     = <optional-i64>
```

In Rust:
```rust
pub struct RouterConfig { tiers: HashMap<String, TierRoute> }
pub struct TierRoute {
    pub provider: ProviderKind,
    pub model: String,
    pub base_url: Option<String>,
    pub seed: Option<i64>,
}
```

## 8. Environment variables consumed

| Variable | Purpose |
|---|---|
| `RR_LLM_CONFIG_PATH` | Override TOML router config path |
| `OLLAMA_HOST` | Ollama base URL (default `http://localhost:11434`) |
| `ANTHROPIC_API_KEY` | Anthropic provider API key |
| `OPENAI_API_KEY` | OpenAI / Groq / Mistral / Kimi / Qwen fallback chain |
| `GROQ_API_KEY` | Explicit Groq key (falls back from OPENAI_API_KEY) |
| `MISTRAL_API_KEY` | Explicit Mistral key |
| `KIMI_API_KEY` / `MOONSHOT_API_KEY` | Kimi / Moonshot key |
| `QWEN_API_KEY` | Qwen key |
| `GOOGLE_API_KEY` | Google Gemini key |

## 9. Test coverage

| File | Tests |
|---|---|
| `cache.rs` | 4 — key stability, field mutation propagation, put/get roundtrip, idempotent put |
| `deterministic.rs` | 3 — cache-hit replay (panic-on-second-call), enforce_local_only refusal, distinct-prompt distinct-cache |
| `router.rs` | 4 — default covers all tiers, TOML roundtrip, missing-file fallback, unknown-tier returns None |
| `providers/ollama.rs` | 2 — request shape, error propagation |
| `providers/openai_compat.rs` | 4 — request shape, non-200, no choices, convenience constructors |
| `providers/anthropic.rs` | 3 — request shape, 401, empty content |
| `providers/google.rs` | 2 — request shape, 403 |

Total: **22 unit tests** covering the substrate, all via `httptest` mocks (no live network).

## 10. Performance characteristics

- **Cache hit:** Single SQLite SELECT by primary key (~sub-millisecond)
- **Cache miss:** HTTP POST + response parse + SQLite INSERT. Dominated by model latency (seconds). Cache write is non-fatal; warning on failure.
- **Memory:** Cache lives on disk. In-process state is an `Arc<ResponseCache>` + provider HTTP clients. Negligible resident footprint.

## 11. Threading and concurrency

- `ResponseCache` wraps its `Connection` in `Mutex`. Concurrent readers serialize on the mutex; SQLite WAL allows concurrent readers at the DB layer, but the `Mutex` is the current bottleneck. Acceptable for daemon-scale load.
- Provider clients hold a `reqwest::Client` with built-in connection pool; safe to share across tasks.
- `DeterministicLLMClient` is `Send + Sync` via its trait bound and `Arc` fields.

## 12. File path index

- `src/traits.rs` — `LLMClient` trait definition
- `src/llm/mod.rs` — module root, `LLMTier`, `ProviderKind`
- `src/llm/cache.rs::ResponseCache::key` — cache key algorithm
- `src/llm/deterministic.rs::DeterministicLLMClient::complete` — wrapper dispatch
- `src/llm/router.rs::LlmRouter::from_env_or_default` — config loading
- `src/llm/providers/ollama.rs:OllamaClient::complete` — Ollama HTTP implementation
- `src/llm/providers/anthropic.rs:AnthropicClient::complete` — Anthropic HTTP implementation
- `src/llm/providers/openai_compat.rs:OpenAiCompatClient::complete` — OpenAI-compat HTTP implementation
- `src/llm/providers/google.rs:GoogleClient::complete` — Google HTTP implementation
- `src/daemon.rs::build_llm_from_router` — daemon-side provider construction
