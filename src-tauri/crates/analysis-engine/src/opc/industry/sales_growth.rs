// 销售增长与营销行业适配器
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

pub struct SalesGrowthIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl SalesGrowthIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("sales_growth", "销售增长与营销") }
    }
}

impl Default for SalesGrowthIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for SalesGrowthIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "name".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "营销活动名称不能为空".to_string(),
            },
            ValidationDef {
                field: "deal_value".to_string(),
                r#type: "positive".to_string(),
                error_message: "交易金额必须大于零".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        vec![
            WorkflowStepDef {
                name: "线索生成".to_string(),
                description: "通过营销活动获取线索".to_string(),
                order: 1,
            },
            WorkflowStepDef {
                name: "线索培育".to_string(),
                description: "跟进并转化线索".to_string(),
                order: 2,
            },
            WorkflowStepDef {
                name: "赢单收盘".to_string(),
                description: "达成交易并维护客户".to_string(),
                order: 3,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef { key: "new_leads".to_string(), name: "新增线索".to_string() },
            KpiCalculationDef { key: "conversion_rate".to_string(), name: "转化率".to_string() },
            KpiCalculationDef {
                key: "pipeline_value".to_string(), name: "管道总值".to_string()
            },
            KpiCalculationDef {
                key: "customer_acquisition_cost".to_string(),
                name: "获客成本".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.automation_rules()
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "leads".to_string(),
                title: "新增线索".to_string(),
                kpi_key: "new_leads".to_string(),
            },
            DashboardCardDef {
                id: "conv".to_string(),
                title: "转化率".to_string(),
                kpi_key: "conversion_rate".to_string(),
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
            "campaign" => {
                if entity_data.get("name").is_none_or(|c| c.as_str().is_none_or(|s| s.is_empty())) {
                    errors.push(ValidationError::new("name", "营销活动名称不能为空"));
                }
                if let Some(budget) = entity_data.get("budget") {
                    if budget.as_f64().is_none_or(|b| b < 0.0) {
                        errors.push(ValidationError::new("budget", "活动预算不能为负数"));
                    }
                }
            },
            "lead" => {
                if entity_data.get("source").is_none_or(|s| s.as_str().is_none_or(|x| x.is_empty()))
                {
                    errors.push(ValidationError::new("source", "线索来源不能为空"));
                }
                if let Some(score) = entity_data.get("lead_score") {
                    if score.as_f64().is_none_or(|s| !(0.0..=100.0).contains(&s)) {
                        errors
                            .push(ValidationError::new("lead_score", "线索评分必须在 0-100 之间"));
                    }
                }
            },
            "deal" => {
                if let Some(value) = entity_data.get("deal_value") {
                    if value.as_f64().is_none_or(|v| v <= 0.0) {
                        errors.push(ValidationError::new("deal_value", "交易金额必须大于零"));
                    }
                }
                if let Some(stage) = entity_data.get("stage") {
                    let valid_stages = [
                        "lead",
                        "qualified",
                        "proposal",
                        "negotiation",
                        "closed_won",
                        "closed_lost",
                    ];
                    if stage.as_str().is_none_or(|s| !valid_stages.contains(&s)) {
                        errors.push(ValidationError::new("stage", "无效的交易阶段"));
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

        let new_leads = data
            .count_customers(&[CustomerStatus::Lead, CustomerStatus::Prospect], from, to)
            .await? as f64;
        let active = data.count_customers(&[CustomerStatus::Active], from, to).await? as f64;
        let total = data.count_customers(&[], from, to).await? as f64;
        let revenue = data.aggregate_invoice_amounts(&[InvoiceStatus::Paid], from, to).await?.total;

        let conversion_rate = if total > 0.0 {
            active / total * 100.0
        } else {
            0.0
        };
        let cac = if active > 0.0 { revenue / active } else { 0.0 };

        Ok(vec![
            KpiValue {
                key: "new_leads".to_string(),
                value: new_leads,
                target: Some(100.0),
                unit: Some("条".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "conversion_rate".to_string(),
                value: conversion_rate,
                target: Some(10.0),
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "pipeline_value".to_string(),
                value: revenue,
                target: Some(500000.0),
                unit: Some("CNY".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "customer_acquisition_cost".to_string(),
                value: cac,
                target: Some(2000.0),
                unit: Some("CNY".to_string()),
                timestamp: now,
            },
        ])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition {
                key: "new_leads".to_string(),
                name: "新增线索".to_string(),
                description: "营销活动获取的新线索数".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(100.0),
                unit: Some("条".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "conversion_rate".to_string(),
                name: "转化率".to_string(),
                description: "线索到客户的转化率".to_string(),
                metric_type: super::super::analytics::MetricType::Percentage,
                target: Some(10.0),
                unit: Some("%".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "pipeline_value".to_string(),
                name: "管道总值".to_string(),
                description: "在谈交易总价值".to_string(),
                metric_type: super::super::analytics::MetricType::Currency,
                target: Some(500000.0),
                unit: Some("CNY".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "customer_acquisition_cost".to_string(),
                name: "获客成本".to_string(),
                description: "平均获取每位客户的成本".to_string(),
                metric_type: super::super::analytics::MetricType::Currency,
                target: Some(2000.0),
                unit: Some("CNY".to_string()),
                ..Default::default()
            },
        ]
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "campaign".to_string(),
            "lead".to_string(),
            "deal".to_string(),
            "contact".to_string(),
            "marketing_channel".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("generate", "线索生成", "通过营销活动获取线索").with_order(1),
            WorkflowStep::new("nurture", "线索培育", "跟进并转化线索").with_order(2),
            WorkflowStep::new("close", "赢单收盘", "达成交易并维护客户").with_order(3),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "lead_followup",
                "线索跟进",
                vec![AutomationCondition::EntityTypeIs { entity_type: "lead".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "#sales".to_string(),
                    message: "新线索需在 24 小时内跟进".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "welcome_sequence",
                "欢迎邮件",
                vec![AutomationCondition::StatusIs { status: "prospect".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "线索邮箱".to_string(),
                    message: "已触发欢迎邮件序列".to_string(),
                }],
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("leads", "新增线索", "new_leads", "条"),
            DashboardCard::new("conv", "转化率", "conversion_rate", "%"),
        ]
    }
}
