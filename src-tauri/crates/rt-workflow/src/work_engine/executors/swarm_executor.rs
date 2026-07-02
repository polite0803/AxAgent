// SPDX-License-Identifier: AGPL-3.0-only

//! Swarm 容器执行器 —— 驱动多 Agent 团队协作执行。
//!
//! 通过 `debate_body_dispatch` 回调（同 LoopBodyDispatchFn 签名）分步驱动
//! 每个群组成员节点（LLM/Agent），在多轮协作中收集各方结果。
//!
//! 完整 Swarm 团队管理（跨进程 JSON 行协议通信、Teammate/Team CRUD）
//! 位于 runtime crate 的 swarm 模块中，供独立团队场景使用。
//! 本 Executor 提供 Workflow DAG 引擎内的 Swarm 节点执行能力。

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};
use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use std::collections::HashMap;

pub struct SwarmExecutor;
impl SwarmExecutor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for SwarmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for SwarmExecutor {
    fn node_type(&self) -> &'static str {
        "swarm"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Swarm(sn) = node else {
            return Err(NodeError::type_mismatch(
                "swarm".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        let agent_steps = &sn.config.agent_steps;
        let max_rounds = sn.config.max_rounds.max(1);
        let topic_var = &sn.config.topic_var;
        let convergence_prompt = sn.config.convergence_prompt.as_deref();
        let output_var = sn.config.output_var.clone();

        if agent_steps.is_empty() {
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "status": "no_agents",
                    "agent_steps": [],
                }),
                output_var: Some(output_var),
            });
        }

        let dispatch_fn = ctx
            .callbacks
            .as_ref()
            .and_then(|cb| cb.debate_body_dispatch.clone())
            .ok_or_else(|| {
                NodeError::exec_failed(
                    "swarm_no_dispatch",
                    "Swarm body dispatch callback not available. Ensure the engine sets debate_body_dispatch."
                        .to_string(),
                )
            })?;

        let topic = ctx
            .variables
            .get(topic_var.as_str())
            .cloned()
            .unwrap_or(serde_json::Value::String("swarm topic".to_string()));

        let mut round_outputs: Vec<HashMap<String, serde_json::Value>> = Vec::new();
        let mut prev_round_snapshot: Option<Vec<serde_json::Value>> = None;

        for round in 0..max_rounds {
            tracing::info!(
                "Swarm round {}/{} with {} agents",
                round + 1,
                max_rounds,
                agent_steps.len()
            );

            let mut round_results: HashMap<String, serde_json::Value> = HashMap::new();

            for step_id in agent_steps {
                let mut round_ctx = ctx.clone();
                round_ctx
                    .variables
                    .insert("__swarm_topic__".to_string(), topic.clone());
                round_ctx
                    .variables
                    .insert("__swarm_round__".to_string(), serde_json::json!(round));
                round_ctx
                    .variables
                    .insert("__swarm_max_rounds__".to_string(), serde_json::json!(max_rounds));
                // 注入收敛提示（LLM 可用此判断是否已达成共识）
                if let Some(cp) = convergence_prompt {
                    round_ctx.variables.insert(
                        "__swarm_convergence_prompt__".to_string(),
                        serde_json::Value::String(cp.to_string()),
                    );
                }
                // 注入前几轮输出供参考
                if let Some(ref snapshot) = prev_round_snapshot {
                    round_ctx
                        .variables
                        .insert("__swarm_history__".to_string(), serde_json::json!(snapshot));
                }

                match dispatch_fn(step_id.clone(), round_ctx).await {
                    Ok(output) => {
                        round_results.insert(step_id.clone(), output.output);
                    },
                    Err(e) => {
                        tracing::warn!(
                            "Swarm agent '{}' failed in round {}: {}",
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
                && Self::check_convergence(&round_results, &round_outputs[round.saturating_sub(1) as usize])
            {
                tracing::info!("Swarm converged at round {}/{}", round + 1, max_rounds);
                break;
            }

            prev_round_snapshot = Some(
                round_results
                    .values()
                    .cloned()
                    .collect::<Vec<serde_json::Value>>(),
            );
        }

        let final_output = serde_json::json!({
            "status": "completed",
            "total_rounds": round_outputs.len(),
            "max_rounds": max_rounds,
            "rounds": round_outputs,
            "consensus": Self::build_consensus(&round_outputs),
        });

        Ok(NodeOutput {
            output: final_output,
            output_var: Some(output_var),
        })
    }
}

impl SwarmExecutor {
    /// 简单收敛检测：基于相邻轮次输出内容相似度
    fn check_convergence(
        current: &HashMap<String, serde_json::Value>,
        previous: &HashMap<String, serde_json::Value>,
    ) -> bool {
        let mut matching = 0u32;
        let mut total = 0u32;
        for (key, cur_val) in current {
            if let Some(prev_val) = previous.get(key) {
                total += 1;
                if cur_val == prev_val {
                    matching += 1;
                } else {
                    let cur_str = serde_json::to_string(cur_val).unwrap_or_default();
                    let prev_str = serde_json::to_string(prev_val).unwrap_or_default();
                    if !cur_str.is_empty()
                        && !prev_str.is_empty()
                        && (cur_str.len().abs_diff(prev_str.len()) as f64
                            / prev_str.len().max(1) as f64)
                            < 0.10
                        && crate::work_engine::executors::simple_similarity(&cur_str, &prev_str)
                            > 0.85
                    {
                        matching += 1;
                        total += 1;
                    }
                }
            }
        }
        if total == 0 {
            return false;
        }
        matching as f64 / total as f64 >= 0.80
    }

    /// 从各轮输出中构建共识结果
    fn build_consensus(round_outputs: &[HashMap<String, serde_json::Value>]) -> serde_json::Value {
        if let Some(last_round) = round_outputs.last() {
            let entries: Vec<serde_json::Value> = last_round
                .iter()
                .map(|(step_id, output)| {
                    serde_json::json!({
                        "agent": step_id,
                        "output": output,
                    })
                })
                .collect();
            serde_json::json!({
                "entries": entries,
                "total_rounds": round_outputs.len(),
            })
        } else {
            serde_json::json!({
                "entries": [],
                "total_rounds": 0,
            })
        }
    }
}
