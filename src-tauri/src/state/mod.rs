//! Domain-specific state containers, decomposed from the monolithic `AppState`.
//!
//! This module is the result of Phase 3 P1 Task 3.1. It defines six focused
//! state groups (`InfraState`, `GatewayState`, `TaskState`, `AgentState`,
//! `MemoryState`, `SkillState`) that mirror the responsibilities of the
//! original `AppState` and expose them as composable sub-states.
//!
//! ## Backwards compatibility
//!
//! `AppState` continues to expose all of its original `pub` fields so that
//! the 200+ `commands/*` call-sites keep working unchanged. The new domain
//! sub-states live alongside as additional `pub` fields and are constructed
//! at app start-up with `Arc`/`tokio::Mutex` references that share ownership
//! of the same data.
//!
//! This is a soft split: it does not move any field, it only introduces
//! grouped views. Future refactors (Phase 3 P2+) can migrate call-sites to
//! use the new sub-states and eventually remove the legacy top-level fields.

pub mod agent;
pub mod gateway;
pub mod infra;
pub mod memory;
pub mod skill;
pub mod task;

pub use agent::AgentState;
pub use gateway::GatewayState;
pub use infra::InfraState;
pub use memory::MemoryState;
pub use skill::{BrowserClientField, SandboxExecutorField, SkillState};
pub use task::TaskState;
