//! Spawn individual RR agents and standard military formations.
//!
//! The factory is the only place in the core library where agent IDs, names,
//! and chain-of-command wiring are established. It keeps a process-local registry
//! so callers can query "what have I spawned in this process" without touching
//! the state store.
//!
//! Formation sizes follow REQUIREMENTS_PHASE1.md Section 5.2:
//!   - Fire team: 4 agents (1 CPL + 3 SPCs)
//!   - Squad: 7 agents (1 SGT + 2 CPLs + 4 SPCs)
//!   - Platoon: 25 agents (1 LT + 3 squads + 3 HQ staff)

use crate::agent::RRAgent;
use crate::models::*;
use crate::traits::DoctrineLoader;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentFactoryError {
    #[error("{rank:?} is not an officer rank")]
    NotOfficer { rank: RankEnum },

    #[error("{rank:?} is not an NCO rank")]
    NotNco { rank: RankEnum },

    #[error("fire team must have at least 2 members")]
    FireTeamTooSmall,

    #[error("fire team leader must be CPL or SGT")]
    InvalidFireTeamLeader,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AgentFactoryError>;

/// Spawn `RRAgent` instances and standard military formations.
///
/// # Example
///
/// ```
/// use rr::{AgentFactory, RankEnum, MOSEnum};
/// use rr::doctrine::FileSystemDoctrineLoader;
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// let doctrine_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("doctrine");
/// let doctrine = Arc::new(FileSystemDoctrineLoader::new(doctrine_dir));
/// let mut factory = AgentFactory::new(doctrine);
///
/// let general = factory.spawn_general(None, None);
/// assert_eq!(general.rank, RankEnum::GEN);
///
/// let squad = factory.spawn_squad(MOSEnum::Infantry, general.id.clone(), None, None).unwrap();
/// assert_eq!(squad.len(), 7);
/// ```
pub struct AgentFactory {
    doctrine: Arc<dyn DoctrineLoader>,
    /// Optional LLM client injected into every spawned `RRAgent`.
    ///
    /// When present, `RRAgent::execute_task` on any agent produced by this
    /// factory will call the LLM directly. When absent, `execute_task`
    /// returns `LLMNotAvailable` and callers must use
    /// [`crate::swarm_orchestrator::SwarmOrchestrator`] instead.
    llm: Option<Arc<dyn crate::traits::LLMClient>>,
    registry: HashMap<String, RRAgent>,
    counter: AtomicUsize,
    total_spawned: usize,
}

impl AgentFactory {
    pub fn new(doctrine: Arc<dyn DoctrineLoader>) -> Self {
        Self {
            doctrine,
            llm: None,
            registry: HashMap::new(),
            counter: AtomicUsize::new(1),
            total_spawned: 0,
        }
    }

    /// Inject an LLM client that every subsequently-spawned agent will
    /// receive. Existing agents in the registry are NOT back-filled — call
    /// this before any `spawn_*` invocations for whole-swarm LLM coverage.
    pub fn with_llm(mut self, llm: Arc<dyn crate::traits::LLMClient>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Returns true if the factory will inject an LLM into newly-spawned
    /// agents. Primarily for diagnostics / tests.
    pub fn has_llm(&self) -> bool {
        self.llm.is_some()
    }

    // ------------------------------------------------------------------
    // Single-agent spawning
    // ------------------------------------------------------------------

    /// Generic spawner. Prefer the `spawn_<role>` shortcuts when possible.
    pub fn spawn_agent(
        &mut self,
        rank: RankEnum,
        mos: MOSEnum,
        name: Option<String>,
        commander_id: Option<String>,
        unit_id: Option<String>,
        swarm_id: Option<String>,
    ) -> RRAgent {
        let counter_value = self.counter.fetch_add(1, Ordering::SeqCst);
        let resolved_name = name.unwrap_or_else(|| {
            format!("{} {} #{}", rank.full_name(), mos.full_name(), counter_value)
        });

        let agent_id = uuid::Uuid::new_v4().to_string();
        let agent = RRAgent::new(
            resolved_name,
            rank,
            mos,
            self.doctrine.clone(),
            commander_id.clone(),
            unit_id,
            swarm_id,
            Some(agent_id),
            self.llm.clone(),
        );
        let agent_id_clone = agent.id.clone();

        self.registry.insert(agent_id_clone.clone(), agent);
        self.total_spawned += 1;

        if let Some(cmd_id) = commander_id {
            if let Some(commander) = self.registry.get_mut(&cmd_id) {
                commander.add_subordinate(agent_id_clone.clone());
            }
        }

        // Re-borrow after mutation
        self.registry
            .get(&agent_id_clone)
            .cloned()
            .expect("agent was just inserted")
    }

    /// Spawn a commanding General. Default MOS is SF Engineer (18B).
    pub fn spawn_general(&mut self, name: Option<String>, swarm_id: Option<String>) -> RRAgent {
        self.spawn_agent(
            RankEnum::GEN,
            MOSEnum::SfEngineer,
            name,
            None, // the General has no commander
            None,
            swarm_id,
        )
    }

    /// Spawn a COL/MAJ/CPT/LT under a named commander.
    pub fn spawn_officer(
        &mut self,
        rank: RankEnum,
        mos: MOSEnum,
        commander_id: String,
        name: Option<String>,
        swarm_id: Option<String>,
    ) -> Result<RRAgent> {
        if rank.category() != RankCategory::Officer {
            return Err(AgentFactoryError::NotOfficer { rank });
        }
        Ok(self.spawn_agent(rank, mos, name, Some(commander_id), None, swarm_id))
    }

    /// Spawn a SGM/MSG/SGT/CPL under a named commander.
    pub fn spawn_nco(
        &mut self,
        rank: RankEnum,
        mos: MOSEnum,
        commander_id: String,
        name: Option<String>,
        swarm_id: Option<String>,
    ) -> Result<RRAgent> {
        if !matches!(rank, RankEnum::SGM | RankEnum::MSG | RankEnum::SGT | RankEnum::CPL) {
            return Err(AgentFactoryError::NotNco { rank });
        }
        Ok(self.spawn_agent(rank, mos, name, Some(commander_id), None, swarm_id))
    }

    /// Spawn a SPC under a named commander.
    pub fn spawn_specialist(
        &mut self,
        mos: MOSEnum,
        commander_id: String,
        name: Option<String>,
        swarm_id: Option<String>,
    ) -> RRAgent {
        self.spawn_agent(
            RankEnum::SPC,
            mos,
            name,
            Some(commander_id),
            None,
            swarm_id,
        )
    }

    // ------------------------------------------------------------------
    // Formation builders
    // ------------------------------------------------------------------

    /// Spawn a fire team: 1 leader (CPL by default) + (team_size - 1) specialists.
    ///
    /// Returns the members with the leader first. The leader's `commander_id`
    /// is the caller-supplied external commander; the specialists' commander is
    /// the freshly-spawned leader.
    pub fn spawn_fire_team(
        &mut self,
        mos: MOSEnum,
        commander_id: String,
        leader_rank: Option<RankEnum>,
        team_size: usize,
        swarm_id: Option<String>,
    ) -> Result<Vec<RRAgent>> {
        if team_size < 2 {
            return Err(AgentFactoryError::FireTeamTooSmall);
        }
        let leader_rank = leader_rank.unwrap_or(RankEnum::CPL);
        if !matches!(leader_rank, RankEnum::CPL | RankEnum::SGT) {
            return Err(AgentFactoryError::InvalidFireTeamLeader);
        }

        let leader = self.spawn_agent(
            leader_rank,
            mos,
            None,
            Some(commander_id.clone()),
            None,
            swarm_id.clone(),
        );
        let leader_id = leader.id.clone();

        let mut specialists = Vec::new();
        for _ in 0..(team_size - 1) {
            let spc = self.spawn_specialist(
                mos,
                leader_id.clone(),
                None,
                swarm_id.clone(),
            );
            specialists.push(spc);
        }

        // Re-fetch leader (it was cloned on insert)
        let leader = self.registry.get(&leader_id).cloned().unwrap();
        let mut members = vec![leader];
        members.extend(specialists);
        Ok(members)
    }

    /// Spawn a 7-agent squad: 1 SGT + 2 CPLs + 4 SPCs.
    ///
    /// Chain of command: SGT reports to the external commander; both CPLs report
    /// to the SGT; two specialists report to each CPL.
    pub fn spawn_squad(
        &mut self,
        primary_mos: MOSEnum,
        commander_id: String,
        support_mos: Option<MOSEnum>,
        swarm_id: Option<String>,
    ) -> Result<Vec<RRAgent>> {
        let support = support_mos.unwrap_or(primary_mos);

        let sgt = self.spawn_nco(
            RankEnum::SGT,
            primary_mos,
            commander_id.clone(),
            None,
            swarm_id.clone(),
        )?;
        let sgt_id = sgt.id.clone();

        let mut members = vec![sgt];

        // Two fire teams under the SGT, each with a CPL leader + 2 SPCs
        for team_index in 0..2 {
            let cpl_mos = if team_index == 0 {
                primary_mos
            } else {
                support
            };
            let cpl =
                self.spawn_nco(RankEnum::CPL, cpl_mos, sgt_id.clone(), None, swarm_id.clone())?;
            let cpl_id = cpl.id.clone();
            members.push(cpl);

            for _ in 0..2 {
                let spc = self.spawn_specialist(
                    if team_index == 0 {
                        primary_mos
                    } else {
                        support
                    },
                    cpl_id.clone(),
                    None,
                    swarm_id.clone(),
                );
                members.push(spc);
            }
        }

        Ok(members)
    }

    /// Spawn a 25-agent platoon: 1 LT + 3 squads (7 each) + 3 HQ staff.
    ///
    /// The HQ staff (SGM deputy + 2 SPCs as comms/medic) are direct reports to
    /// the LT and provide platoon-level support.
    pub fn spawn_platoon(
        &mut self,
        primary_mos: MOSEnum,
        commander_id: String,
        support_mos_list: Option<Vec<MOSEnum>>,
        swarm_id: Option<String>,
    ) -> Result<Vec<RRAgent>> {
        let lt = self.spawn_officer(
            RankEnum::LT,
            primary_mos,
            commander_id.clone(),
            None,
            swarm_id.clone(),
        )?;
        let lt_id = lt.id.clone();

        let mut members = vec![lt];

        // HQ staff element
        let sgm = self.spawn_nco(
            RankEnum::SGM,
            primary_mos,
            lt_id.clone(),
            None,
            swarm_id.clone(),
        )?;
        members.push(sgm);

        for _ in 0..2 {
            let spc = self.spawn_specialist(
                MOSEnum::Transport, // comms / medic stand-in
                lt_id.clone(),
                None,
                swarm_id.clone(),
            );
            members.push(spc);
        }

        // Three squads
        let support_list = support_mos_list.unwrap_or_else(|| vec![primary_mos]);
        for i in 0..3 {
            let support = support_list[i % support_list.len()];
            let squad = self.spawn_squad(
                primary_mos,
                lt_id.clone(),
                Some(support),
                swarm_id.clone(),
            )?;
            members.extend(squad);
        }

        Ok(members)
    }

    // ------------------------------------------------------------------
    // Registry lookups
    // ------------------------------------------------------------------

    pub fn get(&self, agent_id: &str) -> Option<&RRAgent> {
        self.registry.get(agent_id)
    }

    pub fn all_agents(&self) -> Vec<&RRAgent> {
        self.registry.values().collect()
    }

    pub fn by_rank(&self, rank: RankEnum) -> Vec<&RRAgent> {
        self.registry.values().filter(|a| a.rank == rank).collect()
    }

    pub fn by_mos(&self, mos: MOSEnum) -> Vec<&RRAgent> {
        self.registry.values().filter(|a| a.mos == mos).collect()
    }

    pub fn by_status(&self, status: StatusEnum) -> Vec<&RRAgent> {
        self.registry.values().filter(|a| a.status == status).collect()
    }

    pub fn by_swarm(&self, swarm_id: &str) -> Vec<&RRAgent> {
        self.registry
            .values()
            .filter(|a| a.swarm_id.as_deref() == Some(swarm_id))
            .collect()
    }

    pub fn deactivate(&mut self, agent_id: &str) -> Option<RRAgent> {
        self.registry.remove(agent_id)
    }

    // ------------------------------------------------------------------
    // Stats
    // ------------------------------------------------------------------

    pub fn stats(&self) -> serde_json::Value {
        let mut by_rank: HashMap<String, usize> = HashMap::new();
        let mut by_status: HashMap<String, usize> = HashMap::new();
        for a in self.registry.values() {
            *by_rank.entry(format!("{:?}", a.rank)).or_insert(0) += 1;
            *by_status
                .entry(format!("{:?}", a.status))
                .or_insert(0) += 1;
        }
        serde_json::json!({
            "active": self.registry.len(),
            "total_spawned": self.total_spawned,
            "by_rank": by_rank,
            "by_status": by_status,
        })
    }
}
