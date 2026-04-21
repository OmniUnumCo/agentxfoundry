# SITREP -- Situation Report

A SITREP flows **up** the chain of command. Every subordinate reports status to their direct commander; commanders aggregate and roll up to their own commanders; the General sees the synthesized picture.

## When to emit a SITREP
- On demand from a commander.
- On any status transition (STANDBY -> ACTIVE, ACTIVE -> ENGAGED, etc.).
- At regular intervals during long-running tasks (daemon-configurable; default: none for v0.1).
- Immediately when a blocker is encountered that cannot be resolved locally.

## Required fields
- `from_agent` -- the reporting agent's id
- `status` -- current StatusEnum
- `current_task` -- task id (if any)
- `progress` -- 0.0 to 1.0
- `message` -- free-form human summary
- `blockers` -- list of blocker descriptions (empty list when unblocked)

## Tone
- **Terse.** Commanders read SITREPs at a glance.
- **Specific.** "Stuck on database migration" > "having trouble."
- **Honest.** Optimistic progress is a command failure.
