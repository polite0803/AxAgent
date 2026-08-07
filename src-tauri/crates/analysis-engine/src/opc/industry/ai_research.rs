// AI 研究与咨询行业适配器
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::project::ProjectStatus;
use super::super::rules::ValidationError;
use super::super::workflow::{DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowStepDef};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct AiResearchIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl AiResearchIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("ai_research", "AI 研究与咨询") }
    }
}

impl Default for AiResearchIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for AiResearchIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "topic".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "研究主题不能为空".to_string(),
            },
            ValidationDef {
                field: "duration_minutes".to_string(),
                r#type: "range".to_string(),
                error_message: "咨询时长必须在 1-480 分钟之间".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        vec![
            WorkflowStepDef {
                name: "立项".to_string(),
                description: "定义研究主题与可交付物".to_string(),
                order: 1,
            },
            WorkflowStepDef {
                name: "研究".to_string(),
                description: "执行模型实验与资料收集".to_string(),
                order: 2,
            },
            WorkflowStepDef {
                name: "交付".to_string(),
                description: "输出报告并收集反馈".to_string(),
                order: 3,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![KpiCalculationDef {
            key: "research_projects_completed".to_string(),
            name: "完成研究项目".to_string(),
        }]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.automation_rules()
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![DashboardCardDef {
            id: "projects".to_string(),
            title: "完成项目".to_string(),
            kpi_key: "research_projects_completed".to_string(),
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
            "research_project" => {
                if entity_data.get("topic").is_none_or(|t| t.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("topic", "研究主题不能为空"));
                }
                if let Some(model) = entity_data.get("model_preference") {
                    let valid_models = ["gpt-4", "claude-3", "gemini-pro", "mixtral"];
                    if model.as_str().is_none_or(|m| !valid_models.contains(&m)) {
                        errors.push(ValidationError::new("model_preference", "不支持的模型偏好"));
                    }
                }
            },
            "consulting_session" => {
                if let Some(duration) = entity_data.get("duration_minutes") {
                    if duration.as_i64().is_none_or(|d| d <= 0 || d > 480) {
                        errors.push(ValidationError::new(
                            "duration_minutes",
                            "咨询时长必须在 1-480 分钟之间",
                        ));
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

        let completed = data.count_projects(&[ProjectStatus::Completed], from, to).await? as f64;

        Ok(vec![KpiValue {
            key: "research_projects_completed".to_string(),
            value: completed,
            target: Some(10.0),
            unit: Some("个".to_string()),
            timestamp: now,
        }])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![KpiDefinition {
            key: "research_projects_completed".to_string(),
            name: "完成研究项目".to_string(),
            description: "已完成的研究项目数量".to_string(),
            metric_type: super::super::analytics::MetricType::Counter,
            target: Some(10.0),
            unit: Some("个".to_string()),
            ..Default::default()
        }]
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "research_project".to_string(),
            "consulting_session".to_string(),
            "experiment".to_string(),
            "paper".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("propose", "立项", "定义研究主题与可交付物").with_order(1),
            WorkflowStep::new("research", "研究", "执行模型实验与资料收集").with_order(2),
            WorkflowStep::new("deliver", "交付", "输出报告并收集反馈").with_order(3),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "research_sync",
                "研究进展同步",
                vec![AutomationCondition::EntityTypeIs {
                    entity_type: "research_project".to_string(),
                }],
                vec![AutomationAction::SendNotification {
                    target: "#research".to_string(),
                    message: "研究项目进展已更新，请同步关注".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "feedback_collect",
                "客户反馈收集",
                vec![
                    AutomationCondition::EntityTypeIs {
                        entity_type: "research_project".to_string(),
                    },
                    AutomationCondition::StatusIs { status: "completed".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "客户邮箱".to_string(),
                    message: "项目已完成，请填写反馈".to_string(),
                }],
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![DashboardCard::new("projects", "完成项目", "research_projects_completed", "个")]
    }
}
