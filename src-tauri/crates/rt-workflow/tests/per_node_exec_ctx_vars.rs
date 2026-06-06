//! 回归测试：per-node `exec_ctx.variables` 必须包含工作流全局变量。
//!
//! 历史 bug（c04c59a3 / f119f053 / d5f870bb / 9060a620）：engine.rs 在 per-node
//! context 构造时直接 `exec_ctx.variables = deps_results`，把
//! `state.variables`（来自 `RunOptions.variables`）与 `state.input_params`
//! （来自 `RunOptions.input`）整体覆盖。导致下游 tool 节点的
//! `input_mapping` 解析 `stock_code` 等全局变量返回 `None`，
//! 触发 "stock_code不能为空" 错误。
//!
//! 本文件验证 3 层 fallback 合并行为：
//!   1) deps_results  优先（上游节点输出）
//!   2) state.variables 其次（`RunOptions.variables`）
//!   3) state.input_params 兜底（`RunOptions.input`）

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axagent_core::workflow_types::{
    EdgeType, Position, RetryConfig, ToolNode, ToolNodeConfig, TriggerConfig, TriggerNode,
    TriggerType, Variable, WorkflowEdge, WorkflowNode, WorkflowNodeBase,
};
use axagent_harness::registry::ProviderRegistry;
use axagent_harness::{
    Tool, ToolCategory, ToolContext, ToolError, ToolInfo, ToolRegistry, ToolResult,
};

use axagent_rt_workflow::work_engine::{RunOptions, WorkEngine};

// ── 最小 ProviderRegistry 实现 ──────────────────────────────────────────
//
// WorkEngine::new 构造时需要 `Arc<dyn ProviderRegistry>`，本测试不消费
// 任何 provider 能力（只用 tool 节点），所以 `get` 返回 `None` 即可。

struct EmptyProviderRegistry;

impl ProviderRegistry for EmptyProviderRegistry {
    fn get(&self, _provider_type: &str) -> Option<Arc<dyn axagent_harness::ProviderAdapter>> {
        None
    }
}

// ── 捕获工具调用的 ToolRegistry ─────────────────────────────────────────

/// 记录每次 `execute_tool` 收到的 `(tool_name, input)`，供测试断言。
#[derive(Default)]
struct CapturingRegistry {
    captured: tokio::sync::Mutex<Vec<(String, serde_json::Value)>>,
}

impl CapturingRegistry {
    async fn captured(&self) -> Vec<(String, serde_json::Value)> {
        self.captured.lock().await.clone()
    }
}

#[async_trait]
impl ToolRegistry for CapturingRegistry {
    fn get(&self, _name: &str) -> Option<Arc<dyn Tool>> {
        None
    }
    fn find(&self, _name: &str) -> Option<Arc<dyn Tool>> {
        None
    }
    fn list(&self) -> Vec<ToolInfo> {
        Vec::new()
    }
    fn list_by_category(&self, _category: ToolCategory) -> Vec<ToolInfo> {
        Vec::new()
    }
    fn is_disabled(&self, _name: &str) -> bool {
        false
    }
    async fn execute_tool(
        &self,
        name: &str,
        input: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        self.captured.lock().await.push((name.to_string(), input));
        Ok(ToolResult::success("captured"))
    }
}

// ── 节点 / 边构造 helper ────────────────────────────────────────────────

fn make_base(id: &str, title: &str) -> WorkflowNodeBase {
    WorkflowNodeBase {
        id: id.to_string(),
        title: title.to_string(),
        description: None,
        position: Position::default(),
        retry: RetryConfig::default(),
        timeout: Some(30),
        enabled: true,
        parent_id: None,
        compensation: None,
    }
}

fn make_trigger(id: &str) -> WorkflowNode {
    WorkflowNode::Trigger(TriggerNode {
        base: make_base(id, "Trigger"),
        config: TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        },
    })
}

fn make_tool(
    id: &str,
    tool_name: &str,
    input_mapping: HashMap<String, String>,
    output_var: &str,
) -> WorkflowNode {
    WorkflowNode::Tool(ToolNode {
        base: make_base(id, "Tool"),
        config: ToolNodeConfig {
            tool_name: tool_name.to_string(),
            input_mapping,
            output_var: output_var.to_string(),
        },
    })
}

fn make_edge(source: &str, target: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: format!("e__{source}__{target}"),
        source: source.to_string(),
        source_handle: None,
        target: target.to_string(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    }
}

/// 构造一个 WorkEngine，使用 in-memory SQLite 连接。
///
/// run_workflow 内部会调 `axagent_core::repo::workflow_execution::create_workflow_execution`
/// 写 DB 审计记录，所以必须用真实连接（不能是 `DatabaseConnection::default()`，那会返回 Disconnected）。
async fn new_engine() -> WorkEngine {
    let handle = axagent_core::db::create_test_pool()
        .await
        .expect("create_test_pool");
    WorkEngine::new(Arc::new(handle.conn), [0u8; 32], Arc::new(EmptyProviderRegistry))
}

// ── 回归测试 1：state.variables 透传到 per-node exec_ctx ────────────────

#[tokio::test]
async fn per_node_exec_ctx_inherits_global_variables() {
    let engine = new_engine().await;
    let reg = Arc::new(CapturingRegistry::default());
    engine.set_tool_registry(reg.clone()).await;

    // workflow: trigger -> tool1(input_mapping: stock_code <- "stock_code")
    let mut input_mapping = HashMap::new();
    input_mapping.insert("stock_code".to_string(), "stock_code".to_string());
    let wf = engine
        .create_workflow(
            "test_inherits_vars",
            vec![
                make_trigger("trigger"),
                make_tool("tool1", "lookup", input_mapping, "result"),
            ],
            vec![make_edge("trigger", "tool1")],
        )
        .await
        .expect("create_workflow");

    // 关键：调用方通过 options.variables 注入全局 stock_code
    let options = RunOptions::new().with_variables(vec![Variable {
        name: "stock_code".to_string(),
        var_type: "string".to_string(),
        value: serde_json::json!("600519"),
        description: None,
        is_secret: false,
    }]);

    let _ = engine
        .run_workflow(&wf.id, options)
        .await
        .expect("run_workflow");

    // 断言：tool 节点执行时收到了 stock_code = "600519"
    let captured = reg.captured().await;
    assert_eq!(captured.len(), 1, "tool 节点应被调用 1 次，实际 {} 次", captured.len());
    let (name, input) = &captured[0];
    assert_eq!(name, "lookup");
    assert_eq!(
        input.get("stock_code"),
        Some(&serde_json::json!("600519")),
        "修复前：stock_code 会被 deps_results 覆盖，tool 拿到 None；修复后：应透传 options.variables"
    );
}

// ── 回归测试 2：state.input_params 兜底透传 ─────────────────────────────

#[tokio::test]
async fn per_node_exec_ctx_input_params_fallback() {
    let engine = new_engine().await;
    let reg = Arc::new(CapturingRegistry::default());
    engine.set_tool_registry(reg.clone()).await;

    let mut input_mapping = HashMap::new();
    input_mapping.insert("stock_code".to_string(), "stock_code".to_string());
    let wf = engine
        .create_workflow(
            "test_input_fallback",
            vec![
                make_trigger("trigger"),
                make_tool("tool1", "lookup", input_mapping, "result"),
            ],
            vec![make_edge("trigger", "tool1")],
        )
        .await
        .expect("create_workflow");

    // 关键：只设 options.input，不设 options.variables
    // 兼容老代码路径：调用方只透传 start_workflow(input)
    let mut options = RunOptions::new().with_max_concurrent(1);
    options.input = Some(serde_json::json!({"stock_code": "600519"}));

    let _ = engine
        .run_workflow(&wf.id, options)
        .await
        .expect("run_workflow");

    let captured = reg.captured().await;
    assert_eq!(captured.len(), 1);
    let (_name, input) = &captured[0];
    assert_eq!(
        input.get("stock_code"),
        Some(&serde_json::json!("600519")),
        "修复前：仅设 input 时 stock_code 被覆盖为 None；修复后：input_params 这层 fallback 也能透传"
    );
}

// ── 回归测试 3：deps_results 优先于 state.variables ─────────────────────

#[tokio::test]
async fn per_node_exec_ctx_deps_results_take_precedence() {
    let engine = new_engine().await;
    let reg = Arc::new(CapturingRegistry::default());
    engine.set_tool_registry(reg.clone()).await;

    // workflow: trigger -> upstream(id="upstream_value") -> downstream
    //
    // 上游节点的 node_id = "upstream_value"，与全局变量同名（这里是 node_id 命名巧合）。
    // deps_results["upstream_value"] = upstream 节点完整输出（含 `result: <callback>`）
    // 同时 options.variables 注入 value = "from_options"
    //
    // 上游 tool 节点的 input_mapping 是 {value: "value"}，即它从全局变量取 value。
    // 由于上游也受同一 bug 影响，修复前它拿不到 value；修复后它能拿到 "from_options"。
    //
    // 下游 tool 节点的 input_mapping 是 {value: "upstream_value.result"}，即从上游取 result。
    // 上游捕获的 result 是 "captured"（CapturingRegistry 的固定返回值）。
    // 期望：downstream 拿到 "captured"（deps_results 走的是 upstream_value.result 这条路径，
    // 解析出 CapturingRegistry 返回的 "captured" 字符串）。
    //
    // 关键断言：deps_results 的合并优先级高于 state.variables。
    // 如果未来 bug 翻转，downstream 拿到的将不再是 "captured" 而是 null。

    let mut upstream_mapping = HashMap::new();
    upstream_mapping.insert("value".to_string(), "value".to_string());
    let mut downstream_mapping = HashMap::new();
    downstream_mapping.insert("value".to_string(), "upstream_value.result".to_string());

    let wf = engine
        .create_workflow(
            "test_deps_precedence",
            vec![
                make_trigger("trigger"),
                make_tool("upstream_value", "upstream_tool", upstream_mapping, "upstream_output"),
                make_tool("downstream", "downstream_tool", downstream_mapping, "downstream_output"),
            ],
            vec![
                make_edge("trigger", "upstream_value"),
                make_edge("upstream_value", "downstream"),
            ],
        )
        .await
        .expect("create_workflow");

    let options = RunOptions::new()
        .with_max_concurrent(1)
        .with_variables(vec![Variable {
            name: "value".to_string(),
            var_type: "string".to_string(),
            value: serde_json::json!("from_options"),
            description: None,
            is_secret: false,
        }]);

    let _ = engine
        .run_workflow(&wf.id, options)
        .await
        .expect("run_workflow");

    let captured = reg.captured().await;
    // upstream + downstream 各执行 1 次
    assert_eq!(captured.len(), 2, "upstream + downstream 各执行 1 次");

    // 断言 1：upstream 收到了来自 options.variables 的 value = "from_options"
    // （证明 state.variables fallback 生效：deps_results 没有 top-level "value" key）
    let upstream_call = captured
        .iter()
        .find(|(n, _)| n == "upstream_tool")
        .expect("upstream_tool 应被调用");
    assert_eq!(
        upstream_call.1.get("value"),
        Some(&serde_json::json!("from_options")),
        "upstream 应通过 state.variables fallback 拿到 value"
    );

    // 断言 2：downstream 通过 deps_results 路径拿到 upstream 的 result = "captured"
    // （证明 deps_results 合并后正确保留了上游节点输出，downstream 能解析 upstream_value.result）
    let downstream_call = captured
        .iter()
        .find(|(n, _)| n == "downstream_tool")
        .expect("downstream_tool 应被调用");
    assert_eq!(
        downstream_call.1.get("value"),
        Some(&serde_json::json!("captured")),
        "downstream 应从 deps_results[\"upstream_value\"].result 拿到 upstream 返回的字符串"
    );
}
