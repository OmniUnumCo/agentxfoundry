# Rational Reserve -- Agent primer

This file is read by Codex CLI, Qwen-Code, OpenCode, and Mistral Vibe (and any other coding agent that follows the `AGENTS.md` convention).

## What this system provides

**Rational Reserve (RR)** is a military-hierarchy swarm orchestration system. It exposes three MCP tools for spawning and querying swarms:

### `rr_deploy_swarm`
Spawn a new swarm. Required parameters:
- `formation`: `single` | `fire_team` | `squad` | `platoon`
- `primary_mos`: Military Occupational Specialty code (e.g. `12B` for Combat Engineer, `35F` for Intelligence Analyst)

Optional:
- `support_mos`: second MOS for mixed formations
- `swarm_name`: human-readable name

### `rr_status`
Return the current state of a swarm by `swarm_id`.

### `rr_roster`
List agents with optional filters (`swarm_id`, `status`, `rank`, `mos`).

## Shared state across agents

All coding agents on this machine share the same RR state store at `~/.rational-reserve/state/rr.db`. A swarm you deploy from this session is visible from every other agent, and vice versa. The underlying daemon is auto-started on first use; you do not need to manage its lifecycle.

## When to use RR

Deploy a swarm when the task would benefit from hierarchical decomposition -- multi-part feature work, cross-cutting refactors, audits that span specialties. Skip it for single edits or read-only questions.

## Military ranks at a glance

Authority ordering (high -> low): `GEN` > `COL` > `MAJ` > `CPT` > `LT` > `SGM` > `MSG` > `SGT` > `CPL` > `SPC` > `PFC` > `PVT`. Higher ranks command lower ranks within the chain of command.

## Military Occupational Specialties at a glance

- **Intel**: `35F` Analyst, `35L` Counterintel, `35N` SIGINT
- **Ops**: `11B` Infantry, `12B` Combat Engineer, `13B` Artillery, `19D` Cavalry Scout
- **Support**: `25B` IT, `42A` HR, `88M` Transport, `92G` Logistics
- **SOF**: `18B` SF Engineer, `75R` Ranger, `160` SOAR

## Skills

RR ships four `SKILL.md` skills (installed in your agent's skill directory by `install.sh`):

- `rr-operations` -- how and when to use the MCP tools
- `rr-rank-doctrine` -- rank authority and behavior reference
- `rr-mos-catalog` -- MOS lookup and selection guide
- `rr-protocols` -- SITREP/FRAGO/CASREP/AAR communication protocols

## Documentation

Full doctrine: `~/.rational-reserve/doctrine/` (or the source tree at `/Users/nnos/Projects/rational-reserve/doctrine/`).

## Version note

v0.1 spawns agents as inert records (state is persisted, no LLM execution). v0.3 wires up execution.
