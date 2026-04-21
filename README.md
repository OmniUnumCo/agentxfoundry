# Rational Reserve

> *"Many minds, one mission."*

Military-hierarchy AI swarm orchestration -- a shared substrate for coordinating multi-agent work from **any** coding agent you use (Claude Code, Codex CLI, Qwen-Code, OpenCode, Mistral Vibe).

## What this is

Rational Reserve (RR) models complex task execution after a military command structure: agents hold military ranks (General -> Specialist) and Military Occupational Specialties (MOS: Combat Engineer, Intelligence Analyst, Cavalry Scout, ...). Missions are decomposed and routed through a chain of command. Status flows back up as SITREPs; adjustments flow down as FRAGOs; failures surface as CASREPs; completed missions generate After-Action Reviews.

Alongside the military hierarchy, RR also supports a parallel **Civilian Reserve (CR)** class of project-specialized contractors that military agents hire for domain-specific work (see the [Civilian Reserve](#civilian-reserve-cr) section below).

This repository is a **deterministic, compiled Rust implementation**. Every agent, router, loader, and handler is statically linked; there is no interpreted runtime in the hot path. Designed from day one to be invoked from any MCP-capable coding agent, sharing a single local state store.

## Architecture in one diagram

```
+------------------------------------------+
|  Host agents (Claude Code, Codex, Qwen,  |
|  OpenCode, Mistral Vibe, ...)            |
+---------------+--------------------------+
                | MCP stdio (JSON-RPC)
+---------------v--------------------------+
|  src/bin/rr_mcp.rs -- stdio shim         |
+---------------+--------------------------+
                | unix socket (line-JSON)
+---------------v--------------------------+
|  src/bin/rr_daemon.rs + src/daemon.rs    |
|  -- singleton daemon + dispatch          |
+---+----------+---------+---------+-------+
    |          |         |         |
    v          v         v         v
 models.rs  persistence c2_router doctrine
 traits.rs  (SQLite WAL)  (chain   (markdown
 agent.rs                 of cmd   + civilian
 factory.rs               + contr) YAML front-
                                   matter)
```

- **`src/`** -- core Rust library. Pure business logic behind `Send + Sync` trait boundaries (`DoctrineLoader`, `StateStore`, `LLMClient`, `Clock`, `EventLogger`). No networking, no SQLite, no LLM imports inside the `lib` crate -- all concrete implementations are injected.
- **`src/persistence.rs`** -- `SqliteStateStore` with WAL journal mode + append-only event log + projection tables for agents, swarms, missions, **civilians**, and **contracts**.
- **`src/daemon.rs` + `src/bin/rr_daemon.rs`** -- the singleton daemon. Binds a unix socket at `~/.rational-reserve/run/rr.sock`, dispatches line-delimited JSON-RPC, is the *one and only* writer to the event log.
- **`src/bin/rr_mcp.rs`** -- the MCP stdio shim. Each host agent spawns its own shim subprocess; all shims connect to the single daemon. This is how N concurrent hosts share one state store without corrupting SQLite.

All state mutations flow through the daemon. The event log is the source of truth; projection tables are derived.

## Civilian Reserve (CR)

Civilian Reserve contractors are a separate class of agents from military MOS ranks. They are project-specialized domain experts that military agents *contract* for work outside their MOS wheelhouse.

### Structure

- **Firms** -- umbrella civilians with internal specialists. Military agents always address the firm; the firm routes work to the right specialist internally.
- **Specialists** -- members of a firm; cannot be contracted directly.
- **Independents** -- flat civilians with a single scope; contracted the same way as a firm.

Civilians live outside the chain of command. They have no `commander_id`, no subordinates, and cannot be ordered via `route_order`. Military agents interact with them exclusively through `route_contract_request`.

### Contract lifecycle

```
requested -> accepted | rejected
accepted  -> in_progress  (implicit on first CONTRACT_SITREP)
in_progress -> closed
```

Every state transition appends a typed event (`CONTRACT_REQUESTED`, `CONTRACT_ACCEPTED`, `CONTRACT_REJECTED`, `CONTRACT_SITREP`, `CONTRACT_CLOSEOUT`) and updates the `contracts` projection row. Registration events (`CIVILIAN_REGISTERED`, `FIRM_REGISTERED`) bootstrap a civilian from its doctrine file.

### Doctrine layout

```
doctrine/
+-- VERSION                 # military doctrine semver
+-- ranks/                  # GEN.md, Colonel.md, ... (military)
+-- mos/                    # 11b-infantry.md, 12b-combat-engineer.md, ...
+-- protocols/              # sitrep.md, frago.md, ...
+-- civilian/
    +-- VERSION             # civilian doctrine semver (independent of military)
    +-- flat/
    |   +-- arena.md        # flat/independent civilians
    +-- firms/
        +-- return42/
        |   +-- firm.md
        |   +-- specialist-morphlex.md
        |   +-- specialist-jasterish.md
        +-- pitchfork/
            +-- firm.md
            +-- specialist-*.md
```

Each civilian doctrine file is markdown with a frontmatter header:

```yaml
---
slug: arena
kind: independent        # firm | specialist | independent
firm: null               # required iff kind=specialist
project_path: ~/Projects/arena
upstream_url: null
languages: [rust]
tier_override: null      # or: strategic|tactical|supervision|execution_guidance|execution
contract_scope: [multi-agent-consensus, drift-detection, ...]
---

# CR-Arena

## Scope
## Core competencies
## When to contract this specialist
## Out of scope
## Deliverables
## Execution guidance
```

Slugs are validated against a strict allowlist (`[a-z0-9][a-z0-9_-]{0,63}`) before any filesystem access; paths are canonicalized and verified to remain within the doctrine root as defense in depth against CWE-22.

### Reserve roster (planned)

**Firms:** CR-Pitchfork, CR-DeveloperPortal, CR-KnockoutEDA, CR-Return42, CR-MAB.
**Flat civilians:** CR-Arena (shipped), CR-BTCWallet, CR-OpenChoreo.

## State lives in `~/.rational-reserve/`

```
~/.rational-reserve/
+-- state/rr.db            # SQLite WAL: event log + current_state projection
+-- run/rr.pid             # Daemon PID
+-- run/rr.sock            # Unix socket for MCP shims
+-- logs/c2.jsonl          # Structured JSONL log stream
+-- doctrine/              # Version-pinned doctrine (copy of source)
+-- config.toml            # Optional: LLM keys, rank-to-LLM overrides (v0.3)
```

The `doctrine_version` field stamped on every event row is formatted as `mil-<X.Y.Z>+civ-<A.B.C>` so military and civilian doctrine can evolve independently while still pinning exact reproducibility per event.

## MCP tool surface

The stdio shim registers the following tools:

**Military:**
- `rr_roster` -- list agents with optional filters
- `rr_deploy_swarm` -- deploy a new swarm (single/fire_team/squad/platoon)
- `rr_status` -- live status projection for a swarm

**Civilian Reserve:**
- `cr_register_civilian` -- register a civilian from its doctrine file
- `cr_list_civilians` -- list all registered civilians
- `cr_contract_request` -- initiate a contract (military -> firm)
- `cr_contract_sitrep` -- submit progress (implicit Accepted->InProgress on first call)
- `cr_contract_closeout` -- close a contract with outcome + deliverables

## Optional guard layers

The repo also contains an **optional**, user-scoped guard layer under `tools/guards/`.
This is separate from the RR daemon and is meant for local audit of new listeners and
launchd persistence changes around agent experiments.

The replacement guard design is intentionally conservative:

- runs as **user LaunchAgents**, not root `LaunchDaemons`
- uses `launchd` `StartInterval` scheduling instead of a hot shell loop
- defaults to **audit-first** logging
- never targets Apple/system processes
- only enforces against explicit user-owned targets from inside `$HOME`

See `docs/guards.md` and `adapters/install-guards.sh`.

There is also a separate **system integrity daemon v2** design under
`tools/system-guard/`. That package is aimed at the broader "system integrity daemon"
scope:

- system-wide visibility
- root daemon deployment
- audit-first findings in SQLite + JSONL
- explicit trust policy
- staged rollout before any enforcement

See `docs/system-integrity-daemon.md`.

## Status

**v0.1 -- Spine** (in progress): core Rust library, SQLite persistence, daemon, MCP stdio shim with `rr_roster` / `rr_deploy_swarm` / `rr_status`. Agents are inert records; no LLM execution yet.

**Civilian Reserve Slice A** (shipped): `AgentClass` / `CivilianTier` / `CivilianRecord` / `AnyAgent` model additions, `civilians` + `contracts` projection tables, new event types, civilian refusal in C2 router, `route_contract_request`, YAML-frontmatter doctrine loader, 5 new MCP tools, CR-Arena reference doctrine, end-to-end integration test.

**Civilian Reserve Slice B** (next): firms with specialists, `ContractBroker` firm-internal routing, CR-Return42 + Morphlex specialist authored.

**v0.2 -- Adapter layer**: portable `SKILL.md` pack + per-host primers + `install.sh`. Proves parity across all 5 target agents.

**v0.3** (follow-up spec): LLM routing, `rr_execute_mission`, SITREP/FRAGO/CASREP flows.
**v0.4** (follow-up spec): AAR engine, `rr_disband`, Claude Code plugin.

## Development

```bash
# Build
cargo build --release

# Run the full test suite
cargo test

# Start the daemon in the foreground (development)
cargo run --bin rr-daemon

# Drive the MCP shim manually
cargo run --bin rr-mcp
```

All three daemon <-> shim paths use line-delimited JSON over a unix socket; see `src/daemon.rs::dispatch` for the method catalog.

---

*Rational Reserve is a work in progress. Interfaces are versioned via doctrine files; the daemon's unix-socket protocol is considered an internal surface until v1.0.*
