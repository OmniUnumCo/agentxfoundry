//! Pydantic models ported from Python: enums and data structs.
//!
//! All models derive `Serialize`, `Deserialize`, `Debug`, `Clone`, and `PartialEq`.
//! UUIDs are strings. Timestamps are `chrono::DateTime<Utc>`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Enums
// ============================================================================

/// Three-way split of the military hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RankCategory {
    Officer,
    Nco,
    Enlisted,
}

/// The 12 military ranks with authority levels.
///
/// Values are the short rank codes for stable JSON serialization.
///
/// # Example
///
/// ```
/// use rr::RankEnum;
///
/// assert_eq!(RankEnum::GEN.authority(), 12);
/// assert_eq!(RankEnum::PVT.authority(), 1);
/// assert!(RankEnum::GEN.outranks(&RankEnum::COL));
/// assert!(!RankEnum::PVT.outranks(&RankEnum::SPC));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub enum RankEnum {
    // Officers (authority 8-12)
    GEN, // General
    COL, // Colonel
    MAJ, // Major
    CPT, // Captain
    LT,  // Lieutenant
    // NCOs (authority 4-7)
    SGM, // Sergeant Major
    MSG, // Master Sergeant
    SGT, // Sergeant
    CPL, // Corporal
    // Enlisted (authority 1-3)
    #[default]
    SPC, // Specialist
    PFC, // Private First Class
    PVT, // Private
}

impl RankEnum {
    /// Numeric authority level (1 = PVT, 12 = GEN).
    pub fn authority(&self) -> u8 {
        match self {
            RankEnum::GEN => 12,
            RankEnum::COL => 11,
            RankEnum::MAJ => 10,
            RankEnum::CPT => 9,
            RankEnum::LT => 8,
            RankEnum::SGM => 7,
            RankEnum::MSG => 6,
            RankEnum::SGT => 5,
            RankEnum::CPL => 4,
            RankEnum::SPC => 3,
            RankEnum::PFC => 2,
            RankEnum::PVT => 1,
        }
    }

    /// Offficer / NCO / Enlisted category.
    pub fn category(&self) -> RankCategory {
        if self.authority() >= 8 {
            RankCategory::Officer
        } else if self.authority() >= 4 {
            RankCategory::Nco
        } else {
            RankCategory::Enlisted
        }
    }

    /// Long human-readable name.
    pub fn full_name(&self) -> &'static str {
        match self {
            RankEnum::GEN => "General",
            RankEnum::COL => "Colonel",
            RankEnum::MAJ => "Major",
            RankEnum::CPT => "Captain",
            RankEnum::LT => "Lieutenant",
            RankEnum::SGM => "Sergeant Major",
            RankEnum::MSG => "Master Sergeant",
            RankEnum::SGT => "Sergeant",
            RankEnum::CPL => "Corporal",
            RankEnum::SPC => "Specialist",
            RankEnum::PFC => "Private First Class",
            RankEnum::PVT => "Private",
        }
    }

    /// True iff this rank has strictly higher authority than *other*.
    pub fn outranks(&self, other: &RankEnum) -> bool {
        self.authority() > other.authority()
    }

    /// True if *self* has sufficient rank to command *other*.
    pub fn can_command_by_rank(&self, other: &RankEnum) -> bool {
        self.outranks(other)
    }
}

/// The four MOS categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MOSCategory {
    Intelligence,
    Operations,
    Support,
    SpecialOperations,
}

/// The 14 Military Occupational Specialties.
///
/// # Example
///
/// ```
/// use rr::{MOSEnum, MOSCategory};
///
/// assert_eq!(MOSEnum::CombatEngineer.category(), MOSCategory::Operations);
/// assert_eq!(MOSEnum::IntelAnalyst.category(), MOSCategory::Intelligence);
/// assert_eq!(MOSEnum::CombatEngineer.full_name(), "Combat Engineer");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MOSEnum {
    // Intelligence (MI)
    #[serde(rename = "35F")]
    IntelAnalyst,   // Intelligence Analyst
    #[serde(rename = "35L")]
    CounterIntel,   // Counterintelligence
    #[serde(rename = "35N")]
    SigintAnalyst,  // SIGINT Analyst
    // Operations (OPS)
    #[serde(rename = "11B")]
    Infantry,       // Infantry
    #[serde(rename = "12B")]
    CombatEngineer, // Combat Engineer
    #[serde(rename = "13B")]
    Artillery,      // Artillery
    #[serde(rename = "19D")]
    CavalryScout,   // Cavalry Scout
    // Support (SPT)
    #[default]
    #[serde(rename = "25B")]
    ItSpecialist,   // IT Specialist
    #[serde(rename = "42A")]
    HrSpecialist,   // HR Specialist
    #[serde(rename = "88M")]
    Transport,      // Transport
    #[serde(rename = "92G")]
    Logistics,      // Logistics
    // Special Operations (SOF)
    #[serde(rename = "18B")]
    SfEngineer,     // Special Forces Engineer
    #[serde(rename = "75R")]
    Ranger,         // 75th Ranger
    #[serde(rename = "160")]
    Soar,           // 160th SOAR
}

impl MOSEnum {
    pub fn category(&self) -> MOSCategory {
        match self {
            MOSEnum::IntelAnalyst
            | MOSEnum::CounterIntel
            | MOSEnum::SigintAnalyst => MOSCategory::Intelligence,
            MOSEnum::Infantry
            | MOSEnum::CombatEngineer
            | MOSEnum::Artillery
            | MOSEnum::CavalryScout => MOSCategory::Operations,
            MOSEnum::ItSpecialist
            | MOSEnum::HrSpecialist
            | MOSEnum::Transport
            | MOSEnum::Logistics => MOSCategory::Support,
            MOSEnum::SfEngineer | MOSEnum::Ranger | MOSEnum::Soar => {
                MOSCategory::SpecialOperations
            }
        }
    }

    pub fn full_name(&self) -> &'static str {
        match self {
            MOSEnum::IntelAnalyst => "Intelligence Analyst",
            MOSEnum::CounterIntel => "Counterintelligence",
            MOSEnum::SigintAnalyst => "SIGINT Analyst",
            MOSEnum::Infantry => "Infantry",
            MOSEnum::CombatEngineer => "Combat Engineer",
            MOSEnum::Artillery => "Artillery",
            MOSEnum::CavalryScout => "Cavalry Scout",
            MOSEnum::ItSpecialist => "IT Specialist",
            MOSEnum::HrSpecialist => "HR Specialist",
            MOSEnum::Transport => "Transport",
            MOSEnum::Logistics => "Logistics",
            MOSEnum::SfEngineer => "Special Forces Engineer",
            MOSEnum::Ranger => "75th Ranger",
            MOSEnum::Soar => "160th SOAR",
        }
    }
}

/// Unified status used by agents, swarms, missions, and tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StatusEnum {
    #[default]
    Standby,
    Active,
    Engaged,
    Complete,
    Failed,
}

/// Mission priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorityEnum {
    Routine,
    Priority,
    Urgent,
    Critical,
}

/// Classification of orders routed through the chain of command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    MissionOrder,
    Frago,
    DirectTask,
    Broadcast,
}

/// Formation sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitType {
    FireTeam,
    Squad,
    Platoon,
    Company,
    Battalion,
}

// ============================================================================
// Core data models
// ============================================================================

fn new_uuid() -> String {
    Uuid::new_v4().to_string()
}

fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// Atomic unit of work that can be assigned to an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    pub status: StatusEnum,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: new_uuid(),
            description: String::new(),
            assigned_to: None,
            status: StatusEnum::Standby,
            dependencies: Vec::new(),
            result: None,
            error: None,
            created_at: now_utc(),
            completed_at: None,
            metadata: HashMap::new(),
        }
    }
}

/// A mission: a natural-language objective plus its decomposition into tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mission {
    pub id: String,
    pub objective: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub priority: PriorityEnum,
    pub status: StatusEnum,
    #[serde(default)]
    pub constraints: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub resources: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_swarm: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for Mission {
    fn default() -> Self {
        Self {
            id: new_uuid(),
            objective: String::new(),
            description: None,
            priority: PriorityEnum::Routine,
            status: StatusEnum::Standby,
            constraints: HashMap::new(),
            resources: HashMap::new(),
            tasks: Vec::new(),
            assigned_swarm: None,
            created_at: now_utc(),
            started_at: None,
            completed_at: None,
            result: None,
            metadata: HashMap::new(),
        }
    }
}

/// An order issued from a superior to a subordinate.
///
/// # Example
///
/// ```
/// use rr::{Order, OrderType};
///
/// let order = Order::new("cmd-1", "soldier-1", OrderType::MissionOrder, "Advance to checkpoint");
/// assert_eq!(order.from_agent, "cmd-1");
/// assert_eq!(order.order_type, OrderType::MissionOrder);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Order {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub order_type: OrderType,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_id: Option<String>,
    pub issued_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Order {
    pub fn new(
        from_agent: impl Into<String>,
        to_agent: impl Into<String>,
        order_type: OrderType,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: new_uuid(),
            from_agent: from_agent.into(),
            to_agent: to_agent.into(),
            order_type,
            content: content.into(),
            task_id: None,
            mission_id: None,
            issued_at: now_utc(),
            acknowledged_at: None,
            metadata: HashMap::new(),
        }
    }
}

/// Situation report — status flowing up the chain of command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SitRep {
    pub id: String,
    pub from_agent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_agent: Option<String>,
    pub status: StatusEnum,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<String>,
    pub progress: f64,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub blockers: Vec<String>,
    pub reported_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SitRep {
    pub fn new(from_agent: impl Into<String>, status: StatusEnum) -> Self {
        Self {
            id: new_uuid(),
            from_agent: from_agent.into(),
            to_agent: None,
            status,
            current_task: None,
            progress: 0.0,
            message: String::new(),
            blockers: Vec::new(),
            reported_at: now_utc(),
            metadata: HashMap::new(),
        }
    }
}

/// Casualty report — agent failure notification, escalated up the chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CasRep {
    pub id: String,
    pub agent_id: String,
    pub failure_type: String,
    pub error_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
    pub reported_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_action: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl CasRep {
    pub fn new(
        agent_id: impl Into<String>,
        failure_type: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Self {
        Self {
            id: new_uuid(),
            agent_id: agent_id.into(),
            failure_type: failure_type.into(),
            error_message: error_message.into(),
            task_id: None,
            stack_trace: None,
            reported_at: now_utc(),
            recovery_action: None,
            metadata: HashMap::new(),
        }
    }
}

/// After-Action Review — generated when a swarm is disbanded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AAR {
    pub id: String,
    pub mission_id: String,
    pub swarm_id: String,
    pub success: bool,
    #[serde(default)]
    pub objectives_met: Vec<String>,
    #[serde(default)]
    pub objectives_missed: Vec<String>,
    #[serde(default)]
    pub lessons_learned: Vec<String>,
    #[serde(default)]
    pub agent_performance: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub resource_usage: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub timeline: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub recommendations: Vec<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Organizational formation (fire team, squad, platoon, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Unit {
    pub id: String,
    pub name: String,
    pub unit_type: UnitType,
    pub commander: String,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_unit: Option<String>,
    #[serde(default)]
    pub sub_units: Vec<String>,
    pub status: StatusEnum,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disbanded_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A top-level swarm — one or more units under a single General.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Swarm {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_id: Option<String>,
    pub general_id: String,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub units: Vec<String>,
    pub status: StatusEnum,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disbanded_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Persistence DTO for an RRAgent.
///
/// `RRAgent` (in `agent.rs`) holds behavior; `AgentRecord` holds pure shape
/// so it can be stored in SQLite as a JSON projection.
///
/// # Example
///
/// ```
/// use rr::{AgentRecord, RankEnum, MOSEnum, StatusEnum};
///
/// let record = AgentRecord {
///     id: "a1".into(),
///     name: "Alpha".into(),
///     rank: RankEnum::CPT,
///     mos: MOSEnum::CombatEngineer,
///     status: StatusEnum::Active,
///     ..AgentRecord::default()
/// };
/// assert!(record.is_officer());
/// assert_eq!(record.rank_authority(), 9);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentRecord {
    pub id: String,
    pub name: String,
    pub rank: RankEnum,
    pub mos: MOSEnum,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commander_id: Option<String>,
    #[serde(default)]
    pub subordinates: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swarm_id: Option<String>,
    pub status: StatusEnum,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_mission: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<String>,
    pub missions_completed: u32,
    pub performance_score: f64,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentRecord {
    pub fn is_officer(&self) -> bool {
        self.rank.category() == RankCategory::Officer
    }

    pub fn is_nco(&self) -> bool {
        self.rank.category() == RankCategory::Nco
    }

    pub fn is_enlisted(&self) -> bool {
        self.rank.category() == RankCategory::Enlisted
    }

    pub fn rank_authority(&self) -> u8 {
        self.rank.authority()
    }
}

// ============================================================================
// Civilian Reserve (CR) — parallel class to military agents
// ============================================================================

/// Top-level classification separating military-rank agents from civilian contractors.
///
/// Military agents hold a `RankEnum` + `MOSEnum` and live in the chain of command.
/// Civilians hold a `CivilianTier` + `specialty_slug` and are contracted ad-hoc
/// by military agents; they never appear as commander or subordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentClass {
    Military,
    Civilian,
}

/// Structural role of a civilian within the Reserve.
///
/// - `Firm`: umbrella entity with internal specialists. Contracts address firms;
///   the firm internally routes work to a specialist.
/// - `Specialist`: a single-role civilian that belongs to a parent firm.
/// - `Independent`: a single-role civilian that has no parent firm (flat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CivilianTier {
    Firm,
    Specialist,
    Independent,
}

/// Persistence DTO for a Civilian Reserve contractor.
///
/// Civilians are a sibling of [`AgentRecord`]: they do NOT carry `commander_id`
/// or `subordinates`, because they are not part of the chain of command. They
/// are contracted, not ordered.
///
/// # Example
///
/// ```
/// use rr::{CivilianRecord, CivilianTier, StatusEnum};
///
/// let toad = CivilianRecord {
///     id: "toad".into(),
///     name: "CR-Toad".into(),
///     tier: CivilianTier::Independent,
///     specialty_slug: "toad".into(),
///     status: StatusEnum::Standby,
///     ..CivilianRecord::default()
/// };
/// assert!(toad.is_independent());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CivilianRecord {
    pub id: String,
    pub name: String,
    pub tier: CivilianTier,
    /// Parent firm id; required iff `tier == Specialist`, None otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firm_id: Option<String>,
    /// Populated only on firms — list of specialist ids.
    #[serde(default)]
    pub specialists: Vec<String>,
    /// Short stable slug used in doctrine paths, e.g. "morphlex", "toad".
    pub specialty_slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_url: Option<String>,
    pub status: StatusEnum,
    #[serde(default)]
    pub active_contracts: Vec<String>,
    pub performance_score: f64,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for CivilianRecord {
    fn default() -> Self {
        Self {
            id: new_uuid(),
            name: String::new(),
            tier: CivilianTier::Independent,
            firm_id: None,
            specialists: Vec::new(),
            specialty_slug: String::new(),
            project_path: None,
            upstream_url: None,
            status: StatusEnum::Standby,
            active_contracts: Vec::new(),
            performance_score: 0.0,
            created_at: now_utc(),
            metadata: HashMap::new(),
        }
    }
}

impl CivilianRecord {
    pub fn is_firm(&self) -> bool {
        self.tier == CivilianTier::Firm
    }

    pub fn is_specialist(&self) -> bool {
        self.tier == CivilianTier::Specialist
    }

    pub fn is_independent(&self) -> bool {
        self.tier == CivilianTier::Independent
    }
}

/// Either a military agent record or a civilian contractor record.
///
/// Used by call sites that need to accept any reserve member without
/// widening `AgentRecord` itself (which would pollute every military-only
/// accessor).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "class", rename_all = "snake_case")]
pub enum AnyAgent {
    #[serde(rename = "military")]
    Mil(AgentRecord),
    #[serde(rename = "civilian")]
    Civ(CivilianRecord),
}

impl AnyAgent {
    pub fn class(&self) -> AgentClass {
        match self {
            AnyAgent::Mil(_) => AgentClass::Military,
            AnyAgent::Civ(_) => AgentClass::Civilian,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            AnyAgent::Mil(a) => &a.id,
            AnyAgent::Civ(c) => &c.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            AnyAgent::Mil(a) => &a.name,
            AnyAgent::Civ(c) => &c.name,
        }
    }
}

impl From<AgentRecord> for AnyAgent {
    fn from(a: AgentRecord) -> Self {
        AnyAgent::Mil(a)
    }
}

impl From<CivilianRecord> for AnyAgent {
    fn from(c: CivilianRecord) -> Self {
        AnyAgent::Civ(c)
    }
}

// ============================================================================
// Contract lifecycle
// ============================================================================

/// Lifecycle state of a civilian contract.
///
/// Transitions:
///   requested -> accepted | rejected
///   accepted  -> in_progress (implicit on first CONTRACT_SITREP)
///   in_progress -> closed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractState {
    Requested,
    Accepted,
    InProgress,
    Closed,
    Rejected,
}

impl Default for ContractState {
    fn default() -> Self {
        ContractState::Requested
    }
}

/// Terminal outcome captured at contract closeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractOutcome {
    Complete,
    Failed,
    Cancelled,
}

/// Persistence DTO for a contract record (projection of the event log).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractRecord {
    pub id: String,
    /// Military agent id that initiated the contract.
    pub contracting_agent: String,
    /// Civilian firm (or independent) id that the contract was addressed to.
    pub firm_id: String,
    /// Specialist id the firm routed the contract to; None until accepted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routed_specialist_id: Option<String>,
    pub state: ContractState,
    pub brief: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_id: Option<String>,
    pub priority: PriorityEnum,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<ContractOutcome>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for ContractRecord {
    fn default() -> Self {
        Self {
            id: new_uuid(),
            contracting_agent: String::new(),
            firm_id: String::new(),
            routed_specialist_id: None,
            state: ContractState::Requested,
            brief: String::new(),
            mission_id: None,
            priority: PriorityEnum::Routine,
            created_at: now_utc(),
            closed_at: None,
            outcome: None,
            metadata: HashMap::new(),
        }
    }
}

/// Inbound DTO for `C2Router::route_contract_request`.
///
/// The contracting military agent populates this; the router is responsible
/// for allocating the contract id and initial state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractRequest {
    pub contracting_agent: String,
    pub firm_id: String,
    pub brief: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_id: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: PriorityEnum,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_priority() -> PriorityEnum {
    PriorityEnum::Routine
}
