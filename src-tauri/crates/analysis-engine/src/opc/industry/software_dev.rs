// 软件开发行业适配器
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
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

pub struct SoftwareDevIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl SoftwareDevIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("software_dev", "软件开发") }
    }
}

impl Default for SoftwareDevIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for SoftwareDevIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "name".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "项目名称不能为空".to_string(),
            },
            ValidationDef {
                field: "version".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "版本号不能为空".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        // 用户输入变量通过 input_mapping 注入 AgentNode context
        let user_inputs = HashMap::from([
            ("project_name".to_string(), "project_name".to_string()),
            ("project_goal".to_string(), "project_goal".to_string()),
            ("tech_stack".to_string(), "tech_stack".to_string()),
        ]);
        vec![
            WorkflowStepDef {
                name: "需求分析".to_string(),
                description: "分析项目需求并拆解功能模块与验收标准".to_string(),
                prompt: Some(
                    "你是一名软件需求分析师。请分析项目需求，拆解功能模块与验收标准。\
                     输出 JSON {requirements, feature_modules, acceptance_criteria}"
                        .to_string(),
                ),
                tools: vec![
                    "OpcCreateProject".to_string(),
                    "OpcListProjects".to_string(),
                    "FileWrite".to_string(),
                ],
                agent_profile_id: None,
                error_handling: "stop".to_string(),
                order: 1,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "技术选型".to_string(),
                description: "评估技术栈选型并设计系统架构".to_string(),
                prompt: Some(
                    "你是一名软件架构师。请评估技术栈选型，设计系统架构。\
                     输出 JSON {tech_decision, architecture, dependencies, tradeoffs}"
                        .to_string(),
                ),
                tools: vec![
                    "WebSearch".to_string(),
                    "OpcAddMilestone".to_string(),
                    "OpcListProjects".to_string(),
                ],
                agent_profile_id: None,
                error_handling: "continue".to_string(),
                order: 2,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "性能优化".to_string(),
                description: "识别性能瓶颈并制定优化方案".to_string(),
                prompt: Some(
                    "你是一名性能优化专家。请识别性能瓶颈并制定优化方案。\
                     输出 JSON {bottlenecks, optimization_plan, expected_gains}"
                        .to_string(),
                ),
                tools: vec![
                    "OpcListProjects".to_string(),
                    "OpcAddMilestone".to_string(),
                    "FileRead".to_string(),
                ],
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
                key: "project_name".to_string(),
                label: "项目名称".to_string(),
                field_type: "string".to_string(),
                required: true,
                placeholder: None,
                default: None,
            },
            WorkflowInputField {
                key: "project_goal".to_string(),
                label: "项目目标".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: None,
                default: None,
            },
            WorkflowInputField {
                key: "tech_stack".to_string(),
                label: "技术栈".to_string(),
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
                key: "tasks_completed".to_string(),
                name: "完成任务数".to_string(),
            },
            KpiCalculationDef {
                key: "code_coverage".to_string(), name: "代码覆盖率".to_string()
            },
            KpiCalculationDef {
                key: "deployment_frequency".to_string(),
                name: "部署频率".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.automation_rules()
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "tasks".to_string(),
                title: "完成任务数".to_string(),
                kpi_key: "tasks_completed".to_string(),
            },
            DashboardCardDef {
                id: "deploys".to_string(),
                title: "部署频率".to_string(),
                kpi_key: "deployment_frequency".to_string(),
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
                if let Some(repo_url) = entity_data.get("repository_url") {
                    if repo_url.as_str().is_none_or(|u| !u.starts_with("https://")) {
                        errors.push(ValidationError::new(
                            "repository_url",
                            "仓库地址必须以 https:// 开头",
                        ));
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
                if let Some(status) = entity_data.get("status") {
                    let valid_statuses = ["todo", "in_progress", "review", "done", "blocked"];
                    if status.as_str().is_none_or(|s| !valid_statuses.contains(&s)) {
                        errors.push(ValidationError::new("status", "无效的任务状态"));
                    }
                }
            },
            "release" => {
                if entity_data
                    .get("version")
                    .is_none_or(|v| v.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("version", "版本号不能为空"));
                }
                if entity_data
                    .get("changelog")
                    .is_none_or(|c| c.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("changelog", "更新日志不能为空"));
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

        let tasks_completed =
            data.count_projects(&[ProjectStatus::Completed], from, to).await? as f64;
        let active = data.count_projects(&[ProjectStatus::Active], from, to).await? as f64;
        let total_projects = data.count_projects(&[], from, to).await? as f64;
        let deployments = data.count_invoices(&[InvoiceStatus::Paid], from, to).await? as f64;

        let code_coverage = if total_projects > 0.0 {
            active / total_projects * 100.0
        } else {
            0.0
        };

        Ok(vec![
            KpiValue {
                key: "tasks_completed".to_string(),
                value: tasks_completed,
                target: Some(50.0),
                unit: Some("个".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "code_coverage".to_string(),
                value: code_coverage,
                target: Some(80.0),
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "deployment_frequency".to_string(),
                value: deployments,
                target: Some(10.0),
                unit: Some("次".to_string()),
                timestamp: now,
            },
        ])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition {
                key: "tasks_completed".to_string(),
                name: "完成任务数".to_string(),
                description: "已完成的开发任务数量".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(50.0),
                unit: Some("个".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "code_coverage".to_string(),
                name: "代码覆盖率".to_string(),
                description: "单元测试代码覆盖率".to_string(),
                metric_type: super::super::analytics::MetricType::Percentage,
                target: Some(80.0),
                unit: Some("%".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "deployment_frequency".to_string(),
                name: "部署频率".to_string(),
                description: "生产环境部署次数".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(10.0),
                unit: Some("次".to_string()),
                ..Default::default()
            },
        ]
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "project".to_string(),
            "task".to_string(),
            "release".to_string(),
            "code_review".to_string(),
            "deployment".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("plan", "需求规划", "梳理需求并制定计划").with_order(1),
            WorkflowStep::new("develop", "开发联调", "编码实现并集成测试").with_order(2),
            WorkflowStep::new("release", "发布上线", "部署生产并监控").with_order(3),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "pr_merged",
                "代码合并",
                vec![AutomationCondition::EntityTypeIs { entity_type: "code_review".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "#dev".to_string(),
                    message: "PR 已合并，请关注后续构建".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "deployment_notify",
                "部署通知",
                vec![AutomationCondition::EntityTypeIs { entity_type: "deployment".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "#devops".to_string(),
                    message: "生产环境部署已完成".to_string(),
                }],
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("tasks", "完成任务数", "tasks_completed", "个"),
            DashboardCard::new("deploys", "部署频率", "deployment_frequency", "次"),
        ]
    }
}
