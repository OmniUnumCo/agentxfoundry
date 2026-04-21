//! Tests for `rr::models`.
//!
//! Covers:
//!   - Enum serialization/deserialization round-trips
//!   - Rank authority and outranking logic
//!   - MOS category lookups
//!   - Default model construction
//!   - AgentRecord convenience methods

use rr::models::*;
use serde_json;

#[test]
fn rank_authority_is_strictly_ordered() {
    // GEN = 12, PVT = 1
    assert_eq!(RankEnum::GEN.authority(), 12);
    assert_eq!(RankEnum::PVT.authority(), 1);
    // Every rank has a unique authority level
    let mut levels: Vec<u8> = vec![
        RankEnum::GEN, RankEnum::COL, RankEnum::MAJ, RankEnum::CPT, RankEnum::LT,
        RankEnum::SGM, RankEnum::MSG, RankEnum::SGT, RankEnum::CPL,
        RankEnum::SPC, RankEnum::PFC, RankEnum::PVT,
    ].iter().map(|r| r.authority()).collect();
    levels.sort();
    levels.dedup();
    assert_eq!(levels.len(), 12, "all authority levels must be unique");
}

#[test]
fn outranks_is_correct() {
    assert!(RankEnum::GEN.outranks(&RankEnum::COL));
    assert!(RankEnum::COL.outranks(&RankEnum::MAJ));
    assert!(RankEnum::CPT.outranks(&RankEnum::LT));
    assert!(RankEnum::LT.outranks(&RankEnum::SGM));
    assert!(RankEnum::SGM.outranks(&RankEnum::MSG));
    assert!(RankEnum::MSG.outranks(&RankEnum::SGT));
    assert!(RankEnum::SGT.outranks(&RankEnum::CPL));
    assert!(RankEnum::CPL.outranks(&RankEnum::SPC));
    assert!(RankEnum::SPC.outranks(&RankEnum::PFC));
    assert!(RankEnum::PFC.outranks(&RankEnum::PVT));
    // No rank outranks itself
    assert!(!RankEnum::GEN.outranks(&RankEnum::GEN));
}

#[test]
fn rank_categories() {
    // Officers
    for rank in [RankEnum::GEN, RankEnum::COL, RankEnum::MAJ, RankEnum::CPT, RankEnum::LT] {
        assert_eq!(rank.category(), RankCategory::Officer, "{:?} should be Officer", rank);
    }
    // NCOs
    for rank in [RankEnum::SGM, RankEnum::MSG, RankEnum::SGT, RankEnum::CPL] {
        assert_eq!(rank.category(), RankCategory::Nco, "{:?} should be NCO", rank);
    }
    // Enlisted
    for rank in [RankEnum::SPC, RankEnum::PFC, RankEnum::PVT] {
        assert_eq!(rank.category(), RankCategory::Enlisted, "{:?} should be Enlisted", rank);
    }
}

#[test]
fn rank_full_names() {
    assert_eq!(RankEnum::GEN.full_name(), "General");
    assert_eq!(RankEnum::COL.full_name(), "Colonel");
    assert_eq!(RankEnum::LT.full_name(), "Lieutenant");
    assert_eq!(RankEnum::SGM.full_name(), "Sergeant Major");
    assert_eq!(RankEnum::PVT.full_name(), "Private");
}

#[test]
fn mos_categories() {
    use MOSEnum::*;
    use MOSCategory::*;
    assert_eq!(IntelAnalyst.category(), Intelligence);
    assert_eq!(CounterIntel.category(), Intelligence);
    assert_eq!(SigintAnalyst.category(), Intelligence);
    assert_eq!(Infantry.category(), Operations);
    assert_eq!(CombatEngineer.category(), Operations);
    assert_eq!(Artillery.category(), Operations);
    assert_eq!(CavalryScout.category(), Operations);
    assert_eq!(ItSpecialist.category(), Support);
    assert_eq!(HrSpecialist.category(), Support);
    assert_eq!(Transport.category(), Support);
    assert_eq!(Logistics.category(), Support);
    assert_eq!(SfEngineer.category(), SpecialOperations);
    assert_eq!(Ranger.category(), SpecialOperations);
    assert_eq!(Soar.category(), SpecialOperations);
}

#[test]
fn mos_full_names() {
    assert_eq!(MOSEnum::IntelAnalyst.full_name(), "Intelligence Analyst");
    assert_eq!(MOSEnum::CombatEngineer.full_name(), "Combat Engineer");
    assert_eq!(MOSEnum::SfEngineer.full_name(), "Special Forces Engineer");
}

#[test]
fn rank_enum_serde_roundtrip() {
    let rank = RankEnum::CPT;
    let json = serde_json::to_string(&rank).unwrap();
    let decoded: RankEnum = serde_json::from_str(&json).unwrap();
    assert_eq!(rank, decoded);
}

#[test]
fn mos_enum_serde_roundtrip() {
    let mos = MOSEnum::CombatEngineer;
    let json = serde_json::to_string(&mos).unwrap();
    let decoded: MOSEnum = serde_json::from_str(&json).unwrap();
    assert_eq!(mos, decoded);
}

#[test]
fn status_enum_serde() {
    for status in [StatusEnum::Standby, StatusEnum::Active, StatusEnum::Engaged, StatusEnum::Complete, StatusEnum::Failed] {
        let json = serde_json::to_string(&status).unwrap();
        let decoded: StatusEnum = serde_json::from_str(&json).unwrap();
        assert_eq!(status, decoded);
    }
}

#[test]
fn task_defaults() {
    let task = Task::default();
    assert!(!task.id.is_empty());
    assert_eq!(task.status, StatusEnum::Standby);
    assert!(task.assigned_to.is_none());
    assert!(task.dependencies.is_empty());
    assert!(task.result.is_none());
    assert!(task.error.is_none());
    assert!(task.completed_at.is_none());
}

#[test]
fn agent_record_convenience_methods() {
    let officer = AgentRecord {
        id: "o1".into(),
        name: "Officer".into(),
        rank: RankEnum::CPT,
        mos: MOSEnum::CombatEngineer,
        ..AgentRecord::default()
    };
    assert!(officer.is_officer());
    assert!(!officer.is_nco());
    assert!(!officer.is_enlisted());
    assert_eq!(officer.rank_authority(), 9);

    let nco = AgentRecord {
        id: "n1".into(),
        name: "NCO".into(),
        rank: RankEnum::SGT,
        mos: MOSEnum::Infantry,
        ..AgentRecord::default()
    };
    assert!(!nco.is_officer());
    assert!(nco.is_nco());
    assert_eq!(nco.rank_authority(), 5);

    let enlisted = AgentRecord {
        id: "e1".into(),
        name: "Enlisted".into(),
        rank: RankEnum::PVT,
        mos: MOSEnum::Infantry,
        ..AgentRecord::default()
    };
    assert!(!enlisted.is_officer());
    assert!(!enlisted.is_nco());
    assert!(enlisted.is_enlisted());
    assert_eq!(enlisted.rank_authority(), 1);
}

#[test]
fn agent_record_serde_roundtrip() {
    let record = AgentRecord {
        id: "agent-1".into(),
        name: "Alpha".into(),
        rank: RankEnum::CPT,
        mos: MOSEnum::CombatEngineer,
        commander_id: Some("gen-1".into()),
        subordinates: vec!["sub-1".into(), "sub-2".into()],
        status: StatusEnum::Active,
        missions_completed: 3,
        performance_score: 0.85,
        ..AgentRecord::default()
    };
    let json = serde_json::to_string(&record).unwrap();
    let decoded: AgentRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(record.id, decoded.id);
    assert_eq!(record.name, decoded.name);
    assert_eq!(record.rank, decoded.rank);
    assert_eq!(record.mos, decoded.mos);
    assert_eq!(record.commander_id, decoded.commander_id);
    assert_eq!(record.subordinates, decoded.subordinates);
    assert_eq!(record.status, decoded.status);
    assert_eq!(record.missions_completed, decoded.missions_completed);
    assert!((record.performance_score - decoded.performance_score).abs() < f64::EPSILON);
}

#[test]
fn swarm_serde_roundtrip() {
    let swarm = Swarm {
        id: "swarm-1".into(),
        name: "Test Swarm".into(),
        general_id: "gen-1".into(),
        agents: vec!["a1".into(), "a2".into()],
        status: StatusEnum::Active,
        ..Swarm::default()
    };
    let json = serde_json::to_string(&swarm).unwrap();
    let decoded: Swarm = serde_json::from_str(&json).unwrap();
    assert_eq!(swarm.id, decoded.id);
    assert_eq!(swarm.name, decoded.name);
    assert_eq!(swarm.agents, decoded.agents);
}

#[test]
fn order_creation() {
    let order = Order::new("superior-1", "subordinate-1", OrderType::DirectTask, "Do this task");
    assert_eq!(order.from_agent, "superior-1");
    assert_eq!(order.to_agent, "subordinate-1");
    assert_eq!(order.order_type, OrderType::DirectTask);
    assert_eq!(order.content, "Do this task");
    assert!(!order.id.is_empty());
}

#[test]
fn sitrep_creation() {
    let sitrep = SitRep::new("agent-1", StatusEnum::Active);
    assert_eq!(sitrep.from_agent, "agent-1");
    assert_eq!(sitrep.status, StatusEnum::Active);
    assert_eq!(sitrep.progress, 0.0);
    assert!(sitrep.to_agent.is_none());
}

#[test]
fn casrep_creation() {
    let casrep = CasRep::new("agent-1", "exception", "KeyError: missing");
    assert_eq!(casrep.agent_id, "agent-1");
    assert_eq!(casrep.failure_type, "exception");
    assert_eq!(casrep.error_message, "KeyError: missing");
}
