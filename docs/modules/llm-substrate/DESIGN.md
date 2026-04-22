# Design Specification — Deterministic LLM Substrate

## 1. Architecture

```
                  +----------------------+
                  |  LLMClient (trait)   |
                  +----------+-----------+
                             |
     +----------+------------+-----+-------------+
     |          |            |     |             |
     v          v            v     v             v
  Ollama  OpenAICompat  Anthropic Google      Kimi (OA-compat)
     \        |            |       /              |
      \       |            |      /               |
       v      v            v     v                v
          +---------------------------+
          |  DeterministicLLMClient   |
          |   temp=0 + seed + cache   |
          +-------------+-------------+
                        |
                        v
                +----------------+      +---------------+
                | ResponseCache  |<---->|   SQLite WAL  |
                | BLAKE3 key     |      | llm_cache.db  |
                +----------------+      +---------------+

                  +----------------------+
                  |     LlmRouter        |
                  |  tier -> TierRoute   |
                  +----------+-----------+
                             |
                             v
                    +-----------------+
                    |  config/llm.toml |
                    +-----------------+
```

## 2. Trait-based dependency injection

`LLMClient` lives in `src/traits.rs` (not `src/llm/`) because it is the shared dependency contract across subsystems. Providers and wrappers live under `src/llm/` and satisfy the trait.

Wrappers compose: `DeterministicLLMClient<OllamaClient>` is itself an `LLMClient`, so it can be passed to `MissionPlanner`, `SwarmOrchestrator`, `PredictiveAgent`, etc., without those modules knowing which concrete provider (or chain of wrappers) is underneath.

## 3. Determinism layers

Three independent guarantees, composable:

1. **Wrapper-level (always-on inside `DeterministicLLMClient`):**
   - `temperature = 0.0` forced at the trait-call boundary
   - `max_tokens` carried through
   - `seed` threading is the provider's responsibility (each provider exposes `with_seed`)
2. **Cache-level:**
   - `ResponseCache` in SQLite WAL, BLAKE3-keyed
   - Key includes provider, model, tier, prompt, and normalized params
   - Cache hit → inner provider is not called (byte-identical replay)
3. **Provider-level (per-call deterministic parameters):**
   - Ollama: `options.seed` + `options.temperature` honored
   - OpenAI-compat: top-level `seed` + `temperature` honored by OpenAI, Groq, Mistral
   - Anthropic: `temperature=0`; no seed exposed on messages API (best-effort)
   - Google: `generationConfig.temperature` + `generationConfig.seed` honored on recent Gemini versions

## 4. Cache key design

```
blake3(
    provider_label  ||  0x1F  ||
    model_label     ||  0x1F  ||
    tier            ||  0x1F  ||
    prompt          ||  0x1F  ||
    normalized_params_json
)
```

`0x1F` is the ASCII unit-separator control character. It does not appear in UTF-8 prompts or model identifiers in practice, making the concatenation unambiguous.

`normalized_params_json` is deliberately hand-built: `{"temperature": 0.0, "max_tokens": <n or null>}`. Using `serde_json::to_string` on a BTreeMap-backed `Value` guarantees field order, but we construct the map literally for clarity.

**Why include `tier` in the key** even though it's often redundant with provider+model: different tiers may use the same model temporarily during testing; the key prevents cross-tier cache collision.

**Why include `provider_label`:** two providers could happen to share a model name (e.g. `llama3:8b` on a self-hosted Ollama vs on a cloud Ollama). Distinct provider_label → distinct cache entry.

## 5. Router configuration

```toml
# config/llm.toml
[tiers.strategic]
provider = "ollama"
model    = "llama3.1:70b-instruct"
base_url = "http://localhost:11434"
seed     = 0

[tiers.tactical]
provider = "ollama"
model    = "llama3.1:8b-instruct"

[tiers.execution_guidance]
provider = "ollama"
model    = "qwen2.5-coder:7b"

[tiers.execution]
provider = "ollama"
model    = "llama3.2:3b"
```

Override via `RR_LLM_CONFIG_PATH`. Missing file → compiled-in all-local defaults.

## 6. Provider-specific design notes

### Ollama
- `POST /api/generate` with `{model, prompt, stream: false, options: {temperature, seed, num_predict}}`
- Response: `{response: string, done: bool, ...}` — concatenation not needed
- 5-minute timeout guards against thrashing model instances

### Anthropic
- `POST /v1/messages` with `x-api-key` + `anthropic-version: 2023-06-01`
- Response: `{content: [{type: "text", text: ...}]}` — we concatenate all text blocks
- `anthropic-version` is pinned; bumping it may change behavior and is a versioning boundary

### OpenAI-compat
- `POST /v1/chat/completions` with `Authorization: Bearer <key>`
- Body: `{model, messages: [{role: "user", content: prompt}], temperature, max_tokens?, seed?}`
- Response: `{choices: [{message: {content}}]}` — `choices[0].message.content` extracted
- Base URL varies per provider (OpenAI, Groq, Mistral, Kimi, Qwen) — convenience constructors handle each

### Google
- `POST /v1beta/models/<model>:generateContent?key=<key>`
- Body: `{contents: [{role: "user", parts: [{text}]}], generationConfig: {temperature, maxOutputTokens?, seed?}}`
- Response: `{candidates: [{content: {parts: [{text}]}}]}` — we concatenate all parts

## 7. Error semantics

All providers return `Result<String, Box<dyn std::error::Error + Send + Sync>>`. Error strings include:
- HTTP status code on non-2xx
- Response body snippet (first ~500 chars) for diagnostic context
- Originating URL

`DeterministicLLMClient` adds one layer: refusing non-local providers when `enforce_local_only = true`, with a distinct error prefix.

## 8. Daemon construction

`Daemon::new` calls `build_llm_from_router(router, cache, paths)` which:

1. Picks the "execution" tier route (the safest default) from the router
2. Constructs the concrete provider from `ProviderKind`
3. Pulls API keys from env (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc.)
4. Wraps the provider in `DeterministicLLMClient` bound to the cache
5. Returns `Arc<dyn LLMClient>` for injection into `DaemonState`

Tier-specific clients (e.g. strategic → Claude Sonnet, execution → local Llama) are a Slice E.3 extension; Slice E.2 uses one client for all tiers, routed via the `tier` parameter passed to `complete()`.

## 9. Key design decisions

1. **Provider strings as labels.** Cache entries are labelled with `ProviderKind`-lowercased strings (`"ollama"`, `"anthropic"`, etc.) rather than full config. Keeps keys short while preserving uniqueness.
2. **Force temperature=0 in wrapper.** Callers that pass `Some(0.5)` are silently overridden. Determinism is a contract, not a preference. A non-deterministic variant would be a separate wrapper.
3. **BLAKE3 over SHA-256.** BLAKE3 is faster and has stronger domain separation built in. Collision resistance sufficient for our keyspace.
4. **Single `LLMClient` trait, not per-tier traits.** Tier is a runtime argument, not a type-level constraint. Simplifies the substitution pattern.
5. **No retries at wrapper level.** Transient failures surface to the caller. Retry policy is a caller/daemon concern.
6. **API keys via env, not config file.** Keeps secrets out of version-controlled config. Matches Python client behavior.

## 10. Future extension paths

- **ONNX Runtime provider:** `src/llm/onnx.rs` (Slice E.3). In-process inference with pinned weights.
- **Streaming:** `complete_stream` trait method returning `impl Stream<Item = String>`.
- **Tier-per-model routing:** `DaemonState` holds a `HashMap<LLMTier, Arc<dyn LLMClient>>` built at construction time from the full router config.
- **Cache eviction:** currently grow-forever. Add TTL or size-based eviction once cache size becomes a concern.
