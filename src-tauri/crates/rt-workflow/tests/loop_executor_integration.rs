// SPDX-License-Identifier: AGPL-3.0-only

//! Loop 节点执行器的集成测试。
//!
//! 覆盖需求中的三个核心场景：
//!  1) forEach 模式：数组输入 → 顺序迭代 → 聚合输出（iter_count=items.len()）
//!  2) interrupt 模式：body 节点触发 `{"status": "pending"}` → 写检查点 → 挂起
//!     → 外部调 resume API → 继续迭代直至完成
//!  3) partial_result 顺序：每次迭代的 PartialResultEvent 严格按 iter_index
//!     0/1/2/... 顺序到达
//!
//! 设计说明：测试不走完整 WorkEngine.run_workflow 路径（那需要 trigger/end 节点
//! 编排 + dispatcher 真实查找），而是直接构造 ExecutionState + 注入三个回调
//! （loop_body_dispatch / loop_checkpoint / partial_result_tx），让测试聚焦
//! 在 LoopExecutor 的迭代控制、checkpoint 恢复、partial 流式这三块逻辑上。

use std::collections::HashMap;
use std::sync::Arc;

use axagent_core::workflow_types::{
    LoopCheckpoint, LoopNode, LoopNodeConfig, LoopType, Position, RetryConfig, WorkflowNode,
    WorkflowNodeBase,
};

use axagent_rt_workflow::work_engine::execution_state::{
    ExecutionContextCallbacks, ExecutionState, LoopBodyDispatchFn, LoopCheckpointOps,
    PartialResultEvent,
};
use axagent_rt_workflow::work_engine::executors::LoopExecutor;
use axagent_rt_workflow::work_engine::{NodeExecutorTrait, NodeOutput};

// ── 公共辅助 ─────────────────────────────────────────────────────────

fn make_loop_base(id: &str) -> WorkflowNodeBase {
    WorkflowNodeBase {
        id: id.to_string(),
        title: "Loop".to_string(),
        description: None,
        position: Position::default(),
        retry: RetryConfig::default(),
        timeout: Some(30),
        enabled: true,
        parent_id: None,
        compensation: None,
    }
}

fn make_loop_node(config: LoopNodeConfig) -> WorkflowNode {
    WorkflowNode::Loop(LoopNode {
        base: make_loop_base("loop1"),
        config,
    })
}

/// 内存版 LoopCheckpointOps —— 用 HashMap 模拟持久化层。
/// key = (execution_id, node_id) 复合主键。
fn in_memory_checkpoint_ops() -> (
    LoopCheckpointOps,
    Arc<tokio::sync::Mutex<HashMap<(String, String), LoopCheckpoint>>>,
) {
    let store: Arc<tokio::sync::Mutex<HashMap<(String, String), LoopCheckpoint>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let s_save = store.clone();
    let s_load = store.clone();
    let s_del = store.clone();
    let ops = LoopCheckpointOps {
        save: Arc::new(move |cp: LoopCheckpoint| {
            let s = s_save.clone();
            Box::pin(async move {
                let mut g = s.lock().await;
                g.insert((cp.execution_id.clone(), cp.node_id.clone()), cp);
                Ok(())
            })
        }),
        load: Arc::new(move |eid: String, nid: String| {
            let s = s_load.clone();
            Box::pin(async move {
                let g = s.lock().await;
                Ok(g.get(&(eid, nid)).cloned())
            })
        }),
        delete: Arc::new(move |eid: String, nid: String| {
            let s = s_del.clone();
            Box::pin(async move {
                let mut g = s.lock().await;
                g.remove(&(eid, nid));
                Ok(())
            })
        }),
    };
    (ops, store)
}

/// 构造一个不修改 context、只把 body_step_id 透传给 caller 的最小 body dispatcher。
/// body_fn 接收 (step_id, iter_ctx)，返回该 step 的 NodeOutput。
fn make_body_dispatch(
    body_fn: Arc<dyn Fn(String, ExecutionState) -> NodeOutput + Send + Sync>,
) -> LoopBodyDispatchFn {
    Arc::new(move |step_id: String, ctx: ExecutionState| {
        let body_fn = body_fn.clone();
        Box::pin(async move { Ok(body_fn(step_id, ctx)) })
    })
}

fn make_state(execution_id: &str) -> ExecutionState {
    let mut s =
        ExecutionState::new(execution_id.to_string(), "wf1".to_string(), serde_json::json!({}));
    s.callbacks = Some(ExecutionContextCallbacks {
        tool_handlers: HashMap::new(),
        tool_fallback: None,
        subworkflow: None,
        loop_body_dispatch: None,
        loop_checkpoint: None,
    });
    s
}

// ── 测试 1：forEach 模式聚合 ─────────────────────────────────────────

#[tokio::test]
async fn loop_foreach_aggregates_results() {
    // 数组输入：交易列表。body 是"翻倍"工具：把当前元素乘 2 输出。
    let mut state = make_state("exec1");
    state
        .variables
        .insert("tx_list".to_string(), serde_json::json!([1, 2, 3, 4]));

    let body_fn: Arc<dyn Fn(String, ExecutionState) -> NodeOutput + Send + Sync> =
        Arc::new(|_step_id, ctx| {
            // 读 iteratee 变量，输出双倍
            let item = ctx
                .variables
                .get("__loop_iteratee__")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let n = item.as_i64().unwrap_or(0);
            NodeOutput {
                output: serde_json::json!({"doubled": n * 2}),
                output_var: Some("step_out".to_string()),
            }
        });
    state.callbacks.as_mut().unwrap().loop_body_dispatch = Some(make_body_dispatch(body_fn));

    let (cp_ops, _cp_store) = in_memory_checkpoint_ops();
    state.callbacks.as_mut().unwrap().loop_checkpoint = Some(cp_ops);

    let node = make_loop_node(LoopNodeConfig {
        loop_type: LoopType::ForEach,
        items_var: None,
        iter_input_var: Some("tx_list".to_string()),
        iteratee_var: Some("__loop_iteratee__".to_string()),
        iter_output_var: Some("iter_output".to_string()),
        partial_result_var: None,
        max_iterations: None,
        continue_condition: None,
        continue_on_error: false,
        body_steps: vec!["step1".to_string()],
        sub_graph: None,
        interrupt_after_each: false,
        interrupt_nodes: vec![],
    });

    let executor = LoopExecutor::new();
    let out = executor.execute(&node, &state).await.expect("execute");

    // 验证聚合：4 个元素都被处理
    assert_eq!(out.output.get("iter_count").and_then(|v| v.as_u64()), Some(4));
    assert_eq!(out.output.get("loop_type").and_then(|v| v.as_str()), Some("forEach"));
    let items = out
        .output
        .get("items")
        .and_then(|v| v.as_array())
        .expect("items array");
    assert_eq!(items.len(), 4);
    let values: Vec<i64> = items
        .iter()
        .map(|v| v.get("doubled").and_then(|x| x.as_i64()).unwrap_or(-1))
        .collect();
    assert_eq!(values, vec![2, 4, 6, 8], "forEach 应按顺序产出 2/4/6/8");
    assert_eq!(out.output_var.as_deref(), Some("iter_output"));
}

// ── 测试 2：interrupt 暂停 + resume ─────────────────────────────────

#[tokio::test]
async fn loop_interrupt_pause_then_resume_continues() {
    let mut state = make_state("exec2");
    state
        .variables
        .insert("tx_list".to_string(), serde_json::json!([10, 20, 30]));

    // 模拟审批节点：第一次返回 pending（触发 interrupt），之后返回 approved。
    let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let body_fn: Arc<dyn Fn(String, ExecutionState) -> NodeOutput + Send + Sync> = {
        let call_count = call_count.clone();
        Arc::new(move |_step_id, _ctx| {
            let n = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                NodeOutput {
                    output: serde_json::json!({"status": "pending", "reviewer": "alice"}),
                    output_var: Some("approval_out".to_string()),
                }
            } else {
                NodeOutput {
                    output: serde_json::json!({"status": "approved"}),
                    output_var: Some("approval_out".to_string()),
                }
            }
        })
    };
    state.callbacks.as_mut().unwrap().loop_body_dispatch = Some(make_body_dispatch(body_fn));

    let (cp_ops, cp_store) = in_memory_checkpoint_ops();
    state.callbacks.as_mut().unwrap().loop_checkpoint = Some(cp_ops);

    // interrupt signal
    let interrupt_signal = Arc::new(tokio::sync::Notify::new());
    state.interrupt_signal = Some(interrupt_signal.clone());

    let node = make_loop_node(LoopNodeConfig {
        loop_type: LoopType::ForEach,
        items_var: None,
        iter_input_var: Some("tx_list".to_string()),
        iteratee_var: Some("__loop_iteratee__".to_string()),
        iter_output_var: Some("iter_output".to_string()),
        partial_result_var: None,
        max_iterations: None,
        continue_condition: None,
        continue_on_error: false,
        body_steps: vec!["approval".to_string()],
        sub_graph: None,
        interrupt_after_each: false,
        // interrupt_nodes 留空：detect_interrupt 第二个分支才会起作用，
        // 即只有 body 输出 `{"status": "pending"}` 时才挂起；approved 不再
        // 触发 interrupt，避免进入"挂起 → 永远挂起"的死循环。
        interrupt_nodes: vec![],
    });

    // 启任务：loop 在第一次审批时进入 interrupt，停在 sig.notified().await
    let node_for_task = node.clone();
    let state_for_task = state.clone();
    let exec_id_for_task = "exec2".to_string();
    let task = tokio::spawn(async move {
        let executor = LoopExecutor::new();
        executor.execute(&node_for_task, &state_for_task).await
    });

    // 等检查点出现（第一次 interrupt 已写盘）
    let mut found_paused = false;
    for _ in 0..200 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        let g = cp_store.lock().await;
        if let Some(cp) = g.get(&(exec_id_for_task.clone(), "loop1".to_string())) {
            if cp.pending_approval_node.is_some() && cp.cursor == 0 {
                found_paused = true;
                break;
            }
        }
    }
    assert!(
        found_paused,
        "interrupt 后检查点应写入：cursor=0, pending_approval_node=Some(approval)"
    );

    // 此时 call_count 应为 1（第一次审批被调用了一次）
    assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);

    // 模拟"用户审批时间"：等待 executor 真正进入 `sig.notified().await`。
    // notify_waiters() 在没有 waiter 时不会存 permit，需要确保 executor 已
    // 到达 await 点。生产路径中 human review 时间远超几十毫秒，这里
    // 200ms 足够覆盖"checkpoint 已写 → executor 走到 notified() await"的窗口。
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // 唤醒 → resume
    interrupt_signal.notify_waiters();

    // 等任务完成
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .expect("task should finish in 5s")
        .expect("task join")
        .expect("execute ok");

    // 3 个元素都完成（2 个 approved + 1 个首次 pending 不计入 partial）
    assert_eq!(
        result.output.get("iter_count").and_then(|v| v.as_u64()),
        Some(3),
        "iter_count 应为 3：首次 interrupt 不 append，后续 2 次 approved append"
    );
    let items = result
        .output
        .get("items")
        .and_then(|v| v.as_array())
        .expect("items");
    assert_eq!(items.len(), 3);
    // 全部 3 次都应被调用（iter 0 在 interrupt 时被调 1 次返回 pending，
    // resume 后 iter 0 再次被调返回 approved；iter 1/2 各调 1 次返回
    // approved → 总计 4 次 body_dispatch 调用，3 个元素都走通）。
    assert_eq!(
        call_count.load(std::sync::atomic::Ordering::SeqCst),
        4,
        "iter 0 interrupt 调 1 次 + iter 0/1/2 resume 后各调 1 次 = 4 次"
    );
    // 检查点清理
    let g = cp_store.lock().await;
    assert!(g.is_empty(), "完成后检查点应被 delete");
}

// ── 测试 3：partial_result 顺序 ─────────────────────────────────────

#[tokio::test]
async fn loop_partial_results_arrive_in_order() {
    let mut state = make_state("exec3");
    state
        .variables
        .insert("tx_list".to_string(), serde_json::json!(["a", "b", "c"]));

    let body_fn: Arc<dyn Fn(String, ExecutionState) -> NodeOutput + Send + Sync> =
        Arc::new(|_step_id, ctx| {
            let item = ctx
                .variables
                .get("__loop_iteratee__")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            NodeOutput {
                output: serde_json::json!({"echo": item}),
                output_var: Some("echo_out".to_string()),
            }
        });
    state.callbacks.as_mut().unwrap().loop_body_dispatch = Some(make_body_dispatch(body_fn));

    let (cp_ops, _cp_store) = in_memory_checkpoint_ops();
    state.callbacks.as_mut().unwrap().loop_checkpoint = Some(cp_ops);

    // partial broadcast
    let (tx, mut rx) = tokio::sync::broadcast::channel::<PartialResultEvent>(16);
    state.partial_result_tx = Some(tx);

    let node = make_loop_node(LoopNodeConfig {
        loop_type: LoopType::ForEach,
        items_var: None,
        iter_input_var: Some("tx_list".to_string()),
        iteratee_var: Some("__loop_iteratee__".to_string()),
        iter_output_var: Some("iter_output".to_string()),
        partial_result_var: None,
        max_iterations: None,
        continue_condition: None,
        continue_on_error: false,
        body_steps: vec!["echo_step".to_string()],
        sub_graph: None,
        interrupt_after_each: false,
        interrupt_nodes: vec![],
    });

    let executor = LoopExecutor::new();
    let _out = executor.execute(&node, &state).await.expect("execute");

    // 收 3 个 completed 事件（顺序应是 0, 1, 2）
    let mut got: Vec<u32> = Vec::new();
    let mut cumulative_lengths: Vec<usize> = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        assert_eq!(ev.phase, "completed", "非 interrupt 路径 phase 应为 completed");
        got.push(ev.iter_index);
        cumulative_lengths.push(ev.cumulative_partial.len());
    }
    assert_eq!(got, vec![0, 1, 2], "事件应按 iter_index 0/1/2 到达");
    assert_eq!(cumulative_lengths, vec![1, 2, 3], "cumulative_partial 应递增：1/2/3");
}
