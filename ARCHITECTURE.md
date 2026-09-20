# Rational Reserve Architecture — Hierarchy & Lifecycle

## Line authority

```
Commander (optional)
    └── Orchestrator
            └── Director(s)     ← own workstreams / operationalize
                    └── sub-agents (as needed)
```

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
| `SPECIALIST` | On-call expert advisor; accepts advisory briefs only |

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

## Assignment contract

On `Order` / `Assignment`:

- `goal`
- `scope_boundary`
- `done_criteria`
- `stop_list`
- `role_name`
- `is_advisory` (true for Specialist briefs)

## Factory spawn helpers

- `spawn_commander`, `spawn_orchestrator`, `spawn_director`, `spawn_specialist`
- `spawn_workstream(orchestrator_id, …)` → 1 Director + advisors
- `spawn_general` **deprecated** → wraps `spawn_orchestrator`

## SitRep

`done` / `blocked` / `next` style via `report_sitrep(kind=...)`.
