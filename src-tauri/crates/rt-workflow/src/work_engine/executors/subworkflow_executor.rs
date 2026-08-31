// SPDX-License-Identifier: AGPL-3.0-only

//! 子工作流执行器 —— 通过引擎内递归执行运行嵌套工作流。
//!
//! 从 ExecutionState.callbacks.subworkflow 获取引擎回调，
//! 直接调用 WorkEngine.run_workflow() 执行子工作流，产生独立 ExecutionState，
//! 支持 parent_execution_id 关联和子执行记录追踪。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, check_cancellation_or_pause, error_code,
};
use async_trait::async_trait;
use axagent_harness::workflow_types::{SubWorkflowNode, WorkflowNode};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
/// 子执行输出 future 类型：成功返回 `(child_execution_id, output)`，失败返回错误串
pub type SubWorkflowOutputFuture =
    Pin<Box<dyn std::future::Future<Output = Result<(String, Value), String>> + Send>>;

/// 子工作流启动句柄 —— 回调在同步阶段即确定子执行 ID 与取消能力。
///
/// 旧实现仅返回 `Future<Output=(child_execution_id, output)>`，父执行在
/// `tokio::time::timeout` 触发时直接把该 future drop 掉。但子执行运行在
/// `spawn_blocking` + thread-local runtime 中，drop future 无法中止 blocking
/// 任务，导致子工作流成为「孤儿执行」继续占用 CPU / 写库。改为同步返回句柄后，
/// 超时路径仍能拿到子执行 ID 并主动 `engine.cancel(child_execution_id)` 回收。
pub struct SubWorkflowLaunch {
    /// 子执行的 execution_id（调用回调时即已确定，无需等待 output future）
    pub child_execution_id: String,
    /// 子执行输出 future，成功返回 `(child_execution_id, output)`，失败返回错误串
    pub output: SubWorkflowOutputFuture,
    /// 主动取消子执行（若仍在运行）。System capability 等同步回调可返回空取消。
    pub cancel: Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
}

/// 子工作流引擎回调 — 接收 (sub_workflow_id, parent_execution_id, input)，
/// 返回启动句柄（含子执行 ID 与取消能力）。内部由 WorkEngine.run_workflow 实现。
pub type SubWorkflowCallback =
    Arc<dyn Fn(String, String, HashMap<String, Value>) -> SubWorkflowLaunch + Send + Sync>;

#[derive(Debug, Clone)]
pub struct SubWorkflowExecutorConfig {
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub cache_enabled: bool,
    pub cache_ttl_secs: u64,
}
impl Default for SubWorkflowExecutorConfig {
    fn default() -> Self {
        Self { timeout_secs: 300, max_retries: 3, cache_enabled: true, cache_ttl_secs: 300 }
    }
}

#[derive(Clone)]
pub struct SubWorkflowExecutor {
    config: SubWorkflowExecutorConfig,
}

/// [`SubWorkflowExecutor::execute_with_retry`] 的运行参数（收敛 8 个散参，clippy `too_many_arguments`）
struct RetryParams<'a> {
    cb: &'a SubWorkflowCallback,
    sub_workflow_id: String,
    parent_execution_id: String,
    input: HashMap<String, Value>,
    max_retries: u32,
    cancel_token: Option<&'a tokio_util::sync::CancellationToken>,
    timeout: Duration,
    child_tracker: &'a Option<Arc<Mutex<HashSet<String>>>>,
}

impl SubWorkflowExecutor {
    pub fn new() -> Self {
        Self::with_config(SubWorkflowExecutorConfig::default())
    }
    pub fn with_config(config: SubWorkflowExecutorConfig) -> Self {
        Self { config }
    }

    fn map_inputs(
        node: &SubWorkflowNode,
        context: &ExecutionState,
    ) -> Result<HashMap<String, Value>, NodeError> {
        let mut mapped = HashMap::new();
        for (target_var, source_var) in &node.config.input_mapping {
            // 支持点分隔路径（如 "l1_result.domain"），与 condition_executor 等保持一致。
            let value = resolve_var_path(source_var, context).ok_or_else(|| {
                NodeError::exec_failed(
                    error_code::SUBWORKFLOW_FAILED,
                    format!("Variable '{}' not found", source_var),
                )
            })?;
            mapped.insert(target_var.clone(), value);
        }
        Ok(mapped)
    }

    async fn execute_with_retry(p: RetryParams<'_>) -> Result<(String, Value), NodeError> {
        let RetryParams {
            cb,
            sub_workflow_id,
            parent_execution_id,
            input,
            max_retries,
            cancel_token,
            timeout,
            child_tracker,
        } = p;
        let mut last_error = None;
        // 总子执行预算：合并所有重试，自首次调用起计（兼容旧语义：父级 300s
        // 上限对「所有重试的累计运行时长」生效），避免无限重试越过上限。
        let deadline = tokio::time::Instant::now() + timeout;
        for attempt in 1..=max_retries + 1 {
            // 同步从回调拿到启动句柄（含子执行 ID 与取消能力），
            // 使超时路径仍能主动取消孤儿子执行，而非 drop future 后无人回收。
            let launch = cb(sub_workflow_id.clone(), parent_execution_id.clone(), input.clone());
            let child_eid = launch.child_execution_id;
            // 登记到共享跟踪器：若本节点级超时导致整个 dispatch future 被丢弃，
            // 引擎仍能通过跟踪器定位并 cancel 该孤儿子执行（真正的回收兜底）。
            if let Some(tr) = child_tracker {
                tr.lock().unwrap().insert(child_eid.clone());
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                // 预算已耗尽且仍未成功 —— 以最后一次错误退出
                if let Some(tr) = child_tracker {
                    tr.lock().unwrap().remove(&child_eid);
                }
                break;
            }
            match tokio::time::timeout(remaining, launch.output).await {
                Ok(Ok(result)) => {
                    if let Some(tr) = child_tracker {
                        tr.lock().unwrap().remove(&child_eid);
                    }
                    return Ok(result);
                },
                Ok(Err(e)) => {
                    if let Some(tr) = child_tracker {
                        tr.lock().unwrap().remove(&child_eid);
                    }
                    let err_msg = e.clone();
                    last_error = Some(e);
                    if attempt > max_retries {
                        break;
                    }
                    if let Some(token) = cancel_token
                        && token.is_cancelled()
                    {
                        return Err(NodeError::exec_failed(
                            error_code::SUBWORKFLOW_FAILED,
                            "Parent cancelled - abort retry".to_string(),
                        ));
                    }
                    // P1-12: 指数退避 + jitter —— 避免多个子工作流同时重试造成雪崩
                    // base = 100ms * 2^(attempt-1); cap = 30s；退避不得越过总预算
                    let base_ms = (100u64).saturating_mul(1u64 << (attempt - 1).min(8));
                    let capped_ms = base_ms.min(30_000);
                    let jitter_ms = rand::random::<u64>() % (capped_ms / 2).max(1);
                    let delay = Duration::from_millis(capped_ms + jitter_ms).min(remaining);
                    tracing::warn!(
                        sub_workflow_id = %sub_workflow_id,
                        attempt,
                        delay_ms = delay.as_millis() as u64,
                        error = %err_msg,
                        "Sub-workflow 失败，指数退避+jitter 后重试"
                    );
                    tokio::time::sleep(delay).await;
                },
                Err(_) => {
                    // 父执行超时（预算耗尽）：子执行在 spawn_blocking +
                    // thread-local runtime 中仍可能运行，主动取消避免孤儿执行。
                    tracing::warn!(
                        sub_workflow_id = %sub_workflow_id,
                        child_execution_id = %child_eid,
                        "Sub-workflow 父执行超时，主动取消孤儿子执行"
                    );
                    launch.cancel.await;
                    if let Some(tr) = child_tracker {
                        tr.lock().unwrap().remove(&child_eid);
                    }
                    return Err(NodeError::timed_out(
                        error_code::SUBWORKFLOW_FAILED,
                        format!("Sub-workflow timeout({}s)", timeout.as_secs()),
                    ));
                },
            }
        }
        Err(last_error
            .map(|e| NodeError::exec_failed(error_code::SUBWORKFLOW_FAILED, e))
            .unwrap_or_else(|| {
                NodeError::exec_failed(
                    error_code::SUBWORKFLOW_FAILED,
                    "Sub-workflow execution failed".to_string(),
                )
            }))
    }
}

impl Default for SubWorkflowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for SubWorkflowExecutor {
    fn node_type(&self) -> &'static str {
        "subWorkflow"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        // 执行前检查取消/暂停状态
        check_cancellation_or_pause(context).await?;

        let sub_node = match node {
            WorkflowNode::SubWorkflow(s) => s,
            _ => {
                return Err(NodeError::type_mismatch(
                    "subWorkflow".to_string(),
                    super::node_type_name(node).to_string(),
                ));
            },
        };

        // ── Dry Run 短路 ──
        // 单步调试模式下不真正递归执行子工作流（避免级联副作用、长时间运行），
        // 返回模拟执行结果。子工作流 ID 与映射输入保留以供下游节点识别节点配置。
        //
        // 注意：此前实现在 execute_with_retry 完成后才检查 dry_run，
        // 导致 dry_run 模式下仍然真正执行子工作流，违背 dry_run 语义。
        if context.dry_run {
            let mapped_input = Self::map_inputs(sub_node, context)?;
            tracing::info!(
                "[SubWorkflowExecutor] dry_run 模式：子工作流 '{}' 短路返回模拟结果",
                sub_node.config.sub_workflow_id
            );
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "status": "dry_run",
                    "sub_workflow_id": sub_node.config.sub_workflow_id,
                    "input": mapped_input,
                    "result": "[DRY RUN] 子工作流模拟执行结果",
                    "dry_run": true,
                    "node_id": node.base_id(),
                }),
                output_var: Some(sub_node.config.output_var.clone()),
                control: None,
            });
        }

        let sub_workflow_id = &sub_node.config.sub_workflow_id;
        // h3：system_* 前缀视为系统能力节点（认知编排器的 L1/L2/RAR/图谱等），
        // 由引擎系统能力回调执行，不回退查询 workflow_templates 表。
        let is_system_capability = sub_workflow_id.starts_with("system_");
        let cb = context
            .callbacks
            .as_ref()
            .and_then(|cbs| {
                if is_system_capability {
                    cbs.system_capability.clone()
                } else {
                    cbs.subworkflow.clone()
                }
            })
            .ok_or_else(|| {
                NodeError::exec_failed(
                    error_code::SUBWORKFLOW_NOT_CONFIGURED,
                    if is_system_capability {
                        "System capability callback not configured".to_string()
                    } else {
                        "Sub-workflow engine callback not configured".to_string()
                    },
                )
            })?;

        let mapped_input = Self::map_inputs(sub_node, context)?;

        let timeout = Duration::from_secs(self.config.timeout_secs);
        // 超时预算已下沉到 execute_with_retry（对其所有重试的累计时长生效），
        // 超时触发时会在内部主动 cancel 孤儿子执行，无需在此再包一层 timeout。
        let (child_execution_id, output) = Self::execute_with_retry(RetryParams {
            cb: &cb,
            sub_workflow_id: sub_node.config.sub_workflow_id.clone(),
            parent_execution_id: context.execution_id.clone(),
            input: mapped_input,
            max_retries: self.config.max_retries,
            cancel_token: context.cancel_token.as_ref(),
            timeout,
            child_tracker: &context.child_executions,
        })
        .await?;

        let child_eid_value = serde_json::Value::String(child_execution_id.clone());

        // dry_run 已在前面短路，此处不会到达；移除原 dry_run 后处理逻辑。
        let mut enriched_output = if output.is_object() {
            let mut obj = output.as_object().cloned().unwrap_or_default();
            // End 节点 wrapper 拆包：子工作流正常终止时最终输出为
            // EndExecutor 构造的 {status:"terminated", node_id, output:<实际结果>, source}。
            // 将实际结果平铺提升到顶层，供主 DAG 按字段直接消费
            // （如 l1_result.category / l1_result.confidence），否则主 DAG
            // 只能读到嵌套在 output 里的字段，条件/断言全部判 null。
            if obj.get("status").and_then(|v| v.as_str()) == Some("terminated")
                && let Some(inner) = obj.remove("output")
            {
                match inner {
                    serde_json::Value::Object(inner_map) => {
                        for (k, v) in inner_map {
                            obj.entry(k).or_insert(v);
                        }
                    },
                    serde_json::Value::Null => {},
                    other => {
                        obj.entry("result".to_string()).or_insert(other);
                    },
                }
            }
            serde_json::Value::Object(obj)
        } else {
            serde_json::Value::Null
        };
        if !enriched_output.is_object() {
            enriched_output = serde_json::json!({
                "result": output,
            });
        }
        tracing::info!(
            sub_workflow_id = %sub_node.config.sub_workflow_id,
            output_var = %sub_node.config.output_var,
            output_preview = %output,
            enriched_preview = %enriched_output,
            "🔍 [DIAG] SubWorkflowExecutor 子工作流执行完毕 — 原始 output 和 解包后 enriched_output"
        );

        if let Some(obj) = enriched_output.as_object_mut() {
            obj.insert("_child_execution_id".to_string(), child_eid_value.clone());
        }

        tracing::info!(
            sub_workflow_id = %sub_node.config.sub_workflow_id,
            final_output = %enriched_output,
            "🔍 [DIAG] SubWorkflowExecutor 返回前 — 最终写入父工作流变量的值"
        );

        Ok(NodeOutput {
            output: enriched_output,
            output_var: Some(sub_node.config.output_var.clone()),
            control: None,
        })
    }
}

/// 从 ExecutionState 变量中解析点分隔路径（与 tool_executor / condition_executor
/// / switch_executor / validation_executor / llm_classifier_executor 保持一致）。
///
/// 解析规则：
/// 1. 空路径直接返回 `None`
/// 2. 尝试按节点输出路径解析：`root = context.variables.get(parts[0])`，
///    然后沿 `parts[1..]` 逐层下钻嵌套字段
/// 3. fallback：root 不是节点 ID 时，将整个 `path` 作为模板变量名直查
fn resolve_var_path(path: &str, context: &ExecutionState) -> Option<serde_json::Value> {
    if path.is_empty() {
        return None;
    }
    let parts: Vec<&str> = path.split('.').collect();
    if let Some(root) = context.variables.get(parts[0]) {
        let mut current = root.clone();
        for part in &parts[1..] {
            current = current.get(part)?.clone();
        }
        return Some(current);
    }
    context.variables.get(path).cloned()
}
