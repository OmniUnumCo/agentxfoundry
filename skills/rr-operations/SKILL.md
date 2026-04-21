---
name: rr-operations
description: Use when deploying or monitoring Rational Reserve swarms for complex task execution. Invoke the rr_deploy_swarm, rr_status, and rr_roster MCP tools to spawn a military-hierarchy agent swarm, check its state, or list agents by rank/MOS/swarm. Triggers include phrases like "deploy a swarm", "spin up RR", "check swarm status", "list the roster".
---

# Rational Reserve -- Operations

Rational Reserve (RR) is a military-hierarchy swarm orchestration system accessible via three MCP tools:

## Tools

### `rr_deploy_swarm`
Spawn a new swarm. Required params: `formation`, `primary_mos`. Optional: `support_mos`, `swarm_name`.

**Formations:**
- `single` -- 1 General + 1 Specialist (2 agents)
- `fire_team` -- 1 General + 1 Corporal + 3 Specialists (5 agents)
- `squad` -- 1 General + 7-agent squad (1 SGT + 2 CPL + 4 SPC) = 8 agents
- `platoon` -- 1 General + 25-agent platoon (1 LT + staff + 3 squads) = 26 agents

**Primary MOS codes** (what specialty the agents embody):
- Intelligence: `35F` Intelligence Analyst, `35L` Counterintelligence, `35N` SIGINT
- Operations: `11B` Infantry, `12B` Combat Engineer, `13B` Artillery, `19D` Cavalry Scout
- Support: `25B` IT Specialist, `42A` HR, `88M` Transport, `92G` Logistics
- Special Ops: `18B` SF Engineer, `75R` Ranger, `160` SOAR

### `rr_status`
Return live swarm state. Required param: `swarm_id` (from `rr_deploy_swarm`).

### `rr_roster`
List agents with optional filters: `swarm_id`, `status`, `rank`, `mos`.

## When to use RR

- **Complex tasks** that benefit from hierarchical decomposition and clear ownership.
- **Cross-cutting work** that touches multiple specialties (code + docs + tests + infra).
- **Tasks needing a specific "voice"** -- the rank+MOS doctrine shapes how agents approach the work.

## When NOT to use RR

- A single focused edit (just do it inline)
- Read-only questions about the codebase
- Tasks under ~5 minutes

## Typical workflow

1. Call `rr_deploy_swarm` with a formation matching the task's complexity.
2. Use the returned `swarm_id` with `rr_status` to observe the hierarchy.
3. (v0.3+) Call `rr_execute_mission` to have the General actually work the task.
4. Call `rr_disband` when finished to generate an AAR.

## Notes

- State is shared across all coding agents on this machine -- a swarm deployed from Claude Code is visible from Codex CLI and vice versa.
- Agents in v0.1 are inert records; LLM execution arrives in v0.3.
- The daemon is started automatically on first use; one daemon serves all host agents.
