//! Learning insight system module
//!
//! Replaces TypeScript `LearningInsightSystem.ts` with Rust implementation.
//! Provides insight generation, report building, and learning metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningInsight {
    pub id: String,
    pub category: InsightCategory,
    pub title: String,
    pub description: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
    #[serde(rename = "suggestedAction")]
    pub suggested_action: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InsightCategory {
    Pattern,
    Preference,
    Improvement,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightReport {
    pub id: String,
    #[serde(rename = "type")]
    pub report_type: ReportType,
    pub timestamp: i64,
    pub summary: String,
    pub sections: Vec<InsightSection>,
    pub metrics: InsightMetrics,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportType {
    Daily,
    Weekly,
    Session,
    Triggered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightSection {
    pub title: String,
    pub content: String,
    pub insights: Vec<LearningInsight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightMetrics {
    #[serde(rename = "totalInteractions")]
    pub total_interactions: usize,
    #[serde(rename = "successRate")]
    pub success_rate: f64,
    #[serde(rename = "mostUsedSkills")]
    pub most_used_skills: Vec<SkillUsage>,
    #[serde(rename = "commonPatterns")]
    pub common_patterns: Vec<String>,
    #[serde(rename = "learningProgress")]
    pub learning_progress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsage {
    #[serde(rename = "skillId")]
    pub skill_id: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    #[serde(rename = "type")]
    pub recommendation_type: RecommendationType,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    #[serde(rename = "expectedImpact")]
    pub expected_impact: String,
    pub effort: Effort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecommendationType {
    SkillCreation,
    MemoryUpdate,
    PreferenceChange,
    PatternAdoption,
    Strategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Low,
    Medium,
    High,
}

pub struct LearningInsightSystem {
    insight_history: Vec<LearningInsight>,
    report_history: Vec<InsightReport>,
    max_insights_stored: usize,
    max_reports_stored: usize,
}

impl Default for LearningInsightSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningInsightSystem {
    pub fn new() -> Self {
        Self {
            insight_history: Vec::new(),
            report_history: Vec::new(),
            max_insights_stored: 100,
            max_reports_stored: 20,
        }
    }

    pub fn with_storage_limits(mut self, insights: usize, reports: usize) -> Self {
        self.max_insights_stored = insights;
        self.max_reports_stored = reports;
        self
    }

    pub fn add_insight(&mut self, mut insight: LearningInsight) {
        insight.created_at = chrono::Utc::now().timestamp_millis();
        if insight.id.is_empty() {
            insight.id = format!("insight_{}", insight.created_at);
        }
        self.insight_history.push(insight);
        self.trim_history();
    }

    pub fn get_insights(&self) -> &[LearningInsight] {
        &self.insight_history
    }

    pub fn get_reports(&self) -> &[InsightReport] {
        &self.report_history
    }

    pub fn get_insights_by_category(&self, category: InsightCategory) -> Vec<&LearningInsight> {
        self.insight_history
            .iter()
            .filter(|i| i.category == category)
            .collect()
    }

    fn trim_history(&mut self) {
        if self.insight_history.len() > self.max_insights_stored {
            let drain_count = self.insight_history.len() - self.max_insights_stored;
            self.insight_history.drain(..drain_count);
        }
        if self.report_history.len() > self.max_reports_stored {
            let drain_count = self.report_history.len() - self.max_reports_stored;
            self.report_history.drain(..drain_count);
        }
    }

    pub fn generate_session_report(
        &mut self,
        session_id: &str,
        message_count: usize,
        insights: Vec<LearningInsight>,
    ) -> InsightReport {
        let insights_by_category = self.categorize_insights(&insights);

        let summary = self.generate_summary(&insights);

        let sections = self.build_sections(&insights_by_category);

        let metrics = self.compute_metrics(message_count);

        let recommendations = self.generate_recommendations(&insights);

        let report = InsightReport {
            id: format!("report_{}_{}", session_id, chrono::Utc::now().timestamp_millis()),
            report_type: ReportType::Session,
            timestamp: chrono::Utc::now().timestamp_millis(),
            summary,
            sections,
            metrics,
            recommendations,
        };

        self.insight_history.extend(insights);
        self.report_history.push(report.clone());
        self.trim_history();

        report
    }

    fn categorize_insights(
        &self,
        insights: &[LearningInsight],
    ) -> HashMap<InsightCategory, Vec<LearningInsight>> {
        let mut categorized: HashMap<InsightCategory, Vec<LearningInsight>> = HashMap::new();
        for insight in insights {
            categorized
                .entry(insight.category)
                .or_insert_with(Vec::new)
                .push(insight.clone());
        }
        categorized
    }

    fn generate_summary(&self, insights: &[LearningInsight]) -> String {
        if insights.is_empty() {
            return "暂无洞察数据".to_string();
        }

        let categories: HashMap<&str, usize> = insights
            .iter()
            .map(|i| match i.category {
                InsightCategory::Pattern => ("模式", 0),
                InsightCategory::Preference => ("偏好", 1),
                InsightCategory::Improvement => ("改进", 2),
                InsightCategory::Warning => ("警告", 3),
            })
            .fold(HashMap::new(), |mut acc, (k, _v)| {
                *acc.entry(k).or_insert(0) += 1;
                acc
            });

        let count = insights.len();
        format!(
            "本次生成{}条洞察，包括{}个模式、{}个偏好、{}个改进点",
            count,
            categories.get("模式").copied().unwrap_or(0),
            categories.get("偏好").copied().unwrap_or(0),
            categories.get("改进").copied().unwrap_or(0),
        )
    }

    fn build_sections(
        &self,
        insights_by_category: &HashMap<InsightCategory, Vec<LearningInsight>>,
    ) -> Vec<InsightSection> {
        let mut sections = Vec::new();

        for (category, insights) in insights_by_category {
            if insights.is_empty() {
                continue;
            }

            let (title, content_prefix) = match category {
                InsightCategory::Pattern => ("发现的行为模式", "检测到以下反复出现的行为模式"),
                InsightCategory::Preference => ("用户偏好", "基于交互分析得出的用户偏好"),
                InsightCategory::Improvement => ("改进机会", "以下是可能需要改进的方面"),
                InsightCategory::Warning => ("需要注意的问题", "以下是需要关注的问题"),
            };

            let content = if insights.len() > 3 {
                format!("{}（展示前3条，共{}条）", content_prefix, insights.len())
            } else {
                format!("{}：", content_prefix)
            };

            sections.push(InsightSection {
                title: title.to_string(),
                content,
                insights: insights.iter().take(3).cloned().collect(),
            });
        }

        sections
    }

    fn compute_metrics(&self, message_count: usize) -> InsightMetrics {
        let total_interactions = message_count;

        let success_insights = self
            .insight_history
            .iter()
            .filter(|i| {
                i.description.to_lowercase().contains("success")
                    || i.description.to_lowercase().contains("成功")
            })
            .count();

        let success_rate = if total_interactions > 0 {
            success_insights as f64 / total_interactions as f64
        } else {
            0.0
        };

        let most_used_skills = self
            .insight_history
            .iter()
            .filter_map(|i| {
                if i.description.contains("skill") || i.description.contains("技能") {
                    Some(SkillUsage {
                        skill_id: i.id.clone(),
                        count: 1,
                    })
                } else {
                    None
                }
            })
            .fold(HashMap::new(), |mut acc, usage| {
                *acc.entry(usage.skill_id).or_insert(0) += usage.count;
                acc
            })
            .into_iter()
            .map(|(skill_id, count)| SkillUsage { skill_id, count })
            .collect::<Vec<_>>();

        let common_patterns: Vec<String> = self
            .insight_history
            .iter()
            .filter(|i| i.category == InsightCategory::Pattern)
            .take(5)
            .map(|i| i.title.clone())
            .collect();

        let learning_progress = if !self.insight_history.is_empty() {
            self.insight_history.len() as f64 / self.max_insights_stored as f64
        } else {
            0.0
        };

        InsightMetrics {
            total_interactions,
            success_rate,
            most_used_skills,
            common_patterns,
            learning_progress,
        }
    }

    fn generate_recommendations(&self, insights: &[LearningInsight]) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        let warning_count = insights
            .iter()
            .filter(|i| i.category == InsightCategory::Warning)
            .count();

        if warning_count > 0 {
            recommendations.push(Recommendation {
                id: format!("rec_{}", chrono::Utc::now().timestamp_millis()),
                recommendation_type: RecommendationType::Strategy,
                title: "解决警告问题".to_string(),
                description: format!(
                    "检测到{}个需要关注的问题，建议优先处理",
                    warning_count
                ),
                priority: Priority::High,
                expected_impact: "提高整体系统稳定性".to_string(),
                effort: Effort::Medium,
            });
        }

        let pattern_count = insights
            .iter()
            .filter(|i| i.category == InsightCategory::Pattern)
            .count();

        if pattern_count >= 3 {
            recommendations.push(Recommendation {
                id: format!("rec_{}", chrono::Utc::now().timestamp_millis() + 1),
                recommendation_type: RecommendationType::PatternAdoption,
                title: "利用已发现模式".to_string(),
                description: format!(
                    "发现{}个行为模式，可以利用这些模式优化工作流程",
                    pattern_count
                ),
                priority: Priority::Medium,
                expected_impact: "提高效率".to_string(),
                effort: Effort::Low,
            });
        }

        let improvement_count = insights
            .iter()
            .filter(|i| i.category == InsightCategory::Improvement)
            .count();

        if improvement_count > 0 {
            recommendations.push(Recommendation {
                id: format!("rec_{}", chrono::Utc::now().timestamp_millis() + 2),
                recommendation_type: RecommendationType::SkillCreation,
                title: "创建优化技能".to_string(),
                description: format!(
                    "基于{}个改进建议，创建一个优化技能来自动化改进流程",
                    improvement_count
                ),
                priority: Priority::Medium,
                expected_impact: "减少手动干预".to_string(),
                effort: Effort::High,
            });
        }

        recommendations
    }

    pub fn generate_daily_report(&mut self) -> InsightReport {
        let recent_insights: Vec<LearningInsight> = self
            .insight_history
            .iter()
            .filter(|i| {
                let day_ago = chrono::Utc::now().timestamp_millis() - 86400 * 1000;
                i.created_at > day_ago
            })
            .cloned()
            .collect();

        self.generate_session_report("daily", recent_insights.len(), recent_insights)
    }

    pub fn generate_weekly_report(&mut self) -> InsightReport {
        let recent_insights: Vec<LearningInsight> = self
            .insight_history
            .iter()
            .filter(|i| {
                let week_ago = chrono::Utc::now().timestamp_millis() - 7 * 86400 * 1000;
                i.created_at > week_ago
            })
            .cloned()
            .collect();

        let mut report = self.generate_session_report("weekly", recent_insights.len(), recent_insights);
        report.report_type = ReportType::Weekly;
        report
    }

    pub fn clear_history(&mut self) {
        self.insight_history.clear();
        self.report_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insight_system() {
        let mut system = LearningInsightSystem::new();

        let insight = LearningInsight {
            id: "test_1".to_string(),
            category: InsightCategory::Pattern,
            title: "Test Pattern".to_string(),
            description: "A test pattern".to_string(),
            confidence: 0.8,
            evidence: vec![],
            suggested_action: None,
            created_at: 0,
        };

        system.add_insight(insight);

        let report = system.generate_session_report("test_session", 10, vec![]);
        assert_eq!(report.report_type, ReportType::Session);
        assert_eq!(system.get_insights().len(), 1);
    }

    #[test]
    fn test_categorization() {
        let system = LearningInsightSystem::new();
        let insights = vec![
            LearningInsight {
                id: "1".to_string(),
                category: InsightCategory::Pattern,
                title: "".to_string(),
                description: "".to_string(),
                confidence: 0.0,
                evidence: vec![],
                suggested_action: None,
                created_at: 0,
            },
            LearningInsight {
                id: "2".to_string(),
                category: InsightCategory::Warning,
                title: "".to_string(),
                description: "".to_string(),
                confidence: 0.0,
                evidence: vec![],
                suggested_action: None,
                created_at: 0,
            },
        ];

        let categorized = system.categorize_insights(&insights);
        assert_eq!(categorized.get(&InsightCategory::Pattern).unwrap().len(), 1);
        assert_eq!(categorized.get(&InsightCategory::Warning).unwrap().len(), 1);
    }
}
