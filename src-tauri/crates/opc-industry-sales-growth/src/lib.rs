// SPDX-License-Identifier: AGPL-3.0-only

//! 销售增长流程 行业适配器
//!
//! 从 YAML 配置迁移而来：config/opc/industries/sales_growth/

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axagent_opc_types::OpcDataService;
use axagent_opc_types::*;

/// 销售增长流程 行业适配器
pub struct SalesGrowthAdapter {
    data_service: Mutex<Option<Arc<dyn OpcDataService>>>,
}

impl SalesGrowthAdapter {
    pub const INDUSTRY_ID: &'static str = "sales_growth";
    pub const INDUSTRY_NAME: &'static str = "销售增长流程";

    pub fn new() -> Self {
        Self { data_service: Mutex::new(None) }
    }
}

impl Default for SalesGrowthAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for SalesGrowthAdapter {
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
            WorkflowStep::new("a-lead", "线索获取", "分析现有客户数据，识别潜在客户").with_order(1),
            WorkflowStep::new("a-outreach", "触达跟进", "制定触达方案").with_order(2),
            WorkflowStep::new("a-close", "签约转化", "推动线索转化签约").with_order(3),
        ]
    }

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("lead_count", "线索数量", "个", MetricType::Count),
            KpiDefinition::new("conversion_rate", "转化率", "%", MetricType::Percentage),
            KpiDefinition::new("deal_value", "成交金额", "CNY", MetricType::Currency),
            KpiDefinition::new("sales_cycle_length", "销售周期", "天", MetricType::Duration),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("lead_card", "本月线索", "lead_count", "-- 个"),
            DashboardCard::new("conversion_card", "转化率", "conversion_rate", "--%"),
            DashboardCard::new("deal_card", "成交金额", "deal_value", "¥--"),
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
            "lead" => {
                let has_email = entity_data
                    .get("email")
                    .and_then(|v| v.as_str())
                    .is_some_and(|e| !e.is_empty());
                let has_phone = entity_data
                    .get("phone")
                    .and_then(|v| v.as_str())
                    .is_some_and(|p| !p.is_empty());
                if !has_email && !has_phone {
                    errors.push(ValidationError::field("email", "线索必须提供邮箱或手机号"));
                }
            },
            "deal" => {
                if let Some(amount) = entity_data.get("amount").and_then(|v| v.as_f64())
                    && amount <= 0.0
                {
                    errors.push(ValidationError::field("amount", "交易金额必须大于 0"));
                }
            },
            _ => {},
        }

        Ok(errors)
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "sales_lead_followup",
                "新线索跟进提醒",
                vec![
                    AutomationCondition::CreatedDaysGte { days: 3 },
                    AutomationCondition::EntityTypeIs { entity_type: "lead".into() },
                    AutomationCondition::StatusIs { status: "new".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "sales_rep".into(),
                    message: "新线索已3天未跟进".into(),
                }],
            ),
            IndustryAutomationRule::new(
                "sales_deal_velocity",
                "交易谈判超时提醒",
                vec![
                    AutomationCondition::FieldBelow { field: "deal_age".into(), threshold: 7.0 },
                    AutomationCondition::EntityTypeIs { entity_type: "deal".into() },
                    AutomationCondition::StatusIs { status: "negotiating".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "manager".into(),
                    message: "交易谈判时间超过7天".into(),
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
}
