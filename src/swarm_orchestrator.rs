//! Swarm-level mission execution.
//!
//! Ports the `execute_mission` path of `rr/swarm_orchestrator.py`:
//! plan -> assign -> execute. Task-assignment matching is rank/MOS-aware;
//! execution walks the DAG with a simple iteration-bounded loop that respects
//! dependencies.
//!
//! Slice E.1 ships the sequential executor. Parallel execution of
//! dependency-independent tasks (via `tokio::join_all`) is deferred to E.3.
//!
//! The orchestrator deliberately holds a `DeterministicLLMClient` handle and
//! routes calls via the tier router rather than delegating to
//! `RRAgent::execute_task` — that keeps the LLM plumbing in one place and
//! lets the agent struct remain a pure data/behavior carrier. Slice E.1.9 may
//! wire a convenience wrapper on `RRAgent` for single-task execution paths
//! outside the orchestrator.

use crate::doctrine::FileSystemDoctrineLoader;
use crate::llm::LlmRouter;
use crate::models::*;
use crate::persistence::SqliteStateStore;
use crate::traits::{DoctrineLoader, LLMClient, StateStore};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Error, Debug)]
pub enum OrchestrateError {
    #[error("unknown swarm: {0}")]
    UnknownSwarm(String),

    #[error("persistence failure: {0}")]
    Persistence(BoxError),

    #[error("no agents in swarm {0}")]
    EmptySwarm(String),

    #[error("planner failed: {0}")]
    PlanFailed(#[from] crate::mission_planner::PlanError),

    #[error("router has no tier `{0}` configured")]
    UnknownTier(String),

    #[error("LLM call failed: {0}")]
    LlmFailure(BoxError),

    #[error("no agent in swarm matched task `{task}` (mos={mos:?}, rank={rank:?})")]
    NoAgentMatch {
        task: String,
        mos: Option<String>,
        rank: Option<String>,
    },

    #[error("dependency deadlock: {remaining} task(s) never became ready")]
    DependencyDeadlock { remaining: usize },
}

/// One row of the orchestrator's execution report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskExecution {
    pub task_id: String,
    pub agent_id: String,
    pub status: StatusEnum,
    pub result: Option<String>,
    pub error: Option<String>,
}

/// Disband outcome returned by `disband_swarm`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DisbandReport {
    pub swarm_id: String,
    pub disbanded_at: chrono::DateTime<chrono::Utc>,
    pub aar: AAR,
}

pub struct SwarmOrchestrator {
    state_store: Arc<SqliteStateStore>,
    doctrine: Arc<FileSystemDoctrineLoader>,
    llm: Arc<dyn LLMClient>,
    router: Arc<LlmRouter>,
}

impl SwarmOrchestrator {
    pub fn new(
        state_store: Arc<SqliteStateStore>,
        doctrine: Arc<FileSystemDoctrineLoader>,
        llm: Arc<dyn LLMClient>,
        router: Arc<LlmRouter>,
    ) -> Self {
        Self {
            state_store,
            doctrine,
            llm,
            router,
        }
    }

    /// Plan + assign + execute a mission for the given swarm.
    ///
    /// Ordering of operations mirrors `rr/swarm_orchestrator.py::execute_mission`:
    /// 1. Load swarm agents from the state store.
    /// 2. Build a `swarm_composition` map and hand it to the planner.
    /// 3. Assign each planned task to the best-matching agent (by MOS, then rank).
    /// 4. Walk the dependency DAG, executing each task and appending events.
    pub async fn execute_mission(
        &self,
        swarm_id: &str,
        mission: Mission,
    ) -> Result<Vec<TaskExecution>, OrchestrateError> {
        // 1. Load swarm + agents.
        let swarm = self
            .state_store
            .load_swarm(swarm_id)
            .map_err(OrchestrateError::Persistence)?
            .ok_or_else(|| OrchestrateError::UnknownSwarm(swarm_id.to_string()))?;
        let agents = self
            .state_store
            .list_agents(Some(swarm_id), None, None, None)
            .map_err(OrchestrateError::Persistence)?;
        if agents.is_empty() {
            return Err(OrchestrateError::EmptySwarm(swarm_id.to_string()));
        }

        // 2. Compose composition summary for the planner.
        let composition = summarize_swarm(&agents);

        // Persist mission row + MISSION_STARTED event.
        self.state_store
            .save_mission(&mission)
            .map_err(OrchestrateError::Persistence)?;
        self.state_store
            .append_event(
                "MISSION_STARTED",
                &json!({
                    "mission_id": mission.id,
                    "swarm_id": swarm.id,
                    "objective": mission.objective,
                }),
                Some(&swarm.id),
                None,
            )
            .map_err(OrchestrateError::Persistence)?;

        // 3. Plan the mission.
        let planner = crate::mission_planner::MissionPlanner::new(self.llm.clone());
        let tasks = planner.plan_mission(&mission, composition).await?;

        // 4. Assign each task to an agent (by MOS preference, rank fallback).
        let assignments = assign_tasks(&tasks, &agents)?;

        // 5. Walk the DAG and execute in dependency order.
        self.execute_task_graph(&swarm, tasks, assignments).await
    }

    async fn execute_task_graph(
        &self,
        swarm: &Swarm,
        tasks: Vec<Task>,
        assignments: HashMap<String, String>, // task_id -> agent_id
    ) -> Result<Vec<TaskExecution>, OrchestrateError> {
        let mut results: Vec<TaskExecution> = Vec::new();
        let mut completed: HashSet<String> = HashSet::new();
        let agents_by_id: HashMap<String, AgentRecord> = self
            .state_store
            .list_agents(Some(&swarm.id), None, None, None)
            .map_err(OrchestrateError::Persistence)?
            .into_iter()
            .map(|a| (a.id.clone(), a))
            .collect();

        let n = tasks.len();
        let max_iter = n * 2;
        for _ in 0..max_iter {
            if completed.len() == n {
                break;
            }
            let mut made_progress = false;

            for task in &tasks {
                if completed.contains(&task.id) {
                    continue;
                }
                // Dependencies must reference planner-level task_ids (from
                // planner output) OR the Task.id we assigned. We use the
                // `planner_task_id` metadata bridge if present; otherwise the
                // literal dependency string is compared against already-
                // completed agent-visible Task.ids.
                let deps_met = task
                    .dependencies
                    .iter()
                    .all(|dep| completed.contains(dep) || completed_by_planner_id(dep, &tasks, &completed));
                if !deps_met {
                    continue;
                }

                let agent_id = match assignments.get(&task.id) {
                    Some(a) => a.clone(),
                    None => {
                        // Should not happen — assign_tasks returns Err if a
                        // match cannot be found. Skip defensively.
                        continue;
                    }
                };
                let agent = match agents_by_id.get(&agent_id) {
                    Some(a) => a,
                    None => {
                        results.push(TaskExecution {
                            task_id: task.id.clone(),
                            agent_id: agent_id.clone(),
                            status: StatusEnum::Failed,
                            result: None,
                            error: Some(format!("agent `{}` not found", agent_id)),
                        });
                        completed.insert(task.id.clone());
                        made_progress = true;
                        continue;
                    }
                };

                // Execute via LLM. On error, capture it; do not short-circuit
                // the rest of the DAG.
                let exec_result = self.execute_single_task(swarm, agent, task).await;
                match exec_result {
                    Ok(response) => {
                        results.push(TaskExecution {
                            task_id: task.id.clone(),
                            agent_id: agent_id.clone(),
                            status: StatusEnum::Complete,
                            result: Some(response),
                            error: None,
                        });
                    }
                    Err(e) => {
                        results.push(TaskExecution {
                            task_id: task.id.clone(),
                            agent_id: agent_id.clone(),
                            status: StatusEnum::Failed,
                            result: None,
                            error: Some(e.to_string()),
                        });
                    }
                }
                completed.insert(task.id.clone());
                made_progress = true;
            }

            if !made_progress {
                return Err(OrchestrateError::DependencyDeadlock {
                    remaining: n - completed.len(),
                });
            }
        }

        Ok(results)
    }

    /// Disband the swarm: mark it Complete, generate an After-Action Review
    /// from the event log, and emit `SWARM_DISBANDED` + `AAR_GENERATED` events.
    ///
    /// Ports `rr/swarm_orchestrator.py::disband_swarm` + `_generate_aar`. The
    /// Rust version sources performance data from the event log rather than
    /// keeping an in-process active_swarms map -- matches RR's "event log is
    /// the source of truth" contract.
    pub async fn disband_swarm(
        &self,
        swarm_id: &str,
    ) -> Result<DisbandReport, OrchestrateError> {
        let mut swarm = self
            .state_store
            .load_swarm(swarm_id)
            .map_err(OrchestrateError::Persistence)?
            .ok_or_else(|| OrchestrateError::UnknownSwarm(swarm_id.to_string()))?;
        let agents = self
            .state_store
            .list_agents(Some(swarm_id), None, None, None)
            .map_err(OrchestrateError::Persistence)?;
        let mission = swarm
            .mission_id
            .as_deref()
            .and_then(|mid| self.state_store.load_mission(mid).ok().flatten());

        // Collect every contract authored by one of this swarm's agents.
        // We iterate the civilian roster and pull each firm's contract list,
        // then filter by contracting_agent. Avoids widening StateStore's
        // trait surface for a single consumer.
        let agent_ids: HashSet<String> = agents.iter().map(|a| a.id.clone()).collect();
        let swarm_contracts = self.collect_swarm_contracts(&agent_ids);

        let aar = generate_aar(&swarm, mission.as_ref(), &agents, &swarm_contracts);

        // Mark swarm disbanded.
        let now = chrono::Utc::now();
        swarm.status = StatusEnum::Complete;
        swarm.disbanded_at = Some(now);
        self.state_store
            .save_swarm(&swarm)
            .map_err(OrchestrateError::Persistence)?;

        // Write disband + AAR events.
        self.state_store
            .append_event(
                "SWARM_DISBANDED",
                &json!({
                    "swarm_id": swarm_id,
                    "agent_count": agents.len(),
                    "success": aar.success,
                }),
                Some(swarm_id),
                None,
            )
            .map_err(OrchestrateError::Persistence)?;
        self.state_store
            .append_event(
                "AAR_GENERATED",
                &serde_json::to_value(&aar).unwrap_or(json!({})),
                Some(swarm_id),
                None,
            )
            .map_err(OrchestrateError::Persistence)?;

        // Emit per-agent deactivation events so audit trails are symmetric
        // with AGENT_SPAWNED.
        for a in &agents {
            let _ = self.state_store.append_event(
                "AGENT_DEACTIVATED",
                &json!({"agent_id": a.id}),
                Some(swarm_id),
                Some(&a.id),
            );
        }

        Ok(DisbandReport {
            swarm_id: swarm_id.to_string(),
            disbanded_at: now,
            aar,
        })
    }

    /// Find every contract authored by any of the given agent ids.
    /// Iterates the civilian roster (firms + independents) and per-civilian
    /// pulls the contract list, keeping rows where `contracting_agent` is in
    /// `agent_ids`. Silent on persistence failures; returns what it can.
    fn collect_swarm_contracts(
        &self,
        agent_ids: &HashSet<String>,
    ) -> Vec<ContractRecord> {
        let civilians = match self.state_store.list_civilians() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut out: Vec<ContractRecord> = Vec::new();
        for civ in &civilians {
            if let Ok(contracts) = self.state_store.list_contracts_by_firm(&civ.id, None) {
                for c in contracts {
                    if agent_ids.contains(&c.contracting_agent) {
                        out.push(c);
                    }
                }
            }
        }
        out
    }

    async fn execute_single_task(
        &self,
        swarm: &Swarm,
        agent: &AgentRecord,
        task: &Task,
    ) -> Result<String, OrchestrateError> {
        let prompt = self
            .doctrine
            .assemble_task_prompt(&agent.rank, &agent.mos, &task.description)
            .map_err(OrchestrateError::LlmFailure)?;
        let tier = crate::agent::tier_for_rank(&agent.rank);
        let route = self
            .router
            .route(tier)
            .ok_or_else(|| OrchestrateError::UnknownTier(tier.to_string()))?;

        // Emit a TASK_STARTED event before the LLM round-trip.
        let _ = self.state_store.append_event(
            "TASK_STARTED",
            &json!({
                "task_id": task.id,
                "agent_id": agent.id,
                "tier": tier,
                "provider": route.provider,
                "model": route.model,
            }),
            Some(&swarm.id),
            Some(&agent.id),
        );

        let response = self
            .llm
            .complete(&prompt, tier, Some(2048), Some(0.0))
            .await
            .map_err(OrchestrateError::LlmFailure)?;

        let _ = self.state_store.append_event(
            "TASK_COMPLETED",
            &json!({
                "task_id": task.id,
                "agent_id": agent.id,
                "response_chars": response.chars().count(),
            }),
            Some(&swarm.id),
            Some(&agent.id),
        );

        Ok(response)
    }
}

/// Build a sorted summary like `{"SGT-Infantry": "3", "CPT-CombatEngineer": "1"}`.
fn summarize_swarm(agents: &[AgentRecord]) -> crate::general_mind::PromptContext {
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for a in agents {
        let key = format!("{:?}-{:?}", a.rank, a.mos);
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect()
}

/// Match planner tasks to concrete agents. Preference order:
/// 1. Exact MOS match (if the task specified one).
/// 2. Exact rank match (if the task specified one).
/// 3. Any available agent (round-robin by index).
///
/// `Err` is returned only if the swarm is empty. A missing MOS/rank constraint
/// just falls through to the next preference level.
fn assign_tasks(
    tasks: &[Task],
    agents: &[AgentRecord],
) -> Result<HashMap<String, String>, OrchestrateError> {
    let mut assignments: HashMap<String, String> = HashMap::new();
    let mut round_robin = 0usize;

    for task in tasks {
        let want_mos = task
            .metadata
            .get("assigned_mos")
            .and_then(|v| v.as_str())
            .map(String::from);
        let want_rank = task
            .metadata
            .get("assigned_rank")
            .and_then(|v| v.as_str())
            .map(String::from);

        let by_mos = want_mos.as_deref().and_then(|code| {
            agents
                .iter()
                .find(|a| mos_code(&a.mos) == code)
                .map(|a| a.id.clone())
        });
        let by_rank = want_rank.as_deref().and_then(|code| {
            agents
                .iter()
                .find(|a| format!("{:?}", a.rank) == code)
                .map(|a| a.id.clone())
        });

        let chosen = by_mos.or(by_rank).unwrap_or_else(|| {
            let a = &agents[round_robin % agents.len()];
            round_robin += 1;
            a.id.clone()
        });
        assignments.insert(task.id.clone(), chosen);
    }

    if assignments.is_empty() && !tasks.is_empty() {
        return Err(OrchestrateError::NoAgentMatch {
            task: tasks[0].description.clone(),
            mos: None,
            rank: None,
        });
    }
    Ok(assignments)
}

/// Convert `MOSEnum` variants back to their short code (`12B`, `35F`, ...).
/// Matches `FileSystemDoctrineLoader::mos_doctrine`'s mapping.
fn mos_code(mos: &MOSEnum) -> &'static str {
    match mos {
        MOSEnum::IntelAnalyst => "35F",
        MOSEnum::CounterIntel => "35L",
        MOSEnum::SigintAnalyst => "35N",
        MOSEnum::Infantry => "11B",
        MOSEnum::CombatEngineer => "12B",
        MOSEnum::Artillery => "13B",
        MOSEnum::CavalryScout => "19D",
        MOSEnum::ItSpecialist => "25B",
        MOSEnum::HrSpecialist => "42A",
        MOSEnum::Transport => "88M",
        MOSEnum::Logistics => "92G",
        MOSEnum::SfEngineer => "18B",
        MOSEnum::Ranger => "75R",
        MOSEnum::Soar => "160",
    }
}

/// Generate an After-Action Review from swarm + mission + agent roster.
///
/// Ports `rr/swarm_orchestrator.py::_generate_aar`. Unlike the Python
/// prototype which hardcoded recommendations, the Rust version populates
/// `agent_performance` from the live roster (rank/MOS markers) and lets
/// the caller (or later reducers) attach lessons/recommendations from the
/// event log.
fn generate_aar(
    swarm: &Swarm,
    mission: Option<&Mission>,
    agents: &[AgentRecord],
    contracts: &[ContractRecord],
) -> AAR {
    let success = mission
        .map(|m| m.status == StatusEnum::Complete)
        .unwrap_or(false);
    let mut agent_performance: HashMap<String, serde_json::Value> = HashMap::new();
    for a in agents {
        agent_performance.insert(
            a.id.clone(),
            json!({
                "name": a.name,
                "rank": format!("{:?}", a.rank),
                "mos": format!("{:?}", a.mos),
                "missions_completed": a.missions_completed,
                "performance_score": a.performance_score,
                "status": a.status,
            }),
        );
    }
    let mut resource_usage: HashMap<String, serde_json::Value> = HashMap::new();
    resource_usage.insert("total_agents".into(), json!(agents.len()));
    resource_usage.insert("total_contracts".into(), json!(contracts.len()));
    let contracts_closed_ok = contracts
        .iter()
        .filter(|c| c.outcome == Some(ContractOutcome::Complete))
        .count();
    let contracts_failed = contracts
        .iter()
        .filter(|c| {
            matches!(c.outcome, Some(ContractOutcome::Failed) | Some(ContractOutcome::Cancelled))
        })
        .count();
    resource_usage.insert("contracts_completed".into(), json!(contracts_closed_ok));
    resource_usage.insert("contracts_failed".into(), json!(contracts_failed));

    // Swarm timeline row.
    let mut swarm_row: HashMap<String, serde_json::Value> = HashMap::new();
    swarm_row.insert("kind".into(), json!("swarm"));
    swarm_row.insert("created".into(), json!(swarm.created_at.to_rfc3339()));
    if let Some(ts) = swarm.activated_at {
        swarm_row.insert("activated".into(), json!(ts.to_rfc3339()));
    }
    swarm_row.insert(
        "disbanded".into(),
        json!(swarm
            .disbanded_at
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339()),
    );

    // Contract timeline rows, sorted by created_at for a stable narrative.
    let mut sorted_contracts: Vec<&ContractRecord> = contracts.iter().collect();
    sorted_contracts.sort_by_key(|c| c.created_at);
    let mut timeline: Vec<HashMap<String, serde_json::Value>> = Vec::with_capacity(1 + sorted_contracts.len());
    timeline.push(swarm_row);
    for c in sorted_contracts {
        let mut row: HashMap<String, serde_json::Value> = HashMap::new();
        row.insert("kind".into(), json!("contract"));
        row.insert("contract_id".into(), json!(c.id));
        row.insert("firm_id".into(), json!(c.firm_id));
        row.insert("contracting_agent".into(), json!(c.contracting_agent));
        row.insert("state".into(), json!(c.state));
        if let Some(ref specialist) = c.routed_specialist_id {
            row.insert("routed_specialist_id".into(), json!(specialist));
        }
        row.insert("created".into(), json!(c.created_at.to_rfc3339()));
        if let Some(ts) = c.closed_at {
            row.insert("closed".into(), json!(ts.to_rfc3339()));
        }
        if let Some(outcome) = c.outcome {
            row.insert("outcome".into(), json!(outcome));
        }
        timeline.push(row);
    }

    let objectives_met = if success {
        mission
            .map(|m| vec![m.objective.clone()])
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let objectives_missed = if !success {
        mission
            .map(|m| vec![m.objective.clone()])
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    AAR {
        id: uuid::Uuid::new_v4().to_string(),
        mission_id: mission.map(|m| m.id.clone()).unwrap_or_else(|| "unknown".into()),
        swarm_id: swarm.id.clone(),
        success,
        objectives_met,
        objectives_missed,
        lessons_learned: Vec::new(),
        agent_performance,
        resource_usage,
        timeline,
        recommendations: Vec::new(),
        created_at: chrono::Utc::now(),
        metadata: HashMap::new(),
    }
}

/// Support tasks whose dependency strings reference the planner-level
/// `task_id` (from the LLM response) rather than our generated UUID
/// [`Task.id`].
fn completed_by_planner_id(dep: &str, tasks: &[Task], completed: &HashSet<String>) -> bool {
    for t in tasks {
        if let Some(serde_json::Value::String(pid)) = t.metadata.get("planner_task_id") {
            if pid == dep && completed.contains(&t.id) {
                return true;
            }
        }
    }
    false
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(name: &str, rank: RankEnum, mos: MOSEnum) -> AgentRecord {
        AgentRecord {
            id: format!("agent-{}", name),
            name: name.to_string(),
            rank,
            mos,
            ..AgentRecord::default()
        }
    }

    fn task(desc: &str, mos: Option<&str>, rank: Option<&str>) -> Task {
        let mut meta: HashMap<String, serde_json::Value> = HashMap::new();
        if let Some(m) = mos {
            meta.insert("assigned_mos".into(), serde_json::Value::String(m.into()));
        }
        if let Some(r) = rank {
            meta.insert("assigned_rank".into(), serde_json::Value::String(r.into()));
        }
        Task {
            id: uuid::Uuid::new_v4().to_string(),
            description: desc.to_string(),
            metadata: meta,
            ..Task::default()
        }
    }

    #[test]
    fn assigns_by_mos_when_possible() {
        let agents = vec![
            agent("alpha", RankEnum::SGT, MOSEnum::CombatEngineer),
            agent("bravo", RankEnum::SGT, MOSEnum::Infantry),
        ];
        let t1 = task("build", Some("12B"), None);
        let t2 = task("secure", Some("11B"), None);
        let tasks = vec![t1.clone(), t2.clone()];
        let assignments = assign_tasks(&tasks, &agents).unwrap();
        assert_eq!(assignments.get(&t1.id).unwrap(), "agent-alpha");
        assert_eq!(assignments.get(&t2.id).unwrap(), "agent-bravo");
    }

    #[test]
    fn assigns_by_rank_when_mos_absent() {
        let agents = vec![
            agent("boss", RankEnum::CPT, MOSEnum::Infantry),
            agent("grunt", RankEnum::SPC, MOSEnum::Infantry),
        ];
        let t = task("direct", None, Some("CPT"));
        let assignments = assign_tasks(&std::slice::from_ref(&t).to_vec(), &agents).unwrap();
        assert_eq!(assignments.get(&t.id).unwrap(), "agent-boss");
    }

    #[test]
    fn falls_back_to_round_robin_when_no_match() {
        let agents = vec![
            agent("one", RankEnum::SPC, MOSEnum::Infantry),
            agent("two", RankEnum::SPC, MOSEnum::Infantry),
        ];
        let tasks = vec![
            task("x", Some("25B"), None), // no matching MOS
            task("y", None, None),        // no constraints
        ];
        let a = assign_tasks(&tasks, &agents).unwrap();
        // round robin: first task -> agent[0], second -> agent[1]
        assert_eq!(a.get(&tasks[0].id).unwrap(), "agent-one");
        assert_eq!(a.get(&tasks[1].id).unwrap(), "agent-two");
    }

    #[test]
    fn aar_marks_success_when_mission_complete() {
        let mut mission = Mission::default();
        mission.status = StatusEnum::Complete;
        mission.objective = "Audit layout".into();
        let swarm = Swarm {
            id: "s1".into(),
            name: "alpha".into(),
            general_id: "g1".into(),
            ..Swarm::default()
        };
        let agents = vec![agent("one", RankEnum::GEN, MOSEnum::SfEngineer)];
        let aar = generate_aar(&swarm, Some(&mission), &agents, &[]);
        assert!(aar.success);
        assert_eq!(aar.objectives_met, vec!["Audit layout".to_string()]);
        assert!(aar.objectives_missed.is_empty());
        assert!(aar.agent_performance.contains_key("agent-one"));
        assert_eq!(
            aar.resource_usage.get("total_agents").and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(
            aar.resource_usage.get("total_contracts").and_then(|v| v.as_u64()),
            Some(0)
        );
        // Exactly one timeline row (swarm, no contracts)
        assert_eq!(aar.timeline.len(), 1);
        assert_eq!(
            aar.timeline[0].get("kind").and_then(|v| v.as_str()),
            Some("swarm")
        );
    }

    #[test]
    fn aar_marks_failure_when_mission_not_complete() {
        let mut mission = Mission::default();
        mission.status = StatusEnum::Failed;
        mission.objective = "Hold the line".into();
        let swarm = Swarm::default();
        let aar = generate_aar(&swarm, Some(&mission), &[], &[]);
        assert!(!aar.success);
        assert_eq!(aar.objectives_missed, vec!["Hold the line".to_string()]);
        assert!(aar.objectives_met.is_empty());
    }

    #[test]
    fn aar_handles_no_mission_at_all() {
        let swarm = Swarm::default();
        let aar = generate_aar(&swarm, None, &[], &[]);
        assert!(!aar.success);
        assert_eq!(aar.mission_id, "unknown");
    }

    #[test]
    fn aar_includes_contract_timeline_rows() {
        let swarm = Swarm {
            id: "s2".into(),
            name: "beta".into(),
            general_id: "g1".into(),
            ..Swarm::default()
        };
        let agents = vec![agent("lead", RankEnum::CPT, MOSEnum::SfEngineer)];

        let now = chrono::Utc::now();
        let c1 = ContractRecord {
            id: "c1".into(),
            contracting_agent: "agent-lead".into(),
            firm_id: "arena".into(),
            routed_specialist_id: None,
            state: ContractState::Closed,
            brief: "first".into(),
            mission_id: None,
            priority: PriorityEnum::Routine,
            created_at: now - chrono::Duration::minutes(10),
            closed_at: Some(now - chrono::Duration::minutes(5)),
            outcome: Some(ContractOutcome::Complete),
            metadata: HashMap::new(),
        };
        let c2 = ContractRecord {
            id: "c2".into(),
            contracting_agent: "agent-lead".into(),
            firm_id: "return42".into(),
            routed_specialist_id: Some("morphlex".into()),
            state: ContractState::Closed,
            brief: "second".into(),
            mission_id: None,
            priority: PriorityEnum::Priority,
            created_at: now - chrono::Duration::minutes(3),
            closed_at: Some(now),
            outcome: Some(ContractOutcome::Failed),
            metadata: HashMap::new(),
        };
        let aar = generate_aar(&swarm, None, &agents, &[c2.clone(), c1.clone()]);

        // swarm row + 2 contract rows
        assert_eq!(aar.timeline.len(), 3);
        assert_eq!(
            aar.timeline[0].get("kind").and_then(|v| v.as_str()),
            Some("swarm")
        );
        // Contracts sorted by created_at ascending, so c1 (older) first.
        assert_eq!(
            aar.timeline[1].get("contract_id").and_then(|v| v.as_str()),
            Some("c1")
        );
        assert_eq!(
            aar.timeline[2].get("contract_id").and_then(|v| v.as_str()),
            Some("c2")
        );
        assert_eq!(
            aar.resource_usage.get("contracts_completed").and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(
            aar.resource_usage.get("contracts_failed").and_then(|v| v.as_u64()),
            Some(1)
        );
    }

    #[test]
    fn swarm_summary_is_sorted_and_counts_by_rank_mos() {
        let agents = vec![
            agent("a", RankEnum::SGT, MOSEnum::Infantry),
            agent("b", RankEnum::SGT, MOSEnum::Infantry),
            agent("c", RankEnum::CPT, MOSEnum::CombatEngineer),
        ];
        let s = summarize_swarm(&agents);
        assert_eq!(s.get("SGT-Infantry"), Some(&"2".to_string()));
        assert_eq!(s.get("CPT-CombatEngineer"), Some(&"1".to_string()));
    }
}
