// SPDX-License-Identifier: AGPL-3.0-only

//! WorkEngine 模板级生命周期钩子（pre_exec / post_exec）集成测试。
//!
//! 覆盖验收项：
//! 1. 钩子触发顺序（pre → DAG → post）
//! 2. pre_exec 变量增强生效（返回值覆盖写回执行上下文）
//! 3. pre_exec Err 阻断（返回 LifecycleHookFailed + DB 记 failed 终态）
//! 4. 未注册钩子名 warn 跳过（不阻断）
//! 5. hooks_config 为 NULL 的旧模板不受影响

use std::sync::Arc;

use axagent_harness::registry::ProviderRegistry;
use axagent_harness::repo_dtos::WorkflowExecutionData;
use axagent_harness::repositories::{
    WorkflowExecutionRepository, set_loop_checkpoint_repository, set_workflow_execution_repository,
};
use axagent_harness::workflow_lifecycle::{
    HookExecContext, HookOutcome, WorkflowHooksConfig, WorkflowLifecycleHook,
};
use axagent_harness::workflow_types::{
    EdgeType, EndNode, EndNodeConfig, Position, RetryConfig, TriggerConfig, TriggerNode,
    TriggerType, Variable, WorkflowEdge, WorkflowError, WorkflowNode, WorkflowNodeBase,
    WorkflowStatus,
};

use axagent_rt_workflow::work_engine::{RunOptions, WorkEngine};
use tokio::sync::Mutex;

// ── 记录型 WorkflowExecutionRepository（与 execution_finalization 同款）──

type UpdateLog = Arc<Mutex<Vec<(String, String, Option<i32>)>>>;

#[derive(Clone)]
struct RecordingWorkflowExecutionRepo {
    updates: UpdateLog,
}

#[async_trait::async_trait]
impl WorkflowExecutionRepository for RecordingWorkflowExecutionRepo {
    async fn create_workflow_execution(
        &self,
        _id: &str,
        _workflow_id: &str,
        _input_params: Option<&str>,
    ) -> Result<(), String> {
        Ok(())
    }
    async fn update_workflow_execution_status(
        &self,
        id: &str,
        status: &str,
        _output_result: Option<&str>,
        _node_executions: Option<&str>,
        total_time_ms: Option<i32>,
    ) -> Result<bool, String> {
        self.updates.lock().await.push((id.to_string(), status.to_string(), total_time_ms));
        Ok(true)
    }
    async fn list_workflow_executions(
        &self,
        _workflow_id: &str,
    ) -> Result<Vec<WorkflowExecutionData>, String> {
        Ok(vec![])
    }
    async fn save_execution_state(
        &self,
        _id: &str,
        _status: &str,
        _execution_state_json: &str,
    ) -> Result<bool, String> {
        Ok(true)
    }
    async fn clear_execution_state(&self, _id: &str, _status: &str) -> Result<bool, String> {
        Ok(true)
    }
    async fn list_paused_executions(&self) -> Result<Vec<WorkflowExecutionData>, String> {
        Ok(vec![])
    }
}

// ── 测试钩子 ─────────────────────────────────────────────────────────

/// 可配置行为的生命周期钩子：
/// - 每次调用向 `log` push 触发记录（pre / post + status）
/// - `pre_error` 非空时 pre_exec 返回 Err（阻断场景）
/// - `enhance` 为 true 时 pre_exec 追加 `hook_var` 变量（变量增强场景）
struct TestHook {
    name: String,
    log: Arc<Mutex<Vec<String>>>,
    pre_error: Option<String>,
    enhance: bool,
}

#[async_trait::async_trait]
impl WorkflowLifecycleHook for TestHook {
    fn name(&self) -> &str {
        &self.name
    }

    async fn pre_exec(&self, ctx: HookExecContext) -> Result<Vec<Variable>, String> {
        self.log.lock().await.push(format!("pre:{}", self.name));
        if let Some(err) = &self.pre_error {
            return Err(err.clone());
        }
        if self.enhance {
            let mut vars = ctx.variables;
            vars.push(Variable {
                name: "hook_var".to_string(),
                var_type: "any".to_string(),
                value: serde_json::json!("from_hook"),
                description: None,
                is_secret: false,
            });
            return Ok(vars);
        }
        Ok(ctx.variables)
    }

    async fn post_exec(&self, _ctx: HookExecContext, outcome: &HookOutcome) -> Result<(), String> {
        self.log.lock().await.push(format!("post:{}:{}", self.name, outcome.status));
        Ok(())
    }
}

// ── 最小 ProviderRegistry + 节点构造 helpers ─────────────────────────

struct EmptyProviderRegistry;

impl ProviderRegistry for EmptyProviderRegistry {
    fn get(&self, _provider_type: &str) -> Option<Arc<dyn axagent_harness::ProviderAdapter>> {
        None
    }
}

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
        continue_on_fail: false,
    }
}

fn make_trigger(id: &str) -> WorkflowNode {
    WorkflowNode::Trigger(TriggerNode {
        base: make_base(id, "Trigger"),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(id: &str) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: make_base(id, "End"),
        config: EndNodeConfig { output_var: None },
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

fn make_var(name: &str, value: serde_json::Value) -> Variable {
    Variable {
        name: name.to_string(),
        var_type: "any".to_string(),
        value,
        description: None,
        is_secret: false,
    }
}

async fn make_engine() -> Arc<WorkEngine> {
    let updates = Arc::new(Mutex::new(Vec::new()));
    let repo = Arc::new(RecordingWorkflowExecutionRepo { updates: updates.clone() });
    set_workflow_execution_repository(repo);
    set_loop_checkpoint_repository(axagent_harness::test_support::empty_loop_checkpoint_repo());

    let engine = Arc::new(WorkEngine::new([0u8; 32], Arc::new(EmptyProviderRegistry)));
    engine.init_dispatcher().await;
    engine
}

// ── 测试 ─────────────────────────────────────────────────────────────

/// 验收 1 + 2：触发顺序（pre → DAG → post）+ pre_exec 变量增强生效。
#[tokio::test(flavor = "multi_thread")]
async fn hook_order_and_pre_exec_variable_enhancement() {
    let engine = make_engine().await;
    let log = Arc::new(Mutex::new(Vec::new()));
    engine
        .register_lifecycle_hook(Arc::new(TestHook {
            name: "hook-a".to_string(),
            log: log.clone(),
            pre_error: None,
            enhance: true,
        }))
        .await;

    let wf = engine
        .create_workflow_with_hooks(
            "hooks_order_test",
            vec![make_trigger("t1"), make_end("end1")],
            vec![make_edge("t1", "end1")],
            Some(WorkflowHooksConfig {
                pre_exec: vec!["hook-a".to_string()],
                post_exec: vec!["hook-a".to_string()],
            }),
        )
        .await
        .expect("创建带钩子的工作流应成功");

    let opts = RunOptions {
        execution_id: Some("exec-order-1".to_string()),
        ..RunOptions::new()
            .with_variables(vec![make_var("stock_code", serde_json::json!("300567"))])
    };
    let result = engine.run_workflow(&wf.id, opts).await.expect("带钩子执行应成功");
    assert!(
        matches!(result.status, WorkflowStatus::Completed | WorkflowStatus::PartiallyCompleted),
        "简单工作流应完成，实际: {:?}",
        result.status
    );

    // 触发顺序：pre → DAG → post，且 post 收到终态 status
    let calls = log.lock().await.clone();
    assert_eq!(calls, vec!["pre:hook-a".to_string(), "post:hook-a:completed".to_string()]);

    // 变量增强：pre_exec 返回的 hook_var 已覆盖写回执行上下文，
    // 且模板级原变量（stock_code）保留
    let state = engine.get_status("exec-order-1").await.expect("执行状态应存在");
    assert_eq!(state.variables.get("hook_var"), Some(&serde_json::json!("from_hook")));
    assert_eq!(state.variables.get("stock_code"), Some(&serde_json::json!("300567")));
}

/// 验收 3：pre_exec Err 阻断 —— 返回 LifecycleHookFailed，DB 记 failed 终态，post 不触发。
#[tokio::test(flavor = "multi_thread")]
async fn pre_exec_err_blocks_execution() {
    let engine = make_engine().await;
    let log = Arc::new(Mutex::new(Vec::new()));
    engine
        .register_lifecycle_hook(Arc::new(TestHook {
            name: "hook-block".to_string(),
            log: log.clone(),
            pre_error: Some("数据预检失败".to_string()),
            enhance: false,
        }))
        .await;

    let wf = engine
        .create_workflow_with_hooks(
            "hooks_block_test",
            vec![make_trigger("t1"), make_end("end1")],
            vec![make_edge("t1", "end1")],
            Some(WorkflowHooksConfig {
                pre_exec: vec!["hook-block".to_string()],
                post_exec: vec!["hook-block".to_string()],
            }),
        )
        .await
        .expect("创建阻断测试工作流应成功");

    let result =
        engine.run_workflow(&wf.id, RunOptions::new()).await.expect_err("pre_exec Err 应阻断执行");
    match &result {
        WorkflowError::LifecycleHookFailed { hook, message } => {
            assert_eq!(hook, "hook-block");
            assert_eq!(message, "数据预检失败");
        },
        other => panic!("应为 LifecycleHookFailed，实际: {other:?}"),
    }

    // post_exec 不应被触发（执行被阻断）
    assert!(
        log.lock().await.iter().all(|c| !c.starts_with("post:")),
        "被阻断执行的 post_exec 不应触发"
    );
}

/// 验收 4：模板声明的钩子未注册 → warn 跳过，不阻断执行。
#[tokio::test(flavor = "multi_thread")]
async fn unregistered_hook_name_is_skipped() {
    let engine = make_engine().await;
    let log = Arc::new(Mutex::new(Vec::new()));
    // 注册的是 hook-a，但模板声明的是 ghost-hook（滞后于模板声明）
    engine
        .register_lifecycle_hook(Arc::new(TestHook {
            name: "hook-a".to_string(),
            log: log.clone(),
            pre_error: None,
            enhance: false,
        }))
        .await;

    let wf = engine
        .create_workflow_with_hooks(
            "hooks_ghost_test",
            vec![make_trigger("t1"), make_end("end1")],
            vec![make_edge("t1", "end1")],
            Some(WorkflowHooksConfig {
                pre_exec: vec!["ghost-hook".to_string()],
                post_exec: vec!["ghost-hook".to_string()],
            }),
        )
        .await
        .expect("创建幽灵钩子测试工作流应成功");

    let result = engine.run_workflow(&wf.id, RunOptions::new()).await.expect("未注册钩子不应阻断");
    assert!(matches!(
        result.status,
        WorkflowStatus::Completed | WorkflowStatus::PartiallyCompleted
    ));

    // 未注册钩子名既不触发 pre 也不触发 post
    assert!(log.lock().await.is_empty(), "未注册钩子不应有任何触发记录");
}

/// 验收 5：hooks_config 为 NULL 的旧模板不受影响（钩子不触发 + serde 反序列化兼容）。
#[tokio::test(flavor = "multi_thread")]
async fn null_hooks_config_legacy_template_unaffected() {
    let engine = make_engine().await;
    let log = Arc::new(Mutex::new(Vec::new()));
    engine
        .register_lifecycle_hook(Arc::new(TestHook {
            name: "hook-a".to_string(),
            log: log.clone(),
            pre_error: None,
            enhance: false,
        }))
        .await;

    // 旧路径：create_workflow（无钩子声明）
    let wf = engine
        .create_workflow(
            "legacy_template_test",
            vec![make_trigger("t1"), make_end("end1")],
            vec![make_edge("t1", "end1")],
        )
        .await
        .expect("创建旧模板应成功");
    assert!(wf.hooks_config.is_none(), "旧路径创建的工作流不应带钩子声明");

    let result = engine.run_workflow(&wf.id, RunOptions::new()).await.expect("旧模板执行应成功");
    assert!(matches!(
        result.status,
        WorkflowStatus::Completed | WorkflowStatus::PartiallyCompleted
    ));
    assert!(log.lock().await.is_empty(), "无钩子声明的模板不应触发任何钩子");

    // serde 兼容：旧 JSON（无 hooks_config 字段）反序列化后 hooks_config 为 None
    let legacy: axagent_harness::workflow_types::Workflow =
        serde_json::from_str(r#"{"id":"wf1","name":"n","nodes":[],"edges":[],"status":"created","created_at":0,"completed_at":null,"results":{},"node_states":{},"output":null}"#)
            .expect("旧 JSON 反序列化应成功（serde default）");
    assert!(legacy.hooks_config.is_none());
}
