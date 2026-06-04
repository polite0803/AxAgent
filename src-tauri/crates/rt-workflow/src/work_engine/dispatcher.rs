use std::collections::HashMap;
use std::sync::Arc;

use axagent_core::workflow_types::WorkflowNode;

use super::execution_state::ExecutionState;
use super::executors::{
    AggregatorExecutor, ApprovalExecutor, CodeExecutor, DataTransformerExecutor,
    DatabaseQueryExecutor, DebateExecutor, DelayExecutor, DocumentParserExecutor, EmailExecutor,
    EndExecutor, FallbackExecutor, FileOperationExecutor, HttpRequestExecutor,
    LlmClassifierExecutor, LoggingExecutor, LoopExecutor, MergeExecutor, NotificationExecutor,
    ParallelExecutor, SubWorkflowExecutor, SwitchExecutor, ToolExecutor, TriggerExecutor,
    ValidationExecutor, VectorRetrieveExecutor, WebhookSendExecutor,
};
use super::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput, node_type_name};

pub struct NodeDispatcher {
    executors: HashMap<&'static str, Arc<dyn NodeExecutorTrait>>,
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
        dispatcher.register(DebateExecutor::new());
        dispatcher.register(FallbackExecutor::new());
        dispatcher.register(HttpRequestExecutor::new());
        dispatcher.register(SwitchExecutor::new());
        dispatcher.register(DatabaseQueryExecutor::new());
        dispatcher.register(NotificationExecutor::new());
        dispatcher.register(ApprovalExecutor::new());
        dispatcher.register(FileOperationExecutor::new());
        dispatcher.register(DataTransformerExecutor::new());
        dispatcher.register(WebhookSendExecutor::new());
        dispatcher.register(LoggingExecutor::new());
        dispatcher.register(LlmClassifierExecutor::default());
        dispatcher.register(AggregatorExecutor::new());
        dispatcher.register(EmailExecutor::new());
        dispatcher
    }

    /// 注册 executor。若同名 executor 已存在，记录 warn 日志
    /// （覆盖仅用于共享 Arc 的"重置"场景，调用方应使用 `register_arc` 共享同一实例）。
    pub fn register<E: NodeExecutorTrait + 'static>(&mut self, executor: E) {
        self.register_arc(Arc::new(executor));
    }

    /// 注册共享实例（与 WorkEngine.agent_executor 配合使用）。
    /// 同名已存在时**直接覆盖**（不打印 warn，因为是同一实例热更新）。
    /// 真正的"防呆"是：业务代码不要再调用 register(E) 重新注册 agent
    /// executor；统一通过 WorkEngine.agent_executor 字段访问并修改状态。
    pub fn register_arc(&mut self, executor: Arc<dyn NodeExecutorTrait>) {
        let key = executor.node_type();
        if self.executors.contains_key(key)
            && !Arc::ptr_eq(self.executors.get(key).expect("checked above"), &executor)
        {
            tracing::warn!(
                node_type = key,
                "dispatcher.register_arc: 覆盖已存在的不同实例（请检查是否还有遗留的重复 register 调用）"
            );
        }
        self.executors.insert(key, executor);
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
