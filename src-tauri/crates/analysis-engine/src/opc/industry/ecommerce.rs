// 电子商务行业适配器
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::customer::CustomerStatus;
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::invoice::InvoiceStatus;
use super::super::rules::ValidationError;
use super::super::workflow::{DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowStepDef};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct EcommerceIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl EcommerceIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("ecommerce", "电子商务") }
    }
}

impl Default for EcommerceIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for EcommerceIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "name".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "产品名称不能为空".to_string(),
            },
            ValidationDef {
                field: "total_amount".to_string(),
                r#type: "positive".to_string(),
                error_message: "订单金额必须大于零".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        vec![
            WorkflowStepDef {
                name: "同步库存".to_string(),
                description: "同步商品库存".to_string(),
                order: 1,
            },
            WorkflowStepDef {
                name: "履约".to_string(),
                description: "处理订单并发货".to_string(),
                order: 2,
            },
            WorkflowStepDef {
                name: "复盘".to_string(),
                description: "分析销售数据".to_string(),
                order: 3,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef { key: "total_revenue".to_string(), name: "总营收".to_string() },
            KpiCalculationDef { key: "orders_count".to_string(), name: "订单数".to_string() },
            KpiCalculationDef { key: "conversion_rate".to_string(), name: "转化率".to_string() },
            KpiCalculationDef {
                key: "customer_retention".to_string(),
                name: "客户留存率".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.automation_rules()
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "rev".to_string(),
                title: "总营收".to_string(),
                kpi_key: "total_revenue".to_string(),
            },
            DashboardCardDef {
                id: "orders".to_string(),
                title: "订单数".to_string(),
                kpi_key: "orders_count".to_string(),
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
            "product" => {
                if entity_data.get("name").is_none_or(|n| n.as_str().is_none_or(|s| s.is_empty())) {
                    errors.push(ValidationError::new("name", "产品名称不能为空"));
                }
                if let Some(price) = entity_data.get("price") {
                    if price.as_f64().is_none_or(|p| p < 0.0) {
                        errors.push(ValidationError::new("price", "产品价格不能为负数"));
                    }
                }
                if let Some(stock) = entity_data.get("stock_quantity") {
                    if stock.as_i64().is_none_or(|s| s < 0) {
                        errors.push(ValidationError::new("stock_quantity", "库存数量不能为负数"));
                    }
                }
            },
            "order" => {
                if let Some(total) = entity_data.get("total_amount") {
                    if total.as_f64().is_none_or(|t| t <= 0.0) {
                        errors.push(ValidationError::new("total_amount", "订单金额必须大于零"));
                    }
                }
                if let Some(status) = entity_data.get("status") {
                    let valid_statuses = ["pending", "paid", "shipped", "delivered", "cancelled"];
                    if status.as_str().is_none_or(|s| !valid_statuses.contains(&s)) {
                        errors.push(ValidationError::new("status", "无效的订单状态"));
                    }
                }
            },
            "customer"
                if entity_data
                    .get("email")
                    .is_none_or(|e| e.as_str().is_none_or(|s| !s.contains('@'))) =>
            {
                errors.push(ValidationError::new("email", "邮箱格式不正确"));
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

        let revenue = data.aggregate_invoice_amounts(&[InvoiceStatus::Paid], from, to).await?.total;
        let orders = data.count_invoices(&[InvoiceStatus::Paid], from, to).await? as f64;
        let active = data.count_customers(&[CustomerStatus::Active], from, to).await? as f64;
        let prospects = data
            .count_customers(&[CustomerStatus::Lead, CustomerStatus::Prospect], from, to)
            .await? as f64;
        let total_customers = data.count_customers(&[], from, to).await? as f64;

        let conversion_rate = if prospects > 0.0 {
            active / prospects * 100.0
        } else {
            0.0
        };
        let retention = if total_customers > 0.0 {
            active / total_customers * 100.0
        } else {
            0.0
        };

        Ok(vec![
            KpiValue {
                key: "total_revenue".to_string(),
                value: revenue,
                target: Some(100000.0),
                unit: Some("CNY".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "orders_count".to_string(),
                value: orders,
                target: Some(500.0),
                unit: Some("单".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "conversion_rate".to_string(),
                value: conversion_rate,
                target: Some(3.0),
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "customer_retention".to_string(),
                value: retention,
                target: Some(40.0),
                unit: Some("%".to_string()),
                timestamp: now,
            },
        ])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition {
                key: "total_revenue".to_string(),
                name: "总营收".to_string(),
                description: "订单总收入".to_string(),
                metric_type: super::super::analytics::MetricType::Currency,
                target: Some(100000.0),
                unit: Some("CNY".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "orders_count".to_string(),
                name: "订单数".to_string(),
                description: "完成订单数量".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(500.0),
                unit: Some("单".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "conversion_rate".to_string(),
                name: "转化率".to_string(),
                description: "访客下单转化率".to_string(),
                metric_type: super::super::analytics::MetricType::Percentage,
                target: Some(3.0),
                unit: Some("%".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "customer_retention".to_string(),
                name: "客户留存率".to_string(),
                description: "活跃客户占全部客户比例".to_string(),
                metric_type: super::super::analytics::MetricType::Percentage,
                target: Some(40.0),
                unit: Some("%".to_string()),
                ..Default::default()
            },
        ]
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "product".to_string(),
            "order".to_string(),
            "customer".to_string(),
            "inventory".to_string(),
            "review".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("sync", "同步库存", "同步商品库存").with_order(1),
            WorkflowStep::new("fulfill", "履约", "处理订单并发货").with_order(2),
            WorkflowStep::new("analyze", "复盘", "分析销售数据").with_order(3),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "low_stock",
                "库存预警",
                vec![AutomationCondition::EntityTypeIs { entity_type: "product".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "#operations".to_string(),
                    message: "商品库存低于阈值，需补充库存".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "ship_notify",
                "发货通知",
                vec![AutomationCondition::EntityTypeIs { entity_type: "order".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "客户邮箱".to_string(),
                    message: "您的订单已发货".to_string(),
                }],
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("rev", "总营收", "total_revenue", "CNY"),
            DashboardCard::new("orders", "订单数", "orders_count", "单"),
        ]
    }
}
