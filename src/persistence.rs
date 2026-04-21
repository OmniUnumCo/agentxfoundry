//! SQLite persistence — append-only event log + current-state projection.
//!
//! Uses `rusqlite` with WAL mode. Schema matches the Python `schema.sql`.

use crate::models::*;
use crate::traits::StateStore;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Get doctrine version from a provider closure.
pub type DoctrineVersionProvider = Box<dyn Fn() -> String + Send + Sync>;

/// SQLite-backed state store.
///
/// Uses WAL mode for concurrent reads. Schema includes an append-only event log
/// and current-state projection tables for agents, swarms, and missions.
///
/// # Example
///
/// ```
/// use rr::persistence::SqliteStateStore;
/// use rr::StateStore;
///
/// let dir = tempfile::tempdir().unwrap();
/// let db_path = dir.path().join("test.db");
/// let store = SqliteStateStore::open(&db_path, Box::new(|| "0.0.0".to_string())).unwrap();
///
/// let id = store.append_event("PING", &serde_json::json!({}), None, None).unwrap();
/// assert_eq!(id, 1);
/// assert_eq!(store.event_count().unwrap(), 1);
/// ```
pub struct SqliteStateStore {
    conn: Mutex<Connection>,
    doctrine_version: DoctrineVersionProvider,
}

impl SqliteStateStore {
    /// Open or create a SQLite database at the given path.
    pub fn open(
        db_path: &Path,
        doctrine_version: DoctrineVersionProvider,
    ) -> Result<Self, BoxError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;

        // WAL mode for concurrent reads
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;",
        )?;

        let store = Self {
            conn: Mutex::new(conn),
            doctrine_version,
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), BoxError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS events (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type  TEXT NOT NULL,
                payload     TEXT NOT NULL,
                swarm_id    TEXT,
                agent_id    TEXT,
                doctrine_version TEXT NOT NULL,
                created_at  TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS agents (
                id                  TEXT PRIMARY KEY,
                name                TEXT NOT NULL,
                rank                TEXT NOT NULL,
                mos                 TEXT NOT NULL,
                commander_id        TEXT,
                subordinates        TEXT NOT NULL DEFAULT '[]',
                unit_id             TEXT,
                swarm_id            TEXT,
                status              TEXT NOT NULL DEFAULT 'standby',
                current_mission     TEXT,
                current_task        TEXT,
                missions_completed  INTEGER NOT NULL DEFAULT 0,
                performance_score   REAL NOT NULL DEFAULT 0.0,
                created_at          TEXT NOT NULL,
                metadata            TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS swarms (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                mission_id      TEXT,
                general_id      TEXT NOT NULL,
                agents          TEXT NOT NULL DEFAULT '[]',
                units           TEXT NOT NULL DEFAULT '[]',
                status          TEXT NOT NULL DEFAULT 'standby',
                created_at      TEXT NOT NULL,
                activated_at    TEXT,
                disbanded_at    TEXT,
                metadata        TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS missions (
                id              TEXT PRIMARY KEY,
                objective       TEXT NOT NULL,
                description     TEXT,
                priority        TEXT NOT NULL DEFAULT 'routine',
                status          TEXT NOT NULL DEFAULT 'standby',
                constraints     TEXT NOT NULL DEFAULT '{}',
                resources       TEXT NOT NULL DEFAULT '{}',
                assigned_swarm  TEXT,
                created_at      TEXT NOT NULL,
                started_at      TEXT,
                completed_at    TEXT,
                result          TEXT,
                metadata        TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS civilians (
                id                  TEXT PRIMARY KEY,
                name                TEXT NOT NULL,
                tier                TEXT NOT NULL,
                firm_id             TEXT,
                specialists         TEXT NOT NULL DEFAULT '[]',
                specialty_slug      TEXT NOT NULL,
                project_path        TEXT,
                upstream_url        TEXT,
                status              TEXT NOT NULL,
                active_contracts    TEXT NOT NULL DEFAULT '[]',
                performance_score   REAL NOT NULL DEFAULT 0.0,
                created_at          TEXT NOT NULL,
                metadata            TEXT NOT NULL DEFAULT '{}'
            );

            CREATE INDEX IF NOT EXISTS civilians_by_firm
                ON civilians(firm_id);
            CREATE INDEX IF NOT EXISTS civilians_by_slug
                ON civilians(specialty_slug);

            CREATE TABLE IF NOT EXISTS contracts (
                id                      TEXT PRIMARY KEY,
                contracting_agent       TEXT NOT NULL,
                firm_id                 TEXT NOT NULL,
                routed_specialist_id    TEXT,
                state                   TEXT NOT NULL,
                brief                   TEXT NOT NULL,
                mission_id              TEXT,
                priority                TEXT NOT NULL,
                created_at              TEXT NOT NULL,
                closed_at               TEXT,
                outcome                 TEXT,
                metadata                TEXT NOT NULL DEFAULT '{}'
            );

            CREATE INDEX IF NOT EXISTS contracts_by_firm
                ON contracts(firm_id);
            CREATE INDEX IF NOT EXISTS contracts_by_state
                ON contracts(state);
            CREATE INDEX IF NOT EXISTS contracts_by_contracting_agent
                ON contracts(contracting_agent);
            ",
        )?;
        Ok(())
    }

    pub fn close(&self) -> Result<(), BoxError> {
        let conn = self.conn.lock().unwrap();
        // rusqlite Connection is dropped and closes automatically
        drop(conn);
        Ok(())
    }

    pub fn event_count(&self) -> Result<usize, BoxError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?;
        Ok(count as usize)
    }

    // -- Helpers for serialization --

    #[allow(dead_code)]
    fn serialize_json<T: serde::Serialize>(&self, value: &T) -> Result<String, BoxError> {
        Ok(serde_json::to_string(value)?)
    }

    fn deserialize_json<T: serde::de::DeserializeOwned>(&self, text: &str) -> Result<T, BoxError> {
        Ok(serde_json::from_str(text)?)
    }
}

impl StateStore for SqliteStateStore {
    fn append_event(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
        swarm_id: Option<&str>,
        agent_id: Option<&str>,
    ) -> Result<i64, BoxError> {
        let conn = self.conn.lock().unwrap();
        let doctrine_ver = (self.doctrine_version)();
        let payload_text = serde_json::to_string(payload)?;

        conn.execute(
            "INSERT INTO events (event_type, payload, swarm_id, agent_id, doctrine_version)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_type, payload_text, swarm_id, agent_id, doctrine_ver],
        )?;

        let id: i64 = conn.last_insert_rowid();
        Ok(id)
    }

    fn event_count(&self) -> Result<usize, BoxError> {
        SqliteStateStore::event_count(self)
    }

    fn load_agent(&self, agent_id: &str) -> Result<Option<AgentRecord>, BoxError> {
        let conn = self.conn.lock().unwrap();
        let row: Option<(
            String, String, String, String, Option<String>, String, Option<String>,
            Option<String>, String, Option<String>, Option<String>, i64, f64, String, String,
        )> = conn
            .query_row(
                "SELECT id, name, rank, mos, commander_id, subordinates, unit_id, swarm_id,
                        status, current_mission, current_task, missions_completed,
                        performance_score, created_at, metadata
                 FROM agents WHERE id = ?1",
                params![agent_id],
                |r| {
                    Ok((
                        r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
                        r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?,
                        r.get(12)?, r.get(13)?, r.get(14)?,
                    ))
                },
            )
            .optional()?;

        let Some((
            id, name, rank_str, mos_str, commander_id, subordinates_text, unit_id, swarm_id,
            status_str, current_mission, current_task, missions_completed,
            performance_score, created_at, metadata_text,
        )) = row
        else {
            return Ok(None);
        };

        Ok(Some(AgentRecord {
            id,
            name,
            rank: self.deserialize_json(&rank_str)?,
            mos: self.deserialize_json(&mos_str)?,
            commander_id,
            subordinates: self.deserialize_json(&subordinates_text)?,
            unit_id,
            swarm_id,
            status: self.deserialize_json(&status_str)?,
            current_mission,
            current_task,
            missions_completed: missions_completed as u32,
            performance_score,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .or_else(|_| chrono::DateTime::parse_from_str(&created_at, "%Y-%m-%d %H:%M:%S"))
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            metadata: if metadata_text.is_empty() {
                Default::default()
            } else {
                self.deserialize_json(&metadata_text)?
            },
        }))
    }

    fn load_swarm(&self, swarm_id: &str) -> Result<Option<Swarm>, BoxError> {
        let conn = self.conn.lock().unwrap();
        let row: Option<(
            String, String, Option<String>, String, String, String,
            String, String, Option<String>, Option<String>, String,
        )> = conn
            .query_row(
                "SELECT id, name, mission_id, general_id, agents, units,
                        status, created_at, activated_at, disbanded_at, metadata
                 FROM swarms WHERE id = ?1",
                params![swarm_id],
                |r| {
                    Ok((
                        r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
                        r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?,
                    ))
                },
            )
            .optional()?;

        let Some((
            id, name, mission_id, general_id, agents_text, units_text,
            status_str, created_at, activated_at, disbanded_at, metadata_text,
        )) = row
        else {
            return Ok(None);
        };

        Ok(Some(Swarm {
            id,
            name,
            mission_id,
            general_id,
            agents: self.deserialize_json(&agents_text)?,
            units: self.deserialize_json(&units_text)?,
            status: self.deserialize_json(&status_str)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            activated_at: activated_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            disbanded_at: disbanded_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            metadata: if metadata_text.is_empty() {
                Default::default()
            } else {
                self.deserialize_json(&metadata_text)?
            },
        }))
    }

    fn load_mission(&self, _mission_id: &str) -> Result<Option<Mission>, BoxError> {
        // Stub — full implementation follows same pattern as load_agent
        Ok(None)
    }

    fn list_agents(
        &self,
        swarm_id: Option<&str>,
        status: Option<StatusEnum>,
        rank: Option<RankEnum>,
        mos: Option<MOSEnum>,
    ) -> Result<Vec<AgentRecord>, BoxError> {
        let mut sql = "SELECT id FROM agents WHERE 1=1".to_string();
        let mut args: Vec<String> = Vec::new();

        if let Some(sid) = swarm_id {
            sql.push_str(" AND swarm_id = ?");
            args.push(sid.to_string());
        }
        if let Some(st) = status {
            sql.push_str(" AND status = ?");
            args.push(serde_json::to_string(&st)?);
        }
        if let Some(r) = rank {
            sql.push_str(" AND rank = ?");
            args.push(serde_json::to_string(&r)?);
        }
        if let Some(m) = mos {
            sql.push_str(" AND mos = ?");
            args.push(serde_json::to_string(&m)?);
        }

        let ids: Vec<String> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query_map(
                rusqlite::params_from_iter(args.iter().map(|s| s.as_str())),
                |r| r.get::<_, String>(0),
            )?;
            let mut collected = Vec::new();
            while let Some(Ok(id)) = rows.next() {
                collected.push(id);
            }
            collected
        };

        // Load each agent
        let mut agents = Vec::new();
        for id in ids {
            if let Some(agent) = SqliteStateStore::load_agent(self, &id)? {
                agents.push(agent);
            }
        }
        Ok(agents)
    }

    fn list_swarms(&self, _status: Option<StatusEnum>) -> Result<Vec<Swarm>, BoxError> {
        let ids: Vec<String> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id FROM swarms")?;
            let mut rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let mut collected = Vec::new();
            while let Some(Ok(id)) = rows.next() {
                collected.push(id);
            }
            collected
        };

        let mut swarms = Vec::new();
        for id in ids {
            if let Some(swarm) = SqliteStateStore::load_swarm(self, &id)? {
                swarms.push(swarm);
            }
        }
        Ok(swarms)
    }

    fn save_agent(&self, agent: &AgentRecord) -> Result<(), BoxError> {
        let conn = self.conn.lock().unwrap();
        let subordinates_text = serde_json::to_string(&agent.subordinates)?;
        let metadata_text = serde_json::to_string(&agent.metadata)?;

        conn.execute(
            "INSERT OR REPLACE INTO agents
             (id, name, rank, mos, commander_id, subordinates, unit_id, swarm_id,
              status, current_mission, current_task, missions_completed,
              performance_score, created_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                agent.id,
                agent.name,
                serde_json::to_string(&agent.rank)?,
                serde_json::to_string(&agent.mos)?,
                agent.commander_id,
                subordinates_text,
                agent.unit_id,
                agent.swarm_id,
                serde_json::to_string(&agent.status)?,
                agent.current_mission,
                agent.current_task,
                agent.missions_completed as i64,
                agent.performance_score,
                agent.created_at.to_rfc3339(),
                metadata_text,
            ],
        )?;
        Ok(())
    }

    fn save_swarm(&self, swarm: &Swarm) -> Result<(), BoxError> {
        let conn = self.conn.lock().unwrap();
        let agents_text = serde_json::to_string(&swarm.agents)?;
        let units_text = serde_json::to_string(&swarm.units)?;
        let metadata_text = serde_json::to_string(&swarm.metadata)?;

        conn.execute(
            "INSERT OR REPLACE INTO swarms
             (id, name, mission_id, general_id, agents, units, status,
              created_at, activated_at, disbanded_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                swarm.id,
                swarm.name,
                swarm.mission_id,
                swarm.general_id,
                agents_text,
                units_text,
                serde_json::to_string(&swarm.status)?,
                swarm.created_at.to_rfc3339(),
                swarm.activated_at.map(|dt| dt.to_rfc3339()),
                swarm.disbanded_at.map(|dt| dt.to_rfc3339()),
                metadata_text,
            ],
        )?;
        Ok(())
    }

    fn save_mission(&self, _mission: &Mission) -> Result<(), BoxError> {
        // Stub — same pattern as save_agent
        Ok(())
    }

    // ------------------------------------------------------------------
    // Civilian Reserve — civilians table
    // ------------------------------------------------------------------

    fn save_civilian(&self, civilian: &CivilianRecord) -> Result<(), BoxError> {
        let conn = self.conn.lock().unwrap();
        let specialists_text = serde_json::to_string(&civilian.specialists)?;
        let active_contracts_text = serde_json::to_string(&civilian.active_contracts)?;
        let metadata_text = serde_json::to_string(&civilian.metadata)?;

        conn.execute(
            "INSERT OR REPLACE INTO civilians
             (id, name, tier, firm_id, specialists, specialty_slug, project_path,
              upstream_url, status, active_contracts, performance_score, created_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                civilian.id,
                civilian.name,
                serde_json::to_string(&civilian.tier)?,
                civilian.firm_id,
                specialists_text,
                civilian.specialty_slug,
                civilian.project_path,
                civilian.upstream_url,
                serde_json::to_string(&civilian.status)?,
                active_contracts_text,
                civilian.performance_score,
                civilian.created_at.to_rfc3339(),
                metadata_text,
            ],
        )?;
        Ok(())
    }

    fn load_civilian(&self, civilian_id: &str) -> Result<Option<CivilianRecord>, BoxError> {
        let conn = self.conn.lock().unwrap();
        let row: Option<(
            String, String, String, Option<String>, String, String,
            Option<String>, Option<String>, String, String, f64, String, String,
        )> = conn
            .query_row(
                "SELECT id, name, tier, firm_id, specialists, specialty_slug,
                        project_path, upstream_url, status, active_contracts,
                        performance_score, created_at, metadata
                 FROM civilians WHERE id = ?1",
                params![civilian_id],
                |r| {
                    Ok((
                        r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
                        r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?, r.get(12)?,
                    ))
                },
            )
            .optional()?;

        let Some((
            id, name, tier_str, firm_id, specialists_text, specialty_slug,
            project_path, upstream_url, status_str, active_contracts_text,
            performance_score, created_at, metadata_text,
        )) = row
        else {
            return Ok(None);
        };

        Ok(Some(CivilianRecord {
            id,
            name,
            tier: self.deserialize_json(&tier_str)?,
            firm_id,
            specialists: self.deserialize_json(&specialists_text)?,
            specialty_slug,
            project_path,
            upstream_url,
            status: self.deserialize_json(&status_str)?,
            active_contracts: self.deserialize_json(&active_contracts_text)?,
            performance_score,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            metadata: if metadata_text.is_empty() {
                Default::default()
            } else {
                self.deserialize_json(&metadata_text)?
            },
        }))
    }

    fn list_civilians(&self) -> Result<Vec<CivilianRecord>, BoxError> {
        let ids: Vec<String> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id FROM civilians")?;
            let mut rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let mut collected = Vec::new();
            while let Some(Ok(id)) = rows.next() {
                collected.push(id);
            }
            collected
        };

        let mut civs = Vec::new();
        for id in ids {
            if let Some(c) = <Self as StateStore>::load_civilian(self, &id)? {
                civs.push(c);
            }
        }
        Ok(civs)
    }

    // ------------------------------------------------------------------
    // Civilian Reserve — contracts table
    // ------------------------------------------------------------------

    fn save_contract(&self, contract: &ContractRecord) -> Result<(), BoxError> {
        let conn = self.conn.lock().unwrap();
        let metadata_text = serde_json::to_string(&contract.metadata)?;
        let outcome_text = match &contract.outcome {
            Some(o) => Some(serde_json::to_string(o)?),
            None => None,
        };

        conn.execute(
            "INSERT OR REPLACE INTO contracts
             (id, contracting_agent, firm_id, routed_specialist_id, state, brief,
              mission_id, priority, created_at, closed_at, outcome, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                contract.id,
                contract.contracting_agent,
                contract.firm_id,
                contract.routed_specialist_id,
                serde_json::to_string(&contract.state)?,
                contract.brief,
                contract.mission_id,
                serde_json::to_string(&contract.priority)?,
                contract.created_at.to_rfc3339(),
                contract.closed_at.map(|dt| dt.to_rfc3339()),
                outcome_text,
                metadata_text,
            ],
        )?;
        Ok(())
    }

    fn load_contract(&self, contract_id: &str) -> Result<Option<ContractRecord>, BoxError> {
        let conn = self.conn.lock().unwrap();
        let row: Option<(
            String, String, String, Option<String>, String, String,
            Option<String>, String, String, Option<String>, Option<String>, String,
        )> = conn
            .query_row(
                "SELECT id, contracting_agent, firm_id, routed_specialist_id, state, brief,
                        mission_id, priority, created_at, closed_at, outcome, metadata
                 FROM contracts WHERE id = ?1",
                params![contract_id],
                |r| {
                    Ok((
                        r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
                        r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?,
                    ))
                },
            )
            .optional()?;

        let Some((
            id, contracting_agent, firm_id, routed_specialist_id, state_str, brief,
            mission_id, priority_str, created_at, closed_at, outcome_str, metadata_text,
        )) = row
        else {
            return Ok(None);
        };

        let outcome = match outcome_str {
            Some(s) => Some(self.deserialize_json(&s)?),
            None => None,
        };

        Ok(Some(ContractRecord {
            id,
            contracting_agent,
            firm_id,
            routed_specialist_id,
            state: self.deserialize_json(&state_str)?,
            brief,
            mission_id,
            priority: self.deserialize_json(&priority_str)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            closed_at: closed_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            outcome,
            metadata: if metadata_text.is_empty() {
                Default::default()
            } else {
                self.deserialize_json(&metadata_text)?
            },
        }))
    }

    fn list_contracts_by_firm(
        &self,
        firm_id: &str,
        state: Option<ContractState>,
    ) -> Result<Vec<ContractRecord>, BoxError> {
        let mut sql = "SELECT id FROM contracts WHERE firm_id = ?1".to_string();
        let mut args: Vec<String> = vec![firm_id.to_string()];
        if let Some(st) = state {
            sql.push_str(" AND state = ?2");
            args.push(serde_json::to_string(&st)?);
        }

        let ids: Vec<String> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query_map(
                rusqlite::params_from_iter(args.iter().map(|s| s.as_str())),
                |r| r.get::<_, String>(0),
            )?;
            let mut collected = Vec::new();
            while let Some(Ok(id)) = rows.next() {
                collected.push(id);
            }
            collected
        };

        let mut contracts = Vec::new();
        for id in ids {
            if let Some(c) = <Self as StateStore>::load_contract(self, &id)? {
                contracts.push(c);
            }
        }
        Ok(contracts)
    }

    fn close(&self) -> Result<(), BoxError> {
        SqliteStateStore::close(self)
    }
}
