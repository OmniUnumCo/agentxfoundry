---
name: rr-protocols
description: Reference for the four Rational Reserve communication protocols -- SITREP (status up), FRAGO (orders adjustment down), CASREP (failure report), AAR (after-action review). Use when interpreting RR event logs, writing code that emits these messages, or reasoning about chain-of-command communication.
---

# Rational Reserve -- Communication Protocols

RR uses four named communication types, borrowed from military doctrine. Every routed message becomes an event in `mission_events` so AARs can reconstruct mission history later.

## SITREP -- Situation Report (upward)

Flows **up** the chain of command. Subordinates report status to their direct commander; commanders aggregate and report to theirs.

**When to emit:** status transitions, on-demand, when blockers arise.

**Fields:** `from_agent`, `to_agent` (auto-resolved to commander if None), `status`, `current_task`, `progress` (0.0-1.0), `message`, `blockers`.

**Rule:** terse, specific, honest. Optimistic progress reports corrupt the chain.

## FRAGO -- Fragmentary Order (downward)

A mid-mission adjustment from commander to subordinate. Modifies the original order; does not replace it.

**When to issue:** plan becomes invalid, external conditions change, peer unit requires re-coordination.

**Fields:** `from_agent`, `to_agent`, `order_type=frago`, `content`, `mission_id`.

**Rule:** don't layer FRAGOs. If a second adjustment is needed, replace the first. Use plain direct conversation for trivial clarifications.

## CASREP -- Casualty Report (escalation)

Announces an agent failure. Flows upward immediately (no batching) and escalates until a rank with authority to respond handles it.

**When to emit:** unrecoverable task failure, internal state inconsistency, missing dependency, 35L-flagged compromise.

**Fields:** `agent_id`, `failure_type`, `error_message`, `task_id`, `stack_trace`.

**Rule:** never swallow a failure. Report once per distinct failure; use metadata for repeat counts.

## AAR -- After-Action Review (disband)

Generated when a swarm is disbanded. Captures what happened so future missions benefit.

**Required sections:**
1. What we were trying to do
2. What actually happened (timeline)
3. What worked
4. What did not work
5. Recommendations

**Rule:** be honest about failure. Separate root causes from symptoms. Write for a future reader who was not present.

## Escalation paths

```
SITREP:  SPC -> CPL -> SGT -> LT -> CPT -> MAJ -> GEN
FRAGO:   GEN -> MAJ -> CPT -> LT -> SGT -> CPL -> SPC
CASREP:  failing agent -> direct commander -> (escalated as needed) -> GEN
AAR:     generated once per swarm at disband time
```

## Full protocol templates

Authoritative templates live at `doctrine/protocols/{sitrep,frago,casrep,aar}.md`.
