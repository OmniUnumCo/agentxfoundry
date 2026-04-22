# Technical Specification — Mission Orchestration

## 1. Module map

| File | Lines of interest | Responsibility |
|---|---|---|
| `src/general_mind.rs` | all | 5 prompt builders + `GeneralMindMode` + `apply_general_mind` dispatcher |
| `src/mission_planner.rs` | all | `MissionPlanner` + `TaskSpec` + `PlanResponse` + `Prediction` + `PlanError` + helpers |
| `src/swarm_orchestrator.rs` | all | `SwarmOrchestrator` + `TaskExecution` + `DisbandReport` + `generate_aar` + assignment helpers + `OrchestrateError` |
| `src/agent.rs` | `execute_task` | Single-task convenience API |

## 2. Public type surface

```rust
// general_mind.rs
pub type PromptContext = BTreeMap<String, String>;
pub enum GeneralMindMode { Full, Recursive, Reactive, Predictive }
pub fn recursive_thinking_prompt(&str, &PromptContext) -> String;
pub fn reactive_complex_prompt(&str, &PromptContext, Option<&str>) -> String;
pub fn predictive_engine_prompt(&str, &PromptContext, u8) -> String;
pub fn unified_strategic_prompt(&str, &PromptContext, GeneralMindMode) -> String;
pub fn mission_decomposition_prompt(&str, &PromptContext, &PromptContext) -> String;
pub fn apply_general_mind(&str, &str, &PromptContext, GeneralMindMode) -> Option<String>;

// mission_planner.rs
pub struct MissionPlanner { llm: Arc<dyn LLMClient> }
pub struct TaskSpec { task_id, description, assigned_mos, assigned_rank,
                      dependencies, is_iterative, pivot_triggers,
                      predicted_duration, success_criteria }
pub struct Prediction { mission_id: String, horizon_depth: u8, raw: String }
pub enum PlanError { LlmFailure, MalformedJson, MissingTasksField, EmptyPlan }
impl MissionPlanner {
    pub fn new(llm: Arc<dyn LLMClient>) -> Self;
    pub async fn plan_mission(&self, &Mission, PromptContext) -> Result<Vec<Task>, PlanError>;
    pub async fn adapt_plan(&self, &Mission, &[Task], &str) -> Result<Vec<Task>, PlanError>;
    pub async fn predict_execution(&self, &Mission, &[Task], &PromptContext, u8)
        -> Result<Prediction, PlanError>;
    pub fn parse_plan(&str, &str) -> Result<Vec<Task>, PlanError>;   // test hook
}

// swarm_orchestrator.rs
pub struct SwarmOrchestrator {
    state_store: Arc<SqliteStateStore>,
    doctrine: Arc<FileSystemDoctrineLoader>,
    llm: Arc<dyn LLMClient>,
    router: Arc<LlmRouter>,
}
pub struct TaskExecution { task_id, agent_id, status, result, error }
pub struct DisbandReport { swarm_id, disbanded_at, aar }
pub enum OrchestrateError { UnknownSwarm, Persistence, EmptySwarm, PlanFailed,
                            UnknownTier, LlmFailure, NoAgentMatch,
                            DependencyDeadlock }
impl SwarmOrchestrator {
    pub fn new(state_store, doctrine, llm, router) -> Self;
    pub async fn execute_mission(&self, &str, Mission) -> Result<Vec<TaskExecution>, OrchestrateError>;
    pub async fn disband_swarm(&self, &str) -> Result<DisbandReport, OrchestrateError>;
}
```

## 3. Plan response JSON schema

```jsonc
{
  "tasks": [
    {
      "task_id": "t1",                         // optional string
      "description": "...",                    // required
      "assigned_mos": "12B",                   // optional
      "assigned_rank": "SGT",                  // optional
      "dependencies": ["t0"],                  // optional array of strings
      "is_iterative": false,                   // optional bool, default false
      "pivot_triggers": ["..."],               // optional array
      "predicted_duration": "2h",              // optional
      "success_criteria": "..."                // optional
    }
  ]
}
```

Parser is strict on structure (`tasks` key required, array required, each task needs `description`). Unknown fields are ignored for forward-compat. Missing optional fields default cleanly via `#[serde(default)]`.

## 4. Markdown fence tolerance

```rust
fn unfence(raw: &str) -> &str {
    let trimmed = raw.trim();
    let without_prefix = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim_start();
    without_prefix.strip_suffix("```").unwrap_or(without_prefix).trim_end()
}
```

Accepts `` ```json ... ``` ``, `` ``` ... ``` ``, or bare JSON. Does not accept partial JSON, HTML, prose before/after the fence, etc.

## 5. Task assignment algorithm

```rust
fn assign_tasks(tasks: &[Task], agents: &[AgentRecord]) -> Result<HashMap<TaskId, AgentId>, _> {
    let mut assignments = HashMap::new();
    let mut round_robin = 0usize;

    for task in tasks {
        let want_mos = task.metadata.get("assigned_mos").and_then(|v| v.as_str());
        let want_rank = task.metadata.get("assigned_rank").and_then(|v| v.as_str());

        let by_mos = want_mos.and_then(|code| agents.iter().find(|a| mos_code(&a.mos) == code));
        let by_rank = want_rank.and_then(|code| agents.iter().find(|a| format!("{:?}", a.rank) == code));

        let chosen = by_mos.map(|a| a.id.clone())
            .or_else(|| by_rank.map(|a| a.id.clone()))
            .unwrap_or_else(|| {
                let a = &agents[round_robin % agents.len()];
                round_robin += 1;
                a.id.clone()
            });
        assignments.insert(task.id.clone(), chosen);
    }
    Ok(assignments)
}
```

## 6. Task graph execution loop

```rust
let n = tasks.len();
let max_iter = n * 2;
for _ in 0..max_iter {
    if completed.len() == n { break; }
    let mut made_progress = false;

    for task in &tasks {
        if completed.contains(&task.id) { continue; }

        let deps_met = task.dependencies.iter().all(|dep| {
            completed.contains(dep) || completed_by_planner_id(dep, &tasks, &completed)
        });
        if !deps_met { continue; }

        let agent_id = assignments.get(&task.id).unwrap().clone();
        let agent = agents_by_id.get(&agent_id).unwrap();

        let exec_result = self.execute_single_task(swarm, agent, task).await;
        match exec_result {
            Ok(response) => results.push(TaskExecution { status: Complete, result: Some(response), ... }),
            Err(e) => results.push(TaskExecution { status: Failed, error: Some(e.to_string()), ... }),
        }
        completed.insert(task.id.clone());
        made_progress = true;
    }

    if !made_progress {
        return Err(OrchestrateError::DependencyDeadlock { remaining: n - completed.len() });
    }
}
```

## 7. `execute_single_task` implementation

```rust
async fn execute_single_task(&self, swarm: &Swarm, agent: &AgentRecord, task: &Task) -> Result<String, _> {
    let prompt = self.doctrine.assemble_task_prompt(&agent.rank, &agent.mos, &task.description)?;
    let tier = crate::agent::tier_for_rank(&agent.rank);
    let route = self.router.route(tier).ok_or_else(|| OrchestrateError::UnknownTier(tier.to_string()))?;

    self.state_store.append_event("TASK_STARTED", &json!({...}), Some(&swarm.id), Some(&agent.id))?;

    let response = self.llm.complete(&prompt, tier, Some(2048), Some(0.0)).await?;

    self.state_store.append_event("TASK_COMPLETED", &json!({...}), Some(&swarm.id), Some(&agent.id))?;
    Ok(response)
}
```

Note `self.llm` is `Arc<dyn LLMClient>` — typically a `DeterministicLLMClient` but the orchestrator doesn't care which concrete wrapper chain is underneath.

## 8. AAR generator

```rust
fn generate_aar(swarm: &Swarm, mission: Option<&Mission>, agents: &[AgentRecord]) -> AAR {
    let success = mission.map(|m| m.status == StatusEnum::Complete).unwrap_or(false);

    let mut agent_performance = HashMap::new();
    for a in agents {
        agent_performance.insert(a.id.clone(), json!({
            "name": a.name, "rank": format!("{:?}", a.rank), "mos": format!("{:?}", a.mos),
            "missions_completed": a.missions_completed,
            "performance_score": a.performance_score,
            "status": a.status,
        }));
    }

    let mut resource_usage = HashMap::new();
    resource_usage.insert("total_agents".into(), json!(agents.len()));

    let mut timeline_row = HashMap::new();
    timeline_row.insert("created".into(), json!(swarm.created_at.to_rfc3339()));
    if let Some(ts) = swarm.activated_at {
        timeline_row.insert("activated".into(), json!(ts.to_rfc3339()));
    }
    timeline_row.insert("disbanded".into(), json!(
        swarm.disbanded_at.unwrap_or_else(chrono::Utc::now).to_rfc3339()
    ));

    AAR {
        id: Uuid::new_v4().to_string(),
        mission_id: mission.map(|m| m.id.clone()).unwrap_or_else(|| "unknown".into()),
        swarm_id: swarm.id.clone(),
        success,
        objectives_met: if success { mission.map(|m| vec![m.objective.clone()]).unwrap_or_default() } else { Vec::new() },
        objectives_missed: if !success { mission.map(|m| vec![m.objective.clone()]).unwrap_or_default() } else { Vec::new() },
        lessons_learned: Vec::new(),
        agent_performance,
        resource_usage,
        timeline: vec![timeline_row],
        recommendations: Vec::new(),
        created_at: chrono::Utc::now(),
        metadata: HashMap::new(),
    }
}
```

## 9. Test coverage

| File | Tests |
|---|---|
| `general_mind.rs` | 13 — per-template structure, byte-stability, mode dispatch, rank gating, parser |
| `mission_planner.rs` | 11 — plan happy path, fence stripping, malformed JSON, empty tasks, missing optionals, adapt happy + fail-loud, predict output preservation, progress/velocity helpers |
| `swarm_orchestrator.rs` | 7 — MOS assignment, rank assignment, round-robin fallback, swarm summary, AAR success/failure/no-mission |

Total: **31 unit tests** covering the orchestration layer, 0 failures.

## 10. Helper function index

- `general_mind.rs::render_context` — sorted BTreeMap → stable multi-line block
- `general_mind.rs::render_composition` — alias for clarity
- `mission_planner.rs::unfence` — strip markdown code fences
- `mission_planner.rs::to_task` — `TaskSpec` → `Task` with metadata copy
- `mission_planner.rs::task_count_by_status` — count tasks in a given status
- `mission_planner.rs::calculate_progress` — fraction of tasks complete
- `mission_planner.rs::calculate_velocity` — tasks completed per hour since earliest created_at
- `swarm_orchestrator.rs::summarize_swarm` — agent roster → BTreeMap of "Rank-MOS" → count
- `swarm_orchestrator.rs::assign_tasks` — task DAG → agent assignment map
- `swarm_orchestrator.rs::mos_code` — `MOSEnum` → "12B" code
- `swarm_orchestrator.rs::completed_by_planner_id` — resolve planner task_id references
- `swarm_orchestrator.rs::generate_aar` — live state → AAR

## 11. Environment dependencies

None beyond what the LLM substrate requires. Mission orchestration has no filesystem state beyond what the state store and doctrine loader provide.

## 12. Upstream / downstream relationships

**Upstream (orchestrator depends on):**
- `SqliteStateStore` (persistence)
- `FileSystemDoctrineLoader` (prompt assembly)
- `LLMClient` trait (inference)
- `LlmRouter` (tier → provider/model)
- `AgentFactory` (not directly — used at swarm spawn time, pre-execute)
- `C2Router` (not directly — orchestrator does not route orders; it executes tasks via direct LLM calls)

**Downstream (uses orchestrator):**
- `daemon.rs::handle_rr_execute_mission` + `handle_rr_disband`
- `api_routes.rs::execute_swarm` + `disband_swarm` + `quick_mission`
