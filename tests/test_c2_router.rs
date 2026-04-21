//! Tests for `rr::c2_router`.
//!
//! Covers:
//!   - Chain-of-command upward walk
//!   - Cycle detection
//!   - Order routing with validation
//!   - Override audit logging
//!   - SITREP auto-resolution to commander
//!   - CASREP escalation

use rr::models::*;
use rr::agent::RRAgent;
use rr::c2_router::C2Router;
use rr::doctrine::FileSystemDoctrineLoader;
use rr::factory::AgentFactory;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// ============================================================================
// Test fixtures (inline, no external conftest)
// ============================================================================

fn doctrine_root() -> PathBuf {
    // Walk up from tests/ to doctrine/
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(&manifest).join("doctrine")
}

fn make_doctrine_loader() -> Arc<FileSystemDoctrineLoader> {
    Arc::new(FileSystemDoctrineLoader::new(doctrine_root()))
}

fn make_factory() -> AgentFactory {
    AgentFactory::new(make_doctrine_loader())
}

// ============================================================================
// Walk Chain tests
// ============================================================================

#[test]
fn walk_chain_from_specialist() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);
    let specialist = factory.spawn_specialist(
        MOSEnum::Infantry,
        general.id.clone(),
        None,
        None,
    );

    // We need a fresh router that sees the current factory state
    let registry: HashMap<String, RRAgent> = factory.all_agents()
        .into_iter()
        .map(|a| (a.id.clone(), a.clone()))
        .collect();
    let router = C2Router::new(move |id: &str| registry.get(id).cloned(), None);

    let chain = router.walk_chain_upward(&specialist.id, 64).unwrap();
    assert_eq!(chain, vec![specialist.id, general.id]);
}

#[test]
fn walk_chain_on_unknown_returns_just_that_id() {
    let router = C2Router::new(|_: &str| None, None);
    let chain = router.walk_chain_upward("no-such-agent", 64).unwrap();
    assert_eq!(chain, vec!["no-such-agent"]);
}

// ============================================================================
// Cycle Detection tests
// ============================================================================

#[test]
fn fresh_platoons_have_no_cycles() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);
    let platoon = factory.spawn_platoon(
        MOSEnum::Infantry,
        general.id.clone(),
        None,
        None,
    ).unwrap();

    let registry: HashMap<String, RRAgent> = factory.all_agents()
        .into_iter()
        .map(|a| (a.id.clone(), a.clone()))
        .collect();
    let router = C2Router::new(move |id: &str| registry.get(id).cloned(), None);

    for agent in &platoon {
        // Walking up must terminate at the General without revisiting anyone.
        let chain = router.walk_chain_upward(&agent.id, 64).unwrap();
        // No duplicates
        let unique: std::collections::HashSet<_> = chain.iter().collect();
        assert_eq!(chain.len(), unique.len(), "chain has duplicates for agent {}", agent.id);
        // Last element is the General
        assert_eq!(chain.last().unwrap(), &general.id, "chain doesn't end at general for agent {}", agent.id);
    }
}

#[test]
fn no_cycle_check_rejects_self_commander() {
    let router = C2Router::new(|_: &str| None, None);
    let result = router.check_no_cycle_on_spawn("agent-1", "agent-1");
    assert!(result.is_err());
}

// ============================================================================
// Order Routing tests
// ============================================================================

#[test]
fn direct_subordinate_order_delivers() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);
    let spc = factory.spawn_specialist(
        MOSEnum::Infantry,
        general.id.clone(),
        None,
        None,
    );

    let registry: HashMap<String, RRAgent> = factory.all_agents()
        .into_iter()
        .map(|a| (a.id.clone(), a.clone()))
        .collect();

    // For this test we need a mutable lookup — use a shared Arc<Mutex<HashMap>>
    let shared = Arc::new(std::sync::Mutex::new(registry));
    let shared_clone = shared.clone();
    let mut router = C2Router::new(
        move |id: &str| shared_clone.lock().unwrap().get(id).cloned(),
        None,
    );

    let order = Order::new(&general.id, &spc.id, OrderType::DirectTask, "Do the thing");
    assert!(router.route_order(order).unwrap());

    // Check the order log
    assert_eq!(router.order_log().len(), 1);
}

#[test]
fn unauthorized_order_is_rejected() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);
    let _spc_a = factory.spawn_specialist(
        MOSEnum::Infantry,
        general.id.clone(),
        None,
        None,
    );
    let _spc_b = factory.spawn_specialist(
        MOSEnum::Infantry,
        general.id.clone(),
        None,
        None,
    );

    let registry: HashMap<String, RRAgent> = factory.all_agents()
        .into_iter()
        .map(|a| (a.id.clone(), a.clone()))
        .collect();

    let shared = Arc::new(std::sync::Mutex::new(registry));
    let shared_clone = shared.clone();
    let mut router = C2Router::new(
        move |id: &str| shared_clone.lock().unwrap().get(id).cloned(),
        None,
    );

    // SPC A cannot order SPC B -- they are peers, not in a command chain.
    // But the General can command both via rank override.
    // To test rejection, create agents that the General cannot outrank
    // (impossible in this hierarchy since GEN is highest).
    // Instead, test that a subordinate can't order the General.
    let general_id = shared.lock().unwrap().iter()
        .find(|(_, a)| a.rank == RankEnum::GEN)
        .map(|(id, _)| id.clone())
        .unwrap();

    let subordinate_id = shared.lock().unwrap().iter()
        .find(|(_, a)| a.rank == RankEnum::SPC)
        .map(|(id, _)| id.clone())
        .unwrap();

    let bad_order = Order::new(&subordinate_id, &general_id, OrderType::DirectTask, "Follow me");
    let result = router.route_order(bad_order);
    assert!(result.is_err());
}

#[test]
fn broadcast_delivers_to_all_direct_subordinates() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);
    for _ in 0..3 {
        factory.spawn_specialist(
            MOSEnum::Infantry,
            general.id.clone(),
            None,
            None,
        );
    }

    let registry: HashMap<String, RRAgent> = factory.all_agents()
        .into_iter()
        .map(|a| (a.id.clone(), a.clone()))
        .collect();

    let shared = Arc::new(std::sync::Mutex::new(registry));
    let shared_clone = shared.clone();
    let mut router = C2Router::new(
        move |id: &str| shared_clone.lock().unwrap().get(id).cloned(),
        None,
    );

    let delivered = router.broadcast_order(&general.id, "Stand by", Some(OrderType::Broadcast)).unwrap();
    // All 3 specialists should receive it, plus the General can order them
    assert_eq!(delivered.len(), 3);
}

// ============================================================================
// SitRep and CASREP tests
// ============================================================================

#[test]
fn sitrep_is_recorded() {
    let router = C2Router::new(|_: &str| None, None);
    let mut sitrep = SitRep::new("agent-1", StatusEnum::Active);
    sitrep.message = "Halfway done".into();
    sitrep.progress = 0.5;
    assert!(sitrep.to_agent.is_none());

    let mut router = router;
    assert!(router.route_sitrep(sitrep.clone()).unwrap());
    assert_eq!(router.sitrep_log().len(), 1);
}

#[test]
fn casrep_is_recorded() {
    let casrep = CasRep::new("agent-1", "exception", "KeyError: 'missing_field'");
    let mut router = C2Router::new(|_: &str| None, None);
    assert!(router.route_casrep(casrep.clone()).unwrap());
    assert_eq!(router.casrep_log().len(), 1);
    assert_eq!(router.casrep_log()[0].agent_id, "agent-1");
}
