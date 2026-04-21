//! Singleton Rational Reserve daemon.
//!
//! Owns the SQLiteStateStore, AgentFactory, DoctrineLoader, and C2Router.
//! Binds a unix socket and dispatches line-delimited JSON-RPC messages.
//!
//! Five host agents each spawn their own `rr-mcp` stdio subprocess. The MCP
//! shims call `ensure_daemon` to guarantee a single background daemon exists,
//! then connect to it over the unix socket. All state mutations flow through
//! the daemon — it is the one and only writer.

use crate::agent::RRAgent;
use crate::config::{load_config, Config, Paths};
use crate::doctrine::{default_doctrine_loader, FileSystemDoctrineLoader};
use crate::factory::AgentFactory;
use crate::logging::JsonlLogger;
use crate::models::*;
use crate::persistence::SqliteStateStore;
use crate::traits::{EventLogger, DoctrineLoader, StateStore};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("state error: {0}")]
    State(String),

    #[error("unknown method: {0}")]
    MethodNotFound(String),

    #[error("handler error: {0}")]
    Handler(String),
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Shared daemon state accessible across tasks.
#[derive(Clone)]
pub struct DaemonState {
    pub factory: Arc<Mutex<AgentFactory>>,
    pub doctrine: Arc<FileSystemDoctrineLoader>,
    pub state_store: Arc<SqliteStateStore>,
    pub logger: Arc<JsonlLogger>,
}

impl DaemonState {
    pub fn new(
        factory: AgentFactory,
        doctrine: FileSystemDoctrineLoader,
        state_store: SqliteStateStore,
        logger: JsonlLogger,
    ) -> Self {
        Self {
            factory: Arc::new(Mutex::new(factory)),
            doctrine: Arc::new(doctrine),
            state_store: Arc::new(state_store),
            logger: Arc::new(logger),
        }
    }
}

// ============================================================================
// Daemon
// ============================================================================

pub struct Daemon {
    #[allow(dead_code)]
    config: Config,
    paths: Paths,
    state: DaemonState,
    stop: Arc<std::sync::atomic::AtomicBool>,
}

impl Daemon {
    pub fn new() -> Result<Self, BoxError> {
        let config = load_config(None);
        let paths = config.paths.clone();
        paths.ensure()?;

        // Doctrine
        let doctrine = default_doctrine_loader().unwrap_or_else(|_| {
            // If doctrine isn't installed, create a minimal loader with no-op behavior
            FileSystemDoctrineLoader::new(
                dirs::home_dir()
                    .unwrap_or_default()
                    .join(".rational-reserve")
                    .join("doctrine"),
            )
        });

        // State store
        let doctrine_clone = Arc::new(doctrine.clone());
        let state_store = SqliteStateStore::open(
            &paths.db_path(),
            Box::new(move || doctrine_clone.version().to_string()),
        )?;

        // Factory
        let factory = AgentFactory::new(Arc::new(doctrine.clone()));

        // Logger
        let logger = JsonlLogger::new(paths.log_file())?;

        let state = DaemonState::new(factory, doctrine, state_store, logger);

        Ok(Self {
            config,
            paths,
            state,
            stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    pub async fn serve_forever(&mut self) -> Result<(), BoxError> {
        let sock_path = self.paths.socket_path();
        if sock_path.exists() {
            let _ = std::fs::remove_file(&sock_path);
        }

        // Ensure parent directory exists
        if let Some(parent) = sock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&sock_path)?;

        self.write_pid_file()?;
        self.state.logger.log(&HashMap::from([
            ("event".to_string(), json!("daemon_started")),
            ("socket".to_string(), json!(sock_path.display().to_string())),
        ]));

        loop {
            if self.stop.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            // Use a timeout to periodically check the stop flag
            match tokio::time::timeout(
                std::time::Duration::from_millis(500),
                listener.accept(),
            )
            .await
            {
                Ok(Ok((stream, _addr))) => {
                    let state = self.state.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, state).await {
                            eprintln!("connection error: {}", e);
                        }
                    });
                }
                Ok(Err(e)) => {
                    eprintln!("accept error: {}", e);
                }
                Err(_) => {
                    // Timeout — check stop flag
                    continue;
                }
            }
        }

        self.shutdown()?;
        Ok(())
    }

    fn write_pid_file(&self) -> std::io::Result<()> {
        let pid = std::process::id();
        std::fs::write(self.paths.pid_file(), format!("{}\n", pid))
    }

    fn remove_pid_file(&self) {
        let _ = std::fs::remove_file(self.paths.pid_file());
    }

    fn shutdown(&self) -> Result<(), BoxError> {
        self.state.logger.log(&HashMap::from([(
            "event".to_string(),
            json!("daemon_shutdown"),
        )]));
        self.state.state_store.close()?;
        let _ = std::fs::remove_file(self.paths.socket_path());
        self.remove_pid_file();
        Ok(())
    }

    pub fn stop(&self) {
        self.stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

// ============================================================================
// Connection handler
// ============================================================================

async fn handle_connection(stream: UnixStream, state: DaemonState) -> Result<(), BoxError> {
    let (read_half, mut write_half) = tokio::io::split(stream);
    let reader = BufReader::new(read_half);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let response = json!({
                    "id": null,
                    "error": {"code": "ParseError", "message": e.to_string()},
                });
                let _ = write_half
                    .write_all(format!("{}\n", response).as_bytes())
                    .await;
                continue;
            }
        };

        let response = dispatch(&request, &state).await;
        let _ = write_half
            .write_all(format!("{}\n", response).as_bytes())
            .await;
    }

    Ok(())
}

async fn dispatch(request: &serde_json::Value, state: &DaemonState) -> serde_json::Value {
    let req_id = request.get("id").cloned();
    let method = request.get("method").and_then(|v| v.as_str());
    let params = request.get("params").cloned().unwrap_or(serde_json::json!({}));

    let Some(method) = method else {
        return json!({
            "id": req_id,
            "error": {"code": "MethodNotFound", "message": "no method specified"},
        });
    };

    let result = match method {
        "ping" => handle_ping(state, &params),
        "rr_roster" => handle_roster(state, &params),
        "rr_deploy_swarm" => handle_deploy_swarm(state, &params),
        "rr_status" => handle_status(state, &params),
        "cr_register_civilian" => handle_cr_register_civilian(state, &params),
        "cr_list_civilians" => handle_cr_list_civilians(state, &params),
        "cr_firm_detail" => handle_cr_firm_detail(state, &params),
        "cr_contract_request" => handle_cr_contract_request(state, &params),
        "cr_contract_accept" => handle_cr_contract_accept(state, &params),
        "cr_contract_sitrep" => handle_cr_contract_sitrep(state, &params),
        "cr_contract_closeout" => handle_cr_contract_closeout(state, &params),
        _ => Err(DaemonError::MethodNotFound(method.to_string())),
    };

    match result {
        Ok(val) => json!({"id": req_id, "result": val}),
        Err(e) => json!({
            "id": req_id,
            "error": {"code": format!("{:?}", e), "message": e.to_string()},
        }),
    }
}

// ============================================================================
// Handlers
// ============================================================================

fn handle_ping(state: &DaemonState, _params: &serde_json::Value) -> Result<serde_json::Value, DaemonError> {
    let event_count = state
        .state_store
        .event_count()
        .map_err(|e| DaemonError::State(e.to_string()))?;
    Ok(json!({
        "pong": true,
        "pid": std::process::id(),
        "doctrine_version": state.doctrine.version(),
        "event_count": event_count,
    }))
}

fn handle_roster(state: &DaemonState, params: &serde_json::Value) -> Result<serde_json::Value, DaemonError> {
    let swarm_id = params.get("swarm_id").and_then(|v| v.as_str());
    let status = params.get("status").and_then(|v| v.as_str()).map(|s| {
        // Parse from snake_case string to StatusEnum
        match s {
            "standby" => StatusEnum::Standby,
            "active" => StatusEnum::Active,
            "engaged" => StatusEnum::Engaged,
            "complete" => StatusEnum::Complete,
            "failed" => StatusEnum::Failed,
            _ => StatusEnum::Standby,
        }
    });
    let rank = params.get("rank").and_then(|v| v.as_str()).map(|s| {
        // Simplified — in production use proper serde deserialization
        match s {
            "GEN" => RankEnum::GEN,
            "COL" => RankEnum::COL,
            "MAJ" => RankEnum::MAJ,
            "CPT" => RankEnum::CPT,
            "LT" => RankEnum::LT,
            "SGM" => RankEnum::SGM,
            "MSG" => RankEnum::MSG,
            "SGT" => RankEnum::SGT,
            "CPL" => RankEnum::CPL,
            "SPC" => RankEnum::SPC,
            "PFC" => RankEnum::PFC,
            "PVT" => RankEnum::PVT,
            _ => RankEnum::PVT,
        }
    });
    let mos = params.get("mos").and_then(|v| v.as_str()).map(|s| {
        match s {
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
            _ => MOSEnum::Infantry,
        }
    });

    let records = state
        .state_store
        .list_agents(swarm_id, status, rank, mos)
        .map_err(|e| DaemonError::State(e.to_string()))?;

    let agents: Vec<_> = records
        .iter()
        .map(|r| serde_json::to_value(r).unwrap_or_default())
        .collect();

    Ok(json!({
        "count": agents.len(),
        "agents": agents,
    }))
}

fn handle_deploy_swarm(state: &DaemonState, params: &serde_json::Value) -> Result<serde_json::Value, DaemonError> {
    let formation = params
        .get("formation")
        .and_then(|v| v.as_str())
        .unwrap_or("single")
        .to_lowercase();

    let primary_mos_code = params
        .get("primary_mos")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("primary_mos is required".to_string()))?;

    let primary_mos = match primary_mos_code {
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
        other => return Err(DaemonError::Handler(format!("unknown MOS: {}", other))),
    };

    let swarm_name = params
        .get("swarm_name")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| format!("{}-{}", formation, primary_mos_code));

    let mut factory = state.factory.lock().map_err(|e| DaemonError::State(e.to_string()))?;

    // Every swarm gets a General
    let general = factory.spawn_general(Some(format!("{} Commander", swarm_name)), None);
    let general_id = general.id.clone();

    let members: Vec<RRAgent>;
    let unit_type: UnitType;

    match formation.as_str() {
        "single" => {
            let specialist = factory.spawn_specialist(primary_mos, general_id.clone(), None, None);
            members = vec![general, specialist];
            unit_type = UnitType::FireTeam;
        }
        "fire_team" => {
            members = factory
                .spawn_fire_team(primary_mos, general_id.clone(), None, 4, None)
                .map_err(|e| DaemonError::Handler(e.to_string()))?;
            unit_type = UnitType::FireTeam;
        }
        "squad" => {
            let support = params
                .get("support_mos")
                .and_then(|v| v.as_str())
                .and_then(|s| match s {
                    "35F" => Some(MOSEnum::IntelAnalyst),
                    "11B" => Some(MOSEnum::Infantry),
                    "12B" => Some(MOSEnum::CombatEngineer),
                    _ => None,
                });
            members = factory
                .spawn_squad(primary_mos, general_id.clone(), support, None)
                .map_err(|e| DaemonError::Handler(e.to_string()))?;
            unit_type = UnitType::Squad;
        }
        "platoon" => {
            let support_list = params
                .get("support_mos")
                .and_then(|v| v.as_str())
                .and_then(|s| match s {
                    "12B" => Some(vec![MOSEnum::CombatEngineer]),
                    _ => None,
                });
            members = factory
                .spawn_platoon(primary_mos, general_id.clone(), support_list, None)
                .map_err(|e| DaemonError::Handler(e.to_string()))?;
            unit_type = UnitType::Platoon;
        }
        other => {
            return Err(DaemonError::Handler(format!(
                "unknown formation: {}",
                other
            )));
        }
    }

    let swarm_id = uuid::Uuid::new_v4().to_string();

    // Wire each agent to the swarm_id and persist
    let agent_ids: Vec<_> = members.iter().map(|a| a.id.clone()).collect();
    let agent_count = members.len();

    // Drop the factory lock before persisting
    drop(factory);

    // Persist agents
    for agent in &members {
        let record = agent.to_record();
        state
            .state_store
            .save_agent(&record)
            .map_err(|e| DaemonError::State(e.to_string()))?;

        state
            .state_store
            .append_event(
                "AGENT_SPAWNED",
                &json!({
                    "agent_id": agent.id,
                    "name": agent.name,
                    "rank": agent.rank.full_name(),
                    "mos": agent.mos.full_name(),
                    "commander_id": agent.commander_id,
                }),
                Some(&swarm_id),
                Some(&agent.id),
            )
            .map_err(|e| DaemonError::State(e.to_string()))?;
    }

    // Persist swarm
    let swarm = Swarm {
        id: swarm_id.clone(),
        name: swarm_name.clone(),
        mission_id: None,
        general_id: general_id.clone(),
        agents: agent_ids.clone(),
        units: Vec::new(),
        status: StatusEnum::Standby,
        created_at: chrono::Utc::now(),
        activated_at: None,
        disbanded_at: None,
        metadata: HashMap::new(),
    };

    state
        .state_store
        .save_swarm(&swarm)
        .map_err(|e| DaemonError::State(e.to_string()))?;

    state
        .state_store
        .append_event(
            "SWARM_DEPLOYED",
            &json!({
                "swarm_id": swarm_id,
                "name": swarm_name,
                "formation": formation,
                "primary_mos": primary_mos_code,
                "agent_count": agent_count,
                "unit_type": format!("{:?}", unit_type),
            }),
            Some(&swarm_id),
            None,
        )
        .map_err(|e| DaemonError::State(e.to_string()))?;

    Ok(json!({
        "swarm_id": swarm_id,
        "name": swarm_name,
        "formation": formation,
        "agent_count": agent_count,
        "general_id": general_id,
        "agent_ids": agent_ids,
    }))
}

fn handle_status(state: &DaemonState, params: &serde_json::Value) -> Result<serde_json::Value, DaemonError> {
    let swarm_id = params
        .get("swarm_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("swarm_id is required".to_string()))?;

    let swarm = state
        .state_store
        .load_swarm(swarm_id)
        .map_err(|e| DaemonError::State(e.to_string()))?
        .ok_or_else(|| DaemonError::Handler(format!("unknown swarm: {}", swarm_id)))?;

    let agents = state
        .state_store
        .list_agents(Some(swarm_id), None, None, None)
        .map_err(|e| DaemonError::State(e.to_string()))?;

    let agent_values: Vec<_> = agents
        .iter()
        .map(|a| serde_json::to_value(a).unwrap_or_default())
        .collect();

    Ok(json!({
        "swarm": serde_json::to_value(&swarm).unwrap_or_default(),
        "agents": agent_values,
        "agent_count": agents.len(),
    }))
}

// ============================================================================
// Civilian Reserve handlers
// ============================================================================

fn handle_cr_register_civilian(
    state: &DaemonState,
    params: &serde_json::Value,
) -> Result<serde_json::Value, DaemonError> {
    let slug = params
        .get("slug")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("slug is required".to_string()))?;

    // Load doctrine — `slug` is passed through the doctrine loader's
    // `validate_slug` guard there, so no extra sanitization needed here.
    let doctrine = state
        .doctrine
        .civilian_doctrine(slug)
        .map_err(|e| DaemonError::Handler(e.to_string()))?;

    let civilian = CivilianRecord {
        id: doctrine.slug.clone(),
        name: format!("CR-{}", doctrine.slug),
        tier: doctrine.kind,
        firm_id: doctrine.firm.clone(),
        specialists: doctrine.specialists.clone(),
        specialty_slug: doctrine.slug.clone(),
        project_path: doctrine.project_path.clone(),
        upstream_url: doctrine.upstream_url.clone(),
        status: StatusEnum::Standby,
        active_contracts: Vec::new(),
        performance_score: 0.0,
        created_at: chrono::Utc::now(),
        metadata: HashMap::new(),
    };

    state
        .state_store
        .save_civilian(&civilian)
        .map_err(|e| DaemonError::State(e.to_string()))?;

    let event_type = if civilian.is_firm() {
        "FIRM_REGISTERED"
    } else {
        "CIVILIAN_REGISTERED"
    };
    state
        .state_store
        .append_event(
            event_type,
            &json!({
                "civilian_id": civilian.id,
                "tier": civilian.tier,
                "firm_id": civilian.firm_id,
                "specialty_slug": civilian.specialty_slug,
            }),
            None,
            None,
        )
        .map_err(|e| DaemonError::State(e.to_string()))?;

    Ok(json!({
        "civilian": serde_json::to_value(&civilian).unwrap_or_default(),
    }))
}

fn handle_cr_list_civilians(
    state: &DaemonState,
    _params: &serde_json::Value,
) -> Result<serde_json::Value, DaemonError> {
    let records = state
        .state_store
        .list_civilians()
        .map_err(|e| DaemonError::State(e.to_string()))?;
    let civs: Vec<_> = records
        .iter()
        .map(|r| serde_json::to_value(r).unwrap_or_default())
        .collect();
    Ok(json!({
        "count": civs.len(),
        "civilians": civs,
    }))
}

fn handle_cr_firm_detail(
    state: &DaemonState,
    params: &serde_json::Value,
) -> Result<serde_json::Value, DaemonError> {
    let firm_id = params
        .get("firm_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("firm_id is required".to_string()))?;

    let firm_record = state
        .state_store
        .load_civilian(firm_id)
        .map_err(|e| DaemonError::State(e.to_string()))?
        .ok_or_else(|| DaemonError::Handler(format!("firm `{}` not found", firm_id)))?;

    if !firm_record.is_firm() && !firm_record.is_independent() {
        return Err(DaemonError::Handler(format!(
            "`{}` is a specialist; cr_firm_detail is only valid for firms and independents",
            firm_id
        )));
    }

    // Load doctrine for routing-rules inspection. Falls back gracefully if
    // the firm doctrine has drifted from the stored record.
    let doctrine = state
        .doctrine
        .civilian_doctrine(firm_id)
        .map_err(|e| DaemonError::Handler(e.to_string()))?;

    let open = state
        .state_store
        .list_contracts_by_firm(firm_id, None)
        .map_err(|e| DaemonError::State(e.to_string()))?;
    let open_contracts: Vec<_> = open
        .iter()
        .filter(|c| !matches!(c.state, ContractState::Closed | ContractState::Rejected))
        .map(|c| serde_json::to_value(c).unwrap_or_default())
        .collect();

    let routing_rules = crate::broker::ContractBroker::rules(&doctrine);

    Ok(json!({
        "firm": serde_json::to_value(&firm_record).unwrap_or_default(),
        "specialists": doctrine.specialists,
        "contract_scope": doctrine.contract_scope,
        "routing_rules": routing_rules
            .iter()
            .map(|(s, kws)| json!({"specialist": s, "keywords": kws}))
            .collect::<Vec<_>>(),
        "open_contracts": open_contracts,
    }))
}

fn handle_cr_contract_accept(
    state: &DaemonState,
    params: &serde_json::Value,
) -> Result<serde_json::Value, DaemonError> {
    let contract_id = params
        .get("contract_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("contract_id is required".to_string()))?;
    let specialist_id = params
        .get("specialist_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("specialist_id is required".to_string()))?;

    let mut contract = state
        .state_store
        .load_contract(contract_id)
        .map_err(|e| DaemonError::State(e.to_string()))?
        .ok_or_else(|| DaemonError::Handler(format!("contract `{}` not found", contract_id)))?;

    if !matches!(contract.state, ContractState::Requested) {
        return Err(DaemonError::Handler(format!(
            "contract `{}` is in state {:?}; only Requested contracts can be accepted",
            contract.id, contract.state
        )));
    }

    // Validate that the specialist is part of the firm according to doctrine.
    let firm_doctrine = state
        .doctrine
        .civilian_doctrine(&contract.firm_id)
        .map_err(|e| DaemonError::Handler(e.to_string()))?;

    // Independents do not have specialists -- they accept work directly.
    // Firms require that the specialist appear in their `specialists:` list.
    let accepted_specialist = if firm_doctrine.kind == CivilianTier::Firm {
        if !firm_doctrine.specialists.iter().any(|s| s == specialist_id) {
            return Err(DaemonError::Handler(format!(
                "specialist `{}` is not a member of firm `{}` (members: {:?})",
                specialist_id, contract.firm_id, firm_doctrine.specialists
            )));
        }
        specialist_id.to_string()
    } else {
        // Independent civilian -- specialist_id is conventionally the
        // civilian's own slug; accept it for uniformity.
        specialist_id.to_string()
    };

    contract.state = ContractState::Accepted;
    contract.routed_specialist_id = Some(accepted_specialist.clone());

    state
        .state_store
        .save_contract(&contract)
        .map_err(|e| DaemonError::State(e.to_string()))?;

    state
        .state_store
        .append_event(
            "CONTRACT_ACCEPTED",
            &json!({
                "contract_id": contract.id,
                "firm_id": contract.firm_id,
                "routed_specialist_id": accepted_specialist,
            }),
            None,
            Some(&contract.contracting_agent),
        )
        .map_err(|e| DaemonError::State(e.to_string()))?;

    Ok(json!({
        "contract": serde_json::to_value(&contract).unwrap_or_default(),
    }))
}

fn handle_cr_contract_request(
    state: &DaemonState,
    params: &serde_json::Value,
) -> Result<serde_json::Value, DaemonError> {
    let contracting_agent = params
        .get("contracting_agent")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("contracting_agent is required".to_string()))?;
    let firm_id = params
        .get("firm_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("firm_id is required".to_string()))?;
    let brief = params
        .get("brief")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("brief is required".to_string()))?;
    let mission_id = params
        .get("mission_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    let priority_str = params
        .get("priority")
        .and_then(|v| v.as_str())
        .unwrap_or("routine");
    let priority = match priority_str {
        "routine" => PriorityEnum::Routine,
        "priority" => PriorityEnum::Priority,
        "urgent" => PriorityEnum::Urgent,
        "critical" => PriorityEnum::Critical,
        other => {
            return Err(DaemonError::Handler(format!(
                "unknown priority: {}",
                other
            )))
        }
    };

    // Validate contracting agent is a military agent we know about.
    let agent = state
        .state_store
        .load_agent(contracting_agent)
        .map_err(|e| DaemonError::State(e.to_string()))?
        .ok_or_else(|| {
            DaemonError::Handler(format!(
                "contracting agent `{}` not found (must be a military agent)",
                contracting_agent
            ))
        })?;

    // Validate firm exists and is not a specialist (must be addressed at firm level).
    let firm = state
        .state_store
        .load_civilian(firm_id)
        .map_err(|e| DaemonError::State(e.to_string()))?
        .ok_or_else(|| DaemonError::Handler(format!("firm `{}` not found", firm_id)))?;
    if firm.is_specialist() {
        return Err(DaemonError::Handler(format!(
            "`{}` is a specialist; address contracts to its firm `{}`",
            firm_id,
            firm.firm_id.unwrap_or_else(|| "<unknown>".into())
        )));
    }

    let contract = ContractRecord {
        id: uuid::Uuid::new_v4().to_string(),
        contracting_agent: agent.id.clone(),
        firm_id: firm.id.clone(),
        routed_specialist_id: None,
        state: ContractState::Requested,
        brief: brief.to_string(),
        mission_id,
        priority,
        created_at: chrono::Utc::now(),
        closed_at: None,
        outcome: None,
        metadata: HashMap::new(),
    };

    state
        .state_store
        .save_contract(&contract)
        .map_err(|e| DaemonError::State(e.to_string()))?;

    state
        .state_store
        .append_event(
            "CONTRACT_REQUESTED",
            &json!({
                "contract_id": contract.id,
                "from_agent": contract.contracting_agent,
                "firm_id": contract.firm_id,
                "brief": contract.brief,
                "mission_id": contract.mission_id,
                "priority": contract.priority,
            }),
            None,
            Some(&contract.contracting_agent),
        )
        .map_err(|e| DaemonError::State(e.to_string()))?;

    Ok(json!({
        "contract": serde_json::to_value(&contract).unwrap_or_default(),
    }))
}

fn handle_cr_contract_sitrep(
    state: &DaemonState,
    params: &serde_json::Value,
) -> Result<serde_json::Value, DaemonError> {
    let contract_id = params
        .get("contract_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("contract_id is required".to_string()))?;
    let status_str = params.get("status").and_then(|v| v.as_str()).unwrap_or("active");
    let progress = params.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let message = params.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let blockers: Vec<String> = params
        .get("blockers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let mut contract = state
        .state_store
        .load_contract(contract_id)
        .map_err(|e| DaemonError::State(e.to_string()))?
        .ok_or_else(|| DaemonError::Handler(format!("contract `{}` not found", contract_id)))?;

    // Implicit state transition: first SITREP on an Accepted contract moves it
    // to InProgress. Requested contracts auto-accept on first SITREP too
    // (covers the minimal-tools path where firms don't explicitly accept).
    if matches!(
        contract.state,
        ContractState::Accepted | ContractState::Requested
    ) {
        contract.state = ContractState::InProgress;
        state
            .state_store
            .save_contract(&contract)
            .map_err(|e| DaemonError::State(e.to_string()))?;
    }

    state
        .state_store
        .append_event(
            "CONTRACT_SITREP",
            &json!({
                "contract_id": contract.id,
                "firm_id": contract.firm_id,
                "status": status_str,
                "progress": progress,
                "message": message,
                "blockers": blockers,
            }),
            None,
            Some(&contract.contracting_agent),
        )
        .map_err(|e| DaemonError::State(e.to_string()))?;

    Ok(json!({
        "contract": serde_json::to_value(&contract).unwrap_or_default(),
    }))
}

fn handle_cr_contract_closeout(
    state: &DaemonState,
    params: &serde_json::Value,
) -> Result<serde_json::Value, DaemonError> {
    let contract_id = params
        .get("contract_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DaemonError::Handler("contract_id is required".to_string()))?;
    let outcome_str = params
        .get("outcome")
        .and_then(|v| v.as_str())
        .unwrap_or("complete");
    let outcome = match outcome_str {
        "complete" => ContractOutcome::Complete,
        "failed" => ContractOutcome::Failed,
        "cancelled" => ContractOutcome::Cancelled,
        other => {
            return Err(DaemonError::Handler(format!(
                "unknown outcome: {}",
                other
            )))
        }
    };
    let deliverables = params.get("deliverables").cloned().unwrap_or(json!(null));

    let mut contract = state
        .state_store
        .load_contract(contract_id)
        .map_err(|e| DaemonError::State(e.to_string()))?
        .ok_or_else(|| DaemonError::Handler(format!("contract `{}` not found", contract_id)))?;

    contract.state = ContractState::Closed;
    contract.outcome = Some(outcome);
    contract.closed_at = Some(chrono::Utc::now());

    state
        .state_store
        .save_contract(&contract)
        .map_err(|e| DaemonError::State(e.to_string()))?;

    state
        .state_store
        .append_event(
            "CONTRACT_CLOSEOUT",
            &json!({
                "contract_id": contract.id,
                "firm_id": contract.firm_id,
                "outcome": outcome,
                "deliverables": deliverables,
            }),
            None,
            Some(&contract.contracting_agent),
        )
        .map_err(|e| DaemonError::State(e.to_string()))?;

    Ok(json!({
        "contract": serde_json::to_value(&contract).unwrap_or_default(),
    }))
}

// ============================================================================
// Shim-facing helpers
// ============================================================================

/// Return True if a daemon appears to be running and responsive.
pub fn is_daemon_alive(paths: &Paths) -> bool {
    let pid_file = paths.pid_file();
    if !pid_file.is_file() {
        return false;
    }

    let pid_str = match std::fs::read_to_string(&pid_file) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let pid = match pid_str.trim().parse::<u32>() {
        Ok(p) => p,
        Err(_) => return false,
    };

    // Check if process is alive
    let alive = unsafe { libc::kill(pid as i32, 0) == 0 };
    if !alive {
        return false;
    }

    paths.socket_path().exists()
}

/// Resolve the path to the rr-daemon binary relative to the current executable.
///
/// If the current executable is already "rr-daemon", return it unchanged.
/// Otherwise, return a sibling binary named "rr-daemon" in the same directory.
pub fn resolve_daemon_binary(current_exe: &std::path::Path) -> std::path::PathBuf {
    let filename = current_exe
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if filename == "rr-daemon" {
        return current_exe.to_path_buf();
    }
    match current_exe.parent() {
        Some(dir) => dir.join("rr-daemon"),
        None => std::path::PathBuf::from("rr-daemon"),
    }
}

/// Guarantee a daemon is running, spawning one if necessary.
pub async fn ensure_daemon(paths: Option<&Paths>, startup_timeout_secs: f64) -> Result<Paths, BoxError> {
    let config = load_config(None);
    let effective = paths.unwrap_or(&config.paths);
    effective.ensure()?;

    if is_daemon_alive(effective) {
        return Ok(effective.clone());
    }

    // Reap stale PID file
    let _ = std::fs::remove_file(effective.pid_file());

    // Launch a fresh daemon in a detached child process
    let exe = std::env::current_exe()?;
    let daemon_bin = resolve_daemon_binary(&exe);

    if !daemon_bin.exists() {
        return Err(format!(
            "rr-daemon binary not found at {}; ensure both binaries are installed together",
            daemon_bin.display()
        ).into());
    }

    let mut cmd = tokio::process::Command::new(&daemon_bin);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // Inherit environment
    cmd.envs(std::env::vars());

    #[cfg(unix)]
    {
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    cmd.spawn()?;

    // Wait for the socket to appear
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs_f64(startup_timeout_secs);

    while std::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        if is_daemon_alive(effective) {
            return Ok(effective.clone());
        }
    }

    Err(format!(
        "rr-daemon failed to start within {:.1}s",
        startup_timeout_secs
    )
    .into())
}
