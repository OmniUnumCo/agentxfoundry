# Requirements — Mission Orchestration

**Module:** `src/general_mind.rs` + `src/mission_planner.rs` + `src/swarm_orchestrator.rs` + `src/agent.rs::execute_task`
**Status:** Implemented — all three planning modes (plan/adapt/predict) + full execute-mission loop + disband + AAR.

## 1. Purpose

Turn a natural-language mission objective and a swarm roster into a sequence of executed tasks with per-task LLM calls, persistence, and an auditable after-action review. Port of the Python `rr/general_mind.py` + `rr/mission_planner.py` + `rr/swarm_orchestrator.py` trio.

## 2. Functional requirements

### 2.1 General-Mind prompts

**FR-MO-01.** The system SHALL provide five prompt builders:
  - `recursive_thinking_prompt(objective, context)`
  - `reactive_complex_prompt(objective, current_state, change_detected?)`
  - `predictive_engine_prompt(objective, current_trajectory, horizon_depth)`
  - `unified_strategic_prompt(objective, context, mode)`
  - `mission_decomposition_prompt(objective, constraints, swarm_composition)`

**FR-MO-02.** The system SHALL expose a `GeneralMindMode` enum (`Full`, `Recursive`, `Reactive`, `Predictive`) with string-parse support.

**FR-MO-03.** The system SHALL provide a top-level `apply_general_mind(agent_rank, objective, context, mode)` dispatcher that returns `Some(prompt)` only when `agent_rank == "GEN"`.

**FR-MO-04.** Prompt output SHALL be byte-stable for fixed inputs (sorted `BTreeMap` rendering, no timestamps, no randomness).

### 2.2 Mission planner

**FR-MO-05.** `MissionPlanner::plan_mission(mission, swarm_composition)` SHALL call the injected `LLMClient` with the mission-decomposition prompt at the `"strategic"` tier and parse the response as `{"tasks": [TaskSpec]}`.

**FR-MO-06.** JSON parsing SHALL fail loud. No text-fallback heuristic parser; no single-task fallback. Malformed JSON → `PlanError::MalformedJson` surfaced to the caller.

**FR-MO-07.** `MissionPlanner::adapt_plan(mission, current_tasks, change_detected)` SHALL call the LLM with the reactive-complex prompt and return an updated `Vec<Task>` via the same JSON schema.

**FR-MO-08.** `MissionPlanner::predict_execution(mission, current_tasks, swarm_status, horizon_depth)` SHALL call the LLM with the predictive-engine prompt and return a `Prediction { mission_id, horizon_depth, raw }` wrapper preserving the free-form response.

**FR-MO-09.** Task metadata SHALL capture `assigned_mos`, `assigned_rank`, `is_iterative`, `pivot_triggers`, `predicted_duration`, `success_criteria`, `mission_id`, `planner_task_id`.

### 2.3 Swarm orchestrator

**FR-MO-10.** `SwarmOrchestrator::execute_mission(swarm_id, mission)` SHALL:
  1. Load the swarm and its agents from persistence (fail if swarm empty/unknown)
  2. Persist the mission and emit `MISSION_STARTED`
  3. Invoke the planner
  4. Assign each task to an agent via MOS match → rank match → round-robin fallback
  5. Execute the task graph in dependency-order
  6. Return `Vec<TaskExecution>`

**FR-MO-11.** Task execution SHALL:
  - Assemble a prompt via `DoctrineLoader::assemble_task_prompt(rank, mos, description)`
  - Resolve the tier via `tier_for_rank(rank)` and `LlmRouter::route`
  - Call `LLMClient::complete(prompt, tier, max_tokens, 0.0)`
  - Emit `TASK_STARTED` and `TASK_COMPLETED` events

**FR-MO-12.** Dependency resolution SHALL support both generated UUID `task.id` and planner-assigned `planner_task_id` strings.

**FR-MO-13.** The orchestrator SHALL detect dependency deadlock (an iteration with no progress) and return `DependencyDeadlock { remaining }`.

**FR-MO-14.** `SwarmOrchestrator::disband_swarm(swarm_id)` SHALL:
  - Load swarm + agents + mission
  - Generate an AAR via `generate_aar(swarm, mission, agents)`
  - Mark swarm `Complete`, set `disbanded_at`
  - Persist the updated swarm
  - Emit `SWARM_DISBANDED`, `AAR_GENERATED`, and per-agent `AGENT_DEACTIVATED` events
  - Return `DisbandReport { swarm_id, disbanded_at, aar }`

### 2.4 Agent execution

**FR-MO-15.** `RRAgent::execute_task(task)` SHALL build the prompt via its injected `DoctrineLoader`, call the injected `LLMClient`, update task state accordingly, and return the updated `Task`.

**FR-MO-16.** `RRAgent::execute_task` SHALL fail with `RRAgentError::LLMNotAvailable` if no `LLMClient` is injected at construction time.

## 3. Non-functional requirements

**NFR-MO-01.** Prompt assembly is deterministic. Given fixed inputs the emitted prompt MUST be byte-identical across any number of invocations.

**NFR-MO-02.** The orchestrator MUST log all state transitions to the event log. Loss of the `TaskExecution` vector in memory MUST be recoverable from the event log.

**NFR-MO-03.** Task assignment MUST not starve agents: if a task has no MOS/rank match, round-robin distributes across the full roster rather than stacking on agent[0].

**NFR-MO-04.** Disband MUST be idempotent: calling it on an already-disbanded swarm should not double-emit disband events (defer — currently not enforced; open issue).

## 4. Scope

**In scope:**
- All five General-Mind prompt templates
- `plan_mission`, `adapt_plan`, `predict_execution`
- `execute_mission` with sequential dependency-respecting task graph
- `disband_swarm` with AAR generation
- Task-to-agent assignment (MOS > rank > round-robin)
- Per-task TASK_STARTED / TASK_COMPLETED / TASK_FAILED events
- `RRAgent::execute_task` as a single-task convenience wrapper

**Out of scope:**
- Parallel task execution (sequential for Slice E.1/E.2; `tokio::join_all` is Slice E.3)
- Task retry logic (failure is terminal)
- Mid-flight plan adaptation (`adapt_plan` exists but orchestrator does not invoke it during execution)
- LLM usage cost tracking (future — would live in event payloads)

## 5. Acceptance criteria

1. A plan LLM response containing valid `{"tasks": [...]}` produces a `Vec<Task>` of the correct length with dependencies preserved.
2. A plan LLM response wrapped in ```` ```json ... ``` ```` fences MUST still parse.
3. A malformed plan response MUST produce `PlanError::MalformedJson`.
4. `execute_mission` against a swarm with a single 18B agent and a mission that decomposes into one task MUST complete with `StatusEnum::Complete`.
5. Dependencies between tasks MUST be respected (parent completes before child executes).
6. A cycle or unsatisfiable dependency MUST terminate as `DependencyDeadlock`, not hang.
7. `disband_swarm` on a completed mission MUST emit an AAR whose `success == true` and `objectives_met` contains the mission objective.
8. AAR generation against `None` mission MUST set `mission_id = "unknown"` and `success = false`.
