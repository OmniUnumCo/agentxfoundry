# Requirements — Deterministic LLM Substrate

**Module:** `src/llm/`
**Status:** Implemented (4 providers + wrapper + cache + router)
**Source:** `src/llm/{mod.rs, deterministic.rs, cache.rs, router.rs, providers/*}`

## 1. Purpose

Provide a uniform, deterministic-by-construction interface for invoking language models. Replace the Python `llm_integrations/clients.py` multi-provider wrapper with compiled Rust code where every call is either replayable from cache or pinned at `temperature=0` with a threaded seed.

## 2. Functional requirements

**FR-LLM-01.** The system SHALL provide a single `LLMClient` async trait (`complete(prompt, tier, max_tokens, temperature) -> Result<String>`) shared across all providers.

**FR-LLM-02.** The system SHALL ship concrete implementations for:
  - Ollama (local HTTP, `/api/generate`)
  - Anthropic Claude (`/v1/messages`)
  - OpenAI-compatible (`/v1/chat/completions`) — covers OpenAI, Groq, Mistral, Devstral, Kimi, Kimi Coder, Qwen, Qwen Coder
  - Google Gemini (`/v1beta/models/<model>:generateContent`)

**FR-LLM-03.** The system SHALL provide a `DeterministicLLMClient` wrapper that:
  - Forces `temperature = 0.0` regardless of caller input
  - Consults a content-addressed cache before invoking the inner provider
  - Stores misses in the cache for replay
  - Optionally refuses non-local providers (`enforce_local_only`)

**FR-LLM-04.** The cache key SHALL be `blake3(provider || 0x1F || model || 0x1F || tier || 0x1F || prompt || 0x1F || normalized_params_json)`. Deterministic, collision-resistant, includes all inputs that affect response.

**FR-LLM-05.** The cache SHALL persist to SQLite (default at `$STATE_DIR/llm_cache.db`) with WAL mode, and support in-memory mode for tests.

**FR-LLM-06.** The system SHALL provide a `LlmRouter` that maps logical tiers (`strategic` / `tactical` / `supervision` / `execution_guidance` / `execution`) to concrete `(provider, model, base_url?, seed?)` tuples.

**FR-LLM-07.** Router configuration SHALL be loaded from `config/llm.toml` with `RR_LLM_CONFIG_PATH` env override. When no file is present, compiled-in local defaults (all tiers → Ollama `llama3.2:3b`) SHALL apply.

**FR-LLM-08.** Provider API keys SHALL be read from environment variables (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GROQ_API_KEY`, etc.) at daemon construction time. API keys MUST NOT be persisted in the event log or cache.

**FR-LLM-09.** Each provider SHALL surface non-2xx HTTP responses as an `Err(BoxError)` with the status code and response body for diagnostics.

**FR-LLM-10.** Each provider SHALL enforce a request timeout (default 300 seconds). A hung request MUST NOT block the caller indefinitely.

## 3. Non-functional requirements

**NFR-LLM-01. Determinism — pure tier.** For every `DeterministicLLMClient` call where the cache is warm, output MUST be byte-identical to the first call. Proven by unit test with a panic-on-second-call fixture.

**NFR-LLM-02. Determinism — provider tier.** Ollama calls with `seed=fixed + temperature=0` SHALL produce reproducible output (relies on Ollama's contract; not a Rust-side guarantee).

**NFR-LLM-03. Prompt injection avoidance.** The cache key includes the full prompt. Two callers submitting "same" prompt with different whitespace/newlines produce different keys — intentional; prevents accidental cross-context cache collision.

**NFR-LLM-04. Key stability across rust versions.** BLAKE3 digests are cryptographically stable; upgrading the blake3 crate MUST NOT invalidate existing cache entries (tested by `key_is_deterministic`).

**NFR-LLM-05. No synchronous blocking in async contexts.** All HTTP calls use `reqwest` async client; no `.block_on` in provider code.

**NFR-LLM-06. Secret handling.** API keys are held on provider struct fields only. They MUST NOT appear in `Debug` output (provider structs do not derive `Debug`) nor in cache keys / event payloads.

## 4. Scope

**In scope:**
- Ollama (local), Anthropic, OpenAI-compat (with per-provider base URLs), Google Gemini
- Temperature override, seed threading, content cache
- TOML-driven tier routing
- Error propagation with diagnostic context

**Out of scope (this module):**
- Streaming responses (request/response only)
- Multi-turn conversation history
- Function calling / tool use
- Embedding generation (see `src/vectordb/embeddings.rs`)
- Retry / circuit-breaker logic (caller concern)

## 5. Acceptance criteria

1. An `OllamaClient` pointed at `httptest`-mocked Ollama SHALL return exactly the `response` field from the mock body.
2. An `AnthropicClient` SHALL concatenate all `content[].text` blocks from the response.
3. An `OpenAiCompatClient` SHALL extract `choices[0].message.content` and surface "no choices" as an Err.
4. A `GoogleClient` SHALL concatenate `candidates[0].content.parts[].text`.
5. `ResponseCache::key` SHALL return identical hex strings across 1000 invocations on fixed inputs.
6. `DeterministicLLMClient` wrapping a mock panic-on-second-call client SHALL not panic on the second request — cache serves it.
7. `LlmRouter::route("strategic")` SHALL return `Some(TierRoute)` for all five tiers under default config.
8. Missing config file path SHALL fall back to defaults without error.
