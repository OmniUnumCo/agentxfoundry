//! Dependency injection traits — the Rust equivalent of Python's `rr.protocols`.
//!
//! The core library depends on the *shape* of its collaborators, not concrete
//! implementations. Every trait here is `Send + Sync` so it can be used across
//! async boundaries. Concrete implementations live in `persistence.rs`,
//! `doctrine.rs`, `logging.rs`, etc.

use crate::models::{
    AgentRecord, CivilianRecord, ContractRecord, ContractState, Mission, MOSEnum, RankEnum,
    StatusEnum, Swarm,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Framework-free interface for an LLM provider.
///
/// Implementations live in a runtime crate and are injected into `RRAgent`
/// instances at execution time. The core never imports `anthropic`, `openai`, etc.
///
/// `tier` is a logical routing hint (`"strategic"`, `"tactical"`,
/// `"supervision"`, `"execution"`) — the LLM router maps these to concrete
/// models based on rank.
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

/// Append-only event store plus current-state projection.
///
/// The schema is defined in `src/rr/persistence/schema.sql`.
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Append a single event; return the assigned event id.
    ///
    /// `doctrine_version` is pinned automatically by the implementation
    /// from the currently-loaded doctrine, so AAR reproducibility is
    /// guaranteed without the caller having to thread it through.
    fn append_event(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
        swarm_id: Option<&str>,
        agent_id: Option<&str>,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>>;

    fn event_count(&self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>>;

    // -- Projection reads --

    fn load_agent(&self, agent_id: &str) -> Result<Option<AgentRecord>, Box<dyn std::error::Error + Send + Sync>>;
    fn load_swarm(&self, swarm_id: &str) -> Result<Option<Swarm>, Box<dyn std::error::Error + Send + Sync>>;
    fn load_mission(&self, mission_id: &str) -> Result<Option<Mission>, Box<dyn std::error::Error + Send + Sync>>;

    fn list_agents(
        &self,
        swarm_id: Option<&str>,
        status: Option<StatusEnum>,
        rank: Option<RankEnum>,
        mos: Option<MOSEnum>,
    ) -> Result<Vec<AgentRecord>, Box<dyn std::error::Error + Send + Sync>>;

    fn list_swarms(
        &self,
        status: Option<StatusEnum>,
    ) -> Result<Vec<Swarm>, Box<dyn std::error::Error + Send + Sync>>;

    // -- Projection writes (called by the daemon after appending events) --

    fn save_agent(&self, agent: &AgentRecord) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn save_swarm(&self, swarm: &Swarm) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn save_mission(&self, mission: &Mission) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // -- Civilian Reserve: projections for civilians and contracts --

    fn save_civilian(
        &self,
        civilian: &CivilianRecord,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    fn load_civilian(
        &self,
        civilian_id: &str,
    ) -> Result<Option<CivilianRecord>, Box<dyn std::error::Error + Send + Sync>>;

    fn list_civilians(
        &self,
    ) -> Result<Vec<CivilianRecord>, Box<dyn std::error::Error + Send + Sync>>;

    fn save_contract(
        &self,
        contract: &ContractRecord,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    fn load_contract(
        &self,
        contract_id: &str,
    ) -> Result<Option<ContractRecord>, Box<dyn std::error::Error + Send + Sync>>;

    fn list_contracts_by_firm(
        &self,
        firm_id: &str,
        state: Option<ContractState>,
    ) -> Result<Vec<ContractRecord>, Box<dyn std::error::Error + Send + Sync>>;

    /// Close the underlying database connection.
    fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Structured civilian doctrine loaded from a frontmatter-prefixed markdown file.
///
/// Frontmatter is a minimal `key: value` block delimited by `---` lines at the
/// top of the file; values may be scalars or inline arrays (`[a, b, c]`).
/// See `doctrine/civilian/flat/*.md` for examples.
#[derive(Debug, Clone, PartialEq)]
pub struct CivilianDoctrine {
    pub slug: String,
    pub kind: crate::models::CivilianTier,
    /// Required iff `kind == Specialist`.
    pub firm: Option<String>,
    pub project_path: Option<String>,
    pub upstream_url: Option<String>,
    pub languages: Vec<String>,
    /// If set, overrides the default civilian LLM routing tier.
    pub tier_override: Option<String>,
    pub contract_scope: Vec<String>,
    /// Populated only when `kind == Firm`.
    pub specialists: Vec<String>,
    /// Full markdown body (after the frontmatter delimiters are stripped).
    pub body: String,
}

/// Read-only interface for loading rank/MOS/protocol doctrine markdown.
///
/// The default implementation resolves paths via `RR_DOCTRINE_PATH` env var,
/// then `~/.rational-reserve/doctrine/`, then a packaged-resource fallback.
pub trait DoctrineLoader: Send + Sync {
    /// Semver string from `doctrine/VERSION`, pinned in every event row.
    fn version(&self) -> String;

    /// Return the markdown body for a rank's doctrine file.
    fn rank_doctrine(&self, rank: &RankEnum) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Return the markdown body for an MOS's doctrine file.
    fn mos_doctrine(&self, mos: &MOSEnum) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Return a communication-protocol template (`"sitrep"`, `"frago"`, etc.).
    fn protocol_template(&self, protocol_name: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Build the standard task prompt from rank + MOS doctrine + task description.
    fn assemble_task_prompt(
        &self,
        rank: &RankEnum,
        mos: &MOSEnum,
        task_description: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    // -- Civilian Reserve --

    /// Load a civilian (flat/independent or firm-root) by its slug.
    ///
    /// Resolution order:
    /// 1. `doctrine/civilian/flat/<slug>.md` — independent / flat civilians
    /// 2. `doctrine/civilian/firms/<slug>/firm.md` — firm root doctrine
    ///
    /// For specialists (within a firm) use [`DoctrineLoader::specialist_doctrine`].
    fn civilian_doctrine(
        &self,
        slug: &str,
    ) -> Result<CivilianDoctrine, Box<dyn std::error::Error + Send + Sync>>;

    /// Load a specialist within a firm by (firm slug, specialist slug).
    ///
    /// Path: `doctrine/civilian/firms/<firm>/specialist-<spec>.md`.
    ///
    /// Default implementation is unimplemented — land in Slice B.
    fn specialist_doctrine(
        &self,
        _firm: &str,
        _spec: &str,
    ) -> Result<CivilianDoctrine, Box<dyn std::error::Error + Send + Sync>> {
        Err("specialist_doctrine is not implemented (Slice B)".into())
    }

    /// Build a contract prompt from a civilian's doctrine and the brief.
    ///
    /// Kept separate from `assemble_task_prompt` so the military prompt shape
    /// is not overloaded with contract-flavored semantics.
    fn assemble_contract_prompt(
        &self,
        civilian: &CivilianDoctrine,
        brief: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(format!(
            "You are CR-{} ({}), a Civilian Reserve contractor.\n\n\
             # Doctrine\n{}\n\n\
             # Contract brief\n{}\n\n\
             Execute this contract within your specialty. Return deliverables as specified.",
            civilian.slug,
            match civilian.kind {
                crate::models::CivilianTier::Firm => "firm",
                crate::models::CivilianTier::Specialist => "specialist",
                crate::models::CivilianTier::Independent => "independent",
            },
            civilian.body,
            brief,
        ))
    }
}

/// Injected source of `DateTime<Utc>` — lets tests freeze time deterministically.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// A default clock using the system clock.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Structured event stream for the `c2.jsonl` log file.
///
/// Separate from `StateStore::append_event` because the JSONL stream captures
/// human-readable debugging output (with arbitrary shapes), whereas the event
/// store captures the canonical append-only history.
pub trait EventLogger: Send + Sync {
    fn log(&self, event: &HashMap<String, serde_json::Value>);
}
