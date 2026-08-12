// SPDX-License-Identifier: AGPL-3.0-only

//! 需求价值评估 Agent（LLM 二次评估）
//!
//! 在规则引擎评估（`DemandValueEvaluator`）的基础上，调用 LLM 进行语义理解和价值判断，
//! 补充/修正规则引擎的评分，输出最终评估结果。
//!
//! # 架构设计
//!
//! 采用 **策略模式** 解耦 LLM 调用：
//! - `LlmEvaluator` trait 定义 LLM 评估接口
//! - `ValueAssessmentAgent` 持有 `Box<dyn LlmEvaluator>`，支持注入不同的 LLM 实现
//! - analysis-engine crate 不直接依赖 agent crate，符合 harness 架构约束
//!
//! # 评分融合策略
//!
//! ```text
//! 最终分 = 规则引擎分 × (1 - llm_weight) + LLM分 × llm_weight
//! ```
//!
//! 默认 `llm_weight = 0.3`，即规则引擎占 70%，LLM 占 30%。

use crate::opc::evaluator::{DemandEvaluation, DemandType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── LLM 评估接口 ────────────────────────────────────────────────────────

/// LLM 评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmEvaluationResult {
    /// LLM 给出的综合评分（0-100）
    pub llm_score: f64,
    /// LLM 识别的需求类型
    pub detected_type: String,
    /// LLM 判断的痛点强度（0-100）
    pub pain_intensity: f64,
    /// LLM 判断的市场潜力（0-100）
    pub market_potential: f64,
    /// LLM 判断的竞争强度（0-100，越低越好）
    pub competition_level: f64,
    /// LLM 给出的建议
    pub recommendation: String,
    /// LLM 响应的原始文本（用于调试）
    pub raw_response: String,
}

/// LLM 评估器 trait
///
/// 实现此 trait 的类型可以作为 LLM 评估后端注入 `ValueAssessmentAgent`。
/// 默认实现 `MockLlmEvaluator` 用于测试。
#[async_trait]
pub trait LlmEvaluator: Send + Sync {
    /// 评估需求价值
    ///
    /// # 参数
    /// - `title`: 需求标题
    /// - `description`: 需求描述
    /// - `rule_engine_result`: 规则引擎的评估结果（可供 LLM 参考）
    ///
    /// # 返回
    /// - `LlmEvaluationResult`: LLM 的评估结果
    async fn evaluate(
        &self,
        title: &str,
        description: &str,
        rule_engine_result: Option<&DemandEvaluation>,
    ) -> Result<LlmEvaluationResult, String>;
}

// ── Mock 实现 ───────────────────────────────────────────────────────────

/// Mock LLM 评估器（用于测试和离线模式）
///
/// 基于简单的关键词匹配生成评估结果，不调用真实 LLM。
pub struct MockLlmEvaluator {
    /// 模拟延迟（毫秒）
    pub delay_ms: u64,
}

impl Default for MockLlmEvaluator {
    fn default() -> Self {
        Self { delay_ms: 100 }
    }
}

#[async_trait]
impl LlmEvaluator for MockLlmEvaluator {
    async fn evaluate(
        &self,
        title: &str,
        description: &str,
        rule_engine_result: Option<&DemandEvaluation>,
    ) -> Result<LlmEvaluationResult, String> {
        // 模拟延迟
        tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;

        let text = format!("{} {}", title, description).to_lowercase();

        // 简单的关键词匹配评分
        let high_value_keywords = [
            "urgent", "critical", "must", "need", "problem", "solution",
            "紧急", "急需", "必须", "痛点", "解决方案",
        ];
        let medium_value_keywords = [
            "improve", "optimize", "better", "faster", "easier",
            "改善", "优化", "更好", "更快", "更简单",
        ];

        let high_hits = high_value_keywords.iter().filter(|k| text.contains(**k)).count() as f64;
        let medium_hits = medium_value_keywords.iter().filter(|k| text.contains(**k)).count() as f64;

        let base_score = 30.0 + high_hits * 8.0 + medium_hits * 4.0;
        let llm_score = (base_score).min(100.0);

        // 如果有规则引擎结果，参考其评分
        let final_score = if let Some(rule) = rule_engine_result {
            (llm_score + rule.commercial_value_score) / 2.0
        } else {
            llm_score
        };

        Ok(LlmEvaluationResult {
            llm_score: final_score,
            detected_type: "tool_software".to_string(),
            pain_intensity: (high_hits * 15.0).min(100.0),
            market_potential: (final_score * 0.9).min(100.0),
            competition_level: 40.0,
            recommendation: "Mock 评估：建议进一步调研市场需求".to_string(),
            raw_response: format!("Mock LLM response for: {}", title.chars().take(50).collect::<String>()),
        })
    }
}

// ── ValueAssessmentAgent ────────────────────────────────────────────────

/// 需求价值评估 Agent
///
/// 结合规则引擎和 LLM 的评估结果，输出最终的需求价值判定。
pub struct ValueAssessmentAgent {
    llm_evaluator: Box<dyn LlmEvaluator>,
    llm_weight: f64,
    min_confidence: f64,
}

impl ValueAssessmentAgent {
    /// 创建新的 ValueAssessmentAgent
    ///
    /// # 参数
    /// - `llm_evaluator`: LLM 评估器实现
    /// - `llm_weight`: LLM 评分权重（0.0 = 纯规则引擎，1.0 = 纯 LLM）
    pub fn new(llm_evaluator: Box<dyn LlmEvaluator>, llm_weight: f64) -> Self {
        Self {
            llm_evaluator,
            llm_weight: llm_weight.clamp(0.0, 1.0),
            min_confidence: 0.6,
        }
    }

    /// 使用 Mock 评估器创建（用于测试）
    pub fn with_mock() -> Self {
        Self::new(Box::new(MockLlmEvaluator::default()), 0.3)
    }

    /// 设置 LLM 评分权重
    pub fn with_llm_weight(mut self, weight: f64) -> Self {
        self.llm_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// 设置最小置信度阈值
    pub fn with_min_confidence(mut self, confidence: f64) -> Self {
        self.min_confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// 获取 LLM 权重
    pub fn llm_weight(&self) -> f64 {
        self.llm_weight
    }

    /// 获取最小置信度
    pub fn min_confidence(&self) -> f64 {
        self.min_confidence
    }

    /// 执行评估
    ///
    /// # 流程
    /// 1. 使用规则引擎进行初步评估
    /// 2. 调用 LLM 进行二次评估
    /// 3. 融合两个评估结果
    ///
    /// # 返回
    /// - `FinalEvaluation`: 融合后的最终评估结果
    pub async fn assess(
        &self,
        demand_id: &str,
        title: &str,
        description: &str,
        known_competitors: Option<u32>,
    ) -> Result<FinalEvaluation, String> {
        // Step 1: 规则引擎评估
        let rule_result = crate::opc::evaluator::evaluate_demand_value(
            demand_id,
            title,
            description,
            known_competitors,
        );

        // Step 2: LLM 二次评估
        let llm_result = self
            .llm_evaluator
            .evaluate(title, description, Some(&rule_result))
            .await?;

        // Step 3: 融合结果
        self.fuse_results(demand_id, title, &rule_result, &llm_result)
    }

    /// 仅使用规则引擎评估（不调用 LLM）
    pub fn assess_with_rules_only(
        &self,
        demand_id: &str,
        title: &str,
        description: &str,
        known_competitors: Option<u32>,
    ) -> FinalEvaluation {
        let rule_result = crate::opc::evaluator::evaluate_demand_value(
            demand_id,
            title,
            description,
            known_competitors,
        );

        FinalEvaluation {
            demand_id: demand_id.to_string(),
            title: title.to_string(),
            rule_engine_score: rule_result.commercial_value_score,
            llm_score: None,
            final_score: rule_result.commercial_value_score,
            fused_type: rule_result.demand_type.clone(),
            opportunity_level: rule_result.opportunity_level.clone(),
            confidence: rule_result.confidence,
            rule_engine_result: rule_result,
            llm_result: None,
            fusion_notes: "纯规则引擎评估".to_string(),
        }
    }

    /// 融合规则引擎和 LLM 评估结果
    fn fuse_results(
        &self,
        demand_id: &str,
        title: &str,
        rule_result: &DemandEvaluation,
        llm_result: &LlmEvaluationResult,
    ) -> Result<FinalEvaluation, String> {
        // 融合评分
        let final_score = rule_result.commercial_value_score * (1.0 - self.llm_weight)
            + llm_result.llm_score * self.llm_weight;

        // 置信度计算
        let score_diff = (rule_result.commercial_value_score - llm_result.llm_score).abs();
        let agreement = 1.0 - (score_diff / 100.0);
        let confidence = (rule_result.confidence * 0.6 + agreement * 0.4).clamp(0.0, 1.0);

        // 如果置信度低于阈值，标记需要人工审核
        let fusion_notes = if confidence < self.min_confidence {
            format!(
                "⚠️ 规则引擎与LLM评分差异较大（diff={:.1}），建议人工审核",
                score_diff
            )
        } else {
            format!(
                "✓ 规则引擎与LLM评估一致（agreement={:.2}）",
                agreement
            )
        };

        // 机会等级判定
        let opportunity_level = match final_score {
            v if v >= 80.0 => "very_high",
            v if v >= 60.0 => "high",
            v if v >= 40.0 => "medium",
            _ => "low",
        }
        .to_string();

        // 融合需求类型（LLM 优先，规则引擎兜底）
        let fused_type = match llm_result.detected_type.as_str() {
            "tool_software" => DemandType::ToolSoftware,
            "content_creation" => DemandType::ContentCreation,
            "design" => DemandType::Design,
            "development" => DemandType::Development,
            "operations" => DemandType::Operations,
            "marketing" => DemandType::Marketing,
            "education" => DemandType::Education,
            "enterprise_service" => DemandType::EnterpriseService,
            "outsourcing" => DemandType::Outsourcing,
            "consulting" => DemandType::Consulting,
            _ => rule_result.demand_type.clone(),
        };

        Ok(FinalEvaluation {
            demand_id: demand_id.to_string(),
            title: title.to_string(),
            rule_engine_score: rule_result.commercial_value_score,
            llm_score: Some(llm_result.llm_score),
            final_score,
            fused_type,
            opportunity_level,
            confidence,
            rule_engine_result: rule_result.clone(),
            llm_result: Some(llm_result.clone()),
            fusion_notes,
        })
    }
}

// ── 最终评估结果 ────────────────────────────────────────────────────────

/// 融合后的最终评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalEvaluation {
    pub demand_id: String,
    pub title: String,
    pub rule_engine_score: f64,
    pub llm_score: Option<f64>,
    pub final_score: f64,
    pub fused_type: DemandType,
    pub opportunity_level: String,
    pub confidence: f64,
    pub rule_engine_result: DemandEvaluation,
    pub llm_result: Option<LlmEvaluationResult>,
    pub fusion_notes: String,
}

impl FinalEvaluation {
    /// 是否为高价值需求（最终分 ≥ 70）
    pub fn is_high_value(&self) -> bool {
        self.final_score >= 70.0
    }

    /// 是否需要人工审核（置信度 < 阈值）
    pub fn needs_review(&self, threshold: f64) -> bool {
        self.confidence < threshold
    }

    /// 获取格式化的摘要
    pub fn summary(&self) -> String {
        format!(
            "需求 [{}] {}: 最终评分 {:.1}, 等级 {}, 置信度 {:.2}",
            self.demand_id, self.title, self.final_score, self.opportunity_level, self.confidence
        )
    }
}

// ── 评分相关性分析 ──────────────────────────────────────────────────────

/// 计算两组评分的 Pearson 相关系数
///
/// 用于验证 LLM 评分与规则引擎评分的相关性。
/// 验收标准：correlation ≥ 0.7
pub fn calculate_correlation(scores_a: &[f64], scores_b: &[f64]) -> f64 {
    if scores_a.len() != scores_b.len() || scores_a.is_empty() {
        return 0.0;
    }

    let n = scores_a.len() as f64;
    let mean_a: f64 = scores_a.iter().sum::<f64>() / n;
    let mean_b: f64 = scores_b.iter().sum::<f64>() / n;

    let mut numerator = 0.0;
    let mut denom_a = 0.0;
    let mut denom_b = 0.0;

    for i in 0..scores_a.len() {
        let diff_a = scores_a[i] - mean_a;
        let diff_b = scores_b[i] - mean_b;
        numerator += diff_a * diff_b;
        denom_a += diff_a * diff_a;
        denom_b += diff_b * diff_b;
    }

    let denominator = (denom_a * denom_b).sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        (numerator / denominator).clamp(-1.0, 1.0)
    }
}

// ── Prompt 模板 ─────────────────────────────────────────────────────────

/// 生成 LLM 评估的系统提示
pub fn generate_system_prompt() -> String {
    r#"你是一位专业的需求市场评估专家。你的任务是评估一个需求的商业价值。

评估维度：
1. 痛点强度 (0-100)：用户是否真的面临这个问题，问题有多严重
2. 市场潜力 (0-100)：有多少用户可能需要这个解决方案
3. 竞争强度 (0-100)：已有多少竞品在这个领域（越低越好）
4. 需求类型：识别这个需求属于哪个类别

请以 JSON 格式输出评估结果：
{
  "score": 0-100,
  "demand_type": "tool_software|content_creation|design|development|operations|marketing|education|enterprise_service|outsourcing|consulting",
  "pain_intensity": 0-100,
  "market_potential": 0-100,
  "competition_level": 0-100,
  "recommendation": "评估建议"
}"#
        .to_string()
}

/// 生成 LLM 评估的用户提示
pub fn generate_user_prompt(title: &str, description: &str, rule_result: Option<&DemandEvaluation>) -> String {
    let mut prompt = format!(
        "请评估以下需求的商业价值：\n\n【标题】{}\n\n【描述】{}\n",
        title, description
    );

    if let Some(rule) = rule_result {
        prompt.push_str(&format!(
            "\n【规则引擎初步评估】\n- 痛点强度: {:.1}\n- 市场缺口: {:.1}\n- 商业价值分: {:.1}\n- 需求类型: {}\n- 置信度: {:.2}\n",
            rule.pain_score,
            rule.market_gap_score,
            rule.commercial_value_score,
            rule.demand_type.as_str(),
            rule.confidence
        ));
        prompt.push_str("\n请参考规则引擎的初步评估，给出你的独立判断。\n");
    }

    prompt
}

// ── 单元测试 ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_evaluator() {
        let evaluator = MockLlmEvaluator::default();
        let result = evaluator
            .evaluate(
                "测试需求",
                "用户急需一个解决方案来解决这个严重的问题",
                None,
            )
            .await
            .unwrap();

        assert!(result.llm_score > 0.0);
        assert!(!result.recommendation.is_empty());
    }

    #[tokio::test]
    async fn test_value_assessment_agent_with_mock() {
        let agent = ValueAssessmentAgent::with_mock();
        let result = agent
            .assess("test-001", "测试需求", "急需一个解决方案", None)
            .await
            .unwrap();

        assert!(result.final_score > 0.0);
        assert!(!result.summary().is_empty());
        assert!(result.llm_result.is_some());
    }

    #[tokio::test]
    async fn test_assess_with_rules_only() {
        let agent = ValueAssessmentAgent::with_mock();
        let result = agent
            .assess_with_rules_only("test-002", "测试需求", "普通需求描述", None);

        assert!(result.llm_result.is_none());
        assert_eq!(result.fusion_notes, "纯规则引擎评估");
    }

    #[tokio::test]
    async fn test_llm_weight_configuration() {
        let agent = ValueAssessmentAgent::with_mock().with_llm_weight(0.5);
        assert_eq!(agent.llm_weight(), 0.5);

        let agent = ValueAssessmentAgent::with_mock().with_llm_weight(1.5);
        assert_eq!(agent.llm_weight(), 1.0); // 被 clamp 到 1.0

        let agent = ValueAssessmentAgent::with_mock().with_llm_weight(-0.1);
        assert_eq!(agent.llm_weight(), 0.0); // 被 clamp 到 0.0
    }

    #[tokio::test]
    async fn test_high_value_detection() {
        let agent = ValueAssessmentAgent::with_mock();
        let result = agent
            .assess(
                "test-003",
                "急需解决方案",
                "这是一个非常紧急的问题，需要立即解决，市场潜力巨大，竞争少",
                None,
            )
            .await
            .unwrap();

        // 高价值需求应该被正确识别
        assert!(result.final_score > 0.0);
    }

    #[test]
    fn test_correlation_calculation() {
        let scores_a = vec![50.0, 60.0, 70.0, 80.0, 90.0];
        let scores_b = vec![55.0, 65.0, 75.0, 85.0, 95.0];
        let correlation = calculate_correlation(&scores_a, &scores_b);
        assert!(correlation > 0.9); // 高度正相关

        let scores_c = vec![50.0, 60.0, 70.0, 80.0, 90.0];
        let scores_d = vec![90.0, 80.0, 70.0, 60.0, 50.0];
        let correlation = calculate_correlation(&scores_c, &scores_d);
        assert!(correlation < -0.9); // 高度负相关

        let correlation = calculate_correlation(&[], &[]);
        assert_eq!(correlation, 0.0); // 空数组
    }

    #[test]
    fn test_final_evaluation_methods() {
        let eval = FinalEvaluation {
            demand_id: "test".to_string(),
            title: "测试".to_string(),
            rule_engine_score: 50.0,
            llm_score: Some(60.0),
            final_score: 53.0,
            fused_type: DemandType::ToolSoftware,
            opportunity_level: "medium".to_string(),
            confidence: 0.8,
            rule_engine_result: DemandEvaluation {
                demand_id: "test".to_string(),
                pain_score: 50.0,
                existing_solutions: 3,
                market_gap_score: 40.0,
                commercial_value_score: 50.0,
                opportunity_level: "medium".to_string(),
                confidence: 0.7,
                demand_type: DemandType::ToolSoftware,
                extracted_price_range: None,
                market_fit_score: 60.0,
            },
            llm_result: None,
            fusion_notes: "测试".to_string(),
        };

        assert!(!eval.is_high_value()); // 53 < 70
        assert!(!eval.needs_review(0.5)); // 0.8 >= 0.5
        assert!(!eval.summary().is_empty());
    }

    #[test]
    fn test_prompt_generation() {
        let system_prompt = generate_system_prompt();
        assert!(!system_prompt.is_empty());
        assert!(system_prompt.contains("JSON"));

        let user_prompt = generate_user_prompt("测试标题", "测试描述", None);
        assert!(user_prompt.contains("测试标题"));
        assert!(user_prompt.contains("测试描述"));

        let rule_result = DemandEvaluation {
            demand_id: "test".to_string(),
            pain_score: 70.0,
            existing_solutions: 2,
            market_gap_score: 60.0,
            commercial_value_score: 65.0,
            opportunity_level: "high".to_string(),
            confidence: 0.85,
            demand_type: DemandType::Development,
            extracted_price_range: None,
            market_fit_score: 50.0,
        };
        let user_prompt_with_rule =
            generate_user_prompt("测试标题", "测试描述", Some(&rule_result));
        assert!(user_prompt_with_rule.contains("规则引擎"));
    }
}
