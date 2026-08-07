// 内容与媒体行业适配器
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::customer::CustomerStatus;
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::project::ProjectStatus;
use super::super::rules::ValidationError;
use super::super::workflow::{DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowStepDef};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct ContentMediaIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl ContentMediaIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("content_media", "内容与媒体") }
    }
}

impl Default for ContentMediaIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for ContentMediaIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "title".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "文章标题不能为空".to_string(),
            },
            ValidationDef {
                field: "duration_seconds".to_string(),
                r#type: "positive".to_string(),
                error_message: "视频时长必须大于零".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        vec![
            WorkflowStepDef {
                name: "创作".to_string(),
                description: "产出内容草稿".to_string(),
                order: 1,
            },
            WorkflowStepDef {
                name: "审核".to_string(),
                description: "内容质量检查".to_string(),
                order: 2,
            },
            WorkflowStepDef {
                name: "发布".to_string(),
                description: "多平台发布".to_string(),
                order: 3,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "content_published".to_string(),
                name: "内容发布量".to_string(),
            },
            KpiCalculationDef {
                key: "subscriber_growth".to_string(),
                name: "订阅增长".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.automation_rules()
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![DashboardCardDef {
            id: "pub".to_string(),
            title: "发布量".to_string(),
            kpi_key: "content_published".to_string(),
        }]
    }

    fn requires_approval(&self) -> bool {
        false
    }

    async fn validate(
        &self,
        entity_type: &str,
        entity_data: &serde_json::Value,
    ) -> OpcResult<Vec<ValidationError>> {
        let mut errors = Vec::new();

        match entity_type {
            "article" => {
                if entity_data.get("title").is_none_or(|t| t.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("title", "文章标题不能为空"));
                }
                if let Some(word_count) = entity_data.get("word_count") {
                    if word_count.as_i64().is_none_or(|w| w < 100) {
                        errors.push(ValidationError::new("word_count", "文章至少需要 100 字"));
                    }
                }
            },
            "video_content" => {
                if let Some(duration) = entity_data.get("duration_seconds") {
                    if duration.as_i64().is_none_or(|d| d <= 0) {
                        errors.push(ValidationError::new("duration_seconds", "视频时长必须大于零"));
                    }
                }
            },
            "social_post" => {
                if let Some(platform) = entity_data.get("platform") {
                    let valid_platforms =
                        ["weibo", "wechat", "douyin", "xiaohongshu", "twitter", "linkedin"];
                    if platform.as_str().is_none_or(|p| !valid_platforms.contains(&p)) {
                        errors.push(ValidationError::new("platform", "不支持的社交平台"));
                    }
                }
            },
            _ => {},
        }

        Ok(errors)
    }

    async fn compute_kpis(&self, time_range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
        let Some(data) = self.data_service() else {
            return Ok(Vec::new());
        };
        let (from, to) = (time_range.start, time_range.end);
        let now = chrono::Utc::now().timestamp();

        let published = data.count_projects(&[ProjectStatus::Completed], from, to).await? as f64;
        let subscribers = data
            .count_customers(&[CustomerStatus::Active, CustomerStatus::Prospect], from, to)
            .await? as f64;

        Ok(vec![
            KpiValue {
                key: "content_published".to_string(),
                value: published,
                target: Some(20.0),
                unit: Some("篇".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "subscriber_growth".to_string(),
                value: subscribers,
                target: Some(500.0),
                unit: Some("人".to_string()),
                timestamp: now,
            },
        ])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition {
                key: "content_published".to_string(),
                name: "内容发布量".to_string(),
                description: "已发布文章/视频/帖子数量".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(20.0),
                unit: Some("篇".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "subscriber_growth".to_string(),
                name: "订阅增长".to_string(),
                description: "新增订阅/关注人数".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(500.0),
                unit: Some("人".to_string()),
                ..Default::default()
            },
        ]
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "article".to_string(),
            "video_content".to_string(),
            "social_post".to_string(),
            "newsletter".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("create", "创作", "产出内容草稿").with_order(1),
            WorkflowStep::new("review", "审核", "内容质量检查").with_order(2),
            WorkflowStep::new("publish", "发布", "多平台发布").with_order(3),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "publish_notify",
                "发布通知",
                vec![AutomationCondition::EntityTypeIs { entity_type: "article".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "#content".to_string(),
                    message: "新内容已发布".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "content_long_article",
                "长文标记",
                vec![AutomationCondition::FieldExceeds {
                    field: "word_count".to_string(),
                    threshold: 3000.0,
                }],
                vec![AutomationAction::UpdateField {
                    field: "is_featured".to_string(),
                    value: serde_json::json!(true),
                }],
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![DashboardCard::new("pub", "发布量", "content_published", "篇")]
    }
}
