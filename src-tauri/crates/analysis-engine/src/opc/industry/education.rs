// 教育培训行业适配器
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

pub struct EducationIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl EducationIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("education", "教育培训") }
    }
}

impl Default for EducationIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for EducationIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "name".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "课程名称不能为空".to_string(),
            },
            ValidationDef {
                field: "email".to_string(),
                r#type: "contains_at".to_string(),
                error_message: "学生邮箱格式不正确".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        // 用户输入变量通过 input_mapping 注入 AgentNode context
        let user_inputs = HashMap::from([
            ("course_topic".to_string(), "course_topic".to_string()),
            ("target_audience".to_string(), "target_audience".to_string()),
            ("course_level".to_string(), "course_level".to_string()),
        ]);
        vec![
            WorkflowStepDef {
                name: "课程体系设计".to_string(),
                description: "设计课程体系与教学大纲".to_string(),
                prompt: Some(
                    "你是一名课程设计专家。请设计课程体系与教学大纲。\
                     输出 JSON {curriculum, modules, learning_objectives}"
                        .to_string(),
                ),
                tools: vec!["OpcCreateContentAsset".to_string(), "WebSearch".to_string()],
                agent_profile_id: None,
                error_handling: "stop".to_string(),
                order: 1,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "学习路径规划".to_string(),
                description: "规划学员学习路径与进度安排".to_string(),
                prompt: Some(
                    "你是一名教育规划专家。请规划学员的学习路径与进度安排。\
                     输出 JSON {learning_path, milestones, assessment_plan}"
                        .to_string(),
                ),
                tools: vec!["OpcCreateLandingPage".to_string(), "FileWrite".to_string()],
                agent_profile_id: None,
                error_handling: "continue".to_string(),
                order: 2,
                inputs: user_inputs.clone(),
            },
            WorkflowStepDef {
                name: "内容开发".to_string(),
                description: "开发高质量教学课件与练习".to_string(),
                prompt: Some(
                    "你是一名课件开发专家。请开发高质量教学课件与练习。\
                     输出 JSON {content_assets, lesson_plans, practice_exercises}"
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
                key: "course_topic".to_string(),
                label: "课程主题".to_string(),
                field_type: "string".to_string(),
                required: true,
                placeholder: Some("如：Python 数据分析入门".to_string()),
                default: None,
            },
            WorkflowInputField {
                key: "target_audience".to_string(),
                label: "目标学员".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: Some("如：零基础职场新人".to_string()),
                default: None,
            },
            WorkflowInputField {
                key: "course_level".to_string(),
                label: "课程难度".to_string(),
                field_type: "string".to_string(),
                required: false,
                placeholder: Some("如：入门 / 进阶 / 高级".to_string()),
                default: None,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "students_enrolled".to_string(),
                name: "报名学生数".to_string(),
            },
            KpiCalculationDef {
                key: "course_completion_rate".to_string(),
                name: "课程完成率".to_string(),
            },
            KpiCalculationDef {
                key: "revenue_per_student".to_string(),
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
                id: "stu".to_string(),
                title: "报名学生".to_string(),
                kpi_key: "students_enrolled".to_string(),
            },
            DashboardCardDef {
                id: "rev".to_string(),
                title: "人均营收".to_string(),
                kpi_key: "revenue_per_student".to_string(),
            },
        ]
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
            "course" => {
                if entity_data.get("name").is_none_or(|c| c.as_str().is_none_or(|s| s.is_empty())) {
                    errors.push(ValidationError::new("name", "课程名称不能为空"));
                }
                if let Some(price) = entity_data.get("price") {
                    if price.as_f64().is_none_or(|p| p < 0.0) {
                        errors.push(ValidationError::new("price", "课程价格不能为负数"));
                    }
                }
            },
            "student" => {
                if entity_data
                    .get("email")
                    .is_none_or(|e| e.as_str().is_none_or(|s| !s.contains('@')))
                {
                    errors.push(ValidationError::new("email", "学生邮箱格式不正确"));
                }
            },
            "enrollment" => {
                if let Some(status) = entity_data.get("status") {
                    let valid_statuses = ["pending", "confirmed", "cancelled", "completed"];
                    if status.as_str().is_none_or(|s| !valid_statuses.contains(&s)) {
                        errors.push(ValidationError::new("status", "无效的报名状态"));
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

        let students = data
            .count_customers(&[CustomerStatus::Active, CustomerStatus::Prospect], from, to)
            .await? as f64;
        let completed = data.count_projects(&[ProjectStatus::Completed], from, to).await? as f64;
        let all_projects = data.count_projects(&[], from, to).await? as f64;
        let revenue = data.aggregate_invoice_amounts(&[InvoiceStatus::Paid], from, to).await?.total;

        let completion = if all_projects > 0.0 {
            completed / all_projects * 100.0
        } else {
            0.0
        };
        let revenue_per_student = if students > 0.0 {
            revenue / students
        } else {
            0.0
        };

        Ok(vec![
            KpiValue {
                key: "students_enrolled".to_string(),
                value: students,
                target: Some(100.0),
                unit: Some("人".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "course_completion_rate".to_string(),
                value: completion,
                target: Some(85.0),
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "revenue_per_student".to_string(),
                value: revenue_per_student,
                target: Some(2000.0),
                unit: Some("CNY".to_string()),
                timestamp: now,
            },
        ])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition {
                key: "students_enrolled".to_string(),
                name: "报名学生数".to_string(),
                description: "已报名课程的学生数量".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(100.0),
                unit: Some("人".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "course_completion_rate".to_string(),
                name: "课程完成率".to_string(),
                description: "完成课程的学生比例".to_string(),
                metric_type: super::super::analytics::MetricType::Percentage,
                target: Some(85.0),
                unit: Some("%".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "revenue_per_student".to_string(),
                name: "人均营收".to_string(),
                description: "平均每位学生带来的营收".to_string(),
                metric_type: super::super::analytics::MetricType::Currency,
                target: Some(2000.0),
                unit: Some("CNY".to_string()),
                ..Default::default()
            },
        ]
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "course".to_string(),
            "student".to_string(),
            "enrollment".to_string(),
            "class".to_string(),
            "certificate".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("enroll", "招生", "新生报名").with_order(1),
            WorkflowStep::new("teach", "教学", "课程授课与作业").with_order(2),
            WorkflowStep::new("certify", "认证", "结课发证").with_order(3),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "course_remind",
                "课前提醒",
                vec![AutomationCondition::EntityTypeIs { entity_type: "course".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "学生邮箱".to_string(),
                    message: "课程开始前一天提醒学生".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "cert_issue",
                "证书发放",
                vec![AutomationCondition::EntityTypeIs { entity_type: "enrollment".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "学生邮箱".to_string(),
                    message: "课程完成后自动发放证书".to_string(),
                }],
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("stu", "报名学生", "students_enrolled", "人"),
            DashboardCard::new("rev", "人均营收", "revenue_per_student", "CNY"),
        ]
    }
}
