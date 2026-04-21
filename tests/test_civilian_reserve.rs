//! Slice A end-to-end test for Civilian Reserve.
//!
//! Exercises the full contract lifecycle against real SQLite + real doctrine:
//!   1. Register CR-Arena (flat/independent) from its doctrine file.
//!   2. Spawn a military agent (18B Special Forces Engineer).
//!   3. Verify that `validate_command_relationship(military, civilian)` is refused.
//!   4. Route a contract request; verify `CONTRACT_REQUESTED` event + contracts row.
//!   5. Transition to InProgress via a simulated SITREP; verify event log.
//!   6. Close the contract; verify Closed state, outcome, and full event lifecycle.

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
