# Design Specification — HTTP Surface

## 1. Architecture

```
+-------------------------+      +-------------------------+
| HTTP clients            |      | MCP clients             |
| (curl, browsers, tools) |      | (Claude Code, Codex...) |
+-----------+-------------+      +------------+------------+
            | HTTP                             | stdio JSON-RPC
            v                                  v
+-------------------------+      +-------------------------+
|  rr-http (axum)         |      |  rr-mcp (stdio shim)    |
|  api_routes::build_     |      |  routes to daemon via   |
|    router(state)        |      |  unix socket            |
+-----------+-------------+      +------------+------------+
            |                                  |
            +---------+------------+-----------+
                      |            |
                      v            v
            +---------------------------------+
            |  Shared DaemonState             |
            |  state_store, doctrine, llm,    |
            |  router, factory, logger        |
            +---------+-----------------------+
                      |
                      v
            +---------------------------------+
            |  SQLite WAL (single file)       |
            +---------------------------------+
```

**Both transports serve the same state.** A mission created via HTTP is visible to MCP clients on the next call; a civilian registered via MCP is visible via HTTP. The daemon is the source of truth.

## 2. axum router composition

```rust
pub fn build_router(state: Arc<DaemonState>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/mission/create", post(create_mission))
        .route("/swarm/{swarm_id}/execute", post(execute_swarm))
        .route("/swarm/{swarm_id}/status", get(get_swarm_status))
        .route("/swarm/{swarm_id}/disband", post(disband_swarm))
        .route("/swarms", get(list_swarms))
        .route("/stats", get(get_stats))
        .route("/agents/roster", get(get_agents_roster))
        .route("/mission/quick", post(quick_mission))
        .with_state(state)
}
```

Handlers extract `State(state): State<Arc<DaemonState>>` and, where needed, `Path(id): Path<String>` + `Json(body): Json<T>`.

## 3. Endpoint → handler mapping

| Endpoint | Handler | Core call |
|---|---|---|
| `GET /` | `root` | hardcoded identity |
| `POST /mission/create` | `create_mission` | `state_store.save_mission` + spawn agents + `state_store.save_swarm` |
| `POST /swarm/{id}/execute` | `execute_swarm` | `SwarmOrchestrator::new(...).execute_mission(swarm_id, mission)` |
| `GET /swarm/{id}/status` | `get_swarm_status` | `state_store.load_swarm` + `list_agents(Some(swarm_id), ...)` |
| `POST /swarm/{id}/disband` | `disband_swarm` | `SwarmOrchestrator::new(...).disband_swarm(swarm_id)` |
| `GET /swarms` | `list_swarms` | `state_store.list_swarms(None)` |
| `GET /stats` | `get_stats` | `event_count` + `list_swarms` + `list_civilians` |
| `GET /agents/roster` | `get_agents_roster` | `list_agents(None, None, None, None)` |
| `POST /mission/quick` | `quick_mission` | composition of `create_mission` + `execute_swarm` |

## 4. Error mapping

```rust
pub struct ApiError { status: StatusCode, message: String }

impl ApiError {
    fn bad_request(msg) -> Self { status: 400, ... }
    fn not_found(msg) -> Self { status: 404, ... }
    fn internal<E: Display>(e) -> Self { status: 500, ... }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(json!({"error": self.message}))).into_response()
    }
}
```

Handlers return `Result<Json<T>, ApiError>`. axum's `Result` responder + our `IntoResponse` yield JSON error bodies with correct status codes automatically.

**Category mapping:**
- Deserialization failure → axum built-in 400 (before handler runs)
- `load_swarm → None` → `ApiError::not_found`
- Unknown MOS code / formation → `ApiError::bad_request`
- `state_store.save_*` error → `ApiError::internal`
- `SwarmOrchestrator` error → `ApiError::internal` (could be more granular — deferred)
- Parse errors (priority, MOS) → `ApiError::bad_request`

## 5. Request / response types

Request DTOs use `#[derive(Deserialize)]` with `#[serde(default = "...")]` for optional fields. Response DTOs use `#[derive(Serialize)]`.

### `CreateMissionRequest`
```rust
struct CreateMissionRequest {
    objective: String,
    description: Option<String>,
    priority: String,          // default "routine"
    formation: String,         // default "single"
    primary_mos: String,       // default "18B"
    swarm_name: Option<String>,
}
```

### `CreateMissionResponse`
```rust
struct CreateMissionResponse {
    mission_id: String,
    swarm_id: String,
    agent_ids: Vec<String>,
    agent_count: usize,
}
```

### `ExecuteMissionResponse`
```rust
struct ExecuteMissionResponse {
    swarm_id: String,
    tasks_executed: usize,
    success: bool,            // all tasks StatusEnum::Complete
}
```

## 6. `rr-http` binary

```rust
#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let daemon = Daemon::new()?;
    let state = Arc::new(daemon.state_clone());
    let app = build_router(state);

    let bind = std::env::var("RR_HTTP_BIND").unwrap_or_else(|_| "127.0.0.1:8787".to_string());
    let addr: SocketAddr = bind.parse()?;
    let listener = TcpListener::bind(addr).await?;

    eprintln!("rr-http listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
```

`Daemon::new()` does all the heavy lifting (LLM router, cache, persistence). `state_clone()` returns an owned `DaemonState` (which is `Clone` because all its fields are `Arc<_>`).

## 7. Concurrency model

- Every endpoint is async, returning `Future<Output = Result<Json<T>, ApiError>>`.
- SwarmOrchestrator is constructed per-request (cheap — just holds `Arc`s).
- SQLite WAL allows concurrent readers. Writes serialize via the `Mutex<Connection>` inside `SqliteStateStore`.
- Long-running LLM calls on `/swarm/{id}/execute` do NOT block other requests (async IO).

## 8. Coexistence with `rr-daemon` and `rr-mcp`

Three binaries, one source-of-truth state:

| Binary | Transport | Port / path |
|---|---|---|
| `rr-daemon` | Unix socket | `$RUN_DIR/rr.sock` |
| `rr-mcp` | stdio (spawned subprocess) | inherited |
| `rr-http` | TCP | `RR_HTTP_BIND` (default `127.0.0.1:8787`) |

All three construct the same `DaemonState` from the same config + env. The underlying SQLite file handles concurrent access.

## 9. Key design decisions

1. **axum over warp or actix-web.** axum's extractor model composes cleanly with `State<Arc<T>>`; Tower ecosystem for middleware.
2. **JSON everywhere.** No HTML, no forms, no XML. Matches the Python FastAPI shape.
3. **No auth in this module.** Production exposure is an ops concern (reverse proxy + OAuth or mTLS). Adding auth would be a separate module layer.
4. **No streaming.** Request/response only. Long-running LLM calls are wrapped in a single response; caller can poll `/swarm/{id}/status` for incremental state.
5. **Structured error bodies.** `{"error": "..."}`  is simpler than full Problem+JSON — and matches what the Python side produced.
6. **Separate binary.** Could be combined with `rr-daemon` (bind both unix-socket and TCP in one process). Kept separate for operational clarity and minimal surface on rr-daemon.

## 10. Operational notes

- **Graceful shutdown:** axum::serve handles SIGTERM on tokio runtime by default. Ctrl-C on a foreground `rr-http` process terminates cleanly.
- **Logging:** Currently stderr-only via `eprintln!`. Full `tracing` integration deferred; would add `tower-http::trace::TraceLayer`.
- **CORS:** `tower-http` imported but not enabled. Production UIs should wire up CORS explicitly in this module when needed.
- **TLS:** terminate at a reverse proxy (nginx, caddy, traefik). TLS in-process deferred.

## 11. Future extension paths

- **OpenAPI spec generation.** `utoipa` or `aide` can generate OpenAPI from the handlers' types.
- **Pagination.** `/agents/roster`, `/swarms` should accept `?limit=&offset=` once rosters grow.
- **Request tracing.** Add `tower-http::trace::TraceLayer` + `tracing-subscriber` for structured logs.
- **WebSocket event stream.** Subscribe to live events from the event log (`tokio::sync::broadcast` fanout from the state store).
- **MCP-over-HTTP.** The MCP stdio shim can optionally expose an HTTP-SSE endpoint for remote MCP clients — would build on this surface.
