// SPDX-License-Identifier: AGPL-3.0-only

//! 电商运营流程 行业适配器
//!
//! 从 YAML 配置迁移而来：config/opc/industries/ecommerce/

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axagent_opc_types::OpcDataService;
use axagent_opc_types::*;

/// 电商运营流程 行业适配器
pub struct EcommerceAdapter {
    data_service: Mutex<Option<Arc<dyn OpcDataService>>>,
}

impl EcommerceAdapter {
    pub const INDUSTRY_ID: &'static str = "ecommerce";
    pub const INDUSTRY_NAME: &'static str = "电商运营流程";

    pub fn new() -> Self {
        Self { data_service: Mutex::new(None) }
    }
}

impl Default for EcommerceAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for EcommerceAdapter {
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
            WorkflowStep::new("a-product", "选品分析", "分析市场和客户需求，确定产品方向")
                .with_order(1),
            WorkflowStep::new("a-page", "上架落地页", "创建产品落地页").with_order(2),
            WorkflowStep::new("a-customer", "客户管理", "创建客户记录，关联产品信息").with_order(3),
        ]
    }

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("gmv", "商品交易总额", "CNY", MetricType::Currency),
            KpiDefinition::new("order_count", "订单数量", "单", MetricType::Count),
            KpiDefinition::new("conversion_rate", "转化率", "%", MetricType::Percentage),
            KpiDefinition::new("return_rate", "退货率", "%", MetricType::Percentage),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("gmv_card", "本月GMV", "gmv", "¥--"),
            DashboardCard::new("order_card", "订单数", "order_count", "-- 单"),
            DashboardCard::new("conversion_card", "转化率", "conversion_rate", "--%"),
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
            "order" => {
                if let Some(quantity) = entity_data.get("quantity").and_then(|v| v.as_i64())
                    && quantity <= 0
                {
                    errors.push(ValidationError::field("quantity", "订单数量必须大于 0"));
                }
                if let Some(price) = entity_data.get("price").and_then(|v| v.as_f64())
                    && price < 0.0
                {
                    errors.push(ValidationError::field("price", "价格不能为负数"));
                }
            },
            "customer"
                if entity_data
                    .get("email")
                    .and_then(|v| v.as_str())
                    .is_none_or(|e| e.is_empty()) =>
            {
                errors.push(ValidationError::field("email", "下单客户必须提供邮箱"));
            },
            _ => {},
        }

        Ok(errors)
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "ecommerce_low_stock",
                "电商低库存预警",
                vec![
                    AutomationCondition::FieldBelow {
                        field: "stock_level".into(),
                        threshold: 10.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "product".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "inventory".into(),
                    message: "库存不足，请及时补货".into(),
                }],
            ),
            IndustryAutomationRule::new(
                "ecommerce_cart_abandon",
                "购物车遗弃提醒",
                vec![
                    AutomationCondition::CreatedDaysGte { days: 1 },
                    AutomationCondition::EntityTypeIs { entity_type: "cart".into() },
                    AutomationCondition::StatusIs { status: "active".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "customer".into(),
                    message: "您的购物车有商品待结账".into(),
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
