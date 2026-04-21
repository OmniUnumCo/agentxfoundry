//! Rational Reserve — military-hierarchy AI swarm orchestration.
//!
//! Deterministic, compiled Rust agents. The core library (`rr`) is framework-free:
//! pure business logic with dependency injection via traits.
//!
//! # Example
//!
//! ```
//! use rr::{RankEnum, MOSEnum, StatusEnum, Order, OrderType};
//!
//! let order = Order::new("gen-1", "spc-1", OrderType::DirectTask, "Secure the perimeter");
//! assert_eq!(order.from_agent, "gen-1");
//! assert_eq!(order.to_agent, "spc-1");
//! ```

pub mod models;
pub mod traits;
pub mod agent;
pub mod factory;
pub mod c2_router;
pub mod doctrine;
pub mod config;
pub mod logging;
pub mod persistence;
pub mod daemon;
pub mod broker;

pub use agent::{tier_for, tier_for_civilian, tier_for_rank, RRAgent};
pub use broker::ContractBroker;
pub use c2_router::C2Router;
pub use factory::AgentFactory;
pub use models::{
    AgentClass, AgentRecord, AnyAgent, CasRep, CivilianRecord, CivilianTier, ContractOutcome,
    ContractRecord, ContractRequest, ContractState, Mission, MOSEnum, MOSCategory, Order,
    OrderType, PriorityEnum, RankCategory, RankEnum, SitRep, StatusEnum, Swarm, Task, Unit,
    UnitType, AAR,
};
pub use traits::{Clock, DoctrineLoader, EventLogger, LLMClient, StateStore, SystemClock};
