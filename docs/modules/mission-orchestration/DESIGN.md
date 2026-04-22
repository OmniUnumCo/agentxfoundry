# Design Specification — Mission Orchestration

## 1. Architecture overview

```
+----------------+    +-------------------+   +---------------+
| DaemonState    |    | FileSystemDoctrine|   | LlmRouter     |
| state_store    |<---|  Loader           |   | + RouterConfig|
| doctrine       |    +-------------------+   +---------------+
| llm            |           ^                       ^
| router         |           |                       |
+----+-----------+           | assemble_task_prompt  | route
     |                       |                       |
     v                                               |
+----------------+    +-------------------+   +---------------+
| SwarmOrchestr. |<---| MissionPlanner    |-->| DeterministicL|
| execute_mission|    | plan/adapt/predict|   | LMClient      |
| disband_swarm  |    +-------------------+   +---------------+
| execute_single |
|   _task        |
+----+-----------+
     |
     v
+----------------+   +-------------------+
| C2Router       |   | AgentFactory      |
| (military)     |   | spawn_general,    |
+----------------+   | spawn_specialist  |
                     +-------------------+
```

## 2. Module responsibilities

| Module | Responsibility |
|---|---|
| `general_mind.rs` | Prompt templates only. Pure string generation, no LLM calls. |
| `mission_planner.rs` | LLM-driven task decomposition + adaptation + prediction. Returns typed task lists. |
| `swarm_orchestrator.rs` | Mission lifecycle: plan → assign → execute DAG → persist events. Disband + AAR. |
| `agent.rs::execute_task` | Single-task convenience API. Orchestrator does not route through this; it executes tasks inline for event-log consistency. |

## 3. General-Mind mode dispatcher

```
unified_strategic_prompt(objective, context, mode)
          |
   +------+------+------+-----------+
   |             |      |           |
 Recursive  Reactive Predictive   Full
 (thinking) (complex) (engine) (all three concatenated)
```

`apply_general_mind` gates on `agent_rank == "GEN"` — only Generals receive the unified framework. Other ranks call specific templates directly.

## 4. Task-graph execution

Sequential loop with iteration-bounded deadlock detection:

```
max_iterations = tasks.len() * 2
for iter in 0..max_iterations:
    if all tasks complete: break
    made_progress = false
    for task in tasks:
        if task already complete: continue
        if not all deps in completed: continue
        execute task → append TaskExecution, mark completed
        made_progress = true
    if not made_progress: return DependencyDeadlock
```

**Why doubling task count?** One extra pass for edge cases where a task has no dependencies and must be picked up in a fresh iteration. `n * 2` is a pragmatic ceiling.

**Why sequential in E.1/E.2?** Parallelism introduces determinism challenges (ordering of event-log writes, race conditions on shared state). Sequential execution keeps the initial port auditable. `tokio::join_all` is Slice E.3 once the single-tenant path is proven.

## 5. Task assignment policy

```
for each task:
    1. If metadata.assigned_mos matches an agent's MOS code → pick first match
    2. Else if metadata.assigned_rank matches an agent's rank → pick first match
    3. Else → round-robin across all agents (advances a cursor)
```

**Why this order?** MOS is more discriminating than rank (an infantry specialist and an engineer specialist both have rank SPC). MOS match → rank match → round-robin balances precision with resilience to LLM output that omits one field.

**Why round-robin fallback instead of error?** Planner may emit tasks without MOS/rank hints. Failing the whole mission because the LLM was terse is fragile. Round-robin is the graceful degradation.

## 6. Prompt assembly — task execution

Orchestrator uses `assemble_task_prompt` (not `assemble_contract_prompt`):

```
You are a {rank} ({rank_full_name}) in the Rational Reserve with MOS {mos_code} ({mos_full_name}).

# Rank doctrine
{rank.md contents}

# MOS doctrine
{mos.md contents}

# Task
{task.description}

Execute this task according to your specialty and rank. Provide a clear, actionable result.
```

Civilian contracts use `assemble_contract_prompt` (different shape, covered in the Civilian Reserve doc triad).

## 7. Event taxonomy

| Event type | When | Payload |
|---|---|---|
| `MISSION_STARTED` | `execute_mission` enter | `{mission_id, swarm_id, objective}` |
| `TASK_STARTED` | before LLM call in `execute_single_task` | `{task_id, agent_id, tier, provider, model}` |
| `TASK_COMPLETED` | after successful LLM response | `{task_id, agent_id, response_chars}` |
| `TASK_FAILED` | (implicit via `TaskExecution.status = Failed`; no dedicated event yet) | — |
| `SWARM_DISBANDED` | `disband_swarm` | `{swarm_id, agent_count, success}` |
| `AAR_GENERATED` | `disband_swarm` after generate_aar | full `AAR` JSON |
| `AGENT_DEACTIVATED` | per-agent during disband | `{agent_id}` |

## 8. AAR generation

```
success = mission.status == Complete
objectives_met = [mission.objective] if success else []
objectives_missed = [mission.objective] if !success else []
agent_performance[agent.id] = {
    name, rank (Debug fmt), mos (Debug fmt),
    missions_completed, performance_score, status
}
resource_usage.total_agents = agents.len()
timeline = [{created, activated, disbanded}]
lessons_learned = []   # populated by future reducers over event log
recommendations = []   # populated by future reducers
```

**Why not call an LLM to write the AAR narrative?** That was the Python v0.1 stub ("Phase 1 implementation: General-only execution"). For the Rust port, AAR is a structured record sourced from live data. A "narrative AAR" LLM pass can be added as a separate call; the structured AAR remains the source of truth.

## 9. Error propagation

```
OrchestrateError::UnknownSwarm          ← load_swarm returned None
OrchestrateError::Persistence           ← state_store call failed
OrchestrateError::EmptySwarm            ← swarm has zero agents
OrchestrateError::PlanFailed            ← wraps PlanError from mission_planner
OrchestrateError::UnknownTier           ← router has no route for tier
OrchestrateError::LlmFailure            ← provider returned Err
OrchestrateError::NoAgentMatch          ← (unreachable given round-robin fallback)
OrchestrateError::DependencyDeadlock    ← iteration exhaustion
```

## 10. Key design decisions

1. **Fail loud on malformed LLM output.** Python had a text-fallback parser; we removed it because silent degradation hides LLM drift. A planner producing unparseable output is a defect — let the caller see it.
2. **Orchestrator holds the LLM, not agents.** Simpler state management. `RRAgent::execute_task` still works when an LLM is explicitly injected, but the main mission path goes through `SwarmOrchestrator::execute_single_task` which calls `state.llm.complete` directly.
3. **Sequential execution.** Event-log auditability first; parallelism later.
4. **Task metadata as source of truth for assignment.** Alternative: assign at plan time. We chose assignment-at-execute-time because it decouples planning from current roster state.
5. **AAR is structured, not narrative.** Narrative AARs can be added as a downstream LLM pass; the structured record is the persistent truth.

## 11. Determinism profile

- **Prompt assembly:** byte-stable (tested 1000x)
- **Task assignment:** deterministic given stable agent list (Vec iteration order)
- **Dependency resolution:** deterministic given stable task list ordering
- **LLM call result:** delegated to `DeterministicLLMClient` wrapper (temp=0 + cache)
- **AAR generation:** deterministic given stable input (agents are iterated in load order; HashMap fields in AAR serialize deterministically via serde_json's BTreeMap-backed Value)

## 12. Open design questions

- **Mid-flight adaptation.** `adapt_plan` exists but is not invoked during `execute_mission`. When a task fails, should the orchestrator call `adapt_plan` and splice in the adapted task list? Deferred.
- **AAR narrative.** Should disband optionally call an LLM to produce a human-readable narrative AAR from the structured one? Deferred; would add a `generate_aar_narrative(aar) → String` function.
- **Task retry.** Currently terminal. Should retryable errors (HTTP 5xx from provider) auto-retry N times? Likely yes, at provider level, not orchestrator level.
