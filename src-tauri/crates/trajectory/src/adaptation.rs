//! Real-time learning and adaptation module
//!
//! Replaces TypeScript `RealTimeLearning.ts` with Rust implementation.
//! Provides feedback processing, adaptation signals, and learning insights.

use crate::insight::{InsightCategory, LearningInsight};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackSignal {
    #[serde(rename = "type")]
    pub feedback_type: FeedbackType,
    pub source: FeedbackSource,
    pub content: String,
    pub timestamp: i64,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackType {
    Success,
    Failure,
    Partial,
    Correction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackSource {
    User,
    System,
    Self_,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationSignal {
    #[serde(rename = "type")]
    pub adaptation_type: AdaptationType,
    pub intensity: f64,
    pub evidence: Vec<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdaptationType {
    Difficulty,
    Engagement,
    Frustration,
    Satisfaction,
    Confusion,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealTimeAdaptation {
    pub response_style: Option<ResponseStyle>,
    pub content_adjustments: Option<Vec<String>>,
    pub skill_suggestions: Option<Vec<String>>,
    pub memory_priorities: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseStyle {
    pub verbosity: Option<Verbosity>,
    pub technical_level: Option<TechnicalLevel>,
    pub format: Option<ContentFormat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    #[default]
    Unchanged,
    Shorter,
    Longer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TechnicalLevel {
    #[default]
    Unchanged,
    Simpler,
    MoreDetailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContentFormat {
    #[default]
    Unchanged,
    List,
    Paragraph,
    Code,
}

pub struct RealTimeLearning {
    feedback_buffer: VecDeque<FeedbackSignal>,
    adaptation_buffer: VecDeque<AdaptationSignal>,
    last_adaptation: RealTimeAdaptation,
    adaptation_cooldown_ms: i64,
    last_adaptation_time: i64,
    max_feedback_size: usize,
}

impl Default for RealTimeLearning {
    fn default() -> Self {
        Self::new()
    }
}

impl RealTimeLearning {
    pub fn new() -> Self {
        Self {
            feedback_buffer: VecDeque::with_capacity(200),
            adaptation_buffer: VecDeque::with_capacity(100),
            last_adaptation: RealTimeAdaptation::default(),
            adaptation_cooldown_ms: 30000,
            last_adaptation_time: 0,
            max_feedback_size: 200,
        }
    }

    pub fn record_feedback(&mut self, mut signal: FeedbackSignal) -> &mut Self {
        let timestamp = chrono::Utc::now().timestamp_millis();
        signal.timestamp = timestamp;
        self.feedback_buffer.push_back(signal.clone());

        if self.feedback_buffer.len() > self.max_feedback_size {
            self.feedback_buffer.pop_front();
        }

        let adaptation = self.process_feedback(&signal);
        if let Some(adaptation) = adaptation {
            self.adaptation_buffer.push_back(adaptation);
        }

        self
    }

    fn process_feedback(&self, signal: &FeedbackSignal) -> Option<AdaptationSignal> {
        let (adaptation_type, intensity) = match signal.feedback_type {
            FeedbackType::Success => (AdaptationType::Satisfaction, 0.5),
            FeedbackType::Failure => (AdaptationType::Frustration, 0.8),
            FeedbackType::Partial => (AdaptationType::Difficulty, 0.6),
            FeedbackType::Correction => (AdaptationType::Confusion, 0.7),
        };

        Some(AdaptationSignal {
            adaptation_type,
            intensity,
            evidence: vec![signal.content.clone()],
            timestamp: signal.timestamp,
        })
    }

    pub fn generate_insights(&self) -> Vec<LearningInsight> {
        let mut insights = Vec::new();

        let failure_count = self
            .feedback_buffer
            .iter()
            .filter(|f| f.feedback_type == FeedbackType::Failure)
            .count();

        if failure_count >= 3 {
            insights.push(LearningInsight {
                id: format!("insight_{}", chrono::Utc::now().timestamp_millis()),
                category: InsightCategory::Warning,
                title: "多次失败检测".to_string(),
                description: format!(
                    "在最近{}条反馈中检测到{}次失败，可能存在系统性问题",
                    self.feedback_buffer.len(),
                    failure_count
                ),
                confidence: 0.8,
                evidence: self
                    .feedback_buffer
                    .iter()
                    .filter(|f| f.feedback_type == FeedbackType::Failure)
                    .take(3)
                    .map(|f| f.content.clone())
                    .collect(),
                suggested_action: Some("建议检查最近的轨迹记录以识别失败模式".to_string()),
                created_at: chrono::Utc::now().timestamp_millis(),
            });
        }

        let correction_count = self
            .feedback_buffer
            .iter()
            .filter(|f| f.feedback_type == FeedbackType::Correction)
            .count();

        if correction_count >= 2 {
            insights.push(LearningInsight {
                id: format!("insight_{}", chrono::Utc::now().timestamp_millis()),
                category: InsightCategory::Improvement,
                title: "用户纠正模式".to_string(),
                description: format!(
                    "检测到{}次用户纠正，表明可能需要调整响应策略",
                    correction_count
                ),
                confidence: 0.7,
                evidence: self
                    .feedback_buffer
                    .iter()
                    .filter(|f| f.feedback_type == FeedbackType::Correction)
                    .take(2)
                    .map(|f| f.content.clone())
                    .collect(),
                suggested_action: Some("建议分析纠正内容以了解用户期望".to_string()),
                created_at: chrono::Utc::now().timestamp_millis(),
            });
        }

        let success_count = self
            .feedback_buffer
            .iter()
            .filter(|f| f.feedback_type == FeedbackType::Success)
            .count();

        if success_count >= 5 && success_count as f64 / self.feedback_buffer.len() as f64 > 0.8 {
            insights.push(LearningInsight {
                id: format!("insight_{}", chrono::Utc::now().timestamp_millis()),
                category: InsightCategory::Preference,
                title: "高成功率".to_string(),
                description: format!(
                    "最近成功率为{:.0}%，当前策略执行良好",
                    success_count as f64 / self.feedback_buffer.len() as f64 * 100.0
                ),
                confidence: 0.9,
                evidence: vec![],
                suggested_action: None,
                created_at: chrono::Utc::now().timestamp_millis(),
            });
        }

        insights
    }

    pub fn compute_adaptation(&mut self) -> RealTimeAdaptation {
        let now = chrono::Utc::now().timestamp_millis();

        if now - self.last_adaptation_time < self.adaptation_cooldown_ms {
            return self.last_adaptation.clone();
        }

        let adaptation = self.calculate_adaptation();
        self.last_adaptation = adaptation.clone();
        self.last_adaptation_time = now;
        adaptation
    }

    fn calculate_adaptation(&self) -> RealTimeAdaptation {
        let frustration_count = self
            .adaptation_buffer
            .iter()
            .filter(|a| a.adaptation_type == AdaptationType::Frustration)
            .count();

        let confusion_count = self
            .adaptation_buffer
            .iter()
            .filter(|a| a.adaptation_type == AdaptationType::Confusion)
            .count();

        if frustration_count > confusion_count {
            RealTimeAdaptation {
                response_style: Some(ResponseStyle {
                    verbosity: Some(Verbosity::Shorter),
                    technical_level: Some(TechnicalLevel::Simpler),
                    format: Some(ContentFormat::List),
                }),
                content_adjustments: Some(vec![
                    "简化回答".to_string(),
                    "使用更直接的方式".to_string(),
                ]),
                skill_suggestions: None,
                memory_priorities: None,
            }
        } else if confusion_count > frustration_count {
            RealTimeAdaptation {
                response_style: Some(ResponseStyle {
                    verbosity: Some(Verbosity::Unchanged),
                    technical_level: Some(TechnicalLevel::Simpler),
                    format: Some(ContentFormat::Paragraph),
                }),
                content_adjustments: Some(vec![
                    "提供更多背景解释".to_string(),
                    "使用清晰的分步说明".to_string(),
                ]),
                skill_suggestions: None,
                memory_priorities: None,
            }
        } else {
            RealTimeAdaptation::default()
        }
    }

    pub fn get_feedback_buffer(&self) -> Vec<FeedbackSignal> {
        self.feedback_buffer.iter().cloned().collect()
    }

    pub fn get_adaptation_buffer(&self) -> Vec<AdaptationSignal> {
        self.adaptation_buffer.iter().cloned().collect()
    }

    pub fn clear_buffers(&mut self) {
        self.feedback_buffer.clear();
        self.adaptation_buffer.clear();
    }
}
