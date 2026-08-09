// 行业咨询行业适配器
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::customer::CustomerStatus;
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::invoice::InvoiceStatus;
use super::super::project::ProjectStatus;
use super::super::rules::ValidationError;
use super::super::workflow::{
    DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowInputField, WorkflowStepDef,
};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct IndustryConsultingIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl IndustryConsultingIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("industry_consulting", "行业咨询") }
    }
}

impl Default for IndustryConsultingIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for IndustryConsultingIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "title".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "项目标题不能为空".to_string(),
            },
            ValidationDef {
                field: "hourly_rate".to_string(),
                r#type: "positive".to_string(),
                error_message: "时薪必须大于零".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        // 用户输入变量通过 input_mapping 注入 AgentNode context
        let user_inputs = HashMap::from([
            ("industry_name".to_string(), "industry_name".to_string()),
            ("client_goal".to_string(), "client_goal".to_string()),
            ("region".to_string(), "region".to_string()),
        ]);
        vec![
            WorkflowStepDef {
                name: "行业扫描".to_string(),
                description: "扫描目标行业全景与市场格局".to_string(),
                prompt: Some(
                    "你是一名产业咨询顾问。请扫描目标行业的全景，分析市场规模、竞争格局与增长驱动。\
                     输出 JSON {industry_overview, market_size, competitive_landscape, growth_drivers}"
                        .to_string(),
                ),
                tools: vec!["OpcSearchWiki".to_string(), "WebSearch".to_string()],
                agent_profile_id: None,
                error_handling: "stop".to_string(),
                order: 1,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "进入评估".to_string(),
                description: "评估客户进入该行业的可行性与风险".to_string(),
                prompt: Some(
                    "你是一名产业进入评估专家。请评估客户进入该行业的可行性与风险。\
                     输出 JSON {feasibility, entry_barriers, risks, recommendation}"
                        .to_string(),
                ),
                tools: vec!["OpcSearchWiki".to_string(), "OpcGetDashboard".to_string()],
                agent_profile_id: None,
                error_handling: "continue".to_string(),
                order: 2,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "战略制定".to_string(),
                description: "制定进入战略与实施路线图".to_string(),
                prompt: Some(
                    "你是一名企业战略专家。请为客户制定进入战略与实施路线图。\
                     输出 JSON {strategy, roadmap, resource_plan, success_metrics}"
                        .to_string(),
                ),
                tools: vec!["OpcCreateContentAsset".to_string(), "FileWrite".to_string()],
                agent_profile_id: None,
                error_handling: "continue".to_string(),
                order: 3,
                inputs: user_inputs,
            },
        ]
    }

    fn input_fields(&self) -> Vec<WorkflowInputField> {
        vec![
            WorkflowInputField {
                key: "industry_name".to_string(),
                label: "行业名称".to_string(),
                field_type: "string".to_string(),
                required: true,
                placeholder: None,
                default: None,
            },
            WorkflowInputField {
                key: "client_goal".to_string(),
                label: "客户目标".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: None,
                default: None,
            },
            WorkflowInputField {
                key: "region".to_string(),
                label: "目标区域".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: None,
                default: None,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "projects_delivered".to_string(),
                name: "交付项目数".to_string(),
            },
            KpiCalculationDef {
                key: "utilization_rate".to_string(),
                name: "人员利用率".to_string(),
            },
            KpiCalculationDef {
                key: "revenue_per_consultant".to_string(),
                name: "人均营收".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.automation_rules()
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "delivered".to_string(),
                title: "交付项目数".to_string(),
                kpi_key: "projects_delivered".to_string(),
            },
            DashboardCardDef {
                id: "util".to_string(),
                title: "人员利用率".to_string(),
                kpi_key: "utilization_rate".to_string(),
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
            "consulting_project" => {
                if entity_data.get("title").is_none_or(|t| t.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("title", "项目标题不能为空"));
                }
                if let Some(duration) = entity_data.get("duration_weeks") {
                    if duration.as_i64().is_none_or(|d| d <= 0 || d > 52) {
                        errors.push(ValidationError::new(
                            "duration_weeks",
                            "项目周期必须在 1-52 周之间",
                        ));
                    }
                }
            },
            "report" => {
                if entity_data
                    .get("executive_summary")
                    .is_none_or(|s| s.as_str().is_none_or(|x| x.is_empty()))
                {
                    errors.push(ValidationError::new("executive_summary", "执行摘要不能为空"));
                }
            },
            "client_engagement" => {
                if let Some(hourly_rate) = entity_data.get("hourly_rate") {
                    if hourly_rate.as_f64().is_none_or(|r| r <= 0.0) {
                        errors.push(ValidationError::new("hourly_rate", "时薪必须大于零"));
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

        let delivered = data.count_projects(&[ProjectStatus::Completed], from, to).await? as f64;
        let active = data.count_projects(&[ProjectStatus::Active], from, to).await? as f64;
        let total_projects = data.count_projects(&[], from, to).await? as f64;
        let revenue = data.aggregate_invoice_amounts(&[InvoiceStatus::Paid], from, to).await?.total;
        let consultants = data.count_customers(&[CustomerStatus::Active], from, to).await? as f64;

        let utilization_rate = if total_projects > 0.0 {
            active / total_projects * 100.0
        } else {
            0.0
        };
        let revenue_per_consultant = if consultants > 0.0 {
            revenue / consultants
        } else {
            0.0
        };

        Ok(vec![
            KpiValue {
                key: "projects_delivered".to_string(),
                value: delivered,
                target: Some(5.0),
                unit: Some("个".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "utilization_rate".to_string(),
                value: utilization_rate,
                target: Some(75.0),
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "revenue_per_consultant".to_string(),
                value: revenue_per_consultant,
                target: Some(500000.0),
                unit: Some("CNY".to_string()),
                timestamp: now,
            },
        ])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition {
                key: "projects_delivered".to_string(),
                name: "交付项目数".to_string(),
                description: "已完成咨询项目数量".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(5.0),
                unit: Some("个".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "utilization_rate".to_string(),
                name: "人员利用率".to_string(),
                description: "可计费时间占比".to_string(),
                metric_type: super::super::analytics::MetricType::Percentage,
                target: Some(75.0),
                unit: Some("%".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "revenue_per_consultant".to_string(),
                name: "人均营收".to_string(),
                description: "每位顾问的平均营收".to_string(),
                metric_type: super::super::analytics::MetricType::Currency,
                target: Some(500000.0),
                unit: Some("CNY".to_string()),
                ..Default::default()
            },
        ]
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "consulting_project".to_string(),
            "report".to_string(),
            "client_engagement".to_string(),
            "deliverable".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("engage", "客户接洽", "评估需求并签订合同").with_order(1),
            WorkflowStep::new("deliver", "项目交付", "执行咨询并输出成果").with_order(2),
            WorkflowStep::new("review", "复盘反馈", "收集反馈并复盘").with_order(3),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "milestone_reminder",
                "里程碑提醒",
                vec![AutomationCondition::EntityTypeIs {
                    entity_type: "consulting_project".to_string(),
                }],
                vec![AutomationAction::SendNotification {
                    target: "#consulting".to_string(),
                    message: "咨询项目关键节点即将到达，需同步".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "report_delivery",
                "报告交付",
                vec![AutomationCondition::EntityTypeIs { entity_type: "report".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "客户邮箱".to_string(),
                    message: "咨询报告已完成，请查收".to_string(),
                }],
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("delivered", "交付项目数", "projects_delivered", "个"),
            DashboardCard::new("util", "人员利用率", "utilization_rate", "%"),
        ]
    }
}
