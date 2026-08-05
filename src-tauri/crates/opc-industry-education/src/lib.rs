// SPDX-License-Identifier: AGPL-3.0-only

//! 教育培训流程 行业适配器
//!
//! 从 YAML 配置迁移而来：config/opc/industries/education/

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axagent_opc_types::OpcDataService;
use axagent_opc_types::*;

/// 教育培训流程 行业适配器
pub struct EducationAdapter {
    data_service: Mutex<Option<Arc<dyn OpcDataService>>>,
}

impl EducationAdapter {
    pub const INDUSTRY_ID: &'static str = "education";
    pub const INDUSTRY_NAME: &'static str = "教育培训流程";

    pub fn new() -> Self {
        Self { data_service: Mutex::new(None) }
    }
}

impl Default for EducationAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for EducationAdapter {
    fn industry_id(&self) -> &str {
        Self::INDUSTRY_ID
    }

    fn industry_name(&self) -> &str {
        Self::INDUSTRY_NAME
    }

    fn version(&self) -> u32 {
        1
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("a-curriculum", "课程设计", "设计课程目标、大纲和评估方式")
                .with_order(1),
            WorkflowStep::new("a-content", "内容制作", "制作课程内容讲义、练习和测验")
                .with_order(2),
            WorkflowStep::new("a-enroll", "学员管理", "创建学员信息，设置学习路径").with_order(3),
        ]
    }

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("student_count", "学员数量", "人", MetricType::Count),
            KpiDefinition::new("course_completion_rate", "课程完成率", "%", MetricType::Percentage),
            KpiDefinition::new("avg_score", "平均分数", "分", MetricType::Count),
            KpiDefinition::new("student_satisfaction", "学员满意度", "%", MetricType::Percentage),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("student_card", "学员总数", "student_count", "-- 人"),
            DashboardCard::new("completion_card", "完成率", "course_completion_rate", "--%"),
            DashboardCard::new("satisfaction_card", "满意度", "student_satisfaction", "--%"),
        ]
    }

    fn set_data_service(&self, data_service: Arc<dyn OpcDataService>) {
        *self.data_service.lock().unwrap() = Some(data_service);
    }

    fn data_service(&self) -> Option<Arc<dyn OpcDataService>> {
        self.data_service.lock().unwrap().clone()
    }

    async fn validate(
        &self,
        entity_type: &str,
        entity_data: &serde_json::Value,
    ) -> OpcResult<Vec<ValidationError>> {
        let mut errors = Vec::new();

        match entity_type {
            "student" => {
                if let Some(age) = entity_data.get("age").and_then(|v| v.as_i64())
                    && age < 5
                {
                    errors.push(ValidationError::field("age", "学员年龄必须大于等于 5 岁"));
                }
            },
            "course" => {
                if let Some(duration) = entity_data.get("duration").and_then(|v| v.as_f64())
                    && duration <= 0.0
                {
                    errors.push(ValidationError::field("duration", "课程时长必须大于 0"));
                }
            },
            _ => {},
        }

        Ok(errors)
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "education_attendance_alert",
                "学员缺席预警",
                vec![
                    AutomationCondition::CreatedDaysGte { days: 3 },
                    AutomationCondition::EntityTypeIs { entity_type: "attendance".into() },
                    AutomationCondition::StatusIs { status: "absent".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "parent".into(),
                    message: "学员已缺席3天，请关注".into(),
                }],
            ),
            IndustryAutomationRule::new(
                "education_completion_ceremony",
                "课程完成典礼",
                vec![
                    AutomationCondition::FieldExceeds {
                        field: "completion_rate".into(),
                        threshold: 95.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "course".into() },
                ],
                vec![AutomationAction::UpdateStatus { status: "completed".into() }],
            ),
        ]
    }

    async fn evaluate_rule(
        &self,
        _rule: &IndustryAutomationRule,
        _context: &RuleContext,
    ) -> OpcResult<bool> {
        Ok(false)
    }

    async fn execute_rule_actions(
        &self,
        _rule: &IndustryAutomationRule,
        _context: &RuleContext,
    ) -> OpcResult<()> {
        Ok(())
    }
}
