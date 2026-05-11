//! Workflow Engine — DAG executor, agent roles, work engine, orchestration.

pub mod agent_orchestrator;
pub mod agent_roles;
pub mod engine_bridge;
pub mod general_engine;
pub mod work_engine;
pub mod workflow_engine;

pub use agent_roles::AgentRole;
pub use engine_bridge::{EngineBridge, EngineId, EngineMessage};
pub use workflow_engine::{
    CircuitBreaker, OnStepFailure, RetryPolicy, SessionCallback, StepStatus, WorkflowEngine,
    WorkflowRunner, WorkflowStep, wrap_executor_with_callback,
};
