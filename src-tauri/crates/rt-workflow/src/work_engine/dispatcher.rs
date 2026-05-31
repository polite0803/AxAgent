//! 节点调度器 —— 根据 WorkflowNode 类型路由到对应执行器。
//!
//! NodeDispatcher::new() 只注册无外部依赖的执行器。
//! 需要 db/master_key 的执行器（Llm、Agent、Condition）由 WorkEngine::new() 注册覆盖。

use std::collections::HashMap;

use axagent_core::workflow_types::WorkflowNode;

use super::execution_state::ExecutionState;
use super::executors::{
    CodeExecutor, DelayExecutor, DocumentParserExecutor, EndExecutor, FallbackExecutor,
    LoopExecutor, MergeExecutor, ParallelExecutor, SubWorkflowExecutor, ToolExecutor,
    TriggerExecutor, ValidationExecutor, VectorRetrieveExecutor,
};
use super::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput, node_type_name};

pub struct NodeDispatcher {
    executors: HashMap<&'static str, Box<dyn NodeExecutorTrait>>,
}

impl Default for NodeDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeDispatcher {
    pub fn new() -> Self {
        let mut dispatcher = Self {
            executors: HashMap::new(),
        };
        dispatcher.register(TriggerExecutor::new());
        dispatcher.register(ParallelExecutor::new());
        dispatcher.register(LoopExecutor::new());
        dispatcher.register(MergeExecutor::new());
        dispatcher.register(DelayExecutor::new());
        dispatcher.register(SubWorkflowExecutor::new());
        dispatcher.register(DocumentParserExecutor::new());
        dispatcher.register(VectorRetrieveExecutor::new());
        dispatcher.register(EndExecutor::new());
        dispatcher.register(ValidationExecutor::new());
        dispatcher.register(ToolExecutor::new());
        dispatcher.register(CodeExecutor::new());
        dispatcher.register(FallbackExecutor::new());
        dispatcher
    }

    pub fn register<E: NodeExecutorTrait + 'static>(&mut self, executor: E) {
        self.executors
            .insert(executor.node_type(), Box::new(executor));
    }

    pub async fn dispatch(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let node_type = node_type_name(node);
        let executor = self.executors.get(node_type).unwrap_or_else(|| {
            self.executors
                .get("fallback")
                .expect("FallbackExecutor must be registered")
        });
        tracing::info!(
            node_id = %node.base_id(),
            node_type,
            executor_type = %executor.node_type(),
            "dispatch"
        );
        executor.execute(node, context).await
    }

    pub fn get_executor(&self, node_type: &str) -> Option<&dyn NodeExecutorTrait> {
        self.executors.get(node_type).map(|e| e.as_ref())
    }

    pub fn registered_types(&self) -> Vec<&'static str> {
        self.executors.keys().copied().collect()
    }
}
