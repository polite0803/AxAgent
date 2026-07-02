// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;

use axagent_core::workflow_types::WorkflowNode;

use crate::expression_engine::{ExpressionContext, resolve_value_templates};

use super::execution_state::ExecutionState;
use super::executors::{
    AggregatorExecutor, ApprovalExecutor, CodeExecutor, DataTransformerExecutor,
    DatabaseQueryExecutor, DebateExecutor, DelayExecutor, DocumentParserExecutor, EmailExecutor,
    EndExecutor, FallbackExecutor, FileOperationExecutor, HttpRequestExecutor, LoggingExecutor,
    LoopExecutor, MergeExecutor, NotificationExecutor, ParallelExecutor, StorageExecutor,
    SubWorkflowExecutor, SwitchExecutor, ToolExecutor, TriggerExecutor, ValidationExecutor,
    VectorRetrieveExecutor, WebhookSendExecutor,
};
use super::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code, node_type_name,
};

/// P0-2: 全部改用 `tokio::sync::RwLock`,并删除 `#[allow(clippy::await_holding_lock)]` 标记。
/// `register_arc`/`register`/`get_executor`/`registered_types` 改 async,调用方通过
/// `WorkEngine::init_dispatcher` 在 tokio runtime 内完成初始化。
pub struct NodeDispatcher {
    executors: Arc<tokio::sync::RwLock<HashMap<&'static str, Arc<dyn NodeExecutorTrait>>>>,
}

impl Default for NodeDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeDispatcher {
    pub fn new() -> Self {
        let dispatcher = Self {
            executors: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };
        dispatcher
    }

    /// 一次性注册所有内置 executor（异步版本）。
    /// 必须在 tokio runtime 中调用（与 `WorkEngine::init_dispatcher` 配套）。
    pub async fn init_builtin(&self) {
        self.register(TriggerExecutor::new()).await;
        self.register(ParallelExecutor::new()).await;
        self.register(LoopExecutor::new()).await;
        self.register(MergeExecutor::new()).await;
        self.register(DelayExecutor::new()).await;
        self.register(SubWorkflowExecutor::new()).await;
        self.register(DocumentParserExecutor::new()).await;
        self.register(VectorRetrieveExecutor::new()).await;
        self.register(EndExecutor::new()).await;
        self.register(ValidationExecutor::new()).await;
        self.register(ToolExecutor::new()).await;
        self.register(CodeExecutor::new()).await;
        self.register(DebateExecutor::new()).await;
        self.register(FallbackExecutor::new()).await;
        self.register(HttpRequestExecutor::new()).await;
        self.register(SwitchExecutor::new()).await;
        self.register(DatabaseQueryExecutor::new()).await;
        self.register(NotificationExecutor::new()).await;
        self.register(ApprovalExecutor::new()).await;
        self.register(FileOperationExecutor::new()).await;
        self.register(DataTransformerExecutor::new()).await;
        self.register(WebhookSendExecutor::new()).await;
        self.register(LoggingExecutor::new()).await;
        self.register(StorageExecutor::new()).await;
        // LlmClassifierExecutor 由 WorkEngine::init_dispatcher 配置并注册
        self.register(AggregatorExecutor::new()).await;
        self.register(EmailExecutor::new()).await;
    }

    /// 注册 executor。若同名 executor 已存在，记录 warn 日志
    /// （覆盖仅用于共享 Arc 的"重置"场景，调用方应使用 `register_arc` 共享同一实例）。
    pub async fn register<E: NodeExecutorTrait + 'static>(&self, executor: E) {
        self.register_arc(Arc::new(executor)).await;
    }

    /// 注册共享实例（与 WorkEngine.agent_executor 配合使用）。
    /// 同名已存在时**直接覆盖**（不打印 warn，因为是同一实例热更新）。
    /// 真正的"防呆"是：业务代码不要再调用 register(E) 重新注册 agent
    /// executor；统一通过 WorkEngine.agent_executor 字段访问并修改状态。
    pub async fn register_arc(&self, executor: Arc<dyn NodeExecutorTrait>) {
        let key = executor.node_type();
        let mut map = self.executors.write().await;
        if map.contains_key(key) && !Arc::ptr_eq(map.get(key).expect("checked above"), &executor) {
            tracing::warn!(
                node_type = key,
                "dispatcher.register_arc: 覆盖已存在的不同实例（请检查是否还有遗留的重复 register 调用）"
            );
        }
        map.insert(key, executor);
    }

    /// 公开注册 API（供外部 crate 注册自定义执行器）。
    /// 与 register_arc 等价，仅命名上明确表示"外部注册"语义。
    pub async fn register_external(&self, executor: Arc<dyn NodeExecutorTrait>) {
        self.register_arc(executor).await;
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

        let executor = {
            let map = self.executors.read().await;
            map.get(node_type).cloned().unwrap_or_else(|| {
                map.get("fallback")
                    .cloned()
                    .expect("FallbackExecutor must be registered")
            })
        };
        tracing::info!(
            node_id = %node.base_id(),
            node_type,
            executor_type = %executor.node_type(),
            "dispatch"
        );
        // ── 表达式模板解析 ──
        // 对节点配置 JSON 递归扫描 {{ expression }} 模板并求值，
        // 注入 $vars / $node / $input / $now / $env 到求值上下文。
        // 任何环节失败均优雅降级回原始节点。
        let resolved_node = {
            let expr_ctx = ExpressionContext {
                variables: context.variables.clone(),
                node_outputs: context.node_outputs.clone(),
                input_params: context.input_params.clone(),
                env: std::env::vars().collect(),
            };
            match serde_json::to_value(node) {
                Ok(node_json) => match resolve_value_templates(&node_json, &expr_ctx) {
                    Ok(resolved_json) => {
                        match serde_json::from_value::<WorkflowNode>(resolved_json) {
                            Ok(n) => n,
                            Err(e) => {
                                tracing::warn!(
                                    node_id = %node.base_id(),
                                    error = %e,
                                    "表达式模板解析后反序列化失败，回退到原始节点"
                                );
                                node.clone()
                            },
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            node_id = %node.base_id(),
                            error = %e,
                            "表达式模板解析失败，回退到原始节点"
                        );
                        node.clone()
                    },
                },
                Err(e) => {
                    tracing::warn!(
                        node_id = %node.base_id(),
                        error = %e,
                        "节点序列化失败，跳过模板解析"
                    );
                    node.clone()
                },
            }
        };
        executor.execute(&resolved_node, context).await
    }

    pub async fn get_executor(&self, node_type: &str) -> Option<Arc<dyn NodeExecutorTrait>> {
        self.executors.read().await.get(node_type).cloned()
    }

    pub async fn registered_types(&self) -> Vec<&'static str> {
        self.executors.read().await.keys().copied().collect()
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

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple executor that just returns the node id in its output.
    struct TestExecutor {
        node_type_str: &'static str,
    }

    impl TestExecutor {
        fn new(key: &'static str) -> Self {
            Self { node_type_str: key }
        }
    }

    #[async_trait::async_trait]
    impl NodeExecutorTrait for TestExecutor {
        fn node_type(&self) -> &'static str {
            self.node_type_str
        }
        async fn execute(
            &self,
            _node: &WorkflowNode,
            _context: &super::ExecutionState,
        ) -> Result<NodeOutput, NodeError> {
            Ok(NodeOutput {
                output: serde_json::json!({"node_type": self.node_type_str}),
                output_var: None,
            })
        }
    }

    fn make_test_exec_state() -> ExecutionState {
        ExecutionState::new("test_exec".into(), "test_wf".into(), serde_json::json!({}))
    }

    #[test]
    fn register_and_lookup() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let disp = NodeDispatcher::new();
        rt.block_on(disp.register(TestExecutor::new("testExec")));
        assert!(rt.block_on(disp.get_executor("testExec")).is_some());
        assert!(rt.block_on(disp.get_executor("nonexistent")).is_none());
    }

    #[test]
    fn registered_types_collects_keys() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let disp = NodeDispatcher::new();
        rt.block_on(disp.register(TestExecutor::new("customA")));
        rt.block_on(disp.register(TestExecutor::new("customB")));
        let types = rt.block_on(disp.registered_types());
        assert!(types.contains(&"customA"));
        assert!(types.contains(&"customB"));
        // The built-in executors are not registered in this empty dispatcher
        // (init_builtin must be called explicitly).
    }

    #[tokio::test]
    async fn dispatch_to_registered_executor() {
        let disp = NodeDispatcher::new();
        disp.register(TestExecutor::new("tool")).await;
        let node = make_tool_node("n1", true);
        let ctx = make_test_exec_state();
        let result = disp.dispatch(&node, &ctx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().output["node_type"], "tool");
    }

    #[tokio::test]
    async fn fallback_executor_when_not_found() {
        let disp = NodeDispatcher::new();
        disp.register(TestExecutor::new("fallback")).await;
        let node = make_tool_node("n2", true);
        let ctx = make_test_exec_state();
        let result = disp.dispatch(&node, &ctx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().output["node_type"], "fallback");
    }

    // Re-use helper from dag_store tests — define locally
    fn make_tool_node(id: &str, enabled: bool) -> WorkflowNode {
        use axagent_harness::workflow_types::{
            Position, RetryConfig, ToolNode, ToolNodeConfig, WorkflowNodeBase,
        };
        WorkflowNode::Tool(ToolNode {
            base: WorkflowNodeBase {
                id: id.to_string(),
                title: format!("Tool {id}"),
                description: None,
                position: Position { x: 0.0, y: 0.0 },
                retry: RetryConfig::default(),
                timeout: None,
                enabled,
                parent_id: None,
                compensation: None,
                continue_on_fail: false,
            },
            config: ToolNodeConfig {
                tool_name: format!("tool_{id}"),
                input_mapping: std::collections::HashMap::new(),
                output_var: format!("out_{id}"),
            },
        })
    }
}
