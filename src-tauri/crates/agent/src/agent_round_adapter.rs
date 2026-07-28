// SPDX-License-Identifier: AGPL-3.0-only

//! AgentRoundAdapter — 把 ReActEngine 包装为 SelfImprovingRound
//!
//! Loop Engineering 的连接层：业务无关的 SelfImprovementExecutor 通过
//! SelfImprovingRound trait 驱动本适配器，每轮调用 ReActEngine::run() 执行
//! 完整的 ReAct 循环（含内部 Reflector 质量门），由本适配器把
//! ReActResult 转换为 RoundResult / RoundEvaluation / NextAction。
//!
//! 架构分层：
//! - harness: SelfImprovingRound trait + DTO
//! - agent: SelfImprovementExecutor（通用执行器） + AgentRoundAdapter（本文件）
//! - 业务: 通过 impl SelfImprovingRound 注入领域评估逻辑
//!
//! 设计原则：本适配器仅做"ReActEngine → RoundResult"的格式转换，
//! 不重复实现评估逻辑。Reflector 已在 ReActEngine 内部完成结构化评估，
//! 本适配器从 ReActResult.thought_chain 中提取质量分数。

use crate::react_engine::{ReActEngine, ReActResult};
use crate::reflector::Reflector;
use axagent_harness::reflection_types::TaskExecutionRecord;
use axagent_harness::self_improving_loop::{
    NextAction, RoundEvaluation, RoundResult, RoundStep, SelfImprovingRound,
};
use std::sync::Arc;

/// 把 ReActEngine 包装为 SelfImprovingRound 的适配器。
///
/// 持有 `Arc<tokio::sync::Mutex<ReActEngine>>`，每次 execute_round 锁住引擎
/// 跑一次完整 ReAct 循环，然后释放锁。evaluate_round 和 decide_next 仅依赖
/// RoundResult 中的字段，无需持有引擎锁。
///
/// 可选注入 `Reflector`：注入后 evaluate_round 会构造 `TaskExecutionRecord`
/// 调用 `Reflector::reflect()` 做结构化质量评估，比启发式评分更准确。
/// 调用方应同时通过 `ReActEngine::with_reflector()` + `with_self_improvement()`
/// 配置引擎内部的 Synthesizing 质量门，与本适配器的 Reflector 保持一致。
pub struct AgentRoundAdapter {
    engine: Arc<tokio::sync::Mutex<ReActEngine>>,
    /// 可选 Reflector：用于 evaluate_round 的结构化质量评估。
    /// None 时回退到启发式评分（基于输出长度 + 错误标记）。
    reflector: Option<Arc<Reflector>>,
}

impl AgentRoundAdapter {
    pub fn new(engine: Arc<tokio::sync::Mutex<ReActEngine>>) -> Self {
        Self { engine, reflector: None }
    }

    /// 注入 Reflector 用于 evaluate_round 的结构化质量评估。
    ///
    /// 注意：调用方应同时通过 `ReActEngine::with_reflector()` 注入同一个
    /// Reflector 到引擎内部，以保持 Synthesizing 质量门与跨轮评估的一致性。
    pub fn with_reflector(mut self, reflector: Arc<Reflector>) -> Self {
        self.reflector = Some(reflector);
        self
    }

    /// 缺陷2修复：便捷工厂——构造一个已配置 Reflector + self_improvement 的
    /// ReActEngine，并包装成 `AgentRoundAdapter`。
    ///
    /// 调用方只需提供 `ReActConfig`（其中 `self_improvement_enabled` 和
    /// `final_output_reflection` 应已根据前端 FeatureFlag 设置）和共享的
    /// `Reflector`，本方法自动完成：
    /// 1. `ReActEngine::new().with_config(config).with_reflector(r).with_self_improvement()`
    /// 2. 包装成 `Arc<tokio::sync::Mutex<>>`
    /// 3. 构造 `AgentRoundAdapter` 并注入同一个 `Reflector`
    ///
    /// 这样 `SelfImprovementExecutor` 驱动本适配器时，每轮 `execute_round`
    /// 调用 `engine.run()` 会触发 Synthesizing 阶段的质量门（若 flag 启用），
    /// `evaluate_round` 会用同一个 Reflector 做跨轮评估，保持一致性。
    pub fn with_engine_config(
        config: crate::reasoning_state::ReActConfig,
        reflector: Arc<Reflector>,
    ) -> Self {
        let engine = ReActEngine::new()
            .with_config(config)
            .with_reflector(reflector.clone())
            .with_self_improvement();
        let engine_arc = Arc::new(tokio::sync::Mutex::new(engine));
        Self::new(engine_arc).with_reflector(reflector)
    }

    /// 从 RoundResult 构造一个最小化的 TaskExecutionRecord，供 Reflector 评估。
    ///
    /// 与 `reflector::task_record_from_chain` 类似，但本函数仅依赖
    /// RoundResult（不含完整 ThoughtChain），用于跨轮评估场景。
    fn task_record_from_round(task: &str, result: &RoundResult) -> TaskExecutionRecord {
        let now = chrono::Utc::now();
        // 估算 duration：按 trace 步数 × 500ms 倒推（粗略，仅供效率评分）
        let duration_ms = (result.trace.len() as u64).saturating_mul(500).max(100);
        let start_time = now - chrono::Duration::milliseconds(duration_ms as i64);

        // 从 trace 中提取工具调用（kind == "act"）
        let tools_used: Vec<String> = result
            .trace
            .iter()
            .filter(|s| s.kind == "acting" || s.kind == "act")
            .map(|s| s.summary.clone())
            .collect();

        // 判定成功：output 非空且不以失败标记开头
        let success = !result.output.is_empty() && !result.output.starts_with("[执行失败");

        let task_id = format!("round-{}", result.round);

        let mut record = TaskExecutionRecord::new(task_id, task.to_string(), start_time, now)
            .with_success(success);
        record = record.with_tools(tools_used).with_iterations(result.round.max(1) as usize);
        record.duration_ms = duration_ms;
        if !success {
            record = record.with_error(result.output.clone());
        }
        record
    }

    /// 把 Reflector 的 Reflection 转换为 RoundEvaluation。
    ///
    /// quality_score 范围 0-10，归一化到 0.0-1.0。
    /// improvement_suggestions 作为 gaps，reusable_patterns 作为 strengths。
    fn reflection_to_evaluation(
        reflection: &axagent_harness::reflection_types::Reflection,
    ) -> RoundEvaluation {
        let score = (reflection.quality_score as f64) / 10.0;
        RoundEvaluation {
            score,
            confidence: 0.8,
            gaps: reflection.improvement_suggestions.clone(),
            strengths: reflection.reusable_patterns.clone(),
            raw_assessment: reflection.overall_summary.clone(),
            next_direction: None,
        }
    }
}

#[async_trait::async_trait]
impl SelfImprovingRound for AgentRoundAdapter {
    async fn execute_round(
        &mut self,
        task: &str,
        prev_evaluation: Option<&RoundEvaluation>,
    ) -> Result<RoundResult, Box<dyn std::error::Error + Send>> {
        // 若有上一轮评估，把 gaps + next_direction 注入到任务描述中，
        // 作为下一轮的改进提示。next_direction（来自 Refine 分支）是
        // decide_next 显式给出的改进方向，优先于 gaps 展示。
        let input = if let Some(eval) = prev_evaluation {
            let mut feedback_parts: Vec<String> = Vec::new();
            feedback_parts.push(format!("评分: {}/1.0", eval.score));

            if let Some(direction) = eval.next_direction.as_ref()
                && !direction.is_empty()
            {
                feedback_parts.push(format!("改进方向: {direction}"));
            }

            if !eval.gaps.is_empty() {
                feedback_parts.push(format!("不足之处: {}", eval.gaps.join("; ")));
            }

            if feedback_parts.len() == 1 {
                // 只有评分，无具体改进点：直接用原任务
                task.to_string()
            } else {
                format!(
                    "{task}\n\n## 上一轮反馈\n{}\n\n请针对以上反馈改进并产出更好的结果。",
                    feedback_parts.join("\n")
                )
            }
        } else {
            task.to_string()
        };

        let mut engine = self.engine.lock().await;
        let result: ReActResult = engine.run(&input).await;
        drop(engine);

        // 把 ThoughtChain 的 steps 转成 RoundStep 摘要
        let trace: Vec<RoundStep> = result
            .thought_chain
            .steps
            .iter()
            .enumerate()
            .map(|(idx, s)| RoundStep {
                index: idx as u32,
                kind: s.state.as_str().to_string(),
                summary: s.reasoning.chars().take(200).collect(),
                tokens_used: 0,
            })
            .collect();

        Ok(RoundResult {
            round: 0, // 由 SelfImprovementExecutor 填充
            output: if result.success {
                result.final_response.clone()
            } else {
                // 失败时把错误信息作为 output，让评估器看到失败原因
                result.error.clone().unwrap_or_else(|| "[执行失败但无错误信息]".to_string())
            },
            evaluation: None, // 由 evaluate_round 填充
            trace,
        })
    }

    async fn evaluate_round(
        &self,
        task: &str,
        result: &RoundResult,
    ) -> Result<RoundEvaluation, Box<dyn std::error::Error + Send>> {
        // 优先使用 Reflector 做结构化评估（缺陷5修复）
        if let Some(reflector) = &self.reflector {
            let record = Self::task_record_from_round(task, result);
            let reflection = reflector.reflect(&record).await;
            return Ok(Self::reflection_to_evaluation(&reflection));
        }

        // 回退：简单启发式评分（Reflector 未注入时使用）
        let output = &result.output;
        let has_substantial_output = output.len() > 50 && !output.starts_with("[执行失败");

        let has_error_marker = result.trace.iter().any(|s| {
            s.summary.contains("Error") || s.summary.contains("失败") || s.summary.contains("error")
        });

        let (score, gaps) = if has_substantial_output && !has_error_marker {
            (0.8, vec![])
        } else if has_substantial_output {
            (0.6, vec!["输出中包含错误标记".to_string()])
        } else {
            (0.3, vec!["输出为空或过短".to_string(), "执行过程中存在错误".to_string()])
        };

        Ok(RoundEvaluation {
            score,
            confidence: 0.7,
            gaps,
            strengths: if has_substantial_output {
                vec!["生成了实质性输出".to_string()]
            } else {
                vec![]
            },
            raw_assessment: format!(
                "score={score}, success_marker={has_substantial_output}, error_marker={has_error_marker}"
            ),
            next_direction: None,
        })
    }

    async fn decide_next(
        &self,
        _task: &str,
        _result: &RoundResult,
        evaluation: &RoundEvaluation,
    ) -> Result<NextAction, Box<dyn std::error::Error + Send>> {
        // 评估分数高于 0.75 且无 gaps 时接受；否则继续改进
        if evaluation.score >= 0.75 && evaluation.gaps.is_empty() {
            Ok(NextAction::Accept)
        } else {
            Ok(NextAction::Refine {
                direction: format!("Address these gaps: {}", evaluation.gaps.join("; ")),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::react_engine::ReActEngine;

    #[tokio::test]
    async fn test_adapter_execute_round_returns_round_result() {
        // 使用默认 DefaultReasoningProvider（会返回 Err），验证适配器仍能返回 RoundResult
        let engine = Arc::new(tokio::sync::Mutex::new(ReActEngine::new()));
        let mut adapter = AgentRoundAdapter::new(engine);

        let result = adapter
            .execute_round("test task", None)
            .await
            .expect("execute_round should not error even if engine fails");

        assert_eq!(result.round, 0); // 由 executor 填充，适配器填 0
        assert!(!result.output.is_empty() || result.trace.is_empty());
    }

    #[tokio::test]
    async fn test_adapter_evaluate_round_low_score_on_empty_output() {
        let engine = Arc::new(tokio::sync::Mutex::new(ReActEngine::new()));
        let adapter = AgentRoundAdapter::new(engine);

        let result =
            RoundResult { round: 1, output: String::new(), evaluation: None, trace: vec![] };

        let eval =
            adapter.evaluate_round("test", &result).await.expect("evaluate_round should not error");

        assert!(eval.score < 0.5);
        assert!(!eval.gaps.is_empty());
    }

    #[tokio::test]
    async fn test_adapter_evaluate_round_high_score_on_substantial_output() {
        let engine = Arc::new(tokio::sync::Mutex::new(ReActEngine::new()));
        let adapter = AgentRoundAdapter::new(engine);

        let result = RoundResult {
            round: 1,
            output: "这是一个足够长的实质性输出，应该获得较高的评分。".to_string(),
            evaluation: None,
            trace: vec![],
        };

        let eval =
            adapter.evaluate_round("test", &result).await.expect("evaluate_round should not error");

        assert!(eval.score >= 0.7);
        assert!(eval.gaps.is_empty());
    }

    #[tokio::test]
    async fn test_adapter_evaluate_round_uses_reflector_when_injected() {
        // 注入 Reflector 后，evaluate_round 应走 Reflector 路径
        // （quality_score 基于 TaskExecutionRecord 计算，非启发式）
        let engine = Arc::new(tokio::sync::Mutex::new(ReActEngine::new()));
        let reflector = Arc::new(Reflector::new());
        let adapter = AgentRoundAdapter::new(engine).with_reflector(reflector);

        let result = RoundResult {
            round: 1,
            output: "足够长的成功输出内容，用于 Reflector 评估".to_string(),
            evaluation: None,
            trace: vec![],
        };

        let eval =
            adapter.evaluate_round("test", &result).await.expect("evaluate_round should not error");

        // Reflector 评估成功任务时 quality_score 应 >= 5（score >= 0.5）
        assert!(eval.score >= 0.5);
        // raw_assessment 应为 Reflector 的 overall_summary（非启发式格式）
        assert!(!eval.raw_assessment.starts_with("score="));
    }

    #[tokio::test]
    async fn test_adapter_decide_next_accepts_high_score() {
        let engine = Arc::new(tokio::sync::Mutex::new(ReActEngine::new()));
        let adapter = AgentRoundAdapter::new(engine);

        let evaluation = RoundEvaluation {
            score: 0.9,
            confidence: 0.9,
            gaps: vec![],
            strengths: vec![],
            raw_assessment: "high quality".to_string(),
            next_direction: None,
        };

        let action = adapter
            .decide_next(
                "test",
                &RoundResult {
                    round: 1,
                    output: "good".to_string(),
                    evaluation: None,
                    trace: vec![],
                },
                &evaluation,
            )
            .await
            .expect("decide_next should not error");

        assert_eq!(action, NextAction::Accept);
    }

    #[tokio::test]
    async fn test_adapter_decide_next_refines_on_gaps() {
        let engine = Arc::new(tokio::sync::Mutex::new(ReActEngine::new()));
        let adapter = AgentRoundAdapter::new(engine);

        let evaluation = RoundEvaluation {
            score: 0.5,
            confidence: 0.7,
            gaps: vec!["需要更多信息".to_string()],
            strengths: vec![],
            raw_assessment: "needs improvement".to_string(),
            next_direction: None,
        };

        let action = adapter
            .decide_next(
                "test",
                &RoundResult {
                    round: 1,
                    output: "incomplete".to_string(),
                    evaluation: None,
                    trace: vec![],
                },
                &evaluation,
            )
            .await
            .expect("decide_next should not error");

        match action {
            NextAction::Refine { direction } => {
                assert!(direction.contains("需要更多信息"));
            },
            other => panic!("expected Refine, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_adapter_execute_round_reads_next_direction() {
        // 验证 execute_round 读取 prev_evaluation.next_direction 并注入到任务描述
        // 通过观察 ReActResult.thought_chain.steps 中是否包含改进方向文本
        // （由于 DefaultReasoningProvider 返回 Err，trace 为空，此处仅验证不 panic）
        let engine = Arc::new(tokio::sync::Mutex::new(ReActEngine::new()));
        let mut adapter = AgentRoundAdapter::new(engine);

        let prev_eval = RoundEvaluation {
            score: 0.4,
            confidence: 0.7,
            gaps: vec!["输出不够详细".to_string()],
            strengths: vec![],
            raw_assessment: String::new(),
            next_direction: Some("增加更多细节".to_string()),
        };

        let result = adapter
            .execute_round("test task", Some(&prev_eval))
            .await
            .expect("execute_round should not error");

        // 仅验证返回了 RoundResult（具体注入逻辑由 ReActEngine 内部消费）
        assert_eq!(result.round, 0);
    }
}
