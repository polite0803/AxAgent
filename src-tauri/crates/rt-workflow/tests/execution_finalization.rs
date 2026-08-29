// SPDX-License-Identifier: AGPL-3.0-only

//! 执行层收尾集成测试。
//!
//! 覆盖本轮修复的两个回归点：
//!  1) 子工作流收尾：此前子工作流 run_workflow 走"收尾捷径"时只构建 output 就 return，
//!     不调用 complete_execution，导致其 DB workflow_execution 记录永远停留在初始态
//!     （孤儿 running 记录）。修复后在捷径内补一次纯 repo 的 update_workflow_execution_status。
//!  2) PartiallyCompleted 的 total_time_ms：workflow 级 completed_at 曾有一处误用毫秒
//!     （timestamp_millis），与 created_at（秒）不一致，导致
//!     total_time_ms = (completed_at - created_at) * 1000 计算出天文数字。修复后统一为秒。

use std::sync::Arc;
use std::sync::Mutex;

use axagent_harness::registry::ProviderRegistry;
use axagent_harness::repo_dtos::WorkflowExecutionData;
use axagent_harness::repositories::{
    WorkflowExecutionRepository, set_loop_checkpoint_repository, set_workflow_execution_repository,
};
use axagent_harness::test_support::empty_loop_checkpoint_repo;
use axagent_harness::workflow_types::{
    EdgeType, EndNode, EndNodeConfig, Position, RetryConfig, TriggerConfig, TriggerNode,
    TriggerType, WorkflowEdge, WorkflowNode, WorkflowNodeBase, WorkflowStatus,
};

use axagent_rt_workflow::work_engine::{RunOptions, WorkEngine};

// ── 记录型 WorkflowExecutionRepository ───────────────────────────────
//
// 记录每次 create_workflow_execution / update_workflow_execution_status 调用，
// 便于断言 DB 是否收到终态与总耗时（total_time_ms）。

#[derive(Clone)]
struct RecordingWorkflowExecutionRepo {
    /// (exec_id, status, total_time_ms)
    updates: Arc<Mutex<Vec<(String, String, Option<i32>)>>>,
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
        self.updates.lock().unwrap().push((id.to_string(), status.to_string(), total_time_ms));
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

// ── 最小 ProviderRegistry + 节点构造 helpers ─────────────────────────

struct EmptyProviderRegistry;

impl ProviderRegistry for EmptyProviderRegistry {
    fn get(&self, _provider_type: &str) -> Option<Arc<dyn axagent_harness::ProviderAdapter>> {
        None
    }
}

fn make_base(id: &str, title: &str, enabled: bool) -> WorkflowNodeBase {
    WorkflowNodeBase {
        id: id.to_string(),
        title: title.to_string(),
        description: None,
        position: Position::default(),
        retry: RetryConfig::default(),
        timeout: Some(30),
        enabled,
        parent_id: None,
        compensation: None,
        continue_on_fail: false,
    }
}

fn make_trigger(id: &str) -> WorkflowNode {
    WorkflowNode::Trigger(TriggerNode {
        base: make_base(id, "Trigger", true),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(id: &str) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: make_base(id, "End", true),
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

// ── 测试 ─────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn sub_workflow_and_partial_finalize_persist_terminal_status() {
    let updates = Arc::new(Mutex::new(Vec::new()));
    let repo = Arc::new(RecordingWorkflowExecutionRepo { updates: updates.clone() });
    // 全局 registry 覆盖式注入；单测试函数内顺序执行，无并行 set 冲突。
    set_workflow_execution_repository(repo);
    set_loop_checkpoint_repository(empty_loop_checkpoint_repo());

    let engine = Arc::new(WorkEngine::new([0u8; 32], Arc::new(EmptyProviderRegistry)));
    engine.init_dispatcher().await;

    // ── 场景 A：子工作流收尾（parent_execution_id 非空 → 收尾捷径）──
    let sub_wf = engine
        .create_workflow(
            "sub_finalize_test",
            vec![make_trigger("t1"), make_end("end1")],
            vec![make_edge("t1", "end1")],
        )
        .await
        .expect("创建子工作流测试用工作流应成功");
    let sub_result = engine
        .run_workflow(
            &sub_wf.id,
            RunOptions { parent_execution_id: Some("parent-1".to_string()), ..RunOptions::new() },
        )
        .await
        .expect("子工作流运行应成功");
    assert!(
        matches!(sub_result.status, WorkflowStatus::Completed | WorkflowStatus::PartiallyCompleted),
        "简单子工作流应完成/部分完成，实际: {:?}",
        sub_result.status
    );

    // 断言子工作流的 DB 收尾确实写入终态（而非停留在初始 running → 无 update 记录）。
    let mut sub_seen_final = false;
    for (_, status, total_time) in updates.lock().unwrap().iter() {
        if status == "completed" {
            let t = total_time.expect("completed 收尾必须写入 total_time_ms");
            assert!((0..3_600_000).contains(&t), "total_time_ms 应为合理毫秒值，实际: {t}");
            sub_seen_final = true;
        }
    }
    assert!(sub_seen_final, "子工作流 DB 记录应收到 completed 终态");

    // ── 场景 B：PartiallyCompleted 时 total_time_ms 合理 ──
    // 正常 t1→end1 外加一个孤立 disabled 节点。disabled 节点初始 Pending 但不参与
    // DAG 就绪调度，主链完成后死锁检测将其置 Skipped，从而 all_ok && any_skipped。
    let disabled_trigger = WorkflowNode::Trigger(TriggerNode {
        base: make_base("disabled1", "Disabled", false),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    });
    let partial_wf = engine
        .create_workflow(
            "partial_finalize_test",
            vec![make_trigger("t1"), make_end("end1"), disabled_trigger],
            vec![make_edge("t1", "end1")],
        )
        .await
        .expect("创建部分完成测试用工作流应成功");
    let partial_result = engine
        .run_workflow(&partial_wf.id, RunOptions::new())
        .await
        .expect("部分完成工作流运行应成功");
    assert_eq!(
        partial_result.status,
        WorkflowStatus::PartiallyCompleted,
        "含孤立 disabled 节点的工作流应部分完成，实际: {:?}",
        partial_result.status
    );

    // 断言 PartiallyCompleted 的 DB 收尾 total_time_ms 合理（非天文数字，即非毫秒×秒混算）。
    let mut partial_seen_final = false;
    for (_, status, total_time) in updates.lock().unwrap().iter() {
        if status == "partially_completed" {
            let t = total_time.expect("partially_completed 收尾必须写入 total_time_ms");
            assert!(
                (0..3_600_000).contains(&t),
                "PartiallyCompleted total_time_ms 应为合理毫秒值，实际: {t}"
            );
            partial_seen_final = true;
        }
    }
    assert!(partial_seen_final, "PartiallyCompleted DB 记录应收到 partially_completed 终态");
}
