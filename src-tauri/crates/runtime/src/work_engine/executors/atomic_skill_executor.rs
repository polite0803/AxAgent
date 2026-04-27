use async_trait::async_trait;
use axagent_core::workflow_types::{AtomicSkillNode, WorkflowNode};

use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait};
use crate::work_engine::{ExecutionState, NodeOutput};

pub struct AtomicSkillExecutor;

impl AtomicSkillExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AtomicSkillExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for AtomicSkillExecutor {
    fn node_type(&self) -> &'static str {
        "atomic_skill"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        match node {
            WorkflowNode::AtomicSkill(skill_node) => {
                Self::execute_atomic_skill(skill_node, context).await
            }
            _ => Err(NodeError::UnsupportedNodeType(format!(
                "Expected AtomicSkill node, got {:?}",
                node
            ))),
        }
    }
}

impl AtomicSkillExecutor {
    async fn execute_atomic_skill(
        node: &AtomicSkillNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let mut input = serde_json::Map::new();
        for (param_name, var_path) in &node.config.input_mapping {
            if let Some(value) = context.get_variable(var_path) {
                input.insert(param_name.clone(), value.clone());
            }
        }

        Ok(NodeOutput {
            output: serde_json::Value::Object(input),
            output_var: Some(node.config.output_var.clone()),
        })
    }
}
