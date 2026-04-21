# Rational Reserve -- Claude Code primer

This repository is set up to use **Rational Reserve (RR)**, a military-hierarchy AI swarm orchestration system accessible via MCP.

## What you can do here

RR exposes three MCP tools:

- `rr_deploy_swarm` -- spawn a new swarm (formation: `single`, `fire_team`, `squad`, or `platoon`)
- `rr_status` -- return the state of a swarm by `swarm_id`
- `rr_roster` -- list agents with optional filters (`swarm_id`, `status`, `rank`, `mos`)

State is shared with every other coding agent on this machine -- a swarm deployed from Claude Code is visible from Codex CLI, Qwen-Code, OpenCode, and Mistral Vibe.

## Skills available

The `rr-operations`, `rr-rank-doctrine`, `rr-mos-catalog`, and `rr-protocols` skills are installed in `~/.claude/skills/`. Use them when:

- Choosing a formation or MOS (-> `rr-operations`, `rr-mos-catalog`)
- Understanding how rank authority works (-> `rr-rank-doctrine`)
- Writing code that emits SITREP/FRAGO/CASREP (-> `rr-protocols`)

## When to deploy an RR swarm

RR is for **complex task execution** that benefits from hierarchical decomposition. Good candidates:
- Multi-part feature work touching code + tests + docs
- Cross-cutting refactors across several modules
- Audit-style work where agents need distinct "voices" per specialty

Skip RR for:
- Single focused edits
- Read-only codebase questions
- Tasks under ~5 minutes

## Version note

v0.1 spawns agents as **inert records** -- state is persisted and queryable, but no LLM execution happens. v0.3 will wire up execution.
