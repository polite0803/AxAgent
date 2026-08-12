// OPC 行业分析/决策层
// 对齐 stock-analysis 的分析引擎，实现行业隔离的分析回合、决策和风控

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use axagent_harness::self_improving_loop::{
    NextAction, RoundEvaluation, RoundResult, RoundStep, SelfImprovingRound,
};

use super::data_service::OpcDataService;
use super::error::OpcResult;

// ── OpcIndustryDecision ────────────────────────────────────────

/// 行业分析决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcIndustryDecision {
    pub industry_id: String,
    pub decision_type: DecisionType,
    pub summary: String,
    pub confidence: f64,
    pub kpis: Vec<super::analytics::KpiValue>,
    pub recommendations: Vec<String>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecisionType {
    PerformanceReview,
    KpiAnalysis,
    TrendForecast,
    RiskAssessment,
    StrategicPlanning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

// ── OpcRiskGate ─────────────────────────────────────────────────

/// 行业风控门控（对齐 position_limits）
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OpcRiskGate {
    industry_id: String,
    max_monthly_expense_ratio: f64,
    min_customer_satisfaction: f64,
    max_project_overdue_days: u32,
}

impl OpcRiskGate {
    pub fn new(industry_id: &str) -> Self {
        Self {
            industry_id: industry_id.to_string(),
            max_monthly_expense_ratio: 0.7,
            min_customer_satisfaction: 3.5,
            max_project_overdue_days: 30,
        }
    }

    /// 风控检查结果
    pub async fn check(&self, kpis: &[super::analytics::KpiValue]) -> OpcResult<RiskCheckResult> {
        let mut violations = Vec::new();

        for kpi in kpis {
            match kpi.key.as_str() {
                "expense_ratio" if kpi.value > self.max_monthly_expense_ratio * 100.0 => {
                    violations.push(RiskViolation {
                        rule: "max_monthly_expense_ratio".to_string(),
                        current: kpi.value,
                        threshold: self.max_monthly_expense_ratio * 100.0,
                        message: format!(
                            "费用率 {:.1}% 超过阈值 {:.0}%",
                            kpi.value,
                            self.max_monthly_expense_ratio * 100.0
                        ),
                    });
                },
                "customer_satisfaction" if kpi.value < self.min_customer_satisfaction => {
                    violations.push(RiskViolation {
                        rule: "min_customer_satisfaction".to_string(),
                        current: kpi.value,
                        threshold: self.min_customer_satisfaction,
                        message: format!(
                            "客户满意度 {:.1} 低于阈值 {:.1}",
                            kpi.value, self.min_customer_satisfaction
                        ),
                    });
                },
                _ => {},
            }
        }

        let risk_level = if violations.is_empty() {
            RiskLevel::Low
        } else if violations.len() == 1 {
            RiskLevel::Medium
        } else if violations.len() <= 3 {
            RiskLevel::High
        } else {
            RiskLevel::Critical
        };

        let passed = violations.is_empty();

        Ok(RiskCheckResult {
            industry_id: self.industry_id.clone(),
            risk_level,
            violations,
            passed,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskCheckResult {
    pub industry_id: String,
    pub risk_level: RiskLevel,
    pub violations: Vec<RiskViolation>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskViolation {
    pub rule: String,
    pub current: f64,
    pub threshold: f64,
    pub message: String,
}

// ── OpcIndustryAnalysisRound ────────────────────────────────────

/// 行业分析回合（实现 SelfImprovingRound trait）
pub struct OpcIndustryAnalysisRound {
    industry_id: String,
    data_service: Arc<dyn OpcDataService>,
    risk_gate: OpcRiskGate,
}

impl OpcIndustryAnalysisRound {
    pub fn new(industry_id: String, data_service: Arc<dyn OpcDataService>) -> Self {
        let risk_gate = OpcRiskGate::new(&industry_id);
        Self { industry_id, data_service, risk_gate }
    }

    /// 执行分析并返回决策
    pub async fn analyze(
        &self,
        time_range: &super::data_service::TimeRange,
    ) -> OpcResult<OpcIndustryDecision> {
        let kpis = super::industry_kpi_service::compute_kpis(
            &self.industry_id,
            &self.data_service,
            time_range,
        )
        .await?;
        let risk_check = self.risk_gate.check(&kpis).await?;

        let recommendations = self.generate_recommendations(&kpis, &risk_check);

        let risk_level = risk_check.risk_level.clone();
        let confidence = self.calculate_confidence(&kpis);

        Ok(OpcIndustryDecision {
            industry_id: self.industry_id.clone(),
            decision_type: DecisionType::PerformanceReview,
            summary: self.generate_summary(&kpis, &risk_check),
            confidence,
            kpis,
            recommendations,
            risk_level,
        })
    }

    fn generate_recommendations(
        &self,
        kpis: &[super::analytics::KpiValue],
        risk_check: &RiskCheckResult,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        if !risk_check.passed {
            for violation in &risk_check.violations {
                recs.push(format!("⚠️ {}: {}", violation.rule, violation.message));
            }
        }

        for kpi in kpis {
            if let Some(target) = kpi.target {
                if kpi.value < target * 0.8 {
                    recs.push(format!(
                        "📈 {} 低于目标 80%（当前 {:.1}，目标 {:.1}）",
                        kpi.key, kpi.value, target
                    ));
                }
            }
        }

        if recs.is_empty() {
            recs.push("✅ 所有指标正常，建议保持当前策略".to_string());
        }

        recs
    }

    fn generate_summary(
        &self,
        kpis: &[super::analytics::KpiValue],
        risk_check: &RiskCheckResult,
    ) -> String {
        let kpi_summary: Vec<String> =
            kpis.iter().map(|k| format!("{}={:.1}", k.key, kpi_value_display(k))).collect();

        let risk_summary = if risk_check.passed {
            "风控检查通过".to_string()
        } else {
            format!("发现 {} 个风控违规", risk_check.violations.len())
        };

        format!("行业「{}」分析：{}。{}", self.industry_id, kpi_summary.join(", "), risk_summary)
    }

    fn calculate_confidence(&self, kpis: &[super::analytics::KpiValue]) -> f64 {
        if kpis.is_empty() {
            return 0.0;
        }
        let with_targets = kpis.iter().filter(|k| k.target.is_some()).count();
        if with_targets == 0 {
            return 0.5;
        }
        let on_track = kpis.iter().filter(|k| k.target.is_some_and(|t| k.value >= t * 0.8)).count();
        on_track as f64 / kpis.len() as f64
    }
}

fn kpi_value_display(kpi: &super::analytics::KpiValue) -> f64 {
    kpi.value
}

#[async_trait]
impl SelfImprovingRound for OpcIndustryAnalysisRound {
    async fn execute_round(
        &mut self,
        _task: &str,
        _prev_evaluation: Option<&RoundEvaluation>,
    ) -> Result<RoundResult, Box<dyn std::error::Error + Send>> {
        let time_range = super::data_service::TimeRange::days(30);
        let decision =
            self.analyze(&time_range).await.map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let output =
            serde_json::to_string_pretty(&decision).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let trace = vec![
            RoundStep {
                index: 0,
                kind: "analyze".to_string(),
                summary: format!("行业 {} KPI 分析完成", self.industry_id),
                tokens_used: 0,
            },
            RoundStep {
                index: 1,
                kind: "risk_check".to_string(),
                summary: format!("风控检查：{:?}", decision.risk_level),
                tokens_used: 0,
            },
        ];

        Ok(RoundResult { round: 0, output, evaluation: None, trace })
    }

    async fn evaluate_round(
        &self,
        _task: &str,
        result: &RoundResult,
    ) -> Result<RoundEvaluation, Box<dyn std::error::Error + Send>> {
        let output_len = result.output.len();
        let has_content = output_len > 100;
        let score = if has_content { 0.8 } else { 0.3 };
        let confidence = if has_content { 0.9 } else { 0.4 };

        let mut gaps = Vec::new();
        if !has_content {
            gaps.push("输出内容不足".to_string());
        }

        let strengths =
            vec![format!("行业 {} 独立分析完成", self.industry_id), "包含风控检查".to_string()];

        Ok(RoundEvaluation {
            score,
            confidence,
            gaps,
            strengths,
            raw_assessment: String::new(),
            next_direction: None,
        })
    }

    async fn decide_next(
        &self,
        _task: &str,
        _result: &RoundResult,
        evaluation: &RoundEvaluation,
    ) -> Result<NextAction, Box<dyn std::error::Error + Send>> {
        if evaluation.score >= 0.7 || evaluation.gaps.is_empty() {
            Ok(NextAction::Accept)
        } else {
            Ok(NextAction::Refine { direction: evaluation.gaps.join("; ") })
        }
    }
}
