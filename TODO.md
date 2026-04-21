# TODO: Rust Conversion of Rational Reserve

## Phase 0: Project Scaffolding

- [x] `Cargo.toml` — crate config, dependencies (serde, tokio, rusqlite, uuid, chrono, thiserror, async-trait, dirs, toml, libc)
- [x] `src/lib.rs` — module declarations, public API
- [x] `src/bin/rr-daemon.rs` — daemon binary entry point
- [x] `src/bin/rr-mcp.rs` — MCP shim binary entry point

## Phase 1: Core Models (no dependencies on runtime)

- [x] `src/models.rs` — all enums and structs
  - [x] `RankCategory`, `RankEnum` (with authority levels, full names, outranks, can_command_by_rank)
  - [x] `MOSCategory`, `MOSEnum` (with category lookup, full names, code serialization)
  - [x] `StatusEnum`, `PriorityEnum`, `OrderType`, `UnitType`
  - [x] `Task`, `Mission`, `Order`, `SitRep`, `CasRep`, `AAR`, `Unit`, `Swarm`, `AgentRecord`
  - [x] All derive `Serialize`, `Deserialize`, `Debug`, `Clone`, `PartialEq`

## Phase 2: Trait Definitions

- [x] `src/traits.rs`
  - [x] `LLMClient` trait (async `complete`)
  - [x] `StateStore` trait (append_event, load/save agent/swarm/mission, list_agents, list_swarms)
  - [x] `DoctrineLoader` trait (version, rank_doctrine, mos_doctrine, protocol_template, assemble_task_prompt)
  - [x] `Clock` trait (now → DateTime<Utc>) + `SystemClock` impl
  - [x] `EventLogger` trait (log event dict)

## Phase 3: RRAgent

- [x] `src/agent.rs`
  - [x] `RRAgent` struct with all fields
  - [x] `RRAgentError`, `CommandAuthorityError`, `LLMNotAvailableError`
  - [x] `is_officer`, `is_nco`, `is_enlisted`, `can_command`
  - [x] `add_subordinate`, `remove_subordinate`
  - [x] `receive_order`
  - [x] `execute_task` (async, returns error if no LLMClient)
  - [x] `delegate_task` → returns `Order`
  - [x] `report_sitrep`, `report_casrep`
  - [x] `to_record`, `from_record`

## Phase 4: AgentFactory

- [x] `src/factory.rs`
  - [x] `AgentFactory` struct with registry, counter, doctrine ref (Arc)
  - [x] `AgentFactoryError`
  - [x] `spawn_agent`, `spawn_general`, `spawn_officer`, `spawn_nco`, `spawn_specialist`
  - [x] `spawn_fire_team`, `spawn_squad`, `spawn_platoon`
  - [x] `get`, `all_agents`, `by_rank`, `by_mos`, `by_status`, `by_swarm`, `deactivate`
  - [x] `stats`

## Phase 5: C2Router

- [x] `src/c2_router.rs`
  - [x] `C2Router` struct with AgentLookup fn, optional StateStore
  - [x] `ChainCycleError`, `CommandAuthorityError`, `AgentNotFound`
  - [x] `walk_chain_upward`, `is_ancestor`, `validate_command_relationship`
  - [x] `check_no_cycle_on_spawn`
  - [x] `route_order`, `broadcast_order`
  - [x] `route_sitrep`, `route_casrep`
  - [x] Audit log accessors

## Phase 6: Doctrine Loader

- [x] `src/doctrine.rs`
  - [x] `DoctrineError`, `DoctrineNotFound`
  - [x] `resolve_doctrine_root` (env var → ~/.rational-reserve/doctrine → packaged fallback)
  - [x] `FileSystemDoctrineLoader` with caching (Clone, Arc-based internals)
  - [x] `assemble_task_prompt`
  - [x] `default_doctrine_loader`

## Phase 7: Config & Paths

- [x] `src/config.rs`
  - [x] `Paths` struct with derived paths (db, pid, socket, log)
  - [x] `Config` struct
  - [x] `load_config` from `~/.rational-reserve/config.toml`

## Phase 8: Logger

- [x] `src/logging.rs`
  - [x] `JsonlLogger` with thread-safe append
  - [x] JSONL output with UTC timestamps

## Phase 9: SQLite Persistence

- [x] `src/persistence.rs`
  - [x] `SqliteStateStore` using `rusqlite`
  - [x] WAL mode, event log table, current state tables
  - [x] Schema from `src/rr/persistence/schema.sql`
  - [x] `close()` method

## Phase 10: Daemon

- [x] `src/daemon.rs`
  - [x] `Daemon` struct: owns StateStore, Factory, Router, Doctrine, Logger
  - [x] Unix socket server (tokio)
  - [x] Line-delimited JSON RPC dispatch
  - [x] Handlers: `ping`, `rr_roster`, `rr_deploy_swarm`, `rr_status`
  - [x] PID file management
  - [x] `is_daemon_alive`, `ensure_daemon` helpers

## Phase 11: MCP Server

- [x] `src/bin/rr_mcp.rs`
  - [x] MCP stdio server
  - [x] `DaemonClient` proxy over unix socket
  - [x] Tool registrations: `rr_roster`, `rr_deploy_swarm`, `rr_status`

## Phase 12: Tests

- [x] `tests/test_models.rs` — 16 tests: enum serialization, rank authority, MOS categories, model round-trips
- [x] `tests/test_c2_router.rs` — 9 tests: chain walking, cycle detection, order routing, SITREP/CASREP
- [x] `tests/test_agent.rs` — 16 tests: agent lifecycle, factory formations, order receipt, reports
- [x] `tests/test_sqlite_store.rs` — 9 tests: schema idempotency, event append, upsert/load, filters

## Phase 13: Build & Verify

- [x] `cargo build` — zero errors ✅
- [x] `cargo test` — **50 passed, 0 failed** ✅
- [x] `cargo clippy` — clean (2 minor type-complexity warnings on SQLite row tuples) ✅

## Dependencies (planned)

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
rusqlite = { version = "0.32", features = ["bundled"] }
thiserror = "2"
toml = "0.8"
tracing = "0.1"
```

## Notes

- Python source of truth: `src/rr/`, `src/rr_runtime/`, `src/rr_mcp/`
- All business logic ported 1:1; no behavioral changes
- Doctrine markdown files remain as-is (filesystem resources, not code)
- Tests should be ported with equivalent coverage; Hypothesis → proptest where applicable
