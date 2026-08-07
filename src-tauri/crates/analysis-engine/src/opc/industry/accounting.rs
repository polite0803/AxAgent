// 会计与财务管理行业适配器
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::invoice::InvoiceStatus;
use super::super::rules::ValidationError;
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};
use super::super::workflow::{
    DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowStepDef,
};

pub struct AccountingIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl AccountingIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("accounting", "会计与财务管理") }
    }
}

impl Default for AccountingIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for AccountingIndustryAdapter {
    impl_industry_base!();

    // ── 工作流元素定义（从 runtime.yaml 转换为标准 WorkflowNode） ──

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "total".to_string(),
                r#type: "non_negative".to_string(),
                error_message: "发票总金额必须大于等于0".to_string(),
            },
            ValidationDef {
                field: "email".to_string(),
                r#type: "contains_at".to_string(),
                error_message: "客户邮箱格式不正确".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        vec![
            WorkflowStepDef {
                name: "创建发票".to_string(),
                description: "根据用户信息创建发票".to_string(),
                order: 1,
            },
            WorkflowStepDef {
                name: "财务审批".to_string(),
                description: "财务审批（24小时超时自动拒绝）".to_string(),
                order: 2,
            },
            WorkflowStepDef {
                name: "通知客户".to_string(),
                description: "发票已审批通过，通知客户".to_string(),
                order: 3,
            },
            WorkflowStepDef {
                name: "登记报表".to_string(),
                description: "记录发票相关关键指标".to_string(),
                order: 4,
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef { key: "invoice_count".to_string(), name: "发票数量".to_string() },
            KpiCalculationDef { key: "total_revenue".to_string(), name: "总营收".to_string() },
            KpiCalculationDef { key: "collection_rate".to_string(), name: "回款率".to_string() },
            KpiCalculationDef { key: "avg_processing_time".to_string(), name: "平均处理时间".to_string() },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "accounting_overdue_alert",
                "发票逾期提醒",
                vec![
                    AutomationCondition::OverdueDaysGte { days: 15 },
                    AutomationCondition::EntityTypeIs { entity_type: "invoice".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "customer".to_string(),
                    message: "您的发票即将逾期".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "accounting_payment_reminder",
                "付款到期提醒",
                vec![
                    AutomationCondition::OverdueDaysGte { days: 7 },
                    AutomationCondition::StatusIs { status: "sent".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "finance_team".to_string(),
                    message: "有发票即将到期".to_string(),
                }],
            ),
        ]
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef { id: "revenue_card".to_string(), title: "本月营收".to_string(), kpi_key: "total_revenue".to_string() },
            DashboardCardDef { id: "invoice_card".to_string(), title: "本月发票数".to_string(), kpi_key: "invoice_count".to_string() },
            DashboardCardDef { id: "collection_card".to_string(), title: "回款率".to_string(), kpi_key: "collection_rate".to_string() },
        ]
    }

    fn requires_approval(&self) -> bool {
        true
    }

    // ── 原有业务逻辑实现（保留，作为节点执行时的调用目标） ──

    async fn validate(
        &self,
        entity_type: &str,
        entity_data: &serde_json::Value,
    ) -> OpcResult<Vec<ValidationError>> {
        let mut errors = Vec::new();

        match entity_type {
            "invoice" => {
                if let Some(amount) = entity_data.get("amount") {
                    if amount.as_f64().is_none_or(|a| a <= 0.0) {
                        errors.push(ValidationError::new("amount", "发票金额必须大于零"));
                    }
                }
                if let Some(status) = entity_data.get("status") {
                    let valid_statuses = [
                        InvoiceStatus::Draft.as_str(),
                        InvoiceStatus::Sent.as_str(),
                        InvoiceStatus::Paid.as_str(),
                        InvoiceStatus::Overdue.as_str(),
                        InvoiceStatus::Cancelled.as_str(),
                        InvoiceStatus::Refunded.as_str(),
                    ];
                    if status.as_str().is_none_or(|s| !valid_statuses.contains(&s)) {
                        errors.push(ValidationError::new("status", "无效的发票状态"));
                    }
                }
            },
            "finance_record" => {
                if let Some(amount) = entity_data.get("amount") {
                    if amount.as_f64().is_none_or(|a| a == 0.0) {
                        errors.push(ValidationError::new("amount", "财务记录金额不能为零"));
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

        let revenue = data.aggregate_invoice_amounts(&[InvoiceStatus::Paid], from, to).await?.total;
        let outstanding =
            data.count_invoices(&[InvoiceStatus::Sent, InvoiceStatus::Overdue], from, to).await? as f64;
        let total = data.count_invoices(&[], from, to).await? as f64;

        let collection_rate = if total > 0.0 {
            let paid = data.count_invoices(&[InvoiceStatus::Paid], from, to).await? as f64;
            paid / total
        } else {
            0.0
        };

        Ok(vec![
            KpiValue {
                key: "total_revenue".to_string(),
                value: revenue,
                target: None,
                unit: Some("元".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "outstanding_invoices".to_string(),
                value: outstanding,
                target: None,
                unit: Some("张".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "collection_rate".to_string(),
                value: collection_rate * 100.0,
                target: None,
                unit: Some("%".to_string()),
                timestamp: now,
            },
        ])
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        use super::super::analytics::MetricType;
        vec![
            KpiDefinition::new("total_revenue", "总营收", "元", MetricType::Currency),
            KpiDefinition::new("outstanding_invoices", "未结清发票", "张", MetricType::Count),
            KpiDefinition::new("collection_rate", "回款率", "%", MetricType::Percentage),
        ]
    }

    fn entity_types(&self) -> Vec<String> {
        vec!["invoice".to_string(), "finance_record".to_string(), "customer".to_string()]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("create_invoice", "创建发票", "根据用户信息创建发票").with_order(1),
            WorkflowStep::new("approval", "财务审批", "财务审批（24小时超时自动拒绝）").with_order(2),
            WorkflowStep::new("notify_customer", "通知客户", "发票已审批通过，通知客户").with_order(3),
            WorkflowStep::new("register_report", "登记报表", "记录发票相关关键指标").with_order(4),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.define_automation_rules()
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        self.define_dashboard_cards()
            .iter()
            .map(|c| DashboardCard::new(&c.id, &c.title, &c.kpi_key, ""))
            .collect()
    }
}
