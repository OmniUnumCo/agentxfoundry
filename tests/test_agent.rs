//! Tests for `rr::agent` and `rr::factory`.
//!
//! Covers:
//!   - Agent construction and record round-trip
//!   - Chain of command (add/remove subordinate)
//!   - Receive order validation
//!   - Report generation (SITREP, CASREP)
//!   - Factory spawning (general, officer, NCO, specialist)
//!   - Formation building (fire team, squad, platoon)
//!   - Factory stats

use rr::models::*;
use rr::agent::RRAgent;
use rr::factory::AgentFactory;
use rr::doctrine::FileSystemDoctrineLoader;
use std::path::PathBuf;
use std::sync::Arc;

// ============================================================================
// Fixtures
// ============================================================================

fn doctrine_root() -> PathBuf {
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
// Agent tests
// ============================================================================

#[test]
fn agent_creation_and_record_roundtrip() {
    let doctrine = make_doctrine_loader();
    let agent = RRAgent::new(
        "Test Agent".into(),
        RankEnum::CPT,
        MOSEnum::CombatEngineer,
        doctrine,
        Some("gen-1".into()),
        None,
        None,
        Some("agent-1".into()),
        None,
    );

    assert_eq!(agent.id, "agent-1");
    assert_eq!(agent.name, "Test Agent");
    assert_eq!(agent.rank, RankEnum::CPT);
    assert_eq!(agent.mos, MOSEnum::CombatEngineer);
    assert!(agent.is_officer());
    assert!(!agent.is_nco());
    assert!(!agent.is_enlisted());
    assert_eq!(agent.status, StatusEnum::Standby);

    // Round-trip through AgentRecord
    let record = agent.to_record();
    assert_eq!(record.id, "agent-1");
    assert_eq!(record.name, "Test Agent");
    assert_eq!(record.rank, RankEnum::CPT);

    let doctrine2 = make_doctrine_loader();
    let restored = RRAgent::from_record(record, doctrine2, None);
    assert_eq!(restored.id, "agent-1");
    assert_eq!(restored.name, "Test Agent");
    assert_eq!(restored.rank, RankEnum::CPT);
    assert_eq!(restored.mos, MOSEnum::CombatEngineer);
}

#[test]
fn chain_of_command_add_remove_subordinate() {
    let doctrine = make_doctrine_loader();
    let mut agent = RRAgent::new(
        "Commander".into(),
        RankEnum::SGT,
        MOSEnum::Infantry,
        doctrine,
        None, None, None, Some("cmdr-1".into()), None,
    );

    assert!(agent.subordinates.is_empty());

    agent.add_subordinate("sub-1".into());
    assert_eq!(agent.subordinates, vec!["sub-1"]);

    agent.add_subordinate("sub-2".into());
    assert_eq!(agent.subordinates.len(), 2);

    agent.remove_subordinate("sub-1");
    assert_eq!(agent.subordinates, vec!["sub-2"]);

    // Cannot add self as subordinate
    agent.add_subordinate("cmdr-1".into());
    assert!(!agent.subordinates.contains(&"cmdr-1".to_string()));
}

#[test]
fn receive_order_from_correct_addressee() {
    let doctrine = make_doctrine_loader();
    let mut agent = RRAgent::new(
        "Soldier".into(),
        RankEnum::SPC,
        MOSEnum::Infantry,
        doctrine,
        None, None, None, Some("soldier-1".into()), None,
    );

    let order = Order::new("general-1", "soldier-1", OrderType::MissionOrder, "Advance");
    assert!(agent.receive_order(order).is_ok());
    assert_eq!(agent.status, StatusEnum::Active);
}

#[test]
fn receive_order_wrong_addressee_fails() {
    let doctrine = make_doctrine_loader();
    let mut agent = RRAgent::new(
        "Soldier".into(),
        RankEnum::SPC,
        MOSEnum::Infantry,
        doctrine,
        None, None, None, Some("soldier-1".into()), None,
    );

    let order = Order::new("general-1", "some-other-agent", OrderType::MissionOrder, "Advance");
    assert!(agent.receive_order(order).is_err());
    assert_eq!(agent.status, StatusEnum::Standby);
}

#[test]
fn execute_task_fails_without_llm() {
    use rr::agent::RRAgentError;
    let doctrine = make_doctrine_loader();
    let mut agent = RRAgent::new(
        "Soldier".into(),
        RankEnum::SPC,
        MOSEnum::Infantry,
        doctrine,
        None, None, None, Some("soldier-1".into()), None,
    );

    let task = Task {
        description: "Build a wall".into(),
        ..Task::default()
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(agent.execute_task(task));
    assert!(matches!(result, Err(RRAgentError::LLMNotAvailable)));
}

#[test]
fn report_sitrep() {
    let doctrine = make_doctrine_loader();
    let agent = RRAgent::new(
        "Soldier".into(),
        RankEnum::SPC,
        MOSEnum::Infantry,
        doctrine,
        Some("sgt-1".into()),
        None, None, Some("soldier-1".into()), None,
    );

    let sitrep = agent.report_sitrep("All clear".into(), 1.0);
    assert_eq!(sitrep.from_agent, "soldier-1");
    assert_eq!(sitrep.to_agent, Some("sgt-1".into()));
    assert_eq!(sitrep.message, "All clear");
    assert!((sitrep.progress - 1.0).abs() < f64::EPSILON);
}

#[test]
fn report_casrep() {
    let doctrine = make_doctrine_loader();
    let agent = RRAgent::new(
        "Soldier".into(),
        RankEnum::SPC,
        MOSEnum::Infantry,
        doctrine,
        None, None, None, Some("soldier-1".into()), None,
    );

    let casrep = agent.report_casrep("exception".into(), "Segfault".into(), None);
    assert_eq!(casrep.agent_id, "soldier-1");
    assert_eq!(casrep.failure_type, "exception");
    assert_eq!(casrep.error_message, "Segfault");
}

// ============================================================================
// Factory tests
// ============================================================================

#[test]
fn spawn_general_has_no_commander() {
    let mut factory = make_factory();
    let general = factory.spawn_general(Some("My General".into()), None);
    assert_eq!(general.rank, RankEnum::GEN);
    assert_eq!(general.mos, MOSEnum::SfEngineer);
    assert!(general.commander_id.is_none());
}

#[test]
fn spawn_officer_requires_officer_rank() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);

    let result = factory.spawn_officer(
        RankEnum::CPT,
        MOSEnum::Infantry,
        general.id.clone(),
        None,
        None,
    );
    assert!(result.is_ok());
    let officer = result.unwrap();
    assert_eq!(officer.rank, RankEnum::CPT);

    // SPC is not an officer
    let result = factory.spawn_officer(
        RankEnum::SPC,
        MOSEnum::Infantry,
        general.id.clone(),
        None,
        None,
    );
    assert!(result.is_err());
}

#[test]
fn spawn_nco_requires_nco_rank() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);

    let result = factory.spawn_nco(
        RankEnum::SGT,
        MOSEnum::Infantry,
        general.id.clone(),
        None,
        None,
    );
    assert!(result.is_ok());

    // GEN is not an NCO
    let result = factory.spawn_nco(
        RankEnum::GEN,
        MOSEnum::Infantry,
        general.id.clone(),
        None,
        None,
    );
    assert!(result.is_err());
}

#[test]
fn spawn_fire_team() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);
    let team = factory
        .spawn_fire_team(MOSEnum::Infantry, general.id.clone(), None, 4, None)
        .unwrap();

    assert_eq!(team.len(), 4);
    // Leader is first element
    assert!(team[0].rank == RankEnum::CPL);
    // Rest are specialists
    for member in &team[1..] {
        assert_eq!(member.rank, RankEnum::SPC);
    }
    // All CPLs and SPCs should have the team leader as commander
    let leader_id = &team[0].id;
    for member in &team[1..] {
        assert_eq!(member.commander_id, Some(leader_id.clone()));
    }
}

#[test]
fn spawn_squad_is_7_agents() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);
    let squad = factory
        .spawn_squad(MOSEnum::Infantry, general.id.clone(), None, None)
        .unwrap();

    // 1 GEN + 1 SGT + 2 CPL + 4 SPC = 8 total in the factory
    // But the squad itself returns 7 members (not including the general)
    assert_eq!(squad.len(), 7);

    // SGT is first
    assert_eq!(squad[0].rank, RankEnum::SGT);
    // CPLs are at indices 1 and 4
    assert_eq!(squad[1].rank, RankEnum::CPL);
    assert_eq!(squad[4].rank, RankEnum::CPL);
    // SPCs fill indices 2,3,5,6
    for member in &squad[2..] {
        assert!(member.rank == RankEnum::CPL || member.rank == RankEnum::SPC);
    }
}

#[test]
fn spawn_platoon_is_25_agents() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);
    let platoon = factory
        .spawn_platoon(MOSEnum::Infantry, general.id.clone(), None, None)
        .unwrap();

    // 1 LT + 1 SGM + 2 SPC (HQ) + 3 * 7 (squads) = 1 + 1 + 2 + 21 = 25
    assert_eq!(platoon.len(), 25);
}

#[test]
fn factory_stats() {
    let mut factory = make_factory();
    factory.spawn_general(None, None);
    factory.spawn_specialist(MOSEnum::Infantry, factory.all_agents()[0].id.clone(), None, None);

    let stats = factory.stats();
    assert_eq!(stats["active"].as_u64().unwrap(), 2);
    assert_eq!(stats["total_spawned"].as_u64().unwrap(), 2);
}

#[test]
fn factory_by_rank() {
    let mut factory = make_factory();
    factory.spawn_general(None, None);
    factory.spawn_specialist(MOSEnum::Infantry, factory.all_agents()[0].id.clone(), None, None);
    factory.spawn_specialist(MOSEnum::Infantry, factory.all_agents()[0].id.clone(), None, None);

    let generals = factory.by_rank(RankEnum::GEN);
    assert_eq!(generals.len(), 1);

    let specialists = factory.by_rank(RankEnum::SPC);
    assert_eq!(specialists.len(), 2);
}

#[test]
fn factory_deactivate() {
    let mut factory = make_factory();
    let general = factory.spawn_general(None, None);
    let specialist = factory.spawn_specialist(
        MOSEnum::Infantry,
        general.id.clone(),
        None,
        None,
    );

    assert_eq!(factory.all_agents().len(), 2);

    let deactivated = factory.deactivate(&specialist.id);
    assert!(deactivated.is_some());
    assert_eq!(factory.all_agents().len(), 1);
}
