// SPDX-License-Identifier: AGPL-3.0-only

//! Workflow Engine — DAG executor, agent roles, work engine, orchestration.

pub mod agent_roles;
pub mod business_rules;
pub mod expression_engine;
pub mod trigger;
pub mod work_engine;
pub mod workflow_engine;
pub mod yaml_io;

pub use agent_roles::{FileRoleRegistry, ResolvedRole, RoleConfig, RoleRegistry, resolve, resolve_with_file_registry};
pub use workflow_engine::{NodeRuntimeState, NodeStatus, Workflow, WorkflowError, WorkflowStatus};
pub use yaml_io::{
    WorkflowYamlFormat, WorkflowYamlMetadata, YamlIoError, export_workflow_yaml,
    import_workflow_yaml,
};
