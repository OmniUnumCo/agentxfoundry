//! Tier -> (provider, model) routing.
//!
//! Loaded from `config/llm.toml` (or `RR_LLM_CONFIG_PATH` override), with a
//! compiled-in default pointing every tier at a local Ollama model. The
//! router itself is framework-free: it maps tier strings to a
//! [`TierRoute`] record; the *caller* decides how to materialize that into
//! an actual `LLMClient`.

use crate::llm::ProviderKind;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// One tier's routing target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TierRoute {
    pub provider: ProviderKind,
    pub model: String,
    /// Optional base URL override. Required for anything other than the
    /// first-party providers' defaults (e.g. the OpenAI-compat adapter talking
    /// to Groq, DeepSeek, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Optional seed override. If unset, providers use their own default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

/// Full router configuration. TOML shape:
///
/// ```toml
/// [tiers.strategic]
/// provider = "ollama"
/// model    = "llama3.1:70b-instruct"
///
/// [tiers.tactical]
/// provider = "ollama"
/// model    = "llama3.1:8b-instruct"
///
/// [tiers.execution]
/// provider = "ollama"
/// model    = "llama3.2:3b"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub tiers: HashMap<String, TierRoute>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        // Safe local-only defaults. Keeps the router usable without any config
        // file present: every tier maps to a small local Ollama model.
        let mut tiers: HashMap<String, TierRoute> = HashMap::new();
        for tier in &[
            "strategic",
            "tactical",
            "supervision",
            "execution_guidance",
            "execution",
        ] {
            tiers.insert(
                (*tier).to_string(),
                TierRoute {
                    provider: ProviderKind::Ollama,
                    model: "llama3.2:3b".to_string(),
                    base_url: Some("http://localhost:11434".to_string()),
                    seed: Some(0),
                },
            );
        }
        RouterConfig { tiers }
    }
}

pub struct LlmRouter {
    config: RouterConfig,
}

impl LlmRouter {
    pub fn new(config: RouterConfig) -> Self {
        Self { config }
    }

    /// Load a config from a TOML file. Returns the default config if the file
    /// does not exist (this is not an error — it means "use built-in defaults").
    pub fn load_from_file(path: &Path) -> Result<Self, BoxError> {
        if !path.is_file() {
            return Ok(Self::new(RouterConfig::default()));
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("reading {}: {}", path.display(), e))?;
        let config: RouterConfig =
            toml::from_str(&raw).map_err(|e| format!("parsing {}: {}", path.display(), e))?;
        Ok(Self::new(config))
    }

    /// Respect `RR_LLM_CONFIG_PATH` if set, else fall back to defaults.
    pub fn from_env_or_default() -> Self {
        if let Ok(path) = std::env::var("RR_LLM_CONFIG_PATH") {
            if let Ok(router) = Self::load_from_file(Path::new(&path)) {
                return router;
            }
        }
        Self::new(RouterConfig::default())
    }

    /// Look up the route for a logical tier string (`strategic`, `tactical`,
    /// `supervision`, `execution_guidance`, `execution`).
    ///
    /// Returns `None` if the tier is unknown. Callers should degrade
    /// gracefully or surface an error.
    pub fn route(&self, tier: &str) -> Option<&TierRoute> {
        self.config.tiers.get(tier)
    }

    /// The raw config (useful for diagnostics and `cr_firm_detail`-style
    /// introspection).
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_covers_all_tiers() {
        let r = LlmRouter::new(RouterConfig::default());
        for tier in &[
            "strategic",
            "tactical",
            "supervision",
            "execution_guidance",
            "execution",
        ] {
            assert!(r.route(tier).is_some(), "tier {} missing", tier);
        }
    }

    #[test]
    fn toml_roundtrip() {
        let toml_text = r#"
            [tiers.strategic]
            provider = "anthropic"
            model = "claude-sonnet-4-20250514"

            [tiers.execution]
            provider = "ollama"
            model = "llama3.2:3b"
            base_url = "http://localhost:11434"
            seed = 42
        "#;
        let config: RouterConfig = toml::from_str(toml_text).unwrap();
        let router = LlmRouter::new(config);
        let strat = router.route("strategic").unwrap();
        assert_eq!(strat.provider, ProviderKind::Anthropic);
        assert_eq!(strat.model, "claude-sonnet-4-20250514");
        let exec = router.route("execution").unwrap();
        assert_eq!(exec.provider, ProviderKind::Ollama);
        assert_eq!(exec.seed, Some(42));
    }

    #[test]
    fn missing_file_falls_back_to_defaults() {
        let router =
            LlmRouter::load_from_file(Path::new("/nonexistent/path/to/llm.toml")).unwrap();
        assert!(router.route("strategic").is_some());
    }

    #[test]
    fn unknown_tier_returns_none() {
        let r = LlmRouter::new(RouterConfig::default());
        assert!(r.route("bogus").is_none());
    }
}
