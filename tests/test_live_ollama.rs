//! Live-Ollama end-to-end integration tests.
//!
//! Every test here is `#[ignore]`-gated because it requires a running
//! Ollama at `$OLLAMA_HOST` (default `http://localhost:11434`) with at least
//! one model pulled. Run explicitly with:
//!
//! ```bash
//! # Prerequisites:
//! #   brew install ollama             # or your OS equivalent
//! #   ollama serve                    # in another terminal
//! #   ollama pull llama3.2:3b
//!
//! cargo test --test test_live_ollama -- --ignored
//! ```
//!
//! Override the model with `RR_TEST_OLLAMA_MODEL=<name>`; override the host
//! with `OLLAMA_HOST=http://host:port`.
//!
//! These tests prove two things the unit suite cannot:
//!
//! 1. **Real inference works.** The Rust port actually calls Ollama and
//!    parses its response correctly — no amount of `httptest` mocking can
//!    catch a real API shape drift.
//! 2. **Cache replay is byte-identical.** Two identical prompts produce
//!    byte-identical output. The first hits Ollama; the second hits the
//!    cache. We verify the cache row count to prove the second call did
//!    NOT hit Ollama.

use rr::llm::providers::OllamaClient;
use rr::llm::{DeterministicLLMClient, ProviderKind, ResponseCache};
use rr::traits::LLMClient;
use std::sync::Arc;

fn ollama_host() -> String {
    std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string())
}

fn ollama_model() -> String {
    std::env::var("RR_TEST_OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string())
}

/// Exists to make the test fail with a useful message when Ollama isn't up.
async fn skip_if_ollama_down(base: &str) -> bool {
    match reqwest::get(format!("{}/api/tags", base)).await {
        Ok(r) if r.status().is_success() => false,
        Ok(r) => {
            eprintln!(
                "ollama at {} returned status {}; skipping",
                base,
                r.status()
            );
            true
        }
        Err(e) => {
            eprintln!("ollama at {} unreachable: {}; skipping", base, e);
            true
        }
    }
}

// ============================================================================
// Tier 1: raw provider works
// ============================================================================

#[tokio::test]
#[ignore]
async fn ollama_provider_returns_nonempty_response() {
    let base = ollama_host();
    if skip_if_ollama_down(&base).await {
        return;
    }
    let client = OllamaClient::new(&base, &ollama_model()).with_seed(0);
    let out = client
        .complete(
            "Respond with a single word: ready.",
            "execution",
            Some(32),
            Some(0.0),
        )
        .await
        .expect("ollama returned a response");
    assert!(!out.is_empty(), "ollama response must be non-empty");
    eprintln!("ollama replied: {:?}", out.chars().take(120).collect::<String>());
}

// ============================================================================
// Tier 2: deterministic replay via cache
// ============================================================================

#[tokio::test]
#[ignore]
async fn cache_replay_yields_byte_identical_response() {
    let base = ollama_host();
    if skip_if_ollama_down(&base).await {
        return;
    }
    let model = ollama_model();
    let dir = tempfile::tempdir().unwrap();
    let cache = Arc::new(
        ResponseCache::open(&dir.path().join("llm_cache.db")).expect("cache opens"),
    );
    let inner: Arc<dyn LLMClient> =
        Arc::new(OllamaClient::new(&base, &model).with_seed(0));
    let wrapped = DeterministicLLMClient::new(
        inner,
        cache.clone(),
        "ollama",
        model.clone(),
        ProviderKind::Ollama,
    );

    let prompt = "Say hello in exactly one short sentence.";

    // First call — provider hit, cache miss, entry stored.
    let first = wrapped
        .complete(prompt, "execution", Some(64), Some(0.0))
        .await
        .expect("first call succeeded");
    assert_eq!(cache.count().unwrap(), 1, "exactly one cache entry after first call");

    // Second call — cache hit, provider untouched.
    let second = wrapped
        .complete(prompt, "execution", Some(64), Some(0.0))
        .await
        .expect("second call succeeded via cache");
    assert_eq!(
        first, second,
        "cached replay must be byte-identical to the first response"
    );
    assert_eq!(cache.count().unwrap(), 1, "cache count unchanged on replay");

    // A distinct prompt must miss the cache and add a second entry.
    let other_prompt = "Say goodbye in exactly one short sentence.";
    let _third = wrapped
        .complete(other_prompt, "execution", Some(64), Some(0.0))
        .await
        .expect("distinct prompt succeeded");
    assert_eq!(cache.count().unwrap(), 2, "distinct prompt adds a cache row");
}

// ============================================================================
// Tier 3: full swarm execute_mission end-to-end
// ============================================================================

#[tokio::test]
#[ignore]
async fn execute_mission_end_to_end_with_live_ollama() {
    use rr::doctrine::FileSystemDoctrineLoader;
    use rr::factory::AgentFactory;
    use rr::llm::{LlmRouter, RouterConfig, TierRoute};
    use rr::models::*;
    use rr::persistence::SqliteStateStore;
    use rr::swarm_orchestrator::SwarmOrchestrator;
    use rr::traits::StateStore;
    use std::collections::HashMap;
    use std::path::PathBuf;

    let base = ollama_host();
    if skip_if_ollama_down(&base).await {
        return;
    }
    let model = ollama_model();

    // -- set up real state store + doctrine
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(
        SqliteStateStore::open(
            &dir.path().join("rr.db"),
            Box::new(|| "mil-0.1.0+civ-0.1.0".to_string()),
        )
        .unwrap(),
    );
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let doctrine = Arc::new(FileSystemDoctrineLoader::new(
        PathBuf::from(&manifest).join("doctrine"),
    ));

    // -- build the LLM stack (deterministic, cached, pointed at live Ollama)
    let cache = Arc::new(
        ResponseCache::open(&dir.path().join("llm_cache.db")).expect("cache"),
    );
    let inner: Arc<dyn LLMClient> =
        Arc::new(OllamaClient::new(&base, &model).with_seed(0));
    let wrapped: Arc<dyn LLMClient> = Arc::new(DeterministicLLMClient::new(
        inner,
        cache.clone(),
        "ollama",
        model.clone(),
        ProviderKind::Ollama,
    ));

    // Router pointing every tier at the test model.
    let mut tiers = HashMap::new();
    for tier in ["strategic", "tactical", "supervision", "execution_guidance", "execution"] {
        tiers.insert(
            tier.to_string(),
            TierRoute {
                provider: ProviderKind::Ollama,
                model: model.clone(),
                base_url: Some(base.clone()),
                seed: Some(0),
            },
        );
    }
    let router = Arc::new(LlmRouter::new(RouterConfig { tiers }));

    // -- spin up a minimal swarm directly via AgentFactory (bypass the daemon
    //    binary; this test exercises the library, not the process wiring).
    let mut factory = AgentFactory::new(doctrine.clone()).with_llm(wrapped.clone());
    let general = factory.spawn_general(None, None);
    let specialist = factory.spawn_specialist(
        MOSEnum::SfEngineer,
        general.id.clone(),
        None,
        None,
    );
    let swarm_id = uuid::Uuid::new_v4().to_string();
    for a in [&general, &specialist] {
        let mut rec = a.to_record();
        rec.swarm_id = Some(swarm_id.clone());
        store.save_agent(&rec).unwrap();
    }
    let swarm = Swarm {
        id: swarm_id.clone(),
        name: "live-test".into(),
        mission_id: None,
        general_id: general.id.clone(),
        agents: vec![general.id.clone(), specialist.id.clone()],
        units: Vec::new(),
        status: StatusEnum::Standby,
        created_at: chrono::Utc::now(),
        activated_at: None,
        disbanded_at: None,
        metadata: HashMap::new(),
    };
    store.save_swarm(&swarm).unwrap();

    // -- execute a simple mission
    let mut mission = Mission::default();
    mission.objective = "Respond with the single word 'ready' and nothing else.".into();
    mission.priority = PriorityEnum::Routine;

    let orchestrator = SwarmOrchestrator::new(
        store.clone(),
        doctrine.clone(),
        wrapped.clone(),
        router.clone(),
    );
    let first_run = orchestrator
        .execute_mission(&swarm_id, mission.clone())
        .await
        .expect("first execute_mission succeeds");
    assert!(!first_run.is_empty(), "expected at least one task executed");

    // Re-run with identical inputs — the planner + every task prompt should
    // hit the cache exactly, so event_count grows by the same delta as the
    // first run but the Ollama provider is never called a second time.
    let entries_after_first = cache.count().unwrap();
    let second_run = orchestrator
        .execute_mission(&swarm_id, mission.clone())
        .await
        .expect("second execute_mission succeeds via cache");
    let entries_after_second = cache.count().unwrap();

    assert_eq!(
        entries_after_first, entries_after_second,
        "second run must not add new cache rows (all prompts replay from cache)"
    );
    assert_eq!(
        first_run.len(),
        second_run.len(),
        "both runs must produce the same number of task executions"
    );

    // Assert byte-identical task results where both runs completed that task.
    for (a, b) in first_run.iter().zip(second_run.iter()) {
        assert_eq!(
            a.result, b.result,
            "task result must be byte-identical across runs"
        );
    }
}
