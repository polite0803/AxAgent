// OPC 行业闭环学习层
// 对齐 stock-analysis 的自我进化机制，实现行业分析结果的收集、评估和改进

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::analysis::{OpcIndustryDecision, RiskLevel};
use super::error::OpcResult;

// ── 学习样本 ─────────────────────────────────────────────────

/// 行业学习样本（一次分析的完整记录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryLearningSample {
    pub sample_id: String,
    pub industry_id: String,
    pub timestamp: i64,
    pub decision: OpcIndustryDecision,
    pub actual_outcome: Option<ActualOutcome>,
    pub feedback_score: Option<f64>,
    pub tags: Vec<String>,
}

/// 实际结果（用于验证决策质量）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualOutcome {
    pub outcome_type: OutcomeType,
    pub description: String,
    pub measured_value: Option<f64>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutcomeType {
    TargetAchieved,
    TargetMissed,
    RiskMaterialized,
    OpportunityCaptured,
    NoChange,
}

// ── 学习指标 ─────────────────────────────────────────────────

/// 行业学习指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryLearningMetrics {
    pub industry_id: String,
    pub total_samples: u64,
    pub decision_accuracy: f64,
    pub risk_prediction_accuracy: f64,
    pub avg_feedback_score: f64,
    pub improvement_trend: ImprovementTrend,
    pub last_updated: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImprovementTrend {
    Improving,
    Stable,
    Declining,
    InsufficientData,
}

// ── 学习引擎 ─────────────────────────────────────────────────

/// 行业学习引擎
pub struct IndustryLearningEngine {
    industry_id: String,
    samples: Arc<RwLock<Vec<IndustryLearningSample>>>,
    max_samples: usize,
}

impl IndustryLearningEngine {
    pub fn new(industry_id: String) -> Self {
        Self { industry_id, samples: Arc::new(RwLock::new(Vec::new())), max_samples: 1000 }
    }

    /// 添加学习样本
    pub async fn add_sample(&self, sample: IndustryLearningSample) -> OpcResult<()> {
        let mut samples = self.samples.write().await;
        samples.push(sample);

        // 保持样本数量在限制内
        if samples.len() > self.max_samples {
            let excess = samples.len() - self.max_samples;
            samples.drain(0..excess);
        }

        Ok(())
    }

    /// 添加带反馈的决策样本
    pub async fn record_decision(
        &self,
        decision: OpcIndustryDecision,
        actual_outcome: Option<ActualOutcome>,
    ) -> OpcResult<IndustryLearningSample> {
        let sample = IndustryLearningSample {
            sample_id: uuid::Uuid::new_v4().to_string(),
            industry_id: self.industry_id.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            feedback_score: None,
            decision,
            actual_outcome,
            tags: Vec::new(),
        };

        self.add_sample(sample.clone()).await?;
        Ok(sample)
    }

    /// 计算学习指标
    pub async fn compute_metrics(&self) -> OpcResult<IndustryLearningMetrics> {
        let samples = self.samples.read().await;

        if samples.is_empty() {
            return Ok(IndustryLearningMetrics {
                industry_id: self.industry_id.clone(),
                total_samples: 0,
                decision_accuracy: 0.0,
                risk_prediction_accuracy: 0.0,
                avg_feedback_score: 0.0,
                improvement_trend: ImprovementTrend::InsufficientData,
                last_updated: chrono::Utc::now().timestamp_millis(),
            });
        }

        let total = samples.len() as u64;

        // 计算决策准确率
        let (correct_decisions, total_with_outcome) = self.calculate_decision_accuracy(&samples);
        let decision_accuracy = if total_with_outcome > 0 {
            correct_decisions as f64 / total_with_outcome as f64
        } else {
            0.0
        };

        // 计算风险预测准确率
        let risk_accuracy = self.calculate_risk_prediction_accuracy(&samples);

        // 计算平均反馈分数
        let feedback_sum: f64 = samples.iter().filter_map(|s| s.feedback_score).sum();
        let feedback_count = samples.iter().filter(|s| s.feedback_score.is_some()).count();
        let avg_feedback = if feedback_count > 0 {
            feedback_sum / feedback_count as f64
        } else {
            0.0
        };

        // 判断改进趋势
        let trend = self.determine_improvement_trend(&samples);

        Ok(IndustryLearningMetrics {
            industry_id: self.industry_id.clone(),
            total_samples: total,
            decision_accuracy,
            risk_prediction_accuracy: risk_accuracy,
            avg_feedback_score: avg_feedback,
            improvement_trend: trend,
            last_updated: chrono::Utc::now().timestamp_millis(),
        })
    }

    fn calculate_decision_accuracy(&self, samples: &[IndustryLearningSample]) -> (usize, usize) {
        let mut correct = 0;
        let mut total = 0;

        for sample in samples {
            if let Some(outcome) = &sample.actual_outcome {
                total += 1;
                match (sample.decision.risk_level.clone(), &outcome.outcome_type) {
                    (RiskLevel::Low, OutcomeType::TargetAchieved) => correct += 1,
                    (RiskLevel::Low, OutcomeType::OpportunityCaptured) => correct += 1,
                    (RiskLevel::High | RiskLevel::Critical, OutcomeType::RiskMaterialized) => {
                        correct += 1
                    },
                    _ => {},
                }
            }
        }

        (correct, total)
    }

    fn calculate_risk_prediction_accuracy(&self, samples: &[IndustryLearningSample]) -> f64 {
        let mut correct = 0;
        let mut total = 0;

        for sample in samples {
            if let Some(outcome) = &sample.actual_outcome {
                total += 1;
                let predicted_high_risk =
                    matches!(sample.decision.risk_level, RiskLevel::High | RiskLevel::Critical);
                let actual_risk = matches!(
                    outcome.outcome_type,
                    OutcomeType::RiskMaterialized | OutcomeType::TargetMissed
                );

                if predicted_high_risk == actual_risk {
                    correct += 1;
                }
            }
        }

        if total > 0 {
            correct as f64 / total as f64
        } else {
            0.0
        }
    }

    fn determine_improvement_trend(&self, samples: &[IndustryLearningSample]) -> ImprovementTrend {
        if samples.len() < 5 {
            return ImprovementTrend::InsufficientData;
        }

        let recent: Vec<&IndustryLearningSample> = samples.iter().rev().take(5).collect();
        let older: Vec<&IndustryLearningSample> = samples.iter().rev().skip(5).take(5).collect();

        if recent.is_empty() || older.is_empty() {
            return ImprovementTrend::InsufficientData;
        }

        let recent_score = self.avg_feedback(&recent);
        let older_score = self.avg_feedback(&older);

        if recent_score > older_score + 0.05 {
            ImprovementTrend::Improving
        } else if recent_score < older_score - 0.05 {
            ImprovementTrend::Declining
        } else {
            ImprovementTrend::Stable
        }
    }

    fn avg_feedback(&self, samples: &[&IndustryLearningSample]) -> f64 {
        let scores: Vec<f64> = samples.iter().filter_map(|s| s.feedback_score).collect();
        if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        }
    }

    /// 获取所有样本
    pub async fn get_samples(&self) -> Vec<IndustryLearningSample> {
        self.samples.read().await.clone()
    }

    /// 获取样本数量
    pub async fn sample_count(&self) -> usize {
        self.samples.read().await.len()
    }
}

// ── 学习管理器 ─────────────────────────────────────────────────

/// 行业学习管理器（全局单例模式）
pub struct IndustryLearningManager {
    engines: HashMap<String, Arc<IndustryLearningEngine>>,
}

impl IndustryLearningManager {
    pub fn new() -> Self {
        Self { engines: HashMap::new() }
    }

    /// 获取或创建行业学习引擎
    pub fn get_or_create(&mut self, industry_id: &str) -> &Arc<IndustryLearningEngine> {
        self.engines
            .entry(industry_id.to_string())
            .or_insert_with(|| Arc::new(IndustryLearningEngine::new(industry_id.to_string())))
    }

    /// 获取行业学习引擎
    pub fn get(&self, industry_id: &str) -> Option<&Arc<IndustryLearningEngine>> {
        self.engines.get(industry_id)
    }

    /// 列出所有行业
    pub fn list_industries(&self) -> Vec<&String> {
        self.engines.keys().collect()
    }
}

impl Default for IndustryLearningManager {
    fn default() -> Self {
        Self::new()
    }
}
