// SPDX-License-Identifier: AGPL-3.0-only

//! Loop 节点执行器 —— 内部驱动 body_steps 迭代。
//!
//! 设计要点：
//!  1) 接收 `iter_input_var`（兼容 `items_var`）指向的数组输入，对每个元素
//!     顺序执行 `body_steps` 中列出的 body 节点。
//!  2) 通过 `ExecutionState.callbacks.loop_body_dispatch` 回调驱动 body
//!     节点。回调由 `WorkEngine::build_loop_body_dispatch` 工厂构造，
//!     内部走 dispatcher —— 保留了 progress_callback / 节点状态切换 /
//!     node_records 等统一埋点。
//!  3) 通过 `ExecutionState.callbacks.loop_checkpoint` 回调读写
//!     `loop_checkpoints` 表，interrupted 后能从 cursor 继续。
//!  4) 通过 `ExecutionState.partial_result_tx` 广播每次迭代的
//!     `PartialResultEvent` 给订阅者。
//!  5) interrupt 触发后：`interrupt_signal.notified().await` 挂起；外部
//!     `WorkEngine::resume_loop_iteration` 调 `notify_waiters()` 唤醒。
//!
//! 端口契约：
//!  - `iter_input_var` (旧名 `items_var`): 数组输入端口
//!  - `iter_output_var` (默认 `iter_output`): 聚合输出端口
//!  - `partial_result_var`: 流式中间结果（每次迭代后写入 context.variables）
//!  - `iteratee_var`: 当前元素写入 context.variables 的 key
//!  - `interrupt_after_each`: 每次迭代后强制挂起
//!  - `interrupt_nodes`: 命中即挂起的 body 节点 ID 集合

use async_trait::async_trait;
use axagent_core::workflow_types::{LoopCheckpoint, LoopType, WorkflowNode};
#[cfg(test)]
use serde_json::json;

use crate::work_engine::execution_state::{ExecutionState, PartialResultEvent};
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

/// Loop 单次迭代最大硬上限。`max_iterations` 超过此值时 clamp 到此值。
const MAX_ITERATIONS_HARD_CAP: u32 = 10_000;

pub struct LoopExecutor;

impl LoopExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoopExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 判定当前 body_step 的输出是否需要 trigger interrupt。
///
/// 判定条件（满足任一即触发）：
///  1) body_step_id ∈ config.interrupt_nodes
///  2) body_step 输出是审批类节点约定的 `{"status": "pending", ...}`
///
/// 返回 (是否触发, 触发的 step_id, 触发的 step_output)。
fn detect_interrupt(
    config: &axagent_core::workflow_types::LoopNodeConfig,
    step_id: &str,
    step_output: &serde_json::Value,
) -> Option<(String, serde_json::Value)> {
    if config.interrupt_nodes.iter().any(|n| n == step_id) {
        return Some((step_id.to_string(), step_output.clone()));
    }
    if let Some(obj) = step_output.as_object()
        && let Some(serde_json::Value::String(s)) = obj.get("status")
        && s == "pending"
    {
        return Some((step_id.to_string(), step_output.clone()));
    }
    None
}

#[async_trait]
impl NodeExecutorTrait for LoopExecutor {
    fn node_type(&self) -> &'static str {
        "loop"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Loop(n) = node else {
            return Err(NodeError::type_mismatch("loop", self.node_type()));
        };
        let c = &n.config;

        if c.body_steps.is_empty() {
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                "Loop node has empty body_steps".to_string(),
            ));
        }
        // while / until 必须有 continue_condition
        if matches!(c.loop_type, LoopType::While | LoopType::Until)
            && c.continue_condition.is_none()
        {
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                "while/until loop requires continue_condition".to_string(),
            ));
        }

        // 解析回调：loop_body_dispatch 必须存在（由引擎注入）。
        let loop_dispatch = context
            .callbacks
            .as_ref()
            .and_then(|cb| cb.loop_body_dispatch.clone())
            .ok_or_else(|| {
                NodeError::exec_failed(
                    error_code::VALIDATION_FAILED,
                    "Loop executor requires loop_body_dispatch callback (engine not initialized)"
                        .to_string(),
                )
            })?;

        let exec_id = context.execution_id.clone();
        let node_id = node.base_id().to_string();

        // ── 解析输入数组 ──
        let input_var = c.effective_input_var();
        let items: Vec<serde_json::Value> = if let Some(var_name) = input_var {
            match context.variables.get(var_name) {
                Some(serde_json::Value::Array(arr)) => arr.clone(),
                Some(other) => vec![other.clone()],
                None => Vec::new(),
            }
        } else {
            // 兜底：forEach 模式若未指定 iter_input_var，跳过并返回空聚合。
            Vec::new()
        };

        // ── 决定迭代次数上限 ──
        let max_iter = c
            .max_iterations
            .unwrap_or(items.len() as u32)
            .min(MAX_ITERATIONS_HARD_CAP);
        let total = (items.len() as u32).min(max_iter);

        // ── 决定起点：恢复路径（读 checkpoint） vs 全新路径 ──
        let mut cursor: u32 = 0;
        let mut partial: Vec<serde_json::Value> = Vec::new();
        let mut input_items = items.clone();
        let mut resumed_from_checkpoint = false;

        if let Some(checkpoint_ops) = context
            .callbacks
            .as_ref()
            .and_then(|cb| cb.loop_checkpoint.clone())
        {
            match (checkpoint_ops.load)(exec_id.clone(), node_id.clone()).await {
                Ok(Some(cp)) => {
                    cursor = cp.cursor;
                    partial = cp.partial_results;
                    // 恢复时优先使用 checkpoint 里存的 input_items（防止
                    // iter_input_var 在中断后变量被重置时丢失数组）。
                    if !cp.input_items.is_empty() {
                        input_items = cp.input_items;
                    }
                    resumed_from_checkpoint = true;
                    tracing::info!(
                        execution_id = %exec_id,
                        node_id = %node_id,
                        cursor,
                        partial_len = partial.len(),
                        "[Loop] 恢复自检查点"
                    );
                },
                Ok(None) => {},
                Err(e) => {
                    tracing::warn!(
                        execution_id = %exec_id,
                        node_id = %node_id,
                        error = %e,
                        "[Loop] 读检查点失败，按全新路径执行"
                    );
                },
            }
        }

        // ── iteratee_var 校验 ──
        let iteratee_var_key = c
            .iteratee_var
            .clone()
            .unwrap_or_else(|| "__loop_iteratee__".to_string());
        let output_var_key = c.effective_output_var().to_string();
        let partial_var_key = c.effective_partial_var().map(|s| s.to_string());

        // ── 主循环：迭代 items[cursor..total] ──
        let mut last_iter_index: i32 = cursor as i32 - 1;
        let mut iter_index: u32 = cursor;

        while (iter_index as usize) < input_items.len() && iter_index < total {
            // 取消：检测 cancel_token
            if let Some(token) = &context.cancel_token
                && token.is_cancelled()
            {
                return Err(NodeError::exec_failed(
                    error_code::TIMEOUT,
                    "Loop iteration cancelled".to_string(),
                ));
            }

            let item = input_items[iter_index as usize].clone();

            // ── 构造本轮 body 调度的 ctx 副本 ──
            let mut iter_ctx = context.clone();
            // 注入 iteratee 变量
            iter_ctx
                .variables
                .insert(iteratee_var_key.clone(), item.clone());
            // 注入 partial_result（用于下游 body_step 看到累计结果）
            iter_ctx
                .variables
                .insert(format!("{output_var_key}__partial"), serde_json::json!(partial));
            // 暴露 loop 元信息
            iter_ctx
                .variables
                .insert("__loop_iter_index__".to_string(), serde_json::json!(iter_index));
            iter_ctx
                .variables
                .insert("__loop_iter_total__".to_string(), serde_json::json!(total));

            // ── 顺序执行 body_steps ──
            let mut last_step_output = serde_json::Value::Null;
            let mut interrupt_hit: Option<(String, serde_json::Value)> = None;

            for body_step_id in &c.body_steps {
                let step_ctx = iter_ctx.clone();
                match loop_dispatch(body_step_id.clone(), step_ctx).await {
                    Ok(out) => {
                        // 把 body_step 的输出写入 iter_ctx.variables 供下游步骤读取
                        // （约定：output_var 是 body_step 配置的输出 key，executor 已经
                        // 把 output 放在 output.output 字段里；这里再写到 context 上）。
                        if let Some(ref out_var) = out.output_var {
                            iter_ctx
                                .variables
                                .insert(out_var.clone(), out.output.clone());
                        }
                        last_step_output = out.output.clone();

                        // 判定 interrupt
                        if let Some(hit) = detect_interrupt(c, body_step_id, &out.output) {
                            interrupt_hit = Some(hit);
                            break;
                        }
                    },
                    Err(e) => {
                        if !c.continue_on_error {
                            // 非继续模式：写检查点（保留 cursor 指向失败的那一轮，
                            // 便于事后排查），返回错误。
                            if let Some(checkpoint_ops) = context
                                .callbacks
                                .as_ref()
                                .and_then(|cb| cb.loop_checkpoint.clone())
                            {
                                let cp = LoopCheckpoint {
                                    execution_id: exec_id.clone(),
                                    node_id: node_id.clone(),
                                    cursor: iter_index,
                                    input_items: input_items.clone(),
                                    partial_results: partial.clone(),
                                    pending_approval_node: None,
                                    pending_step_output: Some(
                                        serde_json::json!({"error": e.to_string()}),
                                    ),
                                    saved_at_ms: chrono::Utc::now().timestamp_millis() as u64,
                                    interrupting_step_id: Some(body_step_id.clone()),
                                };
                                let _ = (checkpoint_ops.save)(cp).await;
                            }
                            return Err(e);
                        }
                        // 继续模式：记错误但继续下一轮
                        last_step_output = serde_json::json!({"error": e.to_string()});
                    },
                }
            }

            // ── 中断分支：保存检查点 + 等待 resume ──
            if let Some((hit_step_id, hit_output)) = interrupt_hit {
                let cp = LoopCheckpoint {
                    execution_id: exec_id.clone(),
                    node_id: node_id.clone(),
                    cursor: iter_index, // 当前 iter_index 即为下一步起点
                    input_items: input_items.clone(),
                    partial_results: partial.clone(),
                    pending_approval_node: Some(hit_step_id.clone()),
                    pending_step_output: Some(hit_output.clone()),
                    saved_at_ms: chrono::Utc::now().timestamp_millis() as u64,
                    interrupting_step_id: Some(hit_step_id.clone()),
                };
                if let Some(checkpoint_ops) = context
                    .callbacks
                    .as_ref()
                    .and_then(|cb| cb.loop_checkpoint.clone())
                    && let Err(e) = (checkpoint_ops.save)(cp).await
                {
                    tracing::error!(
                        execution_id = %exec_id,
                        node_id = %node_id,
                        error = %e,
                        "[Loop] 保存 interrupt 检查点失败"
                    );
                }

                // 广播 partial_result（phase=interrupt），供前端 UI 立即显示
                if let Some(tx) = &context.partial_result_tx {
                    let _ = tx.send(PartialResultEvent {
                        execution_id: exec_id.clone(),
                        node_id: node_id.clone(),
                        iter_index,
                        item: item.clone(),
                        step_output: hit_output.clone(),
                        cumulative_partial: partial.clone(),
                        phase: "interrupt".to_string(),
                        emitted_at_ms: chrono::Utc::now().timestamp_millis(),
                    });
                }

                // 等待 resume signal
                if let Some(sig) = &context.interrupt_signal {
                    tracing::info!(
                        execution_id = %exec_id,
                        node_id = %node_id,
                        iter_index,
                        hit_step = %hit_step_id,
                        "[Loop] 等待 resume signal..."
                    );
                    sig.notified().await;
                    tracing::info!(
                        execution_id = %exec_id,
                        node_id = %node_id,
                        "[Loop] resume signal 收到，继续迭代"
                    );
                }

                // 重新读 checkpoint 决定下一步（resume_loop_iteration 可能更新了
                // partial 或 cursor）。
                if let Some(checkpoint_ops) = context
                    .callbacks
                    .as_ref()
                    .and_then(|cb| cb.loop_checkpoint.clone())
                    && let Ok(Some(updated_cp)) =
                        (checkpoint_ops.load)(exec_id.clone(), node_id.clone()).await
                {
                    cursor = updated_cp.cursor;
                    partial = updated_cp.partial_results;
                }
                iter_index = cursor;
                continue;
            }

            // ── 正常分支：本轮完成，把结果追加到 partial ──
            partial.push(last_step_output.clone());
            last_iter_index = iter_index as i32;

            // 广播 partial_result 事件（实时流式输出）
            // partial_result_var 通过 iter_ctx 注入到下游 body_step 即可，
            // 最终聚合通过 NodeOutput.output_var 落到 context。
            if let Some(tx) = &context.partial_result_tx {
                let _ = tx.send(PartialResultEvent {
                    execution_id: exec_id.clone(),
                    node_id: node_id.clone(),
                    iter_index,
                    item: item.clone(),
                    step_output: last_step_output,
                    cumulative_partial: partial.clone(),
                    phase: "completed".to_string(),
                    emitted_at_ms: chrono::Utc::now().timestamp_millis(),
                });
            }

            // interrupt_after_each：每轮都挂起
            if c.interrupt_after_each
                && let Some(sig) = &context.interrupt_signal
            {
                let cp = LoopCheckpoint {
                    execution_id: exec_id.clone(),
                    node_id: node_id.clone(),
                    cursor: iter_index + 1,
                    input_items: input_items.clone(),
                    partial_results: partial.clone(),
                    pending_approval_node: None,
                    pending_step_output: None,
                    saved_at_ms: chrono::Utc::now().timestamp_millis() as u64,
                    interrupting_step_id: None,
                };
                if let Some(checkpoint_ops) = context
                    .callbacks
                    .as_ref()
                    .and_then(|cb| cb.loop_checkpoint.clone())
                {
                    let _ = (checkpoint_ops.save)(cp).await;
                }
                tracing::info!(
                    execution_id = %exec_id,
                    node_id = %node_id,
                    iter_index,
                    "[Loop] interrupt_after_each 触发，挂起"
                );
                sig.notified().await;
                // resume 后读最新 cursor
                if let Some(checkpoint_ops) = context
                    .callbacks
                    .as_ref()
                    .and_then(|cb| cb.loop_checkpoint.clone())
                    && let Ok(Some(updated_cp)) =
                        (checkpoint_ops.load)(exec_id.clone(), node_id.clone()).await
                {
                    cursor = updated_cp.cursor;
                    partial = updated_cp.partial_results;
                }
                iter_index = cursor;
                continue;
            }

            // while/until 条件检查
            if matches!(c.loop_type, LoopType::While | LoopType::Until)
                && let Some(ref cond) = c.continue_condition
            {
                let should_stop = !evaluate_continue_condition(cond, &partial, iter_index);
                if should_stop {
                    break;
                }
            }

            iter_index += 1;
        }

        // ── 全部完成：清理检查点、聚合结果通过 output_var 暴露 ──
        if let Some(checkpoint_ops) = context
            .callbacks
            .as_ref()
            .and_then(|cb| cb.loop_checkpoint.clone())
        {
            let _ = (checkpoint_ops.delete)(exec_id.clone(), node_id.clone()).await;
        }

        // iter_output 的最终值通过 NodeOutput.output 暴露给引擎，由引擎统一
        // 写入 context.variables[output_var_key]（见 engine.rs 中对 NodeOutput 的处理）。
        // 这里仅做日志/审计参考。
        if let Some(ref pvar) = partial_var_key {
            tracing::debug!(
                execution_id = %exec_id,
                node_id = %node_id,
                partial_var = %pvar,
                iter_count = partial.len(),
                "[Loop] 全部完成，最终 partial_result 累计完成"
            );
        }

        let loop_type_label = match c.loop_type {
            LoopType::ForEach => "forEach",
            LoopType::While => "while",
            LoopType::DoWhile => "doWhile",
            LoopType::Until => "until",
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "loop_type": loop_type_label,
                "iter_count": partial.len(),
                "last_iter_index": last_iter_index,
                "resumed_from_checkpoint": resumed_from_checkpoint,
                "interrupted": false,
                "items": partial,
                "iter_output_var": output_var_key,
                "iter_input_var": input_var,
                "node_id": node_id,
            }),
            output_var: Some(output_var_key),
        })
    }
}

/// 简易 continue_condition 求值（true 表示继续）。
///
/// 支持的语法：
///  - 字面量 `true` / `false`
///  - 单个 `iter_index < N` / `iter_index >= N`（数字比较）
///
/// 不支持任意表达式 —— 复杂条件由用户在 body 内部用条件节点判断。
/// 返回 `true` 表示"继续"，`false` 表示"停止"。
fn evaluate_continue_condition(cond: &str, partial: &[serde_json::Value], iter_index: u32) -> bool {
    let trimmed = cond.trim();
    if trimmed == "true" {
        return true;
    }
    if trimmed == "false" {
        return false;
    }
    // 形式：`iter_index <op> <number>`
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() == 3 && parts[0] == "iter_index" {
        let op = parts[1];
        let rhs: u32 = match parts[2].parse() {
            Ok(n) => n,
            Err(_) => return true,
        };
        match op {
            "<" => return iter_index < rhs,
            "<=" => return iter_index <= rhs,
            ">" => return iter_index > rhs,
            ">=" => return iter_index >= rhs,
            "==" => return iter_index == rhs,
            "!=" => return iter_index != rhs,
            _ => {},
        }
    }
    // 形式：`partial.length <op> <number>`
    if parts.len() == 3 && parts[0] == "partial.length" {
        let op = parts[1];
        let rhs: usize = match parts[2].parse() {
            Ok(n) => n,
            Err(_) => return true,
        };
        let lhs = partial.len();
        match op {
            "<" => return lhs < rhs,
            "<=" => return lhs <= rhs,
            ">" => return lhs > rhs,
            ">=" => return lhs >= rhs,
            "==" => return lhs == rhs,
            "!=" => return lhs != rhs,
            _ => {},
        }
    }
    // 默认继续
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_interrupt_by_node_id() {
        let cfg = axagent_core::workflow_types::LoopNodeConfig {
            loop_type: LoopType::ForEach,
            items_var: None,
            iter_input_var: None,
            iteratee_var: None,
            iter_output_var: None,
            partial_result_var: None,
            max_iterations: None,
            continue_condition: None,
            continue_on_error: false,
            body_steps: vec!["approval".to_string()],
            sub_graph: None,
            interrupt_after_each: false,
            interrupt_nodes: vec!["approval".to_string()],
        };
        let out = serde_json::json!({"status": "pending"});
        let hit = detect_interrupt(&cfg, "approval", &out);
        assert!(hit.is_some());
    }

    #[test]
    fn detect_interrupt_by_pending_status() {
        let cfg = axagent_core::workflow_types::LoopNodeConfig {
            loop_type: LoopType::ForEach,
            items_var: None,
            iter_input_var: None,
            iteratee_var: None,
            iter_output_var: None,
            partial_result_var: None,
            max_iterations: None,
            continue_condition: None,
            continue_on_error: false,
            body_steps: vec!["approval".to_string()],
            sub_graph: None,
            interrupt_after_each: false,
            interrupt_nodes: vec![],
        };
        let out = serde_json::json!({"status": "pending", "msg": "review needed"});
        let hit = detect_interrupt(&cfg, "approval", &out);
        assert!(hit.is_some());
    }

    #[test]
    fn detect_interrupt_negative() {
        let cfg = axagent_core::workflow_types::LoopNodeConfig {
            loop_type: LoopType::ForEach,
            items_var: None,
            iter_input_var: None,
            iteratee_var: None,
            iter_output_var: None,
            partial_result_var: None,
            max_iterations: None,
            continue_condition: None,
            continue_on_error: false,
            body_steps: vec!["tool".to_string()],
            sub_graph: None,
            interrupt_after_each: false,
            interrupt_nodes: vec!["approval".to_string()],
        };
        let out = serde_json::json!({"result": "ok"});
        let hit = detect_interrupt(&cfg, "tool", &out);
        assert!(hit.is_none());
    }

    #[test]
    fn continue_condition_eval() {
        assert!(evaluate_continue_condition("true", &[], 0));
        assert!(!evaluate_continue_condition("false", &[], 0));
        assert!(evaluate_continue_condition("iter_index < 3", &[], 2));
        assert!(!evaluate_continue_condition("iter_index < 3", &[], 5));
        assert!(evaluate_continue_condition("partial.length < 2", &[json!(1)], 0));
        assert!(!evaluate_continue_condition("partial.length < 2", &[json!(1), json!(2)], 0));
    }
}
