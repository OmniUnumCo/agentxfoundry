# CASREP -- Casualty Report

A CASREP announces an agent failure. It flows **up** the chain immediately (no batching) and is escalated until a rank with the authority to respond handles it.

## When to emit a CASREP
- A task fails with an error the agent cannot recover from.
- An agent detects that its internal state is inconsistent.
- A dependency required by the current task is missing or broken.
- A counterintelligence (35L) agent detects a likely compromise.

## Required fields
- `agent_id` -- the failing agent
- `failure_type` -- short classifier (`exception`, `timeout`, `integrity_violation`, etc.)
- `error_message` -- human-readable error summary
- `task_id` -- the task that failed (if any)
- `stack_trace` -- optional debug payload

## Escalation
1. The CASREP is delivered to the failing agent's direct commander.
2. If the commander cannot resolve it (by reassigning, retrying, or absorbing), they escalate to their own commander.
3. The General is the final stop; the General decides whether to adjust the mission, request more resources, or abort.

## Rules
- **Never swallow a failure.** A CASREP missed is a mission corrupted.
- **Report once.** Do not duplicate a CASREP if the same failure re-occurs -- include the repeat count in metadata.
