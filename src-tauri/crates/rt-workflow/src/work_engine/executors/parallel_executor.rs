// SPDX-License-Identifier: AGPL-3.0-only

//! ParallelExecutor —— **声明式并行节点**。
//!
//! ## 重要：当前实际行为（与命名不完全一致）
//!
//! `Parallel` 节点**不**在 execute() 内真正派发分支子节点。execute() 只做：
//! 1. 校验 `branches` 非空
//! 2. 为每个分支准备 `branch_inputs`（按 `auto_input_from_parent` 继承父 variables
//!    快照或用空对象）
//! 3. 输出 `branch_configs`（超时 / 降级策略）让上游调度器（WorkEngine 的
//!    `dispatch_node` 路径）拿到后做真正的并行 spawn。
//!
//! 也就是说：**真正的并发调度是 WorkEngine 在 DAG 调度阶段完成的**，
//! Parallel 节点只是元数据提供方。读者请勿在 execute() 中查找 `tokio::join!` /
//! `futures::join!` —— 它们在 `WorkEngine::execute_node` 调度的下游
//! `loop_partial_txs` / `get_ready_steps` 路径里。
//!
//! ## 历史与未来
//!
//! 早期实现尝试在 execute() 内 join 所有分支，但遇到两个问题：
//! 1. `NodeOutput` 是同步返回类型，无法表达 join 后的"未来值"；
//! 2. 每个分支可能又是 DAG 子图（嵌套 Parallel / Loop），同步 join 会无限递归。
//!
//! 因此改为"声明 + 调度分离"：本节点声明并行意图 + 输入 + 策略，WorkEngine
//! 真正在调度循环里 spawn 各分支任务并 collect 结果。
//!
//! 关联：见 `WorkEngine::dispatch_node` 与 `loop_partial_txs` 的下游使用。
//!
//! ## 配置字段语义
//!
//! - `branches`: 至少 1 个分支，每个含 `id` / `input_var` / `branch_timeout_ms` /
//!   `degrade_strategy`
//! - `auto_input_from_parent`: true 时分支继承父 variables 快照；false 时要求
//!   分支显式声明 `input_var`（由上游节点显式填充）
//! - `wait_for_all`: true → 等所有分支完成（失败由 degrade_strategy 处理）；
//!   false → 任意分支成功即可（race/any）
//! - `aggregation`: All / Any / Race / Majority，决定如何从分支结果合成最终输出

use async_trait::async_trait;
use axagent_core::workflow_types::{MergeStrategy, WorkflowNode};

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

pub struct ParallelExecutor;

impl ParallelExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 把 auto_input_from_parent + wait_for_all 翻译为 engine 调度时需要的信息，
/// 并把当前 context 的 variables 拷贝成可被下游分支读取的"父输入"。
///
/// 同时输出每个分支的超时和降级策略，供 engine 的 spawner 在分支子节点超时时
/// 根据 degrade_strategy 做降级处理。
#[async_trait]
impl NodeExecutorTrait for ParallelExecutor {
    fn node_type(&self) -> &'static str {
        "parallel"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Parallel(n) = node else {
            return Err(NodeError::type_mismatch("parallel", self.node_type()));
        };
        let c = &n.config;

        if c.branches.is_empty() {
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                "Parallel node has no branches".to_string(),
            ));
        }

        // 收集每个 branch 的入口数据：auto_input_from_parent=true 时继承 context.variables
        // 快照，否则要求显式 input_var。
        let mut branch_inputs = serde_json::Map::new();
        let mut branch_configs = serde_json::Map::new();
        for branch in &c.branches {
            let value = if c.auto_input_from_parent {
                serde_json::to_value(&context.variables).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            };
            branch_inputs.insert(branch.id.clone(), value);

            // 输出分支级别的超时和降级配置
            branch_configs.insert(
                branch.id.clone(),
                serde_json::json!({
                    "timeout_ms": branch.branch_timeout_ms,
                    "degrade_strategy": branch.degrade_strategy,
                }),
            );
        }

        let aggregation = c.aggregation.clone().unwrap_or_default();
        let merge_label = match aggregation {
            MergeStrategy::All => "all",
            MergeStrategy::Any => "any",
            MergeStrategy::Race => "race",
            MergeStrategy::Majority => "majority",
        };

        Ok(NodeOutput {
            output: serde_json::json!({
                "branch_count": c.branches.len(),
                "wait_for_all": c.wait_for_all,
                "timeout": c.timeout,
                "aggregation": merge_label,
                "auto_input_from_parent": c.auto_input_from_parent,
                "branch_inputs": branch_inputs,
                "branch_configs": branch_configs,
                "node_id": node.base_id(),
            }),
            output_var: None,
        })
    }
}
