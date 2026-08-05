// SPDX-License-Identifier: AGPL-3.0-only

//! 行业咨询流程 行业适配器
//!
//! 从 YAML 配置迁移而来：config/opc/industries/industry_consulting/

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axagent_opc_types::*;

/// 行业咨询流程 行业适配器
pub struct IndustryConsultingAdapter {
    data_service: Mutex<Option<Arc<dyn OpcDataService>>>,
}

impl IndustryConsultingAdapter {
    pub const INDUSTRY_ID: &'static str = "industry_consulting";
    pub const INDUSTRY_NAME: &'static str = "行业咨询流程";

    pub fn new() -> Self {
        Self { data_service: Mutex::new(None) }
    }
}

impl Default for IndustryConsultingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for IndustryConsultingAdapter {
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
            "consulting_project" => {
                if entity_data
                    .get("client_name")
                    .and_then(|v| v.as_str())
                    .is_none_or(|s| s.is_empty())
                {
                    errors.push(ValidationError::field("client_name", "客户名称不能为空"));
                }
            },
            "report"
                if entity_data
                    .get("executive_summary")
                    .and_then(|v| v.as_str())
                    .is_none_or(|s| s.is_empty()) =>
            {
                errors.push(ValidationError::field("executive_summary", "报告摘要不能为空"));
            },
            _ => {},
        }

        Ok(errors)
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("a-intake", "调研启动", "明确咨询目标和调研范围").with_order(1),
            WorkflowStep::new("a-research", "信息采集", "系统性收集行业数据和情报").with_order(2),
            WorkflowStep::new("a-validate", "调研验证", "验证采集信息的质量和充分性").with_order(3),
            WorkflowStep::new("a-design", "方案设计", "设计针对性的咨询方案").with_order(4),
            WorkflowStep::new("a-approval", "方案审批", "客户审批咨询方案").with_order(5),
            WorkflowStep::new("a-report", "报告撰写", "生成完整的行业咨询报告").with_order(6),
            WorkflowStep::new("a-deliver", "交付归档", "整理最终交付物包").with_order(7),
        ]
    }

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("consulting_count", "咨询项目数", "个", MetricType::Count),
            KpiDefinition::new("client_satisfaction", "客户满意度", "%", MetricType::Percentage),
            KpiDefinition::new("delivery_quality_score", "交付质量分", "分", MetricType::Count),
            KpiDefinition::new("market_insight_depth", "市场洞察深度", "分", MetricType::Count),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "consulting_delivery_warning",
                "报告交付预警",
                vec![
                    AutomationCondition::CreatedDaysGte { days: 30 },
                    AutomationCondition::EntityTypeIs { entity_type: "report".to_string() },
                    AutomationCondition::StatusIs { status: "draft".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "consultant".to_string(),
                    message: "报告草稿已创建30天，需要尽快交付".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "consulting_client_followup",
                "客户跟进提醒",
                vec![
                    AutomationCondition::OverdueDaysGte { days: 7 },
                    AutomationCondition::EntityTypeIs { entity_type: "invoice".to_string() },
                    AutomationCondition::StatusIs { status: "sent".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "client".to_string(),
                    message: "咨询费用已到期，请及时付款".to_string(),
                }],
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
            DashboardCard::new("consulting_card", "咨询项目数", "consulting_count", "-- 个"),
            DashboardCard::new("satisfaction_card", "客户满意度", "client_satisfaction", "--%"),
            DashboardCard::new("quality_card", "交付质量", "delivery_quality_score", "-- 分"),
        ]
    }
}
