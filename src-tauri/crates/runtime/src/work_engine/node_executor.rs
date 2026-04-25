use axagent_core::workflow_types::{WorkflowNode, AtomicSkillNode, AgentNode, LLMNode};

use super::execution_state::ExecutionState;

/// Node execution result
#[derive(Debug, Clone)]
pub struct NodeExecutionResult {
    pub output: serde_json::Value,
    pub output_var: Option<String>,
}

/// Node executor that dispatches execution based on node type
pub struct NodeExecutor;

impl NodeExecutor {
    /// Execute a workflow node based on its type.
    ///
    /// For AtomicSkill and Agent nodes, this dispatches to the appropriate executor.
    /// For flow control nodes (Condition, Parallel, Loop, etc.), the DAG engine handles them.
    pub async fn execute_node(
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeExecutionResult, NodeError> {
        match node {
            WorkflowNode::AtomicSkill(n) => Self::execute_atomic_skill(n, context).await,
            WorkflowNode::Agent(n) => Self::execute_agent(n, context).await,
            WorkflowNode::Llm(n) => Self::execute_llm(n, context).await,
            // Flow control nodes are handled by the DAG engine directly
            _ => Err(NodeError::unsupported_node_type(format!(
                "Node type {:?} is handled by the DAG engine",
                node_type_name(node)
            ))),
        }
    }

    /// Execute an AtomicSkill node
    async fn execute_atomic_skill(
        node: &AtomicSkillNode,
        context: &ExecutionState,
    ) -> Result<NodeExecutionResult, NodeError> {
        // Map input parameters from workflow variables
        let mut input = serde_json::Map::new();
        for (param_name, var_path) in &node.config.input_mapping {
            if let Some(value) = context.get_variable(var_path) {
                input.insert(param_name.clone(), value.clone());
            }
        }

        // The actual execution is delegated to AtomicSkillExecutor
        // which is called from the Tauri command layer that has access to AppState.
        // Here we just prepare the input and return a marker that the
        // command layer should invoke the skill.
        Ok(NodeExecutionResult {
            output: serde_json::Value::Object(input),
            output_var: Some(node.config.output_var.clone()),
        })
    }

    /// Execute an Agent node
    async fn execute_agent(
        node: &AgentNode,
        _context: &ExecutionState,
    ) -> Result<NodeExecutionResult, NodeError> {
        // Agent execution is delegated to the Agent system
        // via the Tauri command layer
        Ok(NodeExecutionResult {
            output: serde_json::json!({
                "role": node.config.role.as_str(),
                "system_prompt": node.config.system_prompt,
                "output_var": node.config.output_var,
            }),
            output_var: Some(node.config.output_var.clone()),
        })
    }

    /// Execute an LLM node
    async fn execute_llm(
        node: &LLMNode,
        _context: &ExecutionState,
    ) -> Result<NodeExecutionResult, NodeError> {
        // LLM execution is delegated to the provider system
        Ok(NodeExecutionResult {
            output: serde_json::json!({
                "model": node.config.model,
                "prompt": node.config.prompt,
            }),
            output_var: None,
        })
    }
}

/// Node execution error
#[derive(Debug, Clone)]
pub struct NodeError {
    pub error_type: String,
    pub message: String,
}

impl NodeError {
    pub fn unsupported_node_type(msg: String) -> Self {
        Self {
            error_type: "unsupported_node_type".to_string(),
            message: msg,
        }
    }
}

impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeError({}): {}", self.error_type, self.message)
    }
}

impl std::error::Error for NodeError {}

fn node_type_name(node: &WorkflowNode) -> &'static str {
    match node {
        WorkflowNode::Trigger(_) => "trigger",
        WorkflowNode::AtomicSkill(_) => "atomic_skill",
        WorkflowNode::Agent(_) => "agent",
        WorkflowNode::Llm(_) => "llm",
        WorkflowNode::Condition(_) => "condition",
        WorkflowNode::Parallel(_) => "parallel",
        WorkflowNode::Loop(_) => "loop",
        WorkflowNode::Merge(_) => "merge",
        WorkflowNode::Delay(_) => "delay",
        WorkflowNode::SubWorkflow(_) => "sub_workflow",
        WorkflowNode::DocumentParser(_) => "document_parser",
        WorkflowNode::VectorRetrieve(_) => "vector_retrieve",
        WorkflowNode::End(_) => "end",
        WorkflowNode::Tool(_) => "tool",
        WorkflowNode::Code(_) => "code",
    }
}
