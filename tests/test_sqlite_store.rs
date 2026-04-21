//! Tests for `rr::persistence::SqliteStateStore`.
//!
//! Covers:
//!   - Schema initialization is idempotent
//!   - Event append persists with doctrine_version pinned
//!   - Projection upsert round-trips through the DB
//!   - list_agents filters work

use rr::models::*;
use rr::persistence::SqliteStateStore;
use rr::StateStore;
use std::path::PathBuf;
use std::sync::Arc;

fn make_store(db_path: PathBuf) -> Arc<SqliteStateStore> {
    Arc::new(
        SqliteStateStore::open(
            &db_path,
            Box::new(|| "0.0.0-test".to_string()),
        )
        .expect("failed to open store"),
    )
}

#[test]
fn schema_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("rr.db");
    let _s1 = make_store(db_path.clone());
    // Opening again should not fail
    let _s2 = make_store(db_path);
}

#[test]
fn append_event_returns_increasing_ids() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_rr.db");
    let store = make_store(db_path);

    let first = store
        .append_event(
            "AGENT_SPAWNED",
            &serde_json::json!({"agent_id": "a1"}),
            None,
            Some("a1"),
        )
        .unwrap();
    let second = store
        .append_event(
            "AGENT_SPAWNED",
            &serde_json::json!({"agent_id": "a2"}),
            None,
            Some("a2"),
        )
        .unwrap();

    assert_eq!(second, first + 1);
    assert_eq!(store.event_count().unwrap(), 2);
}

#[test]
fn agent_upsert_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_rr.db");
    let store = make_store(db_path);

    let record = AgentRecord {
        id: "agent-1".into(),
        name: "Alpha".into(),
        rank: RankEnum::CPT,
        mos: MOSEnum::CombatEngineer,
        ..AgentRecord::default()
    };
    store.save_agent(&record).unwrap();

    let loaded = store.load_agent("agent-1").unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.name, "Alpha");
    assert_eq!(loaded.rank, RankEnum::CPT);
    assert_eq!(loaded.mos, MOSEnum::CombatEngineer);
}

#[test]
fn swarm_upsert_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_rr.db");
    let store = make_store(db_path);

    let swarm = Swarm {
        id: "swarm-1".into(),
        name: "Alpha".into(),
        general_id: "gen-1".into(),
        ..Swarm::default()
    };
    store.save_swarm(&swarm).unwrap();

    let loaded = store.load_swarm("swarm-1").unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().name, "Alpha");
}

#[test]
fn list_agents_filters() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_rr.db");
    let store = make_store(db_path);

    let a1 = AgentRecord {
        id: "a1".into(),
        name: "A1".into(),
        rank: RankEnum::CPT,
        mos: MOSEnum::CombatEngineer,
        swarm_id: Some("s1".into()),
        ..AgentRecord::default()
    };
    let a2 = AgentRecord {
        id: "a2".into(),
        name: "A2".into(),
        rank: RankEnum::SPC,
        mos: MOSEnum::Infantry,
        swarm_id: Some("s1".into()),
        ..AgentRecord::default()
    };
    let a3 = AgentRecord {
        id: "a3".into(),
        name: "A3".into(),
        rank: RankEnum::SPC,
        mos: MOSEnum::Infantry,
        swarm_id: Some("s2".into()),
        ..AgentRecord::default()
    };
    for rec in [&a1, &a2, &a3] {
        store.save_agent(rec).unwrap();
    }

    // Filter by swarm
    let in_s1 = store.list_agents(Some("s1"), None, None, None).unwrap();
    let names: std::collections::HashSet<_> = in_s1.iter().map(|a| &a.name).collect();
    assert!(names.contains(&"A1".to_string()));
    assert!(names.contains(&"A2".to_string()));

    // Filter by rank
    let specs = store.list_agents(None, None, Some(RankEnum::SPC), None).unwrap();
    let names: std::collections::HashSet<_> = specs.iter().map(|a| &a.name).collect();
    assert!(names.contains(&"A2".to_string()));
    assert!(names.contains(&"A3".to_string()));

    // Filter by MOS
    let infantry = store.list_agents(None, None, None, Some(MOSEnum::Infantry)).unwrap();
    let names: std::collections::HashSet<_> = infantry.iter().map(|a| &a.name).collect();
    assert!(names.contains(&"A2".to_string()));
    assert!(names.contains(&"A3".to_string()));
}

#[test]
fn load_unknown_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_rr.db");
    let store = make_store(db_path);

    assert!(store.load_agent("no-such-id").unwrap().is_none());
    assert!(store.load_swarm("no-such-swarm").unwrap().is_none());
}

#[test]
fn agent_roundtrip_with_subordinates() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_rr.db");
    let store = make_store(db_path);

    let record = AgentRecord {
        id: "cmdr-1".into(),
        name: "Commander".into(),
        rank: RankEnum::SGT,
        mos: MOSEnum::Infantry,
        subordinates: vec!["sub-1".into(), "sub-2".into()],
        swarm_id: Some("s1".into()),
        status: StatusEnum::Active,
        missions_completed: 5,
        performance_score: 0.92,
        ..AgentRecord::default()
    };
    store.save_agent(&record).unwrap();

    let loaded = store.load_agent("cmdr-1").unwrap().unwrap();
    assert_eq!(loaded.subordinates, vec!["sub-1", "sub-2"]);
    assert_eq!(loaded.missions_completed, 5);
    assert!((loaded.performance_score - 0.92).abs() < f64::EPSILON);
    assert_eq!(loaded.status, StatusEnum::Active);
    assert_eq!(loaded.swarm_id, Some("s1".into()));
}

#[test]
fn list_swarms() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_rr.db");
    let store = make_store(db_path);

    let s1 = Swarm {
        id: "s1".into(),
        name: "Swarm One".into(),
        general_id: "gen-1".into(),
        ..Swarm::default()
    };
    let s2 = Swarm {
        id: "s2".into(),
        name: "Swarm Two".into(),
        general_id: "gen-2".into(),
        ..Swarm::default()
    };
    store.save_swarm(&s1).unwrap();
    store.save_swarm(&s2).unwrap();

    let swarms = store.list_swarms(None).unwrap();
    assert_eq!(swarms.len(), 2);
    let names: std::collections::HashSet<_> = swarms.iter().map(|s| &s.name).collect();
    assert!(names.contains(&"Swarm One".to_string()));
    assert!(names.contains(&"Swarm Two".to_string()));
}

#[test]
fn swarm_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_rr.db");
    let store = make_store(db_path);

    let swarm = Swarm {
        id: "swarm-1".into(),
        name: "Test Swarm".into(),
        general_id: "gen-1".into(),
        agents: vec!["a1".into(), "a2".into(), "a3".into()],
        units: vec!["u1".into()],
        status: StatusEnum::Active,
        ..Swarm::default()
    };
    store.save_swarm(&swarm).unwrap();

    let loaded = store.load_swarm("swarm-1").unwrap().unwrap();
    assert_eq!(loaded.name, "Test Swarm");
    assert_eq!(loaded.agents, vec!["a1", "a2", "a3"]);
    assert_eq!(loaded.units, vec!["u1"]);
    assert_eq!(loaded.status, StatusEnum::Active);
}
