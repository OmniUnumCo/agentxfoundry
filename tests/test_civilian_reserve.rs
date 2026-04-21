//! Slice A end-to-end test for Civilian Reserve.
//!
//! Exercises the full contract lifecycle against real SQLite + real doctrine:
//!   1. Register CR-Arena (flat/independent) from its doctrine file.
//!   2. Spawn a military agent (18B Special Forces Engineer).
//!   3. Verify that `validate_command_relationship(military, civilian)` is refused.
//!   4. Route a contract request; verify `CONTRACT_REQUESTED` event + contracts row.
//!   5. Transition to InProgress via a simulated SITREP; verify event log.
//!   6. Close the contract; verify Closed state, outcome, and full event lifecycle.

use rr::broker::ContractBroker;
use rr::c2_router::{C2Router, C2RouterError};
use rr::doctrine::FileSystemDoctrineLoader;
use rr::factory::AgentFactory;
use rr::models::*;
use rr::persistence::SqliteStateStore;
use rr::traits::{DoctrineLoader, StateStore};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// ============================================================================
// Fixtures
// ============================================================================

fn doctrine_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(&manifest).join("doctrine")
}

fn make_loader() -> FileSystemDoctrineLoader {
    FileSystemDoctrineLoader::new(doctrine_root())
}

fn open_store(dir: &tempfile::TempDir) -> SqliteStateStore {
    let db_path = dir.path().join("rr.db");
    SqliteStateStore::open(&db_path, Box::new(|| "mil-0.1.0+civ-0.1.0".to_string())).unwrap()
}

// ============================================================================
// Individual step tests
// ============================================================================

#[test]
fn civilian_doctrine_parses_arena() {
    let loader = make_loader();
    let doctrine = loader.civilian_doctrine("arena").expect("CR-Arena doctrine loads");
    assert_eq!(doctrine.slug, "arena");
    assert_eq!(doctrine.kind, CivilianTier::Independent);
    assert_eq!(doctrine.project_path.as_deref(), Some("~/Projects/arena"));
    assert_eq!(doctrine.languages, vec!["rust"]);
    assert!(doctrine.contract_scope.contains(&"multi-agent-consensus".to_string()));
    assert!(!doctrine.body.is_empty(), "body should contain the markdown");
}

#[test]
fn civilian_doctrine_rejects_traversal_slug() {
    let loader = make_loader();
    for bad in ["../secret", "civ/../other", "..", "", "A", "civ!", "..\\win"] {
        let result = loader.civilian_doctrine(bad);
        assert!(result.is_err(), "slug `{}` must be rejected", bad);
    }
}

#[test]
fn register_civilian_and_persist() {
    let dir = tempfile::tempdir().unwrap();
    let store = open_store(&dir);
    let loader = make_loader();

    let doctrine = loader.civilian_doctrine("arena").unwrap();
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
    store.save_civilian(&civilian).unwrap();

    let loaded = store.load_civilian("arena").unwrap().expect("civilian persisted");
    assert_eq!(loaded.id, "arena");
    assert_eq!(loaded.tier, CivilianTier::Independent);
    assert_eq!(loaded.name, "CR-arena");

    let listed = store.list_civilians().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, "arena");
}

// ============================================================================
// End-to-end lifecycle
// ============================================================================

#[test]
fn contract_lifecycle_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let store = open_store(&dir);
    let loader = Arc::new(make_loader());

    // 1. Register CR-Arena from its doctrine.
    let doctrine = loader.civilian_doctrine("arena").unwrap();
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
    store.save_civilian(&civilian).unwrap();
    store
        .append_event(
            "CIVILIAN_REGISTERED",
            &serde_json::json!({"civilian_id": civilian.id, "tier": civilian.tier}),
            None,
            None,
        )
        .unwrap();

    // 2. Spawn a military 18B SfEngineer.
    let mut factory = AgentFactory::new(loader.clone());
    let general = factory.spawn_general(None, None);
    let sf_engineer = factory.spawn_specialist(
        MOSEnum::SfEngineer,
        general.id.clone(),
        None,
        None,
    );
    store.save_agent(&sf_engineer.to_record()).unwrap();
    store.save_agent(&general.to_record()).unwrap();

    // 3. Verify civilian refusal via C2Router + civilian lookup.
    let registry: HashMap<String, rr::RRAgent> = factory
        .all_agents()
        .into_iter()
        .map(|a| (a.id.clone(), a.clone()))
        .collect();
    let civilians_for_lookup: HashMap<String, CivilianRecord> =
        std::iter::once((civilian.id.clone(), civilian.clone())).collect();

    // Build a separate SqliteStateStore handle for the router's event log.
    let router_store = SqliteStateStore::open(
        &dir.path().join("rr.db"),
        Box::new(|| "mil-0.1.0+civ-0.1.0".to_string()),
    )
    .unwrap();

    let mut router = C2Router::new(
        move |id: &str| registry.get(id).cloned(),
        Some(Box::new(router_store)),
    )
    .with_civilian_lookup(move |id: &str| civilians_for_lookup.get(id).cloned());

    let (valid, _is_override) =
        router.validate_command_relationship(&sf_engineer.id, "arena");
    assert!(
        !valid,
        "civilian targets must never appear as valid subordinates of a military agent"
    );

    // Direct order attempt against a civilian fails with CommandAuthority.
    let order = Order::new(
        &sf_engineer.id,
        "arena",
        OrderType::DirectTask,
        "this should be refused",
    );
    let order_err = router.route_order(order).unwrap_err();
    assert!(
        matches!(order_err, C2RouterError::CommandAuthority(_, _)),
        "route_order against a civilian must return CommandAuthority, got {:?}",
        order_err
    );

    // 4. Route a contract request.
    let request = ContractRequest {
        contracting_agent: sf_engineer.id.clone(),
        firm_id: "arena".to_string(),
        brief: "Review Arena council-mode consensus thresholds".to_string(),
        mission_id: None,
        priority: PriorityEnum::Priority,
        metadata: HashMap::new(),
    };
    let contract = router.route_contract_request(request).unwrap();
    assert_eq!(contract.state, ContractState::Requested);
    assert_eq!(contract.firm_id, "arena");
    assert_eq!(contract.contracting_agent, sf_engineer.id);

    // Verify DB projection and event log.
    let persisted = store.load_contract(&contract.id).unwrap().unwrap();
    assert_eq!(persisted.state, ContractState::Requested);

    let by_firm = store
        .list_contracts_by_firm("arena", Some(ContractState::Requested))
        .unwrap();
    assert_eq!(by_firm.len(), 1);
    assert_eq!(by_firm[0].id, contract.id);

    // 5. Simulate SITREP -> implicit transition to InProgress.
    let mut updated = persisted.clone();
    updated.state = ContractState::InProgress;
    store.save_contract(&updated).unwrap();
    store
        .append_event(
            "CONTRACT_SITREP",
            &serde_json::json!({
                "contract_id": contract.id,
                "firm_id": "arena",
                "status": "active",
                "progress": 0.5,
                "message": "drafting comparison matrix",
                "blockers": []
            }),
            None,
            Some(&sf_engineer.id),
        )
        .unwrap();

    let mid = store.load_contract(&contract.id).unwrap().unwrap();
    assert_eq!(mid.state, ContractState::InProgress);

    // 6. Closeout.
    let mut closed = mid.clone();
    closed.state = ContractState::Closed;
    closed.outcome = Some(ContractOutcome::Complete);
    closed.closed_at = Some(chrono::Utc::now());
    store.save_contract(&closed).unwrap();
    store
        .append_event(
            "CONTRACT_CLOSEOUT",
            &serde_json::json!({
                "contract_id": contract.id,
                "firm_id": "arena",
                "outcome": "complete",
                "deliverables": "comparison matrix: arena vs rr orchestrator primitives"
            }),
            None,
            Some(&sf_engineer.id),
        )
        .unwrap();

    let final_state = store.load_contract(&contract.id).unwrap().unwrap();
    assert_eq!(final_state.state, ContractState::Closed);
    assert_eq!(final_state.outcome, Some(ContractOutcome::Complete));
    assert!(final_state.closed_at.is_some());

    // 7. Verify the full event lifecycle is recorded.
    let event_count = store.event_count().unwrap();
    // At minimum: CIVILIAN_REGISTERED + CONTRACT_REQUESTED + CONTRACT_SITREP + CONTRACT_CLOSEOUT
    assert!(
        event_count >= 4,
        "expected at least 4 events (civilian + contract lifecycle), got {}",
        event_count
    );
}

// ============================================================================
// Slice B -- firm + specialist with broker routing
// ============================================================================

#[test]
fn firm_doctrine_loads_return42() {
    let loader = make_loader();
    let doctrine = loader.civilian_doctrine("return42").expect("CR-Return42 firm loads");
    assert_eq!(doctrine.slug, "return42");
    assert_eq!(doctrine.kind, CivilianTier::Firm);
    assert_eq!(
        doctrine.specialists,
        vec!["morphlex".to_string(), "jasterish".to_string(), "custom-llm".to_string()]
    );
    assert_eq!(doctrine.project_path.as_deref(), Some("~/Projects/apps"));
}

#[test]
fn specialist_doctrine_loads_morphlex() {
    let loader = make_loader();
    let doctrine = loader
        .specialist_doctrine("return42", "morphlex")
        .expect("morphlex specialist loads");
    assert_eq!(doctrine.slug, "morphlex");
    assert_eq!(doctrine.kind, CivilianTier::Specialist);
    assert_eq!(doctrine.firm.as_deref(), Some("return42"));
    assert!(doctrine
        .contract_scope
        .iter()
        .any(|s| s == "tokenizer" || s == "token-vector-layout"));
}

#[test]
fn broker_routes_by_return42_firm_rules() {
    let loader = make_loader();
    let firm = loader.civilian_doctrine("return42").unwrap();

    // Each brief includes a keyword from exactly one specialist's routing rule.
    assert_eq!(
        ContractBroker::route(&firm, "Extend the tokenizer morphology to handle infixes."),
        Some("morphlex".to_string())
    );
    assert_eq!(
        ContractBroker::route(&firm, "Plan the x86-64 codegen path for the JStar backend."),
        Some("jasterish".to_string())
    );
    assert_eq!(
        ContractBroker::route(
            &firm,
            "Discuss attention patterns appropriate for the custom LLM architecture."
        ),
        Some("custom-llm".to_string())
    );
}

#[test]
fn firm_contract_lifecycle_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStateStore::open(
        &dir.path().join("rr.db"),
        Box::new(|| "mil-0.1.0+civ-0.1.0".to_string()),
    )
    .unwrap();
    let loader = Arc::new(make_loader());

    // 1. Register CR-Return42 firm.
    let firm_doctrine = loader.civilian_doctrine("return42").unwrap();
    let firm_civ = CivilianRecord {
        id: firm_doctrine.slug.clone(),
        name: format!("CR-{}", firm_doctrine.slug),
        tier: firm_doctrine.kind,
        firm_id: firm_doctrine.firm.clone(),
        specialists: firm_doctrine.specialists.clone(),
        specialty_slug: firm_doctrine.slug.clone(),
        project_path: firm_doctrine.project_path.clone(),
        upstream_url: firm_doctrine.upstream_url.clone(),
        status: StatusEnum::Standby,
        active_contracts: Vec::new(),
        performance_score: 0.0,
        created_at: chrono::Utc::now(),
        metadata: HashMap::new(),
    };
    store.save_civilian(&firm_civ).unwrap();
    store
        .append_event(
            "FIRM_REGISTERED",
            &serde_json::json!({"civilian_id": firm_civ.id}),
            None,
            None,
        )
        .unwrap();

    // 2. Register CR-Return42-Morphlex specialist (optional but realistic).
    let spec_doctrine = loader
        .specialist_doctrine("return42", "morphlex")
        .unwrap();
    let spec_civ = CivilianRecord {
        id: format!("{}:{}", firm_doctrine.slug, spec_doctrine.slug),
        name: format!("CR-{}::{}", firm_doctrine.slug, spec_doctrine.slug),
        tier: spec_doctrine.kind,
        firm_id: spec_doctrine.firm.clone(),
        specialists: Vec::new(),
        specialty_slug: spec_doctrine.slug.clone(),
        project_path: spec_doctrine.project_path.clone(),
        upstream_url: spec_doctrine.upstream_url.clone(),
        status: StatusEnum::Standby,
        active_contracts: Vec::new(),
        performance_score: 0.0,
        created_at: chrono::Utc::now(),
        metadata: HashMap::new(),
    };
    store.save_civilian(&spec_civ).unwrap();
    store
        .append_event(
            "CIVILIAN_REGISTERED",
            &serde_json::json!({"civilian_id": spec_civ.id, "firm_id": "return42"}),
            None,
            None,
        )
        .unwrap();

    // 3. Spawn a military 18B SfEngineer.
    let mut factory = AgentFactory::new(loader.clone());
    let general = factory.spawn_general(None, None);
    let sf_engineer = factory.spawn_specialist(
        MOSEnum::SfEngineer,
        general.id.clone(),
        None,
        None,
    );
    store.save_agent(&sf_engineer.to_record()).unwrap();
    store.save_agent(&general.to_record()).unwrap();

    // 4. Build the C2 router with a civilian lookup that covers both firm and
    //    specialist records -- specialists should NOT be directly contractable
    //    (enforced by `route_contract_request`).
    let registry: HashMap<String, rr::RRAgent> = factory
        .all_agents()
        .into_iter()
        .map(|a| (a.id.clone(), a.clone()))
        .collect();
    let firm_for_lookup = firm_civ.clone();
    let spec_for_lookup = spec_civ.clone();
    let router_store = SqliteStateStore::open(
        &dir.path().join("rr.db"),
        Box::new(|| "mil-0.1.0+civ-0.1.0".to_string()),
    )
    .unwrap();
    let mut router = C2Router::new(
        move |id: &str| registry.get(id).cloned(),
        Some(Box::new(router_store)),
    )
    .with_civilian_lookup(move |id: &str| {
        if id == firm_for_lookup.id {
            Some(firm_for_lookup.clone())
        } else if id == spec_for_lookup.id {
            Some(spec_for_lookup.clone())
        } else {
            None
        }
    });

    // 5. Attempting to contract a specialist directly must fail.
    let specialist_request = ContractRequest {
        contracting_agent: sf_engineer.id.clone(),
        firm_id: spec_civ.id.clone(),
        brief: "direct to specialist -- not allowed".into(),
        mission_id: None,
        priority: PriorityEnum::Routine,
        metadata: HashMap::new(),
    };
    let err = router.route_contract_request(specialist_request).unwrap_err();
    assert!(
        matches!(err, C2RouterError::SpecialistDirectlyContracted(_, _)),
        "direct-to-specialist must be refused at the firm-level contract boundary, got {:?}",
        err
    );

    // 6. Contract the firm with a brief that should route to morphlex.
    let request = ContractRequest {
        contracting_agent: sf_engineer.id.clone(),
        firm_id: firm_civ.id.clone(),
        brief: "Extend the tokenizer to classify zero-width joiners as morpheme boundaries".into(),
        mission_id: None,
        priority: PriorityEnum::Priority,
        metadata: HashMap::new(),
    };
    let contract = router.route_contract_request(request).unwrap();
    assert_eq!(contract.state, ContractState::Requested);
    assert_eq!(contract.firm_id, "return42");
    assert!(contract.routed_specialist_id.is_none());

    // 7. Broker picks the right specialist from the firm doctrine.
    let chosen = ContractBroker::route(&firm_doctrine, &contract.brief).unwrap();
    assert_eq!(chosen, "morphlex");
    assert!(
        firm_doctrine.specialists.iter().any(|s| s == &chosen),
        "broker chose a specialist that is not in the firm's roster: {}",
        chosen
    );

    // 8. Simulate cr_contract_accept: Requested -> Accepted + routed_specialist_id.
    let mut accepted = contract.clone();
    accepted.state = ContractState::Accepted;
    accepted.routed_specialist_id = Some(chosen.clone());
    store.save_contract(&accepted).unwrap();
    store
        .append_event(
            "CONTRACT_ACCEPTED",
            &serde_json::json!({
                "contract_id": accepted.id,
                "firm_id": "return42",
                "routed_specialist_id": chosen,
            }),
            None,
            Some(&sf_engineer.id),
        )
        .unwrap();

    let after_accept = store.load_contract(&accepted.id).unwrap().unwrap();
    assert_eq!(after_accept.state, ContractState::Accepted);
    assert_eq!(after_accept.routed_specialist_id.as_deref(), Some("morphlex"));

    // 9. SITREP -> implicit InProgress.
    let mut in_progress = after_accept.clone();
    in_progress.state = ContractState::InProgress;
    store.save_contract(&in_progress).unwrap();
    store
        .append_event(
            "CONTRACT_SITREP",
            &serde_json::json!({
                "contract_id": in_progress.id,
                "firm_id": "return42",
                "status": "active",
                "progress": 0.4,
                "message": "drafted the morpheme-boundary decision table",
                "blockers": []
            }),
            None,
            Some(&sf_engineer.id),
        )
        .unwrap();

    let mid = store.load_contract(&in_progress.id).unwrap().unwrap();
    assert_eq!(mid.state, ContractState::InProgress);

    // 10. Closeout.
    let mut closed = mid.clone();
    closed.state = ContractState::Closed;
    closed.outcome = Some(ContractOutcome::Complete);
    closed.closed_at = Some(chrono::Utc::now());
    store.save_contract(&closed).unwrap();
    store
        .append_event(
            "CONTRACT_CLOSEOUT",
            &serde_json::json!({
                "contract_id": closed.id,
                "firm_id": "return42",
                "outcome": "complete",
                "deliverables": "decision table + proposed morphology rule patch"
            }),
            None,
            Some(&sf_engineer.id),
        )
        .unwrap();

    let terminal = store.load_contract(&closed.id).unwrap().unwrap();
    assert_eq!(terminal.state, ContractState::Closed);
    assert_eq!(terminal.outcome, Some(ContractOutcome::Complete));
    assert_eq!(terminal.routed_specialist_id.as_deref(), Some("morphlex"));

    // 11. Verify full event lifecycle: FIRM_REGISTERED + CIVILIAN_REGISTERED
    //     + CONTRACT_REQUESTED + CONTRACT_ACCEPTED + CONTRACT_SITREP + CONTRACT_CLOSEOUT
    let event_count = store.event_count().unwrap();
    assert!(
        event_count >= 6,
        "expected at least 6 events for the full firm-contract lifecycle, got {}",
        event_count
    );
}
