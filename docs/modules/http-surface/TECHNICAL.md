# Technical Specification — HTTP Surface

## 1. Module map

| File | Responsibility |
|---|---|
| `src/api_routes.rs` | Router construction, handlers, DTOs, `ApiError` |
| `src/bin/rr_http.rs` | Binary entry point: construct daemon, bind TCP, serve router |
| `Cargo.toml` | `axum = "0.8"`, `tower = "0.5"`, `tower-http = "0.6"` |

## 2. Dependencies added

```toml
axum = "0.8"
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors"] }
```

`tower` and `tower-http` are included for future middleware; neither currently wraps the router.

## 3. Cargo.toml `[[bin]]` entry

```toml
[[bin]]
name = "rr-http"
path = "src/bin/rr_http.rs"
```

Binary name hyphenated (`rr-http`) to match `rr-daemon` and `rr-mcp` conventions.

## 4. `build_router` signature

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

Axum 0.8 syntax: path parameters use `{name}` (not `:name`).

## 5. Handler signatures

```rust
async fn root() -> Json<RootResponse>;
async fn create_mission(
    State(state): State<Arc<DaemonState>>,
    Json(req): Json<CreateMissionRequest>,
) -> Result<Json<CreateMissionResponse>, ApiError>;
async fn execute_swarm(
    Path(swarm_id): Path<String>,
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<ExecuteMissionResponse>, ApiError>;
async fn get_swarm_status(
    Path(swarm_id): Path<String>,
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<serde_json::Value>, ApiError>;
async fn disband_swarm(
    Path(swarm_id): Path<String>,
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<serde_json::Value>, ApiError>;
async fn list_swarms(State(state)) -> Result<Json<serde_json::Value>, ApiError>;
async fn get_stats(State(state)) -> Result<Json<serde_json::Value>, ApiError>;
async fn get_agents_roster(State(state)) -> Result<Json<serde_json::Value>, ApiError>;
async fn quick_mission(
    State(state): State<Arc<DaemonState>>,
    Json(req): Json<CreateMissionRequest>,
) -> Result<Json<serde_json::Value>, ApiError>;
```

## 6. Request/response DTOs

```rust
#[derive(Deserialize)]
struct CreateMissionRequest {
    objective: String,
    #[serde(default)]           description: Option<String>,
    #[serde(default = "default_priority")]    priority: String,
    #[serde(default = "default_formation")]   formation: String,
    #[serde(default = "default_primary_mos")] primary_mos: String,
    #[serde(default)]           swarm_name: Option<String>,
}
fn default_priority()    -> String { "routine".into() }
fn default_formation()   -> String { "single".into() }
fn default_primary_mos() -> String { "18B".into() }

#[derive(Serialize)]
struct CreateMissionResponse {
    mission_id: String,
    swarm_id: String,
    agent_ids: Vec<String>,
    agent_count: usize,
}

#[derive(Serialize)]
struct ExecuteMissionResponse {
    swarm_id: String,
    tasks_executed: usize,
    success: bool,
}

#[derive(Serialize)]
struct RootResponse {
    service: &'static str,
    version: &'static str,
    motto: &'static str,
}
```

## 7. `ApiError` implementation

```rust
pub struct ApiError { status: StatusCode, message: String }

impl ApiError {
    fn bad_request(msg: impl Into<String>) -> Self { ... }
    fn not_found(msg: impl Into<String>) -> Self { ... }
    fn internal<E: Display>(e: E) -> Self { ... }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(json!({"error": self.message}))).into_response()
    }
}
```

## 8. Binary entry point

```rust
// src/bin/rr_http.rs
use rr::api_routes::build_router;
use rr::daemon::Daemon;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

## 9. `Daemon::state_clone`

```rust
// src/daemon.rs
impl Daemon {
    pub fn state_clone(&self) -> DaemonState { self.state.clone() }
}
```

`DaemonState` is `#[derive(Clone)]` — all its fields are `Arc<_>`, so cloning is cheap.

## 10. Environment variables

| Variable | Purpose | Default |
|---|---|---|
| `RR_HTTP_BIND` | `host:port` to bind | `127.0.0.1:8787` |
| *(inherits all daemon env vars: RR_LLM_CONFIG_PATH, OLLAMA_HOST, provider API keys...)* | | |

## 11. HTTP status code reference

| Status | Triggers |
|---|---|
| `200 OK` | Successful handler return |
| `400 Bad Request` | axum deserialization failure (missing required field, wrong type), `ApiError::bad_request` |
| `404 Not Found` | `ApiError::not_found` — unknown swarm/mission |
| `500 Internal Server Error` | `ApiError::internal` — state_store / orchestrator failure |
| `422 Unprocessable Entity` | axum extractor failure (JSON parse) |

## 12. Integration with the rest of the crate

- Calls `SwarmOrchestrator::new(state.state_store.clone(), state.doctrine.clone(), state.llm.clone(), state.router.clone())` per request (lightweight — all `Arc`s).
- Reads civilians via `state.state_store.list_civilians()` for `/stats`.
- Spawns agents via `factory.lock()` + `spawn_*` in `create_mission`.

## 13. Build + test

```bash
cargo build --release --bin rr-http          # produces target/release/rr-http
cargo check                                   # validates api_routes.rs compiles
```

No dedicated HTTP tests in Slice E.1/E.2 — handlers are thin wrappers over the state store + orchestrator, both of which have unit + integration tests. A future `tests/test_http.rs` could spin up `build_router` against an in-memory `SqliteStateStore` and use `axum::body::to_bytes` + `tower::ServiceExt::oneshot` for handler-level tests.

## 14. File path index

- `src/api_routes.rs::build_router` — router construction
- `src/api_routes.rs::create_mission` — POST /mission/create
- `src/api_routes.rs::execute_swarm` — POST /swarm/{id}/execute
- `src/api_routes.rs::get_swarm_status` — GET /swarm/{id}/status
- `src/api_routes.rs::disband_swarm` — POST /swarm/{id}/disband
- `src/api_routes.rs::list_swarms` — GET /swarms
- `src/api_routes.rs::get_stats` — GET /stats
- `src/api_routes.rs::get_agents_roster` — GET /agents/roster
- `src/api_routes.rs::quick_mission` — POST /mission/quick
- `src/api_routes.rs::ApiError` — structured error type
- `src/bin/rr_http.rs::main` — binary entry point
- `src/daemon.rs::Daemon::state_clone` — cheap state handle
