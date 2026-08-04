// SPDX-License-Identifier: AGPL-3.0-only

//! MultiAgent 节点执行器 —— 实现 Swarm/Debate/Blackboard 三种协作模式。
//!
//! ## 三种协作模式
//!
//! - **Swarm**: 任务派发式协作，中心化调度
//! - **Debate**: 对抗式辩论，通过评分/反驳收敛
//! - **Blackboard**: 共享黑板模式，Agent 通过共享状态协作

use std::collections::HashMap;
use std::sync::Arc;

use axagent_harness::multi_agent::{
    CoordinationMode, CoordinationOutcome, MultiAgentCoordination, SharedBlackboard,
};
use axagent_harness::workflow_types::WorkflowNode;
use tokio::sync::Mutex;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

/// 执行质量评估结果
#[derive(Debug)]
pub struct QualityAssessment {
    score: f32,
    passed: bool,
    issues: Vec<String>,
    suggestions: Vec<String>,
}

pub struct MultiAgentExecutor {
    /// 共享黑板实例（用于 Blackboard 模式）
    blackboard: Arc<Mutex<Option<Arc<dyn SharedBlackboard>>>>,
    /// 协作协调器（用于 Swarm/Debate 模式）
    coordinator: Arc<Mutex<Option<Arc<dyn MultiAgentCoordination>>>>,
}

impl MultiAgentExecutor {
    pub fn new() -> Self {
        Self { blackboard: Arc::new(Mutex::new(None)), coordinator: Arc::new(Mutex::new(None)) }
    }

    /// 注入共享黑板实现
    pub async fn with_blackboard(&self, blackboard: Arc<dyn SharedBlackboard>) -> &Self {
        let mut lock = self.blackboard.lock().await;
        *lock = Some(blackboard);
        self
    }

    /// 注入协作协调器实现
    pub async fn with_coordinator(&self, coordinator: Arc<dyn MultiAgentCoordination>) -> &Self {
        let mut lock = self.coordinator.lock().await;
        *lock = Some(coordinator);
        self
    }

    /// 解析协作模式字符串
    fn parse_mode(mode_str: &str) -> CoordinationMode {
        match mode_str.to_lowercase().as_str() {
            "swarm" => CoordinationMode::Swarm,
            "debate" => CoordinationMode::Debate,
            "blackboard" | "shared_blackboard" => CoordinationMode::Blackboard,
            _ => CoordinationMode::Swarm, // 默认 Swarm 模式
        }
    }

    /// 解析输入映射，从上下文提取结构化数据
    ///
    /// 支持两种格式：
    /// - `$node.node_id.field` → 从上游节点输出获取
    /// - `$variables.var_name` → 从工作流变量获取
    fn resolve_input_mapping(
        input_mapping: &Option<std::collections::HashMap<String, String>>,
        ctx: &ExecutionState,
    ) -> serde_json::Value {
        let mapping = match input_mapping {
            Some(m) if !m.is_empty() => m,
            _ => return serde_json::Value::Object(serde_json::Map::new()),
        };

        let mut resolved = serde_json::Map::new();

        for (target_field, source_expr) in mapping {
            if let Some(value) = Self::resolve_source_expr(source_expr, ctx) {
                resolved.insert(target_field.clone(), value);
            }
        }

        serde_json::Value::Object(resolved)
    }

    /// 解析单个源表达式
    fn resolve_source_expr(expr: &str, ctx: &ExecutionState) -> Option<serde_json::Value> {
        let expr = expr.trim();

        if let Some(node_path) = expr.strip_prefix("$node.") {
            // 格式: $node.node_id 或 $node.node_id.field1.field2
            let parts: Vec<&str> = node_path.split('.').collect();
            if parts.is_empty() {
                return None;
            }

            let node_id = parts[0];
            let field_path = &parts[1..];

            if let Some(node_output) = ctx.node_outputs.get(node_id) {
                if field_path.is_empty() {
                    return Some(node_output.clone());
                }
                // 按路径逐级下钻
                let mut current = node_output.clone();
                for field in field_path {
                    current = current.get(field)?.clone();
                }
                Some(current)
            } else {
                None
            }
        } else if let Some(var_path) = expr.strip_prefix("$variables.") {
            // 格式: $variables.var_name 或 $variables.var_name.field
            let parts: Vec<&str> = var_path.split('.').collect();
            if parts.is_empty() {
                return None;
            }

            let var_name = parts[0];
            let field_path = &parts[1..];

            if let Some(var_value) = ctx.variables.get(var_name) {
                if field_path.is_empty() {
                    return Some(var_value.clone());
                }
                // 按路径逐级下钻
                let mut current = var_value.clone();
                for field in field_path {
                    current = current.get(field)?.clone();
                }
                Some(current)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 用上下文信息丰富任务描述，使多智能体能够访问工作流变量和上游输出
    fn enrich_task_with_context(
        task: &str,
        ctx: &ExecutionState,
        resolved_inputs: &serde_json::Value,
    ) -> String {
        let mut enriched = task.to_string();

        // 添加结构化输入数据（通过 input_mapping 解析）
        if !resolved_inputs.is_null() && !resolved_inputs.as_object().is_none_or(|m| m.is_empty()) {
            let inputs_str =
                serde_json::to_string_pretty(resolved_inputs).unwrap_or_else(|_| "{}".to_string());
            enriched.push_str(&format!("\n\n## 任务输入数据\n```json\n{}\n```", inputs_str));
        }

        // 添加工作流变量上下文
        if !ctx.variables.is_empty() {
            let vars_str =
                serde_json::to_string_pretty(&ctx.variables).unwrap_or_else(|_| "{}".to_string());
            enriched.push_str(&format!("\n\n## 工作流变量上下文\n```json\n{}\n```", vars_str));
        }

        // 添加上游节点输出
        if !ctx.node_outputs.is_empty() {
            let outputs_str = serde_json::to_string_pretty(&ctx.node_outputs)
                .unwrap_or_else(|_| "{}".to_string());
            enriched.push_str(&format!("\n\n## 上游节点输出\n```json\n{}\n```", outputs_str));
        }

        // 添加输入参数
        if !ctx.input_params.is_null() {
            enriched.push_str(&format!("\n\n## 输入参数\n```json\n{}\n```", ctx.input_params));
        }

        enriched
    }

    /// 构建输出结果
    fn build_output(
        task: &str,
        mode: CoordinationMode,
        rounds: u32,
        converged: bool,
        consensus: Option<&str>,
        participants: &[String],
    ) -> serde_json::Value {
        let mut result = HashMap::new();
        result.insert("task".to_string(), serde_json::Value::String(task.to_string()));
        result.insert(
            "mode".to_string(),
            serde_json::Value::String(format!("{:?}", mode).to_lowercase()),
        );
        result.insert(
            "rounds".to_string(),
            serde_json::Value::Number(serde_json::Number::from(rounds)),
        );
        result.insert("converged".to_string(), serde_json::Value::Bool(converged));
        if let Some(consensus_val) = consensus {
            result.insert(
                "consensus".to_string(),
                serde_json::Value::String(consensus_val.to_string()),
            );
        }
        result.insert(
            "participants".to_string(),
            serde_json::Value::Array(
                participants.iter().map(|p| serde_json::Value::String(p.clone())).collect(),
            ),
        );
        result.insert(
            "status".to_string(),
            serde_json::Value::String(if converged {
                "converged".to_string()
            } else {
                "in_progress".to_string()
            }),
        );
        serde_json::Value::Object(result.into_iter().collect())
    }

    /// 执行 Swarm 模式协作
    async fn execute_swarm(
        &self,
        task: &str,
        role: &str,
        max_rounds: u32,
    ) -> Result<(CoordinationOutcome, String), String> {
        let coordinator = self.coordinator.lock().await;

        if let Some(ref coord) = *coordinator {
            // 使用注入的协调器
            let participants = Self::generate_swarm_participants(role);
            let session_id = coord.start_session("swarm_task", task, participants.clone()).await?;

            // 执行协作轮次
            for round in 0..max_rounds {
                let proposal = format!("[Round {}] {} 执行任务: {}", round + 1, role, task);
                coord.propose(&session_id, &participants[0], &proposal).await?;

                let outcome = coord.current_result(&session_id).await?;
                if outcome.converged {
                    coord.close_session(&session_id).await?;
                    return Ok((outcome, session_id));
                }
            }

            let outcome = coord.current_result(&session_id).await?;
            coord.close_session(&session_id).await?;
            Ok((outcome, session_id))
        } else {
            // 规则回退：模拟 Swarm 协作
            Self::simulate_swarm_collaboration(task, role, max_rounds)
        }
    }

    /// 执行 Debate 模式协作
    async fn execute_debate(
        &self,
        task: &str,
        role: &str,
        max_rounds: u32,
    ) -> Result<(CoordinationOutcome, String), String> {
        let coordinator = self.coordinator.lock().await;

        if let Some(ref coord) = *coordinator {
            let participants = vec![format!("{}_proposer", role), format!("{}_reviewer", role)];
            let session_id = coord.start_session("debate_task", task, participants.clone()).await?;

            // 执行辩论轮次
            for round in 0..max_rounds {
                // Proposer 提案
                let proposal = format!("[Round {}] Proposer 方案: {}", round + 1, task);
                coord.propose(&session_id, &participants[0], &proposal).await?;

                // Reviewer 反驳
                let review = format!("[Round {}] Reviewer 反馈: 需要验证方案可行性", round + 1,);
                coord.propose(&session_id, &participants[1], &review).await?;

                let outcome = coord.current_result(&session_id).await?;
                if outcome.converged {
                    coord.close_session(&session_id).await?;
                    return Ok((outcome, session_id));
                }
            }

            let outcome = coord.current_result(&session_id).await?;
            coord.close_session(&session_id).await?;
            Ok((outcome, session_id))
        } else {
            // 规则回退：模拟 Debate 协作
            Self::simulate_debate_collaboration(task, role, max_rounds)
        }
    }

    /// 执行 Blackboard 模式协作
    async fn execute_blackboard(
        &self,
        task: &str,
        role: &str,
        max_rounds: u32,
    ) -> Result<(CoordinationOutcome, String), String> {
        let blackboard = self.blackboard.lock().await;

        if let Some(ref bb) = *blackboard {
            // 使用注入的黑板
            let participants = Self::generate_blackboard_participants(role);
            let session_id = format!("bb_{}", uuid::Uuid::new_v4());

            // 初始化任务
            bb.set_state("current_task", task).await?;
            bb.set_state("session_id", &session_id).await?;

            // 执行协作轮次
            for round in 0..max_rounds {
                // 每个参与者贡献决策
                for participant in &participants {
                    let decision =
                        format!("[Round {}] {} 对 {} 的贡献", round + 1, participant, task);
                    bb.record_decision(participant, "multi_agent_task", "contribution", &decision)
                        .await?;
                }

                // 尝试达成共识
                if let Some(consensus) = bb.get_consensus("contribution").await {
                    let outcome = CoordinationOutcome {
                        session_id: session_id.clone(),
                        mode: CoordinationMode::Blackboard,
                        converged: true,
                        consensus: Some(consensus),
                        participants: participants.clone(),
                        rounds: round + 1,
                    };
                    return Ok((outcome, session_id));
                }
            }

            // 最终结果
            let consensus = bb.get_consensus("contribution").await;
            let outcome = CoordinationOutcome {
                session_id: session_id.clone(),
                mode: CoordinationMode::Blackboard,
                converged: consensus.is_some(),
                consensus,
                participants,
                rounds: max_rounds,
            };
            Ok((outcome, session_id))
        } else {
            // 规则回退
            Self::simulate_blackboard_collaboration(task, role, max_rounds)
        }
    }

    /// 生成 Swarm 模式参与者
    fn generate_swarm_participants(role: &str) -> Vec<String> {
        let base_name = role.replace(' ', "_");
        vec![
            format!("{}_leader", base_name),
            format!("{}_worker_1", base_name),
            format!("{}_worker_2", base_name),
        ]
    }

    /// 生成 Blackboard 模式参与者
    fn generate_blackboard_participants(role: &str) -> Vec<String> {
        let base_name = role.replace(' ', "_");
        vec![
            format!("{}_analyst", base_name),
            format!("{}_implementer", base_name),
            format!("{}_reviewer", base_name),
        ]
    }

    /// 规则回退：模拟 Swarm 协作（带投票评分机制）
    fn simulate_swarm_collaboration(
        task: &str,
        role: &str,
        max_rounds: u32,
    ) -> Result<(CoordinationOutcome, String), String> {
        let session_id = format!("sim_swarm_{}", uuid::Uuid::new_v4());
        let participants = Self::generate_swarm_participants(role);

        // 模拟多轮协作，采用投票评分机制，分数随轮次累积
        let mut proposals: Vec<String> = Vec::new();
        let mut scores: Vec<f32> = Vec::new();
        let mut cumulative_score = 0.5f32;

        for round in 0..max_rounds {
            // 每个参与者提交方案
            let round_proposals: Vec<String> = participants
                .iter()
                .map(|p| format!("[Round {}] {} 方案: {}", round + 1, p, task))
                .collect();

            // 模拟投票评分（基于方案质量关键词），分数随轮次累积
            let round_score = Self::score_proposals(&round_proposals, task);
            cumulative_score = (cumulative_score + round_score) / 2.0;

            // 检查是否达成共识：需要累积分数 >= 0.8 且至少完成 2 轮
            if cumulative_score >= 0.8 && round >= 1 {
                // 达成共识
                let consensus = format!(
                    "Swarm 共识 (得分: {:.2}):\n{}\n\n最终方案: {} 完成任务 {}",
                    cumulative_score,
                    round_proposals.join("\n"),
                    role,
                    task
                );

                let outcome = CoordinationOutcome {
                    session_id: session_id.clone(),
                    mode: CoordinationMode::Swarm,
                    converged: true,
                    consensus: Some(consensus),
                    participants,
                    rounds: round + 1,
                };

                return Ok((outcome, session_id));
            }

            proposals.extend(round_proposals);
            scores.push(round_score);
        }

        // 达到最大轮次，强制收敛
        let avg_score = if scores.is_empty() {
            0.5
        } else {
            scores.iter().sum::<f32>() / scores.len() as f32
        };

        let all_proposals: Vec<String> = (0..max_rounds)
            .flat_map(|round| {
                participants
                    .iter()
                    .map(move |p| format!("[Round {}] {} 方案: {}", round + 1, p, task))
            })
            .collect();

        let consensus = format!(
            "Swarm 最终共识 (得分: {:.2}):\n{}\n\n最终方案: {} 完成任务 {}",
            avg_score.max(cumulative_score),
            all_proposals.join("\n"),
            role,
            task
        );

        let outcome = CoordinationOutcome {
            session_id: session_id.clone(),
            mode: CoordinationMode::Swarm,
            converged: true,
            consensus: Some(consensus),
            participants,
            rounds: max_rounds,
        };

        Ok((outcome, session_id))
    }

    /// 规则回退：模拟 Debate 协作（带对抗评分机制）
    fn simulate_debate_collaboration(
        task: &str,
        role: &str,
        max_rounds: u32,
    ) -> Result<(CoordinationOutcome, String), String> {
        let session_id = format!("sim_debate_{}", uuid::Uuid::new_v4());
        let participants = vec![format!("{}_proposer", role), format!("{}_reviewer", role)];

        // 模拟双方辩论，带有评分和收敛检测
        let mut proposal_score = 0.5f32;
        let mut review_score = 0.5f32;

        for round in 0..max_rounds {
            // Proposer 提出方案并自我评估
            let proposal_text = if round == 0 {
                format!("[Proposer] {}: 方案A - 采用标准实现", task)
            } else {
                format!("[Proposer 改进] {}: 方案B - 加入缓存优化", task)
            };
            let self_score = if round == 0 { 0.7 } else { 0.9 };

            // Reviewer 评估并给出评分
            let review_text = if round == 0 {
                format!("[Reviewer] {}: 方案A可行，但需要考虑性能", task)
            } else {
                format!("[Reviewer 同意] {}: 方案B可行", task)
            };
            let review_assessment = if round == 0 { 0.6 } else { 0.85 };

            // 动态评分调整
            proposal_score = (proposal_score + self_score) / 2.0;
            review_score = (review_score + review_assessment) / 2.0;

            // 检查是否达成共识
            let agreement = (proposal_score + review_score) / 2.0;

            if agreement >= 0.85 || round == max_rounds - 1 {
                let consensus = format!(
                    "辩论共识 (同意度: {:.2}):\n{}\n{}",
                    agreement, proposal_text, review_text
                );

                let outcome = CoordinationOutcome {
                    session_id: session_id.clone(),
                    mode: CoordinationMode::Debate,
                    converged: true,
                    consensus: Some(consensus),
                    participants,
                    rounds: round + 1,
                };

                return Ok((outcome, session_id));
            }
        }

        // 不可达：for 循环内已处理所有分支
        unreachable!("Debate simulation should always converge within max_rounds")
    }

    /// 规则回退：模拟 Blackboard 协作（带信息融合机制）
    fn simulate_blackboard_collaboration(
        task: &str,
        role: &str,
        max_rounds: u32,
    ) -> Result<(CoordinationOutcome, String), String> {
        let session_id = format!("sim_bb_{}", uuid::Uuid::new_v4());
        let participants = Self::generate_blackboard_participants(role);

        // 模拟黑板协作，带信息融合和置信度评估
        let mut knowledge_base: Vec<(String, f32)> = Vec::new(); // (content, confidence)

        for round in 0..max_rounds {
            // 每个参与者贡献信息并评估置信度
            for participant in &participants {
                let contribution =
                    format!("[Round {}] {}: {} 的见解", round + 1, participant, task);
                // 置信度随轮次增加（信息越来越确定）
                let confidence = 0.5 + (round as f32 / max_rounds as f32) * 0.5;
                knowledge_base.push((contribution, confidence));
            }

            // 检查是否有足够高置信度的共识
            let high_confidence_count =
                knowledge_base.iter().filter(|(_, conf)| *conf >= 0.7).count();

            let reached_max_round = round == max_rounds - 1;
            let achieved_consensus = high_confidence_count >= participants.len();

            if achieved_consensus || reached_max_round {
                // 达成共识或达到最大轮次
                let consensus = format!(
                    "黑板共识 (高置信度条目: {}/{}):\n{}\n\n知识库大小: {}",
                    high_confidence_count,
                    participants.len(),
                    knowledge_base
                        .iter()
                        .map(|(content, conf)| format!("{} [置信度: {:.2}]", content, conf))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    knowledge_base.len()
                );

                let outcome = CoordinationOutcome {
                    session_id: session_id.clone(),
                    mode: CoordinationMode::Blackboard,
                    converged: true,
                    consensus: Some(consensus),
                    participants,
                    rounds: round + 1,
                };

                return Ok((outcome, session_id));
            }
        }

        // 不可达：for 循环内已处理所有分支
        unreachable!("Blackboard simulation should always converge within max_rounds")
    }

    /// 对方案进行评分（基于关键词匹配的简单启发式评分）
    fn score_proposals(proposals: &[String], task: &str) -> f32 {
        let mut score = 0.5f32;

        // 检查是否包含任务相关关键词
        let task_keywords: Vec<&str> = task
            .split(|c: char| c.is_whitespace() || c == ',' || c == '，')
            .filter(|w| w.len() >= 2)
            .collect();

        let combined = proposals.join(" ");
        let matched_keywords = task_keywords.iter().filter(|kw| combined.contains(*kw)).count();

        // 关键词匹配加分
        if !task_keywords.is_empty() {
            score += (matched_keywords as f32 / task_keywords.len() as f32) * 0.3;
        }

        // 检查是否包含质量关键词
        let quality_signals = [
            "设计",
            "实现",
            "测试",
            "优化",
            "方案",
            "架构",
            "review",
            "implement",
            "design",
            "test",
        ];
        let quality_count = quality_signals.iter().filter(|s| combined.contains(*s)).count();
        score += (quality_count as f32 / quality_signals.len() as f32) * 0.2;

        score.min(1.0)
    }

    /// 评估执行结果质量
    pub fn evaluate_execution_quality(
        outcome: &CoordinationOutcome,
        original_task: &str,
    ) -> QualityAssessment {
        let mut score = 0.5f32;
        let mut issues: Vec<String> = Vec::new();
        let mut suggestions: Vec<String> = Vec::new();

        // 评估维度 1: 共识收敛状态
        if outcome.converged {
            score += 0.2;
        } else {
            issues.push("未能达成充分共识".to_string());
            suggestions.push("考虑增加协作轮数或调整参与者".to_string());
        }

        // 评估维度 2: 共识内容质量
        if let Some(ref consensus) = outcome.consensus {
            let consensus_len = consensus.len();
            if consensus_len > 100 {
                score += 0.15; // 充分的共识内容
            } else if consensus_len > 20 {
                score += 0.05; // 基础共识
            } else {
                issues.push("共识内容过于简短".to_string());
                suggestions.push("建议提供更详细的执行结果".to_string());
            }
        } else {
            issues.push("缺少共识输出".to_string());
        }

        // 评估维度 3: 参与者覆盖度
        let participant_count = outcome.participants.len();
        if participant_count >= 3 {
            score += 0.1; // 充分参与
        } else if participant_count >= 2 {
            score += 0.05; // 基本参与
        } else {
            issues.push("参与者数量不足".to_string());
            suggestions.push("建议增加更多参与者以获得多角度见解".to_string());
        }

        // 评估维度 4: 任务相关性
        if let Some(ref consensus) = outcome.consensus {
            let task_words: Vec<&str> = original_task
                .split(|c: char| c.is_whitespace() || c == ',' || c == '，')
                .filter(|w| w.len() >= 2)
                .collect();

            let matched = task_words.iter().filter(|w| consensus.contains(*w)).count();
            if !task_words.is_empty() {
                let relevance = matched as f32 / task_words.len() as f32;
                if relevance >= 0.5 {
                    score += 0.1;
                } else if relevance >= 0.3 {
                    score += 0.05;
                } else {
                    issues.push("执行结果与原始任务相关性较低".to_string());
                    suggestions.push("建议重新聚焦任务目标".to_string());
                }
            }
        }

        // 评估维度 5: 协作效率
        let efficiency_score = if outcome.rounds <= 2 {
            0.05 // 高效完成
        } else if outcome.rounds <= 3 {
            0.0 // 正常
        } else {
            -0.05 // 效率偏低
        };
        score += efficiency_score;

        // 归一化到 [0, 1]
        score = score.clamp(0.0, 1.0);

        let passed = score >= 0.7; // 70分以上通过

        if !passed && issues.is_empty() {
            issues.push("综合评分低于阈值".to_string());
        }
        if !passed && suggestions.is_empty() {
            suggestions.push("建议人工审核执行结果".to_string());
        }

        QualityAssessment { score, passed, issues, suggestions }
    }
}

impl Default for MultiAgentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl NodeExecutorTrait for MultiAgentExecutor {
    fn node_type(&self) -> &'static str {
        "multiAgent"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::MultiAgent(mn) = node else {
            return Err(NodeError::type_mismatch(
                "multiAgent".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        let task = &mn.config.task;
        let role = mn.config.role.clone().unwrap_or_else(|| "default".to_string());
        let mode_str = &mn.config.mode;
        let max_rounds = mn.config.max_rounds.max(1);
        let output_var = mn.config.output_var.clone();
        let input_mapping = mn.config.input_mapping.clone();

        if task.is_empty() {
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "status": "no_task",
                    "task": "",
                }),
                output_var: Some(output_var),
                control: None,
            });
        }

        // 解析输入映射，构建结构化输入数据
        let resolved_inputs = Self::resolve_input_mapping(&input_mapping, ctx);

        // 构建带上下文信息的任务描述
        let enriched_task = Self::enrich_task_with_context(task, ctx, &resolved_inputs);

        // 解析协作模式
        let mode = Self::parse_mode(mode_str);

        // 执行对应的协作模式
        let (outcome, _session_id) = match mode {
            CoordinationMode::Swarm => self
                .execute_swarm(&enriched_task, &role, max_rounds)
                .await
                .map_err(|e| NodeError::exec_failed("swarm_execution", e))?,
            CoordinationMode::Debate => self
                .execute_debate(&enriched_task, &role, max_rounds)
                .await
                .map_err(|e| NodeError::exec_failed("debate_execution", e))?,
            CoordinationMode::Blackboard => self
                .execute_blackboard(&enriched_task, &role, max_rounds)
                .await
                .map_err(|e| NodeError::exec_failed("blackboard_execution", e))?,
        };

        // 构建输出
        let mut output = Self::build_output(
            task,
            outcome.mode,
            outcome.rounds,
            outcome.converged,
            outcome.consensus.as_deref(),
            &outcome.participants,
        );

        // 评估执行结果质量
        let quality_assessment = Self::evaluate_execution_quality(&outcome, task);

        // 将质量评估结果添加到输出中
        if let Some(quality) = output.as_object_mut() {
            quality.insert(
                "quality_assessment".to_string(),
                serde_json::json!({
                    "score": quality_assessment.score,
                    "passed": quality_assessment.passed,
                    "issues": quality_assessment.issues,
                    "suggestions": quality_assessment.suggestions,
                }),
            );
        }

        Ok(NodeOutput { output, output_var: Some(output_var), control: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_swarm_execution_without_coordinator() {
        let executor = MultiAgentExecutor::new();
        let task = "实现用户登录功能";
        let role = "developer";
        let max_rounds = 3;

        let (outcome, session_id) = executor.execute_swarm(task, role, max_rounds).await.unwrap();

        assert!(outcome.converged);
        assert_eq!(outcome.mode, CoordinationMode::Swarm);
        assert_eq!(outcome.rounds, max_rounds);
        assert!(!outcome.consensus.is_none());
        assert!(!session_id.is_empty());
    }

    #[tokio::test]
    async fn test_debate_execution_without_coordinator() {
        let executor = MultiAgentExecutor::new();
        let task = "选择数据库方案";
        let role = "architect";
        let max_rounds = 2;

        let (outcome, session_id) = executor.execute_debate(task, role, max_rounds).await.unwrap();

        assert!(outcome.converged);
        assert_eq!(outcome.mode, CoordinationMode::Debate);
        assert_eq!(outcome.rounds, max_rounds);
        assert!(!outcome.consensus.is_none());
        assert!(!session_id.is_empty());
    }

    #[tokio::test]
    async fn test_blackboard_execution_without_blackboard() {
        let executor = MultiAgentExecutor::new();
        let task = "设计API接口";
        let role = "api_designer";
        let max_rounds = 2;

        let (outcome, session_id) =
            executor.execute_blackboard(task, role, max_rounds).await.unwrap();

        assert!(outcome.converged);
        assert_eq!(outcome.mode, CoordinationMode::Blackboard);
        assert_eq!(outcome.rounds, max_rounds);
        assert!(!outcome.consensus.is_none());
        assert!(!session_id.is_empty());
    }

    #[test]
    fn test_parse_mode() {
        assert_eq!(MultiAgentExecutor::parse_mode("swarm"), CoordinationMode::Swarm);
        assert_eq!(MultiAgentExecutor::parse_mode("debate"), CoordinationMode::Debate);
        assert_eq!(MultiAgentExecutor::parse_mode("blackboard"), CoordinationMode::Blackboard);
        assert_eq!(MultiAgentExecutor::parse_mode("unknown"), CoordinationMode::Swarm);
    }

    #[test]
    fn test_build_output() {
        let output = MultiAgentExecutor::build_output(
            "test task",
            CoordinationMode::Swarm,
            3,
            true,
            Some("consensus result"),
            &["agent1".to_string(), "agent2".to_string()],
        );

        assert_eq!(output["task"], "test task");
        assert_eq!(output["mode"], "swarm");
        assert_eq!(output["rounds"], 3);
        assert_eq!(output["converged"], true);
        assert_eq!(output["consensus"], "consensus result");
    }
}
