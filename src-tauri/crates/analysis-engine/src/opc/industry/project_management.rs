// 项目管理行业适配器
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue, MetricType};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::rules::ValidationError;
use super::super::workflow::{DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowStepDef};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct ProjectManagementIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl ProjectManagementIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("project_management", "项目管理") }
    }
}

impl Default for ProjectManagementIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for ProjectManagementIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "name".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "项目名称不能为空".to_string(),
            },
            ValidationDef {
                field: "start_date".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "开始日期不能为空".to_string(),
            },
            ValidationDef {
                field: "end_date".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "结束日期不能为空".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        vec![
            WorkflowStepDef {
                name: "项目立项".to_string(),
                description: "定义项目目标、范围和资源".to_string(),
                order: 1,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "需求分析".to_string(),
                description: "收集和分析项目需求".to_string(),
                order: 2,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "计划制定".to_string(),
                description: "制定详细的项目计划和里程碑".to_string(),
                order: 3,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "执行监控".to_string(),
                description: "跟踪项目进度、风险管理和质量保证".to_string(),
                order: 4,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "项目收尾".to_string(),
                description: "完成交付物、总结经验教训".to_string(),
                order: 5,
                ..Default::default()
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "on_time_delivery".to_string(),
                name: "按时交付率".to_string(),
            },
            KpiCalculationDef {
                key: "budget_utilization".to_string(),
                name: "预算使用率".to_string(),
            },
            KpiCalculationDef {
                key: "stakeholder_satisfaction".to_string(),
                name: "干系人满意度".to_string(),
            },
            KpiCalculationDef {
                key: "scope_changes".to_string(),
                name: "范围变更次数".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "milestone_deadline_reminder",
                "里程碑截止提醒",
                vec![
                    AutomationCondition::OverdueDaysGte { days: 3 },
                    AutomationCondition::EntityTypeIs {
                        entity_type: "project_milestone".to_string(),
                    },
                ],
                vec![AutomationAction::SendNotification {
                    target: "project_team".to_string(),
                    message: "里程碑将在 3 天内到期".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "project_overdue_alert",
                "项目逾期告警",
                vec![
                    AutomationCondition::StatusIs { status: "overdue".to_string() },
                    AutomationCondition::EntityTypeIs { entity_type: "project".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "project_manager".to_string(),
                    message: "项目已逾期，需要立即处理".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "budget_exceed_warning",
                "预算超标预警",
                vec![
                    AutomationCondition::FieldExceeds {
                        field: "budget_used_percentage".to_string(),
                        threshold: 0.9,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "project".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "finance_team".to_string(),
                    message: "项目预算已使用超过 90%".to_string(),
                }],
            ),
        ]
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "active_projects".to_string(),
                title: "进行中项目".to_string(),
                kpi_key: "active_projects_count".to_string(),
            },
            DashboardCardDef {
                id: "on_time_rate".to_string(),
                title: "按时交付率".to_string(),
                kpi_key: "on_time_delivery".to_string(),
            },
            DashboardCardDef {
                id: "budget_status".to_string(),
                title: "预算使用情况".to_string(),
                kpi_key: "budget_utilization".to_string(),
            },
        ]
    }

    fn requires_approval(&self) -> bool {
        true
    }

    async fn validate(
        &self,
        entity_type: &str,
        entity_data: &serde_json::Value,
    ) -> OpcResult<Vec<ValidationError>> {
        let mut errors = Vec::new();

        match entity_type {
            "project" => {
                if entity_data.get("name").is_none_or(|p| p.as_str().is_none_or(|s| s.is_empty())) {
                    errors.push(ValidationError::new("name", "项目名称不能为空"));
                }
                if entity_data
                    .get("start_date")
                    .is_none_or(|d| d.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("start_date", "开始日期不能为空"));
                }
                if entity_data
                    .get("end_date")
                    .is_none_or(|d| d.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("end_date", "结束日期不能为空"));
                }
                if let Some(status) = entity_data.get("status") {
                    let valid_statuses = [
                        "initiation",
                        "planning",
                        "execution",
                        "monitoring",
                        "closing",
                        "cancelled",
                    ];
                    if status.as_str().is_none_or(|s| !valid_statuses.contains(&s)) {
                        errors.push(ValidationError::new("status", "无效的项目状态"));
                    }
                }
            },
            "task" => {
                if entity_data.get("title").is_none_or(|t| t.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("title", "任务标题不能为空"));
                }
                if let Some(priority) = entity_data.get("priority") {
                    let valid_priorities = ["low", "medium", "high", "critical"];
                    if priority.as_str().is_none_or(|p| !valid_priorities.contains(&p)) {
                        errors.push(ValidationError::new("priority", "无效的优先级"));
                    }
                }
            },
            "milestone" => {
                if entity_data
                    .get("due_date")
                    .is_none_or(|d| d.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("due_date", "截止日期不能为空"));
                }
            },
            _ => {},
        }

        Ok(errors)
    }

    fn entity_types(&self) -> Vec<String> {
        vec!["project".to_string(), "task".to_string(), "milestone".to_string(), "risk".to_string()]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("project_initiation", "项目立项", "定义项目目标、范围和资源")
                .with_order(1),
            WorkflowStep::new("requirements_analysis", "需求分析", "收集和分析项目需求")
                .with_order(2),
            WorkflowStep::new("plan_development", "计划制定", "制定详细的项目计划和里程碑")
                .with_order(3),
            WorkflowStep::new(
                "execution_monitoring",
                "执行监控",
                "跟踪项目进度、风险管理和质量保证",
            )
            .with_order(4),
            WorkflowStep::new("project_closing", "项目收尾", "完成交付物、总结经验教训")
                .with_order(5),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.define_automation_rules()
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        self.define_dashboard_cards()
            .into_iter()
            .map(|c| DashboardCard::new(&c.id, &c.title, &c.kpi_key, "—"))
            .collect()
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("on_time_delivery", "按时交付率", "%", MetricType::Percentage),
            KpiDefinition::new("budget_utilization", "预算使用率", "%", MetricType::Percentage),
            KpiDefinition::new("stakeholder_satisfaction", "干系人满意度", "分", MetricType::Gauge),
            KpiDefinition::new("scope_changes", "范围变更次数", "次", MetricType::Count),
        ]
    }

    async fn compute_kpis(&self, time_range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
        let _ = time_range;
        let now = chrono::Utc::now().timestamp();
        Ok(vec![
            KpiValue {
                key: "on_time_delivery".to_string(),
                value: 0.0,
                target: None,
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "budget_utilization".to_string(),
                value: 0.0,
                target: None,
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "stakeholder_satisfaction".to_string(),
                value: 0.0,
                target: None,
                unit: Some("分".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "scope_changes".to_string(),
                value: 0.0,
                target: None,
                unit: Some("次".to_string()),
                timestamp: now,
            },
        ])
    }
}
