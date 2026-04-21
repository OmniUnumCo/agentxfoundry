# SESSION: Rust Conversion of Rational Reserve

## Context

Rational Reserve was originally built in Python (interpreted) as a military-hierarchy AI swarm orchestration system with MCP integration for multi-agent coordination across coding agents (Claude Code, Codex, Qwen-Code, etc.).

The user requested **deterministic agents** — **compiled, not interpreted**. The Python version was a mistake. This session covers the full port from Python to Rust.

## Project Structure (Python source of truth)

```
rational-reserve/
├── src/
│   ├── rr/                    # Core library (framework-free pure logic)
│   │   ├── models.py          # Pydantic models: RankEnum, MOSEnum, StatusEnum, AgentRecord, Mission, Task, Order, SitRep, CasRep, AAR, Unit, Swarm
│   │   ├── protocols.py       # Dependency injection protocols: LLMClient, StateStore, DoctrineLoader, Clock, EventLogger
│   │   ├── rr_agent.py        # RRAgent class: orders, task execution, delegation, reports
│   │   ├── agent_factory.py   # AgentFactory: spawn_agent, spawn_fire_team/squad/platoon
│   │   ├── c2_router.py       # C2Router: chain-of-command validation, order/SITREP/CASREP routing
│   │   ├── doctrine.py        # DoctrineLoader: markdown doctrine resolution and caching
│   │   └── persistence/
│   │       └── schema.sql     # SQLite schema (event log + current state)
│   ├── rr_runtime/            # Runtime layer
│   │   ├── daemon.py          # Singleton daemon: unix socket server, RPC dispatch, handlers
│   │   ├── config.py          # Config/Paths: filesystem location resolution
│   │   └── logging.py         # JsonlLogger: structured JSONL event logging
│   └── rr_mcp/                # MCP stdio server (thin shim)
│       ├── server.py          # MCP server, DaemonClient unix-socket proxy
│       └── tools/             # Tool descriptors: roster, deploy, status
├── tests/
│   ├── unit/                  # test_c2_router, test_sqlite_store
│   └── integration/           # test_daemon
├── doctrine/                  # Markdown doctrine files (ranks, MOS, protocols)
├── skills/                    # SKILL.md packs for host agents
└── adapters/                  # Per-host agent primers + install.sh
```

## Rust Target Architecture

```
rational-reserve/
├── Cargo.toml                 # Workspace or single crate
├── src/
│   ├── lib.rs                 # Public API
│   ├── models.rs              # All enums + structs with serde
│   ├── protocols.rs           # Trait definitions (StateStore, DoctrineLoader, etc.)
│   ├── agent.rs               # RRAgent
│   ├── factory.rs             # AgentFactory + formation builders
│   ├── c2_router.rs           # C2Router + chain validation
│   ├── doctrine.rs            # DoctrineLoader (filesystem)
│   ├── config.rs              # Config + Paths
│   ├── logging.rs             # JsonlLogger
│   ├── daemon.rs              # Unix socket server + RPC dispatch
│   ├── persistence.rs         # SQLite store (rusqlite)
│   └── mcp/                   # MCP server (optional, or separate binary)
└── tests/
```

## Key Design Decisions

- **Compiled, deterministic**: Rust, not Python. No interpreter.
- **Dependency injection via traits**: Same Protocol pattern from Python, but using Rust traits.
- **serde for serialization**: All models derive Serialize + Deserialize for JSON/SQLite round-tripping.
- **tokio for async**: The daemon and LLM client paths use tokio runtime.
- **unix sockets**: `tokio::net::UnixStream` for the shim-to-daemon protocol.
- **SQLite**: `rusqlite` for persistence, same WAL mode schema.

## Status

Session started. No Rust code exists yet. Full Python codebase is the source of truth.
