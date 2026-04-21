//! The live RRAgent — rank, MOS, chain-of-command ties, and behavior.
//!
//! An `RRAgent` holds *behavior*: it accepts orders, executes tasks, delegates
//! down the chain, emits SITREPs/CASREPs. The persistence DTO is [`AgentRecord`]
//! (in `models`); the agent can be materialized from a record and rendered back
//! to one.
//!
//! In v0.1 agents are inert records: `execute_task` raises if no LLM client is
//! injected. The v0.1 MCP tools exercise `receive_order`, `delegate_task`,
//! `report_sitrep`, `report_casrep`, and `to_record` — they do not invoke the
//! LLM.

use crate::models::*;
use crate::traits::{DoctrineLoader, LLMClient};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// Map a military rank to a logical LLM routing tier.
///
/// Follows REQUIREMENTS_PHASE1.md Section 3.2 (rank-based LLM assignment). Tiers
/// are logical strings; the concrete LLMClient implementation decides which model
/// each tier resolves to.
pub fn tier_for_rank(rank: &RankEnum) -> &'static str {
    match rank {
        RankEnum::GEN | RankEnum::COL => "strategic",
        RankEnum::MAJ | RankEnum::CPT => "tactical",
        RankEnum::LT | RankEnum::SGM => "supervision",
        RankEnum::MSG | RankEnum::SGT | RankEnum::CPL => "execution_guidance",
        // Enlisted
        _ => "execution",
    }
}

/// Default LLM routing tier for a civilian contractor.
///
/// Firms dispatch internally (tactical coordination). Specialists and
/// independents execute contracted work (execution_guidance). Doctrine
/// frontmatter may set `tier_override` to force a specific tier — e.g.
/// CR-MAB MOA-Reasoning could override to "strategic".
pub fn tier_for_civilian(civ: &CivilianRecord) -> &'static str {
    if let Some(serde_json::Value::String(override_tier)) = civ.metadata.get("tier_override") {
        return match override_tier.as_str() {
            "strategic" => "strategic",
            "tactical" => "tactical",
            "supervision" => "supervision",
            "execution_guidance" => "execution_guidance",
            "execution" => "execution",
            _ => default_civilian_tier(civ.tier),
        };
    }
    default_civilian_tier(civ.tier)
}

fn default_civilian_tier(tier: CivilianTier) -> &'static str {
    match tier {
        CivilianTier::Firm => "tactical",
        CivilianTier::Specialist | CivilianTier::Independent => "execution_guidance",
    }
}

/// Map any reserve member (military or civilian) to its LLM routing tier.
pub fn tier_for(agent: &AnyAgent) -> &'static str {
    match agent {
        AnyAgent::Mil(rec) => tier_for_rank(&rec.rank),
        AnyAgent::Civ(rec) => tier_for_civilian(rec),
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum RRAgentError {
    #[error("agent has no LLMClient; execute_task is unavailable")]
    LLMNotAvailable,

    #[error("command authority: {0}")]
    CommandAuthority(String),

    #[error("LLM execution failed: {0}")]
    LLMExecution(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, RRAgentError>;

// ============================================================================
// RRAgent
// ============================================================================

/// A live agent with rank, MOS, and chain-of-command ties.
///
/// Construction is intentionally verbose: we want every dependency to be
/// explicitly passed in so tests can substitute fakes. There is no module-level
/// global here.
///
/// # Example
///
/// ```
/// use rr::{RRAgent, RankEnum, MOSEnum};
/// use rr::doctrine::FileSystemDoctrineLoader;
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// let doctrine_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("doctrine");
/// let doctrine = Arc::new(FileSystemDoctrineLoader::new(doctrine_dir));
///
/// let agent = RRAgent::new(
///     "Alpha".into(),
///     RankEnum::SGT,
///     MOSEnum::Infantry,
///     doctrine,
///     None, None, None, Some("agent-1".into()), None,
/// );
/// assert_eq!(agent.name, "Alpha");
/// assert_eq!(agent.rank, RankEnum::SGT);
/// assert!(agent.is_nco());
/// ```
pub struct RRAgent {
    pub id: String,
    pub name: String,
    pub rank: RankEnum,
    pub mos: MOSEnum,
    pub commander_id: Option<String>,
    pub subordinates: Vec<String>,
    pub unit_id: Option<String>,
    pub swarm_id: Option<String>,

    pub status: StatusEnum,
    pub current_mission: Option<String>,
    pub current_task: Option<Task>,
    pub missions_completed: u32,
    pub performance_score: f64,
    pub created_at: DateTime<Utc>,

    doctrine: Arc<dyn DoctrineLoader>,
    llm: Option<Arc<dyn LLMClient>>,
    received_orders: Vec<Order>,
    metadata: HashMap<String, serde_json::Value>,
}

impl Clone for RRAgent {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            rank: self.rank,
            mos: self.mos,
            commander_id: self.commander_id.clone(),
            subordinates: self.subordinates.clone(),
            unit_id: self.unit_id.clone(),
            swarm_id: self.swarm_id.clone(),
            status: self.status,
            current_mission: self.current_mission.clone(),
            current_task: self.current_task.clone(),
            missions_completed: self.missions_completed,
            performance_score: self.performance_score,
            created_at: self.created_at,
            doctrine: self.doctrine.clone(),
            llm: self.llm.clone(),
            received_orders: self.received_orders.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

impl RRAgent {
    /// Construct a new agent with explicit dependencies.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        rank: RankEnum,
        mos: MOSEnum,
        doctrine: Arc<dyn DoctrineLoader>,
        commander_id: Option<String>,
        unit_id: Option<String>,
        swarm_id: Option<String>,
        agent_id: Option<String>,
        llm: Option<Arc<dyn LLMClient>>,
    ) -> Self {
        Self {
            id: agent_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            name,
            rank,
            mos,
            commander_id,
            subordinates: Vec::new(),
            unit_id,
            swarm_id,
            status: StatusEnum::Standby,
            current_mission: None,
            current_task: None,
            missions_completed: 0,
            performance_score: 0.0,
            created_at: now_utc(),
            doctrine,
            llm,
            received_orders: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    // ------------------------------------------------------------------
    // Rank / MOS convenience
    // ------------------------------------------------------------------

    pub fn is_officer(&self) -> bool {
        self.rank.category() == RankCategory::Officer
    }

    pub fn is_nco(&self) -> bool {
        self.rank.category() == RankCategory::Nco
    }

    pub fn is_enlisted(&self) -> bool {
        self.rank.category() == RankCategory::Enlisted
    }

    /// Pure rank-based authority check.
    ///
    /// This does NOT consult the live chain of command — use
    /// [`C2Router::validate_command_relationship`] for the full hybrid check
    /// (direct subordinate OR rank override).
    pub fn can_command(&self, other_rank: &RankEnum) -> bool {
        self.rank.outranks(other_rank)
    }

    // ------------------------------------------------------------------
    // Chain of command
    // ------------------------------------------------------------------

    pub fn add_subordinate(&mut self, subordinate_id: String) {
        if subordinate_id == self.id {
            // silently ignore — or could panic; Python version raises but we
            // keep it simple for v0.1
            return;
        }
        if !self.subordinates.contains(&subordinate_id) {
            self.subordinates.push(subordinate_id);
        }
    }

    pub fn remove_subordinate(&mut self, subordinate_id: &str) {
        self.subordinates.retain(|id| id != subordinate_id);
    }

    // ------------------------------------------------------------------
    // Orders (incoming)
    // ------------------------------------------------------------------

    /// Accept an order from a superior.
    ///
    /// Receive-side validation is intentionally minimal: the agent trusts
    /// that `C2Router` has already validated the command relationship.
    /// The agent only checks that *it* is in fact the addressee.
    pub fn receive_order(&mut self, mut order: Order) -> Result<()> {
        if order.to_agent != self.id {
            return Err(RRAgentError::CommandAuthority(format!(
                "order {} addressed to {}, not {}",
                order.id, order.to_agent, self.id
            )));
        }
        order.acknowledged_at = Some(now_utc());
        self.received_orders.push(order);
        if self.status == StatusEnum::Standby {
            self.status = StatusEnum::Active;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Task execution (stub in v0.1 — throws if LLM unavailable)
    // ------------------------------------------------------------------

    /// Execute a task using the injected LLM client.
    ///
    /// In v0.1, RRAgent instances are constructed without an LLMClient, so
    /// this raises `LLMNotAvailable`. The v0.1 MCP tools do not call this
    /// method — they only create agents as inert records.
    pub async fn execute_task(&mut self, mut task: Task) -> Result<Task> {
        let llm = self.llm.as_ref().ok_or(RRAgentError::LLMNotAvailable)?;

        let prompt = self
            .doctrine
            .assemble_task_prompt(&self.rank, &self.mos, &task.description)
            .map_err(RRAgentError::LLMExecution)?;

        self.current_task = Some(task.clone());
        self.status = StatusEnum::Engaged;
        task.status = StatusEnum::Active;
        task.assigned_to = Some(self.id.clone());

        let tier = tier_for_rank(&self.rank);
        match llm.complete(&prompt, tier, None, None).await {
            Ok(result) => {
                task.result = Some(result);
                task.status = StatusEnum::Complete;
                task.completed_at = Some(now_utc());
                self.current_task = None;
                self.status = StatusEnum::Standby;
                self.missions_completed += 1;
                Ok(task)
            }
            Err(exc) => {
                task.status = StatusEnum::Failed;
                task.error = Some(exc.to_string());
                task.completed_at = Some(now_utc());
                self.status = StatusEnum::Failed;
                Err(RRAgentError::LLMExecution(exc))
            }
        }
    }

    /// Create a delegation [`Order`] for a direct subordinate.
    ///
    /// The returned Order must be routed through `C2Router` — this method does
    /// not touch the state store or the LLM.
    pub fn delegate_task(&self, mut task: Task, subordinate_id: &str) -> Result<Order> {
        if !self.subordinates.iter().any(|id| id == subordinate_id) {
            return Err(RRAgentError::CommandAuthority(format!(
                "{} cannot delegate to {}: not a direct subordinate",
                self.id, subordinate_id
            )));
        }
        task.assigned_to = Some(subordinate_id.to_string());
        Ok(Order::new(
            &self.id,
            subordinate_id,
            OrderType::DirectTask,
            &task.description,
        ))
    }

    // ------------------------------------------------------------------
    // Reports
    // ------------------------------------------------------------------

    /// Generate a SITREP addressed (by default) to the agent's commander.
    pub fn report_sitrep(&self, message: String, progress: f64) -> SitRep {
        let mut sitrep = SitRep::new(&self.id, self.status);
        sitrep.message = message;
        sitrep.progress = progress;
        sitrep.to_agent = self.commander_id.clone();
        if let Some(ref task) = self.current_task {
            sitrep.current_task = Some(task.id.clone());
        }
        sitrep
    }

    /// Generate a CASREP for the current task (or the agent as a whole).
    pub fn report_casrep(
        &self,
        failure_type: String,
        error_message: String,
        stack_trace: Option<String>,
    ) -> CasRep {
        let mut casrep = CasRep::new(&self.id, failure_type, error_message);
        casrep.stack_trace = stack_trace;
        if let Some(ref task) = self.current_task {
            casrep.task_id = Some(task.id.clone());
        }
        casrep
    }

    // ------------------------------------------------------------------
    // Persistence DTO
    // ------------------------------------------------------------------

    /// Render a persistence-ready [`AgentRecord`].
    pub fn to_record(&self) -> AgentRecord {
        AgentRecord {
            id: self.id.clone(),
            name: self.name.clone(),
            rank: self.rank,
            mos: self.mos,
            commander_id: self.commander_id.clone(),
            subordinates: self.subordinates.clone(),
            unit_id: self.unit_id.clone(),
            swarm_id: self.swarm_id.clone(),
            status: self.status,
            current_mission: self.current_mission.clone(),
            current_task: self.current_task.as_ref().map(|t| t.id.clone()),
            missions_completed: self.missions_completed,
            performance_score: self.performance_score,
            created_at: self.created_at,
            metadata: self.metadata.clone(),
        }
    }

    /// Rehydrate a live agent from a persistence record.
    pub fn from_record(
        record: AgentRecord,
        doctrine: Arc<dyn DoctrineLoader>,
        llm: Option<Arc<dyn LLMClient>>,
    ) -> Self {
        let mut agent = Self::new(
            record.name,
            record.rank,
            record.mos,
            doctrine,
            record.commander_id,
            record.unit_id,
            record.swarm_id,
            Some(record.id),
            llm,
        );
        agent.subordinates = record.subordinates;
        agent.status = record.status;
        agent.current_mission = record.current_mission;
        agent.missions_completed = record.missions_completed;
        agent.performance_score = record.performance_score;
        agent.created_at = record.created_at;
        agent.metadata = record.metadata;
        agent
    }
}

impl std::fmt::Debug for RRAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RRAgent")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("rank", &self.rank)
            .field("mos", &self.mos)
            .field("status", &self.status)
            .finish()
    }
}
