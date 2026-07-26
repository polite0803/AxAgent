// SPDX-License-Identifier: AGPL-3.0-only

//! Workflow Engine — DAG executor, agent roles, work engine, orchestration.

// rt-workflow 使用嵌套 if let 模式（let-chain 需要 Rust 1.88+），
// 与主 bin 保持一致允许 clippy::collapsible_if。
#![allow(clippy::collapsible_if)]

pub mod agent_roles;
pub mod business_rules;
pub mod expression_engine;
pub mod task_contract;
pub mod trigger;
pub mod work_engine;
pub mod workflow_engine;
pub mod yaml_io;

pub use agent_roles::{
    FileRoleRegistry, ResolvedRole, RoleConfig, RoleRegistry, resolve, resolve_with_file_registry,
};
// G12: Task/Pipeline 契约系统
pub use task_contract::{
    AcceptanceCriteria, AcceptanceResult, ContractStatus, HarnessProfile, TaskContract,
};
pub use workflow_engine::{NodeRuntimeState, NodeStatus, Workflow, WorkflowError, WorkflowStatus};
pub use yaml_io::{
    WorkflowYamlFormat, WorkflowYamlMetadata, YamlIoError, export_workflow_yaml,
    import_workflow_yaml,
};
