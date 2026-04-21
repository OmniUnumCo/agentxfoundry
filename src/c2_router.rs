//! Command & Control router — chain-of-command validation and message routing.
//!
//! The router is the *only* place in the core that walks the live chain of
//! command. Models hold pure rank-authority data; agent holds behavior;
//! c2_router applies the hybrid validation rule from REQUIREMENTS_PHASE1.md
//! Section 7.2:
//!
//! 1. Direct subordinates are always valid targets for orders.
//! 2. Higher-ranking agents may override and command non-direct subordinates,
//!    but every override is recorded as an audit event.
//! 3. All routed messages flow through an injected `StateStore` so the daemon's
//!    event log is the single source of truth.

use crate::agent::RRAgent;
use crate::models::*;
use crate::traits::StateStore;
use std::collections::HashSet;
use thiserror::Error;

// ============================================================================
// Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum C2RouterError {
    #[error("chain-of-command cycle detected: {cycle:?}")]
    ChainCycle { cycle: Vec<String> },

    #[error("{0} has no authority to order {1}")]
    CommandAuthority(String, String),

    #[error("unknown agent: {0}")]
    AgentNotFound(String),

    #[error("unknown civilian firm: {0}")]
    FirmNotFound(String),

    #[error("{0} is a specialist; contracts must address the firm `{1}`")]
    SpecialistDirectlyContracted(String, String),

    #[error("contracting agent `{0}` is not a military agent")]
    NonMilitaryContractingAgent(String),

    #[error("state store is required for {0}")]
    StateStoreRequired(&'static str),
}

pub type Result<T> = std::result::Result<T, C2RouterError>;

// ============================================================================
// C2Router
// ============================================================================

type AgentLookup = Box<dyn Fn(&str) -> Option<RRAgent> + Send + Sync>;
type CivilianLookup = Box<dyn Fn(&str) -> Option<CivilianRecord> + Send + Sync>;

/// Route orders, SITREPs, and CASREPs through the chain of command.
///
/// # Example
///
/// ```
/// use rr::{C2Router, RRAgent, RankEnum, MOSEnum, Order, OrderType, StatusEnum};
/// use rr::doctrine::FileSystemDoctrineLoader;
/// use std::collections::HashMap;
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// let doctrine_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("doctrine");
/// let doctrine = Arc::new(FileSystemDoctrineLoader::new(doctrine_dir));
///
/// let mut gen = RRAgent::new("General".into(), RankEnum::GEN, MOSEnum::SfEngineer,
///     doctrine.clone(), None, None, None, Some("gen-1".into()), None);
/// let spc = RRAgent::new("Soldier".into(), RankEnum::SPC, MOSEnum::Infantry,
///     doctrine, Some("gen-1".into()), None, None, Some("spc-1".into()), None);
/// gen.add_subordinate(spc.id.clone());
///
/// let registry: HashMap<String, RRAgent> = [
///     (gen.id.clone(), gen), (spc.id.clone(), spc),
/// ].into_iter().collect();
/// let mut router = C2Router::new(
///     move |id: &str| registry.get(id).cloned(),
///     None,
/// );
///
/// let order = Order::new("gen-1", "spc-1", OrderType::DirectTask, "Patrol north");
/// assert!(router.route_order(order).is_ok());
/// ```
pub struct C2Router {
    lookup: AgentLookup,
    civilian_lookup: Option<CivilianLookup>,
    state_store: Option<Box<dyn StateStore>>,
    order_log: Vec<Order>,
    sitrep_log: Vec<SitRep>,
    casrep_log: Vec<CasRep>,
    override_log: Vec<serde_json::Value>,
    contract_log: Vec<ContractRecord>,
}

impl C2Router {
    pub fn new(
        lookup: impl Fn(&str) -> Option<RRAgent> + Send + Sync + 'static,
        state_store: Option<Box<dyn StateStore>>,
    ) -> Self {
        Self {
            lookup: Box::new(lookup),
            civilian_lookup: None,
            state_store,
            order_log: Vec::new(),
            sitrep_log: Vec::new(),
            casrep_log: Vec::new(),
            override_log: Vec::new(),
            contract_log: Vec::new(),
        }
    }

    /// Attach a civilian lookup for contract routing and civilian refusal.
    ///
    /// Without one, `validate_command_relationship` cannot detect civilian
    /// targets and `route_contract_request` will always fail with
    /// [`C2RouterError::FirmNotFound`].
    pub fn with_civilian_lookup(
        mut self,
        civilian_lookup: impl Fn(&str) -> Option<CivilianRecord> + Send + Sync + 'static,
    ) -> Self {
        self.civilian_lookup = Some(Box::new(civilian_lookup));
        self
    }

    fn lookup_civilian(&self, id: &str) -> Option<CivilianRecord> {
        self.civilian_lookup.as_ref().and_then(|f| f(id))
    }

    // ------------------------------------------------------------------
    // Chain-of-command validation
    // ------------------------------------------------------------------

    /// Return the list of ancestors from `agent_id` toward the General.
    ///
    /// The returned list starts at `agent_id` itself and ends at the topmost
    /// commander (whose `commander_id` is None). Raises `ChainCycleError`
    /// if the walk loops.
    pub fn walk_chain_upward(&self, agent_id: &str, max_depth: usize) -> Result<Vec<String>> {
        let mut visited = Vec::new();
        let mut seen = HashSet::new();
        let mut current: Option<String> = Some(agent_id.to_string());
        let mut depth = 0;

        while let Some(cur) = current {
            if seen.contains(&cur) {
                visited.push(cur.clone());
                return Err(C2RouterError::ChainCycle {
                    cycle: visited,
                });
            }
            if depth >= max_depth {
                visited.push(cur.clone());
                return Err(C2RouterError::ChainCycle {
                    cycle: visited,
                });
            }
            seen.insert(cur.clone());
            visited.push(cur.clone());

            let agent = (self.lookup)(&cur);
            current = agent.and_then(|a| a.commander_id);
            depth += 1;
        }

        Ok(visited)
    }

    /// Return True if `ancestor_id` appears in the upward chain of `descendant_id`.
    pub fn is_ancestor(&self, ancestor_id: &str, descendant_id: &str) -> bool {
        if ancestor_id == descendant_id {
            return false;
        }
        match self.walk_chain_upward(descendant_id, 64) {
            Ok(chain) => chain.iter().any(|id| id == ancestor_id),
            Err(_) => false,
        }
    }

    /// Apply the hybrid rule from Section 7.2.
    ///
    /// Returns a tuple `(is_valid, is_override)`:
    ///   - `(true, false)` — direct subordinate relationship (normal case)
    ///   - `(true, true)` — not a direct subordinate, but superior outranks;
    ///     the caller is expected to log an override audit entry
    ///   - `(false, false)` — neither direct nor rank-authorized; order refused
    pub fn validate_command_relationship(
        &self,
        superior_id: &str,
        subordinate_id: &str,
    ) -> (bool, bool) {
        // Civilians are outside the chain of command. A military agent must
        // contract civilians via `route_contract_request`, not order them —
        // so any attempt to command a civilian is refused here.
        if self.lookup_civilian(subordinate_id).is_some() {
            return (false, false);
        }

        let superior = match (self.lookup)(superior_id) {
            Some(a) => a,
            None => return (false, false),
        };
        let subordinate = match (self.lookup)(subordinate_id) {
            Some(a) => a,
            None => return (false, false),
        };

        // Case 1: direct subordinate (the common case)
        if subordinate.subordinates.iter().any(|_| false) // check superior's subordinates
            || superior.subordinates.iter().any(|id| id == subordinate_id)
        {
            return (true, false);
        }

        // Case 2: ancestor in the chain — also "direct" in the transitive sense
        if self.is_ancestor(superior_id, subordinate_id) {
            return (true, false);
        }

        // Case 3: rank override — allowed, but flagged for audit
        if superior.rank.outranks(&subordinate.rank) {
            return (true, true);
        }

        (false, false)
    }

    /// Safety check before wiring a new agent into the chain.
    ///
    /// Raises `ChainCycleError` if setting `new_agent.commander_id = prospective_commander_id`
    /// would introduce a cycle.
    pub fn check_no_cycle_on_spawn(
        &self,
        new_agent_id: &str,
        prospective_commander_id: &str,
    ) -> Result<()> {
        if new_agent_id == prospective_commander_id {
            return Err(C2RouterError::ChainCycle {
                cycle: vec![new_agent_id.to_string(), prospective_commander_id.to_string()],
            });
        }
        let chain = self.walk_chain_upward(prospective_commander_id, 64)?;
        if chain.iter().any(|id| id == new_agent_id) {
            let idx = chain.iter().position(|id| id == new_agent_id).unwrap();
            let mut cycle = vec![new_agent_id.to_string()];
            cycle.extend(chain[..=idx].iter().cloned());
            return Err(C2RouterError::ChainCycle { cycle });
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Order routing
    // ------------------------------------------------------------------

    /// Validate and deliver an order from superior to subordinate.
    ///
    /// Returns True on successful delivery. Appends an event to the state
    /// store if one is configured.
    pub fn route_order(&mut self, mut order: Order) -> Result<bool> {
        let (is_valid, is_override) = self.validate_command_relationship(
            &order.from_agent,
            &order.to_agent,
        );

        if !is_valid {
            return Err(C2RouterError::CommandAuthority(
                order.from_agent.clone(),
                order.to_agent.clone(),
            ));
        }

        let _subordinate = match (self.lookup)(&order.to_agent) {
            Some(a) => a,
            None => return Err(C2RouterError::AgentNotFound(order.to_agent.clone())),
        };

        // We need a mutable agent to receive the order, but our lookup returns
        // owned RRAgent. In the real daemon, the lookup would return &mut or
        // the agent registry would be an Arc<Mutex<HashMap>>. For v0.1 the
        // order acknowledgement is recorded in the event log instead.
        order.acknowledged_at = Some(chrono::Utc::now());
        self.order_log.push(order.clone());

        if is_override {
            self.override_log.push(serde_json::json!({
                "order_id": order.id,
                "from_agent": order.from_agent,
                "to_agent": order.to_agent,
            }));
        }

        if let Some(ref store) = self.state_store {
            let _ = store.append_event(
                "ORDER_ROUTED",
                &serde_json::json!({
                    "order_id": order.id,
                    "order_type": format!("{:?}", order.order_type),
                    "from_agent": order.from_agent,
                    "to_agent": order.to_agent,
                    "task_id": order.task_id,
                    "mission_id": order.mission_id,
                    "override": is_override,
                }),
                None,
                Some(&order.to_agent),
            );
        }

        Ok(true)
    }

    /// Send an order to every direct subordinate of `from_agent_id`.
    pub fn broadcast_order(
        &mut self,
        from_agent_id: &str,
        content: &str,
        order_type: Option<OrderType>,
    ) -> Result<Vec<Order>> {
        let superior = match (self.lookup)(from_agent_id) {
            Some(a) => a,
            None => return Err(C2RouterError::AgentNotFound(from_agent_id.to_string())),
        };

        let order_type = order_type.unwrap_or(OrderType::Broadcast);
        let mut delivered = Vec::new();

        for sub_id in &superior.subordinates {
            let order = Order::new(from_agent_id, sub_id, order_type, content);
            match self.route_order(order.clone()) {
                Ok(true) => delivered.push(order),
                Ok(false) => {}
                Err(C2RouterError::CommandAuthority(_, _)) => continue,
                Err(e) => return Err(e),
            }
        }

        Ok(delivered)
    }

    // ------------------------------------------------------------------
    // SITREP routing
    // ------------------------------------------------------------------

    /// Route a SITREP upward. Auto-resolves recipient to the reporter's commander.
    pub fn route_sitrep(&mut self, mut sitrep: SitRep) -> Result<bool> {
        if sitrep.to_agent.is_none() {
            if let Some(reporter) = (self.lookup)(&sitrep.from_agent) {
                sitrep.to_agent = reporter.commander_id;
            }
        }

        self.sitrep_log.push(sitrep.clone());

        if let Some(ref store) = self.state_store {
            let _ = store.append_event(
                "SITREP_ROUTED",
                &serde_json::json!({
                    "sitrep_id": sitrep.id,
                    "from_agent": sitrep.from_agent,
                    "to_agent": sitrep.to_agent,
                    "status": format!("{:?}", sitrep.status),
                    "progress": sitrep.progress,
                    "message": sitrep.message,
                    "blockers": sitrep.blockers,
                }),
                None,
                Some(&sitrep.from_agent),
            );
        }

        Ok(true)
    }

    // ------------------------------------------------------------------
    // CASREP routing (escalation)
    // ------------------------------------------------------------------

    /// Route a CASREP — escalate to the failed agent's commander.
    pub fn route_casrep(&mut self, casrep: CasRep) -> Result<bool> {
        self.casrep_log.push(casrep.clone());

        let failed = (self.lookup)(&casrep.agent_id);
        let escalate_to = failed.and_then(|a| a.commander_id);

        if let Some(ref store) = self.state_store {
            let _ = store.append_event(
                "CASREP_ROUTED",
                &serde_json::json!({
                    "casrep_id": casrep.id,
                    "failed_agent": casrep.agent_id,
                    "failure_type": casrep.failure_type,
                    "error_message": casrep.error_message,
                    "task_id": casrep.task_id,
                    "escalated_to": escalate_to,
                }),
                None,
                Some(&casrep.agent_id),
            );
        }

        Ok(true)
    }

    // ------------------------------------------------------------------
    // Audit log accessors
    // ------------------------------------------------------------------

    pub fn order_log(&self) -> &[Order] {
        &self.order_log
    }

    pub fn sitrep_log(&self) -> &[SitRep] {
        &self.sitrep_log
    }

    pub fn casrep_log(&self) -> &[CasRep] {
        &self.casrep_log
    }

    pub fn override_log(&self) -> &[serde_json::Value] {
        &self.override_log
    }

    pub fn contract_log(&self) -> &[ContractRecord] {
        &self.contract_log
    }

    // ------------------------------------------------------------------
    // Contract routing (Civilian Reserve)
    // ------------------------------------------------------------------

    /// Initiate a contract from a military agent to a civilian firm (or
    /// independent).
    ///
    /// Validation:
    /// 1. `contracting_agent` must resolve to a military `RRAgent`.
    /// 2. `firm_id` must resolve to a civilian record whose tier is `Firm` or
    ///    `Independent` (specialists cannot be contracted directly —
    ///    contracts address the firm; the firm routes internally).
    ///
    /// On success, the contract is appended to the state-store event log
    /// (`CONTRACT_REQUESTED`) and to the `contracts` projection table, and is
    /// returned to the caller in `Requested` state.
    pub fn route_contract_request(
        &mut self,
        request: ContractRequest,
    ) -> Result<ContractRecord> {
        // 1. Verify the contracting agent exists and is military.
        if (self.lookup)(&request.contracting_agent).is_none() {
            return Err(C2RouterError::NonMilitaryContractingAgent(
                request.contracting_agent.clone(),
            ));
        }

        // 2. Verify the firm exists, is a civilian, and is not a specialist.
        let firm = self
            .lookup_civilian(&request.firm_id)
            .ok_or_else(|| C2RouterError::FirmNotFound(request.firm_id.clone()))?;
        if firm.is_specialist() {
            let parent = firm.firm_id.clone().unwrap_or_else(|| "<unknown>".into());
            return Err(C2RouterError::SpecialistDirectlyContracted(
                request.firm_id.clone(),
                parent,
            ));
        }

        let mut contract = ContractRecord {
            id: uuid::Uuid::new_v4().to_string(),
            contracting_agent: request.contracting_agent,
            firm_id: request.firm_id,
            routed_specialist_id: None,
            state: ContractState::Requested,
            brief: request.brief,
            mission_id: request.mission_id,
            priority: request.priority,
            created_at: chrono::Utc::now(),
            closed_at: None,
            outcome: None,
            metadata: request.metadata,
        };

        // 3. Persist: append event + upsert projection.
        if let Some(ref store) = self.state_store {
            let payload = serde_json::json!({
                "contract_id": contract.id,
                "from_agent": contract.contracting_agent,
                "firm_id": contract.firm_id,
                "brief": contract.brief,
                "mission_id": contract.mission_id,
                "priority": contract.priority,
            });
            store
                .append_event(
                    "CONTRACT_REQUESTED",
                    &payload,
                    None,
                    Some(&contract.contracting_agent),
                )
                .map_err(|e| C2RouterError::CommandAuthority(
                    contract.contracting_agent.clone(),
                    format!("event log write failed: {}", e),
                ))?;
            store.save_contract(&contract).map_err(|e| {
                C2RouterError::CommandAuthority(
                    contract.contracting_agent.clone(),
                    format!("contract projection write failed: {}", e),
                )
            })?;
        } else {
            return Err(C2RouterError::StateStoreRequired("route_contract_request"));
        }

        // Mark the contract as open-and-awaiting-accept in our local log.
        contract.state = ContractState::Requested;
        self.contract_log.push(contract.clone());

        Ok(contract)
    }
}
