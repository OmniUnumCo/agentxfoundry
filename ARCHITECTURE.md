# Rational Reserve Architecture — Hierarchy & Lifecycle

## Line authority

```
Commander (optional)
    ├── Orchestrator
    │       └── Director(s)     ← own workstreams / operationalize
    │               └── sub-agents (as needed)
    └── SpecOps                 ← reports DIRECTLY to Commander only
```

**SpecOps** bypasses the Orchestrator and Directors entirely: it accepts orders
only from its Commander (`spawn_specops` requires a Commander parent), reports
only to the Commander, and never issues line orders or delegates.

**Specialists** are **freelance advisors** to Directors (and may be engaged by the
Orchestrator to advise a named Director). They **consult**; they do **not** own
the program and do **not** issue orders to Directors.

```
Orchestrator ──engages──► Specialist (advisor)
Director     ◄──advises── Specialist
```

### RoleEnum

| Role | Authority |
|------|-----------|
| `COMMANDER` | Optional top of line; commands Orchestrator |
| `ORCHESTRATOR` | Mission planning & delegation; assigns Directors |
| `DIRECTOR` | Owns a workstream; may attach Specialist advisors; assigns sub-tasks |
| `SPECIALIST` | On-call expert advisor; accepts advisory briefs only from its Orchestrator or the Director it advises |
| `SPECOPS` | Commander-only direct report; outside the Orchestrator/Director line |

### Legacy mapping (Rank / General → RoleEnum)

| Old | New |
|-----|-----|
| General (`GEN`) | `ORCHESTRATOR` (or `COMMANDER` if top-level) |
| COL / MAJ / CPT | `DIRECTOR` |
| SPC specialty experts | `SPECIALIST` (advisor) |
| LT / NCO / enlisted | Operational support under a Director |

`RankEnum` and `MOSEnum` remain as **optional specialty metadata**. Chain-of-command
and factory spawn paths use **`RoleEnum`**.

### Assignment caps

- Orchestrator → **1 Director** per workstream (+ **0–2 Specialist** advisors)
- No flat General→everyone fan-out
- Orchestrator never executes workstream tasks; unmatched tasks stay `BLOCKED`/unassigned
- `C2Router` validates by `RoleEnum` (`VALID_COMMAND_TARGETS` + direct commander); no rank-based overrides

## Lifecycle

```
Idle → Briefed → Executing → Reporting → Idle
```

| Status | Meaning |
|--------|---------|
| `IDLE` | Available (legacy alias: `STANDBY`) |
| `BRIEFED` | Assignment received & acknowledged |
| `EXECUTING` | Doing the work (aliases: `ACTIVE`, `ENGAGED`) |
| `REPORTING` | SitRep done/blocked/next (alias: `COMPLETE` then back to Idle) |
| `FAILED` / `BLOCKED` | Terminal / waiting |

Enforcement: `execute_task` refuses to run unless the agent is `BRIEFED`
(or resuming from `BLOCKED`); the swarm briefs each agent with a full
assignment before execution, and swarm activation moves the Orchestrator
`IDLE → BRIEFED` rather than writing `EXECUTING` directly. Persistence keeps the
legacy `complete` literal for finished missions.

## Assignment contract

On `Order` / `Assignment`:

- `goal`
- `scope_boundary`
- `done_criteria`
- `stop_list`
- `role_name`
- `is_advisory` (true for Specialist briefs)

A line `assignment` missing any of the five contract fields is rejected
(`Assignment.missing_fields()`).

## Factory spawn helpers

- `spawn_commander`, `spawn_orchestrator`, `spawn_director`, `spawn_specialist`, `spawn_specops(commander_id)`
- `spawn_workstream(orchestrator_id, …)` → 1 Director + advisors
- `spawn_general` **deprecated** → wraps `spawn_orchestrator`

## SitRep

`done` / `blocked` / `next` style via `report_sitrep(kind=...)`.
