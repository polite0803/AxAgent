// 金融投资行业适配器
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::invoice::InvoiceStatus;
use super::super::project::ProjectStatus;
use super::super::rules::ValidationError;
use super::super::workflow::{DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowStepDef};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct FinanceInvestIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl FinanceInvestIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("finance_invest", "金融投资") }
    }
}

impl Default for FinanceInvestIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for FinanceInvestIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "total_value".to_string(),
                r#type: "non_negative".to_string(),
                error_message: "组合总价值不能为负数".to_string(),
            },
            ValidationDef {
                field: "amount".to_string(),
                r#type: "positive".to_string(),
                error_message: "交易金额必须大于零".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        vec![
            WorkflowStepDef {
                name: "市场分析".to_string(),
                description: "宏观经济与市场趋势分析".to_string(),
                prompt: Some(
                    "你是一名资深投资分析师。请分析宏观经济与市场趋势，识别投资机会。输出 JSON {market_view, key_sectors, risk_factors}".to_string(),
                ),
                tools: vec!["OpcGetDashboard".to_string(), "OpcListKpis".to_string(), "OpcListCustomers".to_string()],
                agent_profile_id: Some("opc-cio-cio-investment-analyst".to_string()),
                error_handling: "stop".to_string(),
                order: 1,
            },
            WorkflowStepDef {
                name: "行业研究".to_string(),
                description: "研究标的与行业趋势".to_string(),
                prompt: Some(
                    "你是一名行业研究专家。请深入研究目标行业与个股。输出 JSON {industry_outlook, stock_analysis, valuation}".to_string(),
                ),
                tools: vec!["OpcSearchWiki".to_string(), "OpcListProjects".to_string()],
                agent_profile_id: Some("opc-cio-cio-industry-researcher".to_string()),
                error_handling: "stop".to_string(),
                order: 2,
            },
            WorkflowStepDef {
                name: "资产配置".to_string(),
                description: "构建并调整投资组合".to_string(),
                prompt: Some(
                    "你是一名资产配置专家。请根据分析结果构建最优投资组合。输出 JSON {allocation, positions, rebalance_plan}".to_string(),
                ),
                tools: vec!["OpcGetFinancialReport".to_string(), "OpcGetDashboard".to_string()],
                agent_profile_id: Some("opc-cio-cio-asset-allocation".to_string()),
                error_handling: "stop".to_string(),
                order: 3,
            },
            WorkflowStepDef {
                name: "交易执行".to_string(),
                description: "执行交易并监控".to_string(),
                prompt: Some(
                    "你是一名交易执行专家。请执行交易并实时监控市场。输出 JSON {executed_trades, pnl, alerts}".to_string(),
                ),
                tools: vec!["OpcSendNotification".to_string(), "OpcGetDashboard".to_string()],
                agent_profile_id: Some("opc-cio-cio-trading-executor".to_string()),
                error_handling: "continue".to_string(),
                order: 4,
            },
            WorkflowStepDef {
                name: "回顾复盘".to_string(),
                description: "分析组合表现并再平衡".to_string(),
                prompt: Some(
                    "你是一名投资回顾专家。请分析组合表现并提出再平衡建议。输出 JSON {performance_attribution, rebalance_recommendation, lessons_learned}".to_string(),
                ),
                tools: vec!["OpcGetFinancialReport".to_string(), "OpcRecordKpi".to_string()],
                agent_profile_id: Some("opc-cio-cio-portfolio-reviewer".to_string()),
                error_handling: "continue".to_string(),
                order: 5,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "portfolio_return".to_string(),
                name: "组合收益率".to_string(),
            },
            KpiCalculationDef { key: "max_drawdown".to_string(), name: "最大回撤".to_string() },
            KpiCalculationDef { key: "sharpe_ratio".to_string(), name: "夏普比率".to_string() },
            KpiCalculationDef {
                key: "trades_executed".to_string(), name: "交易笔数".to_string()
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.automation_rules()
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "ret".to_string(),
                title: "组合收益率".to_string(),
                kpi_key: "portfolio_return".to_string(),
            },
            DashboardCardDef {
                id: "trades".to_string(),
                title: "交易笔数".to_string(),
                kpi_key: "trades_executed".to_string(),
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
            "portfolio" => {
                if entity_data.get("total_value").is_none_or(|v| v.as_f64().is_none_or(|x| x < 0.0))
                {
                    errors.push(ValidationError::new("total_value", "组合总价值不能为负数"));
                }
                if let Some(allocations) = entity_data.get("allocations") {
                    if allocations.as_array().is_some_and(|arr| {
                        arr.iter().any(|a| {
                            a.get("weight")
                                .and_then(|w| w.as_f64())
                                .is_some_and(|w| !(0.0..=1.0).contains(&w))
                        })
                    }) {
                        errors.push(ValidationError::new(
                            "allocations",
                            "资产配置权重必须在 0-1 之间",
                        ));
                    }
                }
            },
            "transaction" => {
                if entity_data.get("amount").is_none_or(|a| a.as_f64().is_none_or(|x| x <= 0.0)) {
                    errors.push(ValidationError::new("amount", "交易金额必须大于零"));
                }
                if let Some(tx_type) = entity_data.get("type") {
                    let valid_types = ["buy", "sell", "dividend", "transfer"];
                    if tx_type.as_str().is_none_or(|t| !valid_types.contains(&t)) {
                        errors.push(ValidationError::new("type", "无效的交易类型"));
                    }
                }
            },
            "research_note"
                if entity_data
                    .get("title")
                    .is_none_or(|t| t.as_str().is_none_or(|s| s.is_empty())) =>
            {
                errors.push(ValidationError::new("title", "研究笔记标题不能为空"));
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

        let invested = data.aggregate_project_budgets(&[], from, to).await?.total;
        let realized =
            data.aggregate_invoice_amounts(&[InvoiceStatus::Paid], from, to).await?.total;
        let trades = data.count_invoices(&[InvoiceStatus::Paid], from, to).await? as f64;
        let risk_projects = data
            .count_projects(&[ProjectStatus::Paused, ProjectStatus::Cancelled], from, to)
            .await? as f64;
        let total_projects = data.count_projects(&[], from, to).await? as f64;

        let portfolio_return = if invested > 0.0 {
            realized / invested * 100.0
        } else {
            0.0
        };
        let max_drawdown = if total_projects > 0.0 {
            risk_projects / total_projects * 100.0
        } else {
            0.0
        };
        let sharpe_ratio = if trades > 0.0 { realized / trades } else { 0.0 };

        Ok(vec![
            KpiValue {
                key: "portfolio_return".to_string(),
                value: portfolio_return,
                target: Some(15.0),
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "max_drawdown".to_string(),
                value: max_drawdown,
                target: Some(10.0),
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "sharpe_ratio".to_string(),
                value: sharpe_ratio,
                target: Some(1.5),
                unit: Some("".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "trades_executed".to_string(),
                value: trades,
                target: Some(50.0),
                unit: Some("笔".to_string()),
                timestamp: now,
            },
        ])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition {
                key: "portfolio_return".to_string(),
                name: "组合收益率".to_string(),
                description: "投资组合总回报".to_string(),
                metric_type: super::super::analytics::MetricType::Percentage,
                target: Some(15.0),
                unit: Some("%".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "max_drawdown".to_string(),
                name: "最大回撤".to_string(),
                description: "历史最大亏损幅度".to_string(),
                metric_type: super::super::analytics::MetricType::Percentage,
                target: Some(10.0),
                unit: Some("%".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "sharpe_ratio".to_string(),
                name: "夏普比率".to_string(),
                description: "风险调整后收益".to_string(),
                metric_type: super::super::analytics::MetricType::Ratio,
                target: Some(1.5),
                unit: Some("".to_string()),
                ..Default::default()
            },
            KpiDefinition {
                key: "trades_executed".to_string(),
                name: "交易笔数".to_string(),
                description: "已完成交易数量".to_string(),
                metric_type: super::super::analytics::MetricType::Counter,
                target: Some(50.0),
                unit: Some("笔".to_string()),
                ..Default::default()
            },
        ]
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "portfolio".to_string(),
            "transaction".to_string(),
            "research_note".to_string(),
            "watchlist".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("research", "行业研究", "研究标的与行业趋势").with_order(1),
            WorkflowStep::new("allocate", "资产配置", "构建并调整投资组合").with_order(2),
            WorkflowStep::new("review", "回顾复盘", "分析组合表现并再平衡").with_order(3),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "risk_alert",
                "风控预警",
                vec![AutomationCondition::EntityTypeIs { entity_type: "portfolio".to_string() }],
                vec![AutomationAction::SendNotification {
                    target: "#risk".to_string(),
                    message: "组合回撤超过阈值，需关注".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "rebalance_reminder",
                "再平衡提醒",
                vec![AutomationCondition::CreatedDaysGte { days: 30 }],
                vec![AutomationAction::SendNotification {
                    target: "投资者邮箱".to_string(),
                    message: "资产配置偏离目标，建议执行再平衡".to_string(),
                }],
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("ret", "组合收益率", "portfolio_return", "%"),
            DashboardCard::new("trades", "交易笔数", "trades_executed", "笔"),
        ]
    }
}
