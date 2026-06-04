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
use super::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code, node_type_name,
};

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

        // ── 业务规则引擎检查（硬约束） ──
        // 在执行之前先检查业务规则。仅对可执行节点类型进行检查。
        if let Some(ref br_engine) = context.business_rule_engine
            && is_business_rule_applicable(node_type)
        {
            let node_input = build_node_input_snapshot(node, context);
            let outcome = br_engine.evaluate(node_type, &node_input);
            use axagent_harness::business_rules::RuleEvaluationOutcome;
            match &outcome {
                RuleEvaluationOutcome::Violation {
                    rule_name,
                    action,
                    reason,
                    ..
                } => {
                    tracing::warn!(
                        node_id = %node.base_id(),
                        node_type,
                        rule = rule_name,
                        reason,
                        "业务规则违规 — 阻断执行"
                    );
                    match action {
                        axagent_harness::business_rules::RuleAction::Block(msg) => {
                            return Err(NodeError::exec_failed(
                                error_code::VALIDATION_FAILED,
                                format!("[业务规则] {msg}: {reason}"),
                            ));
                        },
                        axagent_harness::business_rules::RuleAction::Warn(msg) => {
                            tracing::warn!(
                                node_id = %node.base_id(),
                                rule = rule_name,
                                msg,
                                reason,
                                "业务规则警告 — 继续执行"
                            );
                        },
                        axagent_harness::business_rules::RuleAction::RequireApproval(msg) => {
                            return Err(NodeError::exec_failed(
                                error_code::VALIDATION_FAILED,
                                format!("[业务规则-需审批] {msg}: {reason}"),
                            ));
                        },
                    }
                },
                RuleEvaluationOutcome::RequiresApproval {
                    rule_name, reason, ..
                } => {
                    tracing::warn!(
                        node_id = %node.base_id(),
                        node_type,
                        rule = rule_name,
                        reason,
                        "业务规则 — 需人工审批"
                    );
                    return Err(NodeError::exec_failed(
                        error_code::VALIDATION_FAILED,
                        format!("[业务规则-需审批] 规则 '{rule_name}': {reason}"),
                    ));
                },
                RuleEvaluationOutcome::Pass => {},
            }
        }

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

// ── 业务规则辅助函数 ──

/// 判断该节点类型是否适用于业务规则检查。
/// 主要对执行"外部操作"的节点类型做检查，纯内部节点跳过。
fn is_business_rule_applicable(node_type: &str) -> bool {
    matches!(
        node_type,
        "agent"
            | "tool"
            | "httpRequest"
            | "webhookSend"
            | "fileOperation"
            | "databaseQuery"
            | "code"
            | "notification"
            | "email"
            | "llm"
            | "llmClassifier"
    )
}

/// 构建节点输入快照，供业务规则评估使用。
/// 提取节点配置中的关键字段（工具名、URL、操作类型、金额等）。
fn build_node_input_snapshot(node: &WorkflowNode, context: &ExecutionState) -> serde_json::Value {
    use axagent_core::workflow_types::WorkflowNode;
    let mut map = serde_json::Map::new();
    map.insert("node_id".to_string(), serde_json::json!(node.base_id()));
    map.insert("node_title".to_string(), serde_json::json!(node.base_title()));

    // 从 context.variables 提取工具调用相关的"input"字段
    if let Some(input_val) = context.variables.get("input")
        && let Some(obj) = input_val.as_object()
    {
        for (k, v) in obj {
            map.insert(k.clone(), v.clone());
        }
    }

    // 根据节点类型提取特定字段
    match node {
        WorkflowNode::Tool(tn) => {
            map.insert("tool_name".to_string(), serde_json::json!(tn.config.tool_name));
            // 将 input_mapping 的值也合并进来
            for (k, v) in &tn.config.input_mapping {
                if let Some(val) = context.variables.get(v) {
                    map.insert(k.clone(), val.clone());
                }
            }
        },
        WorkflowNode::HttpRequest(hn) => {
            map.insert("url".to_string(), serde_json::json!(hn.config.url));
            map.insert("method".to_string(), serde_json::json!(hn.config.method));
            if let Some(body) = &hn.config.body {
                map.insert("body".to_string(), serde_json::json!(body));
            }
        },
        WorkflowNode::FileOperation(fn_node) => {
            map.insert("operation".to_string(), serde_json::json!(fn_node.config.operation));
            map.insert("file_path".to_string(), serde_json::json!(fn_node.config.file_path));
        },
        WorkflowNode::WebhookSend(wn) => {
            map.insert("url".to_string(), serde_json::json!(wn.config.url));
        },
        WorkflowNode::DatabaseQuery(dn) => {
            map.insert("query".to_string(), serde_json::json!(dn.config.query));
        },
        _ => {},
    }

    serde_json::Value::Object(map)
}
