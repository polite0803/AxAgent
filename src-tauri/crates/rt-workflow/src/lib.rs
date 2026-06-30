// SPDX-License-Identifier: AGPL-3.0-only

//! Workflow Engine — DAG executor, agent roles, work engine, orchestration.

pub mod agent_roles;
pub mod work_engine;
pub mod workflow_engine;

pub use agent_roles::AgentRole;
// 节点类型统一为 axagent_core::workflow_types::WorkflowNode（28 种）。
// 运行时状态类型定义在 workflow_engine 模块中。
pub use workflow_engine::{NodeRuntimeState, NodeStatus, Workflow, WorkflowError, WorkflowStatus};
