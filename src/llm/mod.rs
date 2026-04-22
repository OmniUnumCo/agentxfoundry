//! Deterministic LLM inference layer.
//!
//! The `LLMClient` trait itself lives in `crate::traits` (shared with other
//! subsystems); this module adds:
//!
//! - **Providers** (`providers::ollama`, ...) — concrete implementations that
//!   satisfy `LLMClient` by talking to a specific backend.
//! - **DeterministicLLMClient** — a wrapper that pins `temperature=0`, threads
//!   a seed through where supported, and consults a content-addressed cache
//!   before delegating to the inner provider.
//! - **ResponseCache** — SQLite-backed cache keyed on
//!   `blake3(provider || model || prompt || tier || normalized_params)`.
//! - **Router** — config-driven mapping from an agent tier
//!   (`strategic` / `tactical` / `supervision` / `execution_guidance` /
//!   `execution`) to a concrete `(provider, model)` pair.
//!
//! Slice E.1 ships the **local-only** path via the Ollama provider.
//! Cloud providers (Anthropic, OpenAI-compat, Gemini, Kimi) land in E.2/E.3
//! behind the same `DeterministicLLMClient` wrapper.

pub mod cache;
pub mod deterministic;
pub mod providers;
pub mod router;

pub use cache::ResponseCache;
pub use deterministic::DeterministicLLMClient;
pub use router::{LlmRouter, RouterConfig, TierRoute};

/// Concrete backend classes. Used by the router and by determinism rules
/// (e.g. "strict-local mode refuses any non-local provider").
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Local HTTP surface (ollama, llama.cpp server, text-generation-inference).
    Ollama,
    /// In-process ONNX Runtime. Not implemented in E.1; reserved for E.3.
    Onnx,
    /// Anthropic Claude messages API.
    Anthropic,
    /// OpenAI Chat Completions protocol — also covers Groq, DeepSeek, Mistral,
    /// Qwen, Devstral via base-URL configuration.
    OpenAiCompat,
    /// Google Gemini REST.
    Google,
    /// Moonshot (Kimi / Kimi Coder).
    Kimi,
}

impl ProviderKind {
    /// True iff this provider runs fully on the local machine (no network).
    ///
    /// Used by `DeterministicLLMClient::enforce_local_only` to reject non-local
    /// providers at the trait boundary before a request is built.
    pub fn is_local(&self) -> bool {
        matches!(self, ProviderKind::Ollama | ProviderKind::Onnx)
    }
}

/// Logical routing tier, mirroring the strings returned by
/// [`crate::agent::tier_for`].
///
/// Tiers are strings in the `LLMClient::complete` call because that interface
/// is deliberately string-typed (the trait is framework-free). This enum is a
/// convenience for code that prefers exhaustive matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LLMTier {
    Strategic,
    Tactical,
    Supervision,
    ExecutionGuidance,
    Execution,
}

impl LLMTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            LLMTier::Strategic => "strategic",
            LLMTier::Tactical => "tactical",
            LLMTier::Supervision => "supervision",
            LLMTier::ExecutionGuidance => "execution_guidance",
            LLMTier::Execution => "execution",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "strategic" => LLMTier::Strategic,
            "tactical" => LLMTier::Tactical,
            "supervision" => LLMTier::Supervision,
            "execution_guidance" => LLMTier::ExecutionGuidance,
            "execution" => LLMTier::Execution,
            _ => return None,
        })
    }
}
