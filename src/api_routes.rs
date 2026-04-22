//! HTTP API surface (axum).
//!
//! Ports `rr/api_routes.py` — the FastAPI layer that exposes mission /
//! swarm / agent operations over HTTP. Endpoints delegate to the same
//! [`crate::daemon::DaemonState`] that the MCP stdio shim uses, so both
//! transports operate against identical state.
//!
//! # Endpoints
//!
//! - `GET /`                         — root health
//! - `POST /mission/create`          — create mission + swarm (returns ids)
//! - `POST /swarm/{swarm_id}/execute` — plan + assign + run the mission
//! - `GET  /swarm/{swarm_id}/status` — live projection
//! - `POST /swarm/{swarm_id}/disband` — disband + AAR
//! - `GET  /swarms`                  — list (swarm_id, name, agent_count)
//! - `GET  /stats`                   — event counts, active-swarm count
//! - `GET  /agents/roster`           — list all persisted agents

use crate::daemon::DaemonState;
use crate::mission_planner::MissionPlanner;
use crate::models::*;
use crate::swarm_orchestrator::SwarmOrchestrator;
use crate::traits::StateStore;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Build the axum router. Wrap with `tower_http::trace::TraceLayer` + CORS at
/// the binary level if needed; this function returns the bare router so
/// callers can compose middleware.
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

// ---------------------------------------------------------------------------
// Root
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct RootResponse {
    service: &'static str,
    version: &'static str,
    motto: &'static str,
}

async fn root() -> Json<RootResponse> {
    Json(RootResponse {
        service: "rational-reserve",
        version: env!("CARGO_PKG_VERSION"),
        motto: "Many minds, one mission.",
    })
}

// ---------------------------------------------------------------------------
// POST /mission/create
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateMissionRequest {
    objective: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default = "default_priority")]
    priority: String,
    #[serde(default = "default_formation")]
    formation: String,
    #[serde(default = "default_primary_mos")]
    primary_mos: String,
    #[serde(default)]
    swarm_name: Option<String>,
}

fn default_priority() -> String {
    "routine".to_string()
}
fn default_formation() -> String {
    "single".to_string()
}
fn default_primary_mos() -> String {
    "18B".to_string()
}

#[derive(Serialize)]
struct CreateMissionResponse {
    mission_id: String,
    swarm_id: String,
    agent_ids: Vec<String>,
    agent_count: usize,
}

async fn create_mission(
    State(state): State<Arc<DaemonState>>,
    Json(req): Json<CreateMissionRequest>,
) -> Result<Json<CreateMissionResponse>, ApiError> {
    // Build mission.
    let mut mission = Mission::default();
    mission.objective = req.objective;
    mission.description = req.description;
    mission.priority = parse_priority(&req.priority).map_err(ApiError::bad_request)?;
    state.state_store.save_mission(&mission).map_err(ApiError::internal)?;

    // Deploy the swarm (reuse the daemon's handle_deploy_swarm logic inline).
    let primary_mos = parse_mos(&req.primary_mos).map_err(ApiError::bad_request)?;
    let swarm_name = req
        .swarm_name
        .unwrap_or_else(|| format!("{}-{}", req.formation, req.primary_mos));

    let (general, members, unit_type) = {
        let mut factory = state
            .factory
            .lock()
            .map_err(|e| ApiError::internal(e.to_string()))?;
        let general = factory.spawn_general(Some(format!("{} Commander", swarm_name)), None);
        let general_id = general.id.clone();
        let (members, unit_type) = match req.formation.as_str() {
            "single" => {
                let spec =
                    factory.spawn_specialist(primary_mos, general_id.clone(), None, None);
                (vec![general.clone(), spec], UnitType::FireTeam)
            }
            "fire_team" => (
                factory
                    .spawn_fire_team(primary_mos, general_id.clone(), None, 4, None)
                    .map_err(|e| ApiError::bad_request(e.to_string()))?,
                UnitType::FireTeam,
            ),
            "squad" => (
                factory
                    .spawn_squad(primary_mos, general_id.clone(), None, None)
                    .map_err(|e| ApiError::bad_request(e.to_string()))?,
                UnitType::Squad,
            ),
            "platoon" => (
                factory
                    .spawn_platoon(primary_mos, general_id.clone(), None, None)
                    .map_err(|e| ApiError::bad_request(e.to_string()))?,
                UnitType::Platoon,
            ),
            other => {
                return Err(ApiError::bad_request(format!(
                    "unknown formation: {}",
                    other
                )));
            }
        };
        (general, members, unit_type)
    };

    let swarm_id = uuid::Uuid::new_v4().to_string();
    let agent_ids: Vec<_> = members.iter().map(|a| a.id.clone()).collect();
    let agent_count = members.len();

    for a in &members {
        let record = a.to_record();
        state
            .state_store
            .save_agent(&record)
            .map_err(ApiError::internal)?;
    }

    let swarm = Swarm {
        id: swarm_id.clone(),
        name: swarm_name.clone(),
        mission_id: Some(mission.id.clone()),
        general_id: general.id,
        agents: agent_ids.clone(),
        units: Vec::new(),
        status: StatusEnum::Standby,
        created_at: chrono::Utc::now(),
        activated_at: None,
        disbanded_at: None,
        metadata: {
            let mut m = std::collections::HashMap::new();
            m.insert(
                "unit_type".to_string(),
                serde_json::json!(format!("{:?}", unit_type)),
            );
            m
        },
    };
    state.state_store.save_swarm(&swarm).map_err(ApiError::internal)?;

    Ok(Json(CreateMissionResponse {
        mission_id: mission.id,
        swarm_id,
        agent_ids,
        agent_count,
    }))
}

// ---------------------------------------------------------------------------
// POST /swarm/{swarm_id}/execute
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ExecuteMissionResponse {
    swarm_id: String,
    tasks_executed: usize,
    success: bool,
}

async fn execute_swarm(
    Path(swarm_id): Path<String>,
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<ExecuteMissionResponse>, ApiError> {
    let swarm = state
        .state_store
        .load_swarm(&swarm_id)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("swarm `{}` not found", swarm_id)))?;
    let mission_id = swarm
        .mission_id
        .clone()
        .ok_or_else(|| ApiError::bad_request("swarm has no assigned mission"))?;
    let mission = state
        .state_store
        .load_mission(&mission_id)
        .map_err(ApiError::internal)?
        .unwrap_or_else(|| {
            let mut m = Mission::default();
            m.id = mission_id.clone();
            m.objective = "(mission not persisted)".into();
            m
        });

    let orchestrator = SwarmOrchestrator::new(
        state.state_store.clone(),
        state.doctrine.clone(),
        state.llm.clone(),
        state.router.clone(),
    );
    let executions = orchestrator
        .execute_mission(&swarm_id, mission)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let success = executions.iter().all(|t| t.status == StatusEnum::Complete);

    Ok(Json(ExecuteMissionResponse {
        swarm_id,
        tasks_executed: executions.len(),
        success,
    }))
}

// ---------------------------------------------------------------------------
// GET /swarm/{swarm_id}/status
// ---------------------------------------------------------------------------

async fn get_swarm_status(
    Path(swarm_id): Path<String>,
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let swarm = state
        .state_store
        .load_swarm(&swarm_id)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("swarm `{}` not found", swarm_id)))?;
    let agents = state
        .state_store
        .list_agents(Some(&swarm_id), None, None, None)
        .map_err(ApiError::internal)?;
    Ok(Json(serde_json::json!({
        "swarm": swarm,
        "agents": agents,
        "agent_count": agents.len(),
    })))
}

// ---------------------------------------------------------------------------
// POST /swarm/{swarm_id}/disband
// ---------------------------------------------------------------------------

async fn disband_swarm(
    Path(swarm_id): Path<String>,
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let orchestrator = SwarmOrchestrator::new(
        state.state_store.clone(),
        state.doctrine.clone(),
        state.llm.clone(),
        state.router.clone(),
    );
    let report = orchestrator
        .disband_swarm(&swarm_id)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(serde_json::to_value(&report).unwrap_or_default()))
}

// ---------------------------------------------------------------------------
// GET /swarms
// ---------------------------------------------------------------------------

async fn list_swarms(
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let swarms = state.state_store.list_swarms(None).map_err(ApiError::internal)?;
    let rows: Vec<_> = swarms
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "status": s.status,
                "agent_count": s.agents.len(),
                "mission_id": s.mission_id,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({"count": rows.len(), "swarms": rows})))
}

// ---------------------------------------------------------------------------
// GET /stats
// ---------------------------------------------------------------------------

async fn get_stats(
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let event_count = state.state_store.event_count().map_err(ApiError::internal)?;
    let swarms = state.state_store.list_swarms(None).map_err(ApiError::internal)?;
    let active = swarms
        .iter()
        .filter(|s| !matches!(s.status, StatusEnum::Complete))
        .count();
    let civilians = state
        .state_store
        .list_civilians()
        .map_err(ApiError::internal)?;
    Ok(Json(serde_json::json!({
        "event_count": event_count,
        "total_swarms": swarms.len(),
        "active_swarms": active,
        "civilians_registered": civilians.len(),
    })))
}

// ---------------------------------------------------------------------------
// GET /agents/roster
// ---------------------------------------------------------------------------

async fn get_agents_roster(
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let agents = state
        .state_store
        .list_agents(None, None, None, None)
        .map_err(ApiError::internal)?;
    Ok(Json(serde_json::json!({
        "count": agents.len(),
        "agents": agents,
    })))
}

// ---------------------------------------------------------------------------
// POST /mission/quick — create + execute in one shot
// ---------------------------------------------------------------------------

async fn quick_mission(
    State(state): State<Arc<DaemonState>>,
    Json(req): Json<CreateMissionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let created = create_mission(State(state.clone()), Json(req))
        .await?
        .0;
    let exec = execute_swarm(Path(created.swarm_id.clone()), State(state))
        .await?
        .0;
    Ok(Json(serde_json::json!({
        "mission_id": created.mission_id,
        "swarm_id": created.swarm_id,
        "agent_count": created.agent_count,
        "tasks_executed": exec.tasks_executed,
        "success": exec.success,
    })))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_priority(s: &str) -> Result<PriorityEnum, String> {
    Ok(match s {
        "routine" => PriorityEnum::Routine,
        "priority" => PriorityEnum::Priority,
        "urgent" => PriorityEnum::Urgent,
        "critical" => PriorityEnum::Critical,
        other => return Err(format!("unknown priority: {}", other)),
    })
}

fn parse_mos(code: &str) -> Result<MOSEnum, String> {
    Ok(match code {
        "35F" => MOSEnum::IntelAnalyst,
        "35L" => MOSEnum::CounterIntel,
        "35N" => MOSEnum::SigintAnalyst,
        "11B" => MOSEnum::Infantry,
        "12B" => MOSEnum::CombatEngineer,
        "13B" => MOSEnum::Artillery,
        "19D" => MOSEnum::CavalryScout,
        "25B" => MOSEnum::ItSpecialist,
        "42A" => MOSEnum::HrSpecialist,
        "88M" => MOSEnum::Transport,
        "92G" => MOSEnum::Logistics,
        "18B" => MOSEnum::SfEngineer,
        "75R" => MOSEnum::Ranger,
        "160" => MOSEnum::Soar,
        other => return Err(format!("unknown MOS: {}", other)),
    })
}

/// Axum error wrapper. Maps category -> HTTP status.
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }
    fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }
    fn internal<E: std::fmt::Display>(e: E) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: e.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(serde_json::json!({"error": self.message})),
        )
            .into_response()
    }
}

/// Convenience: build a MissionPlanner bound to the current daemon state.
/// Not used by routes directly (SwarmOrchestrator constructs its own) but
/// exported for callers that want standalone planning.
pub fn build_mission_planner(state: &DaemonState) -> MissionPlanner {
    MissionPlanner::new(state.llm.clone())
}
