//! Workflow Engine — DAG executor, agent roles, work engine, orchestration.

pub mod agent_roles;
pub mod engine_bridge;
pub mod general_engine;
pub mod work_engine;
pub mod workflow_engine;

pub use agent_roles::AgentRole;
pub use engine_bridge::{EngineBridge, EngineId, EngineMessage};
// 节点类型统一为 axagent_core::workflow_types::WorkflowNode（28 种）。
// 运行时状态类型定义在 workflow_engine 模块中。
pub use workflow_engine::{NodeRuntimeState, NodeStatus, Workflow, WorkflowError, WorkflowStatus};
