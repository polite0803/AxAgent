// SPDX-License-Identifier: AGPL-3.0-only

//! AI 科技研究报告 行业适配器
//!
//! 从 YAML 配置迁移而来：config/opc/industries/ai_research/

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axagent_opc_types::*;

/// AI 科技研究报告 行业适配器
pub struct AiResearchAdapter {
    data_service: Mutex<Option<Arc<dyn OpcDataService>>>,
}

impl AiResearchAdapter {
    pub const INDUSTRY_ID: &'static str = "ai_research";
    pub const INDUSTRY_NAME: &'static str = "AI 科技研究报告";

    pub fn new() -> Self {
        Self { data_service: Mutex::new(None) }
    }
}

impl Default for AiResearchAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for AiResearchAdapter {
    fn industry_id(&self) -> &str {
        Self::INDUSTRY_ID
    }

    fn industry_name(&self) -> &str {
        Self::INDUSTRY_NAME
    }

    fn version(&self) -> u32 {
        1
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
            "research" => {
                if entity_data.get("title").and_then(|v| v.as_str()).is_none_or(|s| s.is_empty()) {
                    errors.push(ValidationError::field("title", "研究项目标题不能为空"));
                }
            },
            "prototype"
                if entity_data
                    .get("tech_stack")
                    .and_then(|v| v.as_str())
                    .is_none_or(|s| s.is_empty()) =>
            {
                errors.push(ValidationError::field("tech_stack", "原型技术栈必须指定"));
            },
            _ => {},
        }

        Ok(errors)
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("a-req", "需求分析", "分析AI研究需求，明确研究范围和评估标准")
                .with_order(1),
            WorkflowStep::new("a-survey", "技术调研", "调研AI技术方案和最新进展").with_order(2),
            WorkflowStep::new("a-prototype", "原型验证", "搭建最小原型，验证关键假设")
                .with_order(3),
            WorkflowStep::new("a-report", "研究报告", "生成AI研究报告").with_order(4),
        ]
    }

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("research_count", "研究数量", "份", MetricType::Count),
            KpiDefinition::new("prototype_success_rate", "原型成功率", "%", MetricType::Percentage),
            KpiDefinition::new("tech_readiness_score", "技术成熟度", "分", MetricType::Count),
            KpiDefinition::new("research_impact_score", "研究影响力", "分", MetricType::Count),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "ai_research_deadline",
                "研究项目期限提醒",
                vec![
                    AutomationCondition::CreatedDaysGte { days: 14 },
                    AutomationCondition::EntityTypeIs { entity_type: "research".to_string() },
                    AutomationCondition::StatusIs { status: "in_progress".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "researcher".to_string(),
                    message: "研究项目已进行14天，请确保按时完成".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "ai_prototype_review",
                "原型评审触发",
                vec![
                    AutomationCondition::FieldExceeds {
                        field: "prototype_score".to_string(),
                        threshold: 80.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "prototype".to_string() },
                ],
                vec![AutomationAction::UpdateStatus { status: "approved".to_string() }],
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

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("research_card", "研究报告数", "research_count", "-- 份"),
            DashboardCard::new("prototype_card", "原型成功率", "prototype_success_rate", "--%"),
            DashboardCard::new("readiness_card", "技术成熟度", "tech_readiness_score", "-- 分"),
        ]
    }
}
