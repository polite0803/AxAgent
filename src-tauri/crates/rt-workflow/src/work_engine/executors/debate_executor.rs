// SPDX-License-Identifier: AGPL-3.0-only

//! 辩论容器执行器 —— 通过 `debate_body_dispatch` 回调驱动多轮辩论。
//!
//! 与 SwarmExecutor 语义一致（多 Agent × 多轮 + 相邻轮次相似度收敛），
//! 区别在于语义标签（debate vs swarm）和注入的上下文变量名。
//! 共享收敛检测与共识构建逻辑见 `executors::check_round_convergence` /
//! `executors::build_round_consensus`。
//!
//! 设计要点：
//!  1) 读取 `debater_steps` 配置，按 `max_rounds` 顺序驱动每个辩手节点。
//!  2) 通过 `ExecutionState.callbacks.debate_body_dispatch` 回调驱动辩手节点
//!     （回调由 `WorkEngine::build_debate_body_dispatch` 工厂构造，内部走
//!     dispatcher，保留 progress_callback / 节点状态切换 / node_records 统一埋点）。
//!  3) 每轮注入 `__debate_topic__` / `__debate_round__` / `__debate_history__`
//!     / `__debate_convergence_prompt__` 供辩手 LLM 参考上下文。
//!  4) 从第 2 轮起做收敛检测（相邻轮次输出相似度 >= 0.80 即停止）。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;
use std::collections::HashMap;

pub struct DebateExecutor;
impl DebateExecutor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for DebateExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for DebateExecutor {
    fn node_type(&self) -> &'static str {
        "debate"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Debate(dn) = node else {
            return Err(NodeError::type_mismatch(
                "debate".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        let debater_steps = &dn.config.debater_steps;
        let max_rounds = dn.config.max_rounds.max(1);
        let topic_var = &dn.config.topic_var;
        let convergence_prompt = dn.config.convergence_prompt.as_deref();
        let output_var = dn.config.output_var.clone();

        if debater_steps.is_empty() {
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "status": "no_debaters",
                    "debater_steps": [],
                }),
                output_var: Some(output_var),
                control: None,
            });
        }

        let dispatch_fn = ctx
            .callbacks
            .as_ref()
            .and_then(|cb| cb.debate_body_dispatch.clone())
            .ok_or_else(|| {
                NodeError::exec_failed(
                    "debate_no_dispatch",
                    "Debate body dispatch callback not available. Ensure the engine sets debate_body_dispatch."
                        .to_string(),
                )
            })?;

        let topic = ctx
            .variables
            .get(topic_var.as_str())
            .cloned()
            .unwrap_or(serde_json::Value::String("debate topic".to_string()));

        let mut round_outputs: Vec<HashMap<String, serde_json::Value>> = Vec::new();
        let mut prev_round_snapshot: Option<Vec<serde_json::Value>> = None;

        for round in 0..max_rounds {
            tracing::info!(
                "Debate round {}/{} with {} debaters",
                round + 1,
                max_rounds,
                debater_steps.len()
            );

            let mut round_results: HashMap<String, serde_json::Value> = HashMap::new();

            for step_id in debater_steps {
                let mut round_ctx = ctx.clone();
                round_ctx.variables.insert("__debate_topic__".to_string(), topic.clone());
                round_ctx
                    .variables
                    .insert("__debate_round__".to_string(), serde_json::json!(round));
                round_ctx
                    .variables
                    .insert("__debate_max_rounds__".to_string(), serde_json::json!(max_rounds));
                // 注入收敛提示（辩手可用此判断是否已达成共识）
                if let Some(cp) = convergence_prompt {
                    round_ctx.variables.insert(
                        "__debate_convergence_prompt__".to_string(),
                        serde_json::Value::String(cp.to_string()),
                    );
                }
                // 注入前几轮输出供参考
                if let Some(ref snapshot) = prev_round_snapshot {
                    round_ctx
                        .variables
                        .insert("__debate_history__".to_string(), serde_json::json!(snapshot));
                }

                match dispatch_fn(step_id.clone(), round_ctx).await {
                    Ok(output) => {
                        round_results.insert(step_id.clone(), output.output);
                    },
                    Err(e) => {
                        tracing::warn!(
                            "Debate debater '{}' failed in round {}: {}",
                            step_id,
                            round,
                            e
                        );
                        round_results.insert(
                            step_id.clone(),
                            serde_json::json!({
                                "error": e.to_string(),
                                "round": round,
                            }),
                        );
                    },
                }
            }

            round_outputs.push(round_results.clone());

            // 收敛检测（从第 2 轮开始）
            if round > 0
                && convergence_prompt.is_some()
                && super::check_round_convergence(
                    &round_results,
                    &round_outputs[round.saturating_sub(1) as usize],
                )
            {
                tracing::info!("Debate converged at round {}/{}", round + 1, max_rounds);
                break;
            }

            prev_round_snapshot =
                Some(round_results.values().cloned().collect::<Vec<serde_json::Value>>());
        }

        let final_output = serde_json::json!({
            "status": "completed",
            "total_rounds": round_outputs.len(),
            "max_rounds": max_rounds,
            "rounds": round_outputs,
            "consensus": super::build_round_consensus(&round_outputs),
        });

        Ok(NodeOutput { output: final_output, output_var: Some(output_var), control: None })
    }
}
