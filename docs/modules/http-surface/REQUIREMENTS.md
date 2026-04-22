# Requirements — HTTP Surface

**Module:** `src/api_routes.rs` + `src/bin/rr_http.rs`
**Status:** Implemented — 8 endpoints, axum 0.8, standalone `rr-http` binary
**Source:** Port of `rr/api_routes.py` (FastAPI)

## 1. Purpose

Expose Rational Reserve's mission / swarm / agent / civilian operations over HTTP alongside the existing MCP stdio surface. Enables browser-based UIs, external monitoring tools, and non-MCP clients to drive RR without the unix-socket dance.

## 2. Functional requirements

### 2.1 Endpoints

**FR-HTTP-01.** `GET /` SHALL return service identity:
  ```json
  {"service":"rational-reserve", "version":"<cargo-pkg-version>", "motto":"Many minds, one mission."}
  ```

**FR-HTTP-02.** `POST /mission/create` SHALL accept:
  ```json
  {"objective": "...", "description": "...", "priority": "routine|priority|urgent|critical",
   "formation": "single|fire_team|squad|platoon", "primary_mos": "18B|...", "swarm_name": "..."}
  ```
  and return `{mission_id, swarm_id, agent_ids, agent_count}` with both mission and swarm persisted.

**FR-HTTP-03.** `POST /swarm/{swarm_id}/execute` SHALL load the swarm's assigned mission, invoke `SwarmOrchestrator::execute_mission`, and return `{swarm_id, tasks_executed, success}`.

**FR-HTTP-04.** `GET /swarm/{swarm_id}/status` SHALL return the full swarm record, agent list, and agent count.

**FR-HTTP-05.** `POST /swarm/{swarm_id}/disband` SHALL invoke `disband_swarm` and return the `DisbandReport`.

**FR-HTTP-06.** `GET /swarms` SHALL return a list of all swarms with `{id, name, status, agent_count, mission_id}` per entry.

**FR-HTTP-07.** `GET /stats` SHALL return `{event_count, total_swarms, active_swarms, civilians_registered}`.

**FR-HTTP-08.** `GET /agents/roster` SHALL return all persisted agents with full records.

**FR-HTTP-09.** `POST /mission/quick` SHALL create + execute in one call and return combined output.

### 2.2 Binary

**FR-HTTP-10.** `rr-http` binary SHALL:
  - Construct a `Daemon` (reusing daemon configuration + state)
  - Bind `RR_HTTP_BIND` env var or default `127.0.0.1:8787`
  - Serve the axum router over the TCP listener
  - Log `"rr-http listening on http://<addr>"` on startup

**FR-HTTP-11.** The binary SHALL accept graceful shutdown on SIGINT/SIGTERM (deferred — axum::serve handles this by default in the runtime).

### 2.3 Error handling

**FR-HTTP-12.** Every endpoint SHALL return structured JSON errors:
  ```json
  {"error": "<message>"}
  ```
  with appropriate HTTP status codes:
  - `400 Bad Request` — malformed input, bad enum values
  - `404 Not Found` — unknown swarm/mission
  - `500 Internal Server Error` — persistence or LLM failure

**FR-HTTP-13.** All endpoints SHALL validate required fields at deserialization time (serde-driven) and missing fields MUST yield `400`.

## 3. Non-functional requirements

**NFR-HTTP-01. State consistency.** The HTTP surface MUST operate on the same `DaemonState` as `rr-daemon`. A mission created via `rr-http` MUST be visible to `rr-mcp` and vice versa.

**NFR-HTTP-02. No new persistence.** HTTP endpoints MUST delegate to existing `state_store` methods. They MUST NOT define new projection tables.

**NFR-HTTP-03. No new LLM pathways.** The same `DeterministicLLMClient` serves both transports. Invoking the same request via HTTP and MCP MUST yield the same cached response after warmup.

**NFR-HTTP-04. Bind address controllable.** The operator MUST be able to change the bind address via env without rebuilding.

**NFR-HTTP-05. Response sizes bounded.** Endpoints returning lists (e.g. `/agents/roster`) currently return everything. Pagination is deferred but flagged.

## 4. Scope

**In scope:**
- 8 REST endpoints matching the Python FastAPI surface
- Axum 0.8 router + JSON extractors
- Standalone `rr-http` binary
- Env-var-driven bind address
- Structured error responses

**Out of scope:**
- Authentication / authorization (operator is expected to front with a reverse proxy if external exposure is needed)
- CORS (scaffolded in `tower-http` but not enforced; caller's concern)
- TLS / HTTPS termination (reverse proxy concern)
- Pagination (deferred until roster sizes warrant it)
- WebSocket / SSE streaming
- OpenAPI / Swagger schema generation
- Rate limiting
- Request ID propagation / tracing

## 5. Acceptance criteria

1. `cargo build --release --bin rr-http` MUST succeed.
2. `curl http://localhost:8787/` on a running server MUST return service identity JSON with HTTP 200.
3. `curl -XPOST http://localhost:8787/mission/create -d '{"objective":"..."}'` MUST persist both mission and swarm.
4. `curl http://localhost:8787/swarm/<unknown-id>/status` MUST return HTTP 404 with `{"error":"..."}`.
5. `curl http://localhost:8787/stats` MUST return valid JSON with the four documented fields.
6. Setting `RR_HTTP_BIND=0.0.0.0:9000` and launching `rr-http` MUST bind that address.
7. Concurrent requests MUST not corrupt state (SQLite WAL handles this at the persistence layer).
