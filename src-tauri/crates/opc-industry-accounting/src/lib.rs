// SPDX-License-Identifier: AGPL-3.0-only

//! 会计财务流程 行业适配器
//!
//! 从 YAML 配置迁移而来：config/opc/industries/accounting/

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axagent_opc_types::*;

/// 会计财务流程 行业适配器
pub struct AccountingAdapter {
    data_service: Mutex<Option<Arc<dyn OpcDataService>>>,
}

impl AccountingAdapter {
    pub const INDUSTRY_ID: &'static str = "accounting";
    pub const INDUSTRY_NAME: &'static str = "会计财务流程";

    pub fn new() -> Self {
        Self { data_service: Mutex::new(None) }
    }
}

impl Default for AccountingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for AccountingAdapter {
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
        let mut guard = self.data_service.lock().unwrap();
        *guard = Some(data_service);
    }

    fn data_service(&self) -> Option<Arc<dyn OpcDataService>> {
        let guard = self.data_service.lock().unwrap();
        guard.clone()
    }

    async fn validate(
        &self,
        entity_type: &str,
        entity_data: &serde_json::Value,
    ) -> OpcResult<Vec<ValidationError>> {
        let mut errors = Vec::new();

        match entity_type {
            "invoice" => {
                if let Some(total) = entity_data.get("total").and_then(|v| v.as_f64())
                    && total < 0.0
                {
                    errors.push(ValidationError::field("total", "发票总金额必须大于等于0"));
                }
            },
            "customer" => {
                if let Some(email) = entity_data.get("email").and_then(|v| v.as_str())
                    && !email.contains('@')
                {
                    errors.push(ValidationError::field("email", "客户邮箱格式不正确"));
                }
            },
            _ => {},
        }

        Ok(errors)
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "accounting_overdue_alert",
                "发票逾期提醒",
                vec![
                    AutomationCondition::OverdueDaysGte { days: 15 },
                    AutomationCondition::EntityTypeIs { entity_type: "invoice".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "customer".into(),
                    message: "您的发票即将逾期".into(),
                }],
            ),
            IndustryAutomationRule::new(
                "accounting_payment_reminder",
                "付款到期提醒",
                vec![
                    AutomationCondition::OverdueDaysGte { days: 7 },
                    AutomationCondition::StatusIs { status: "sent".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "finance_team".into(),
                    message: "有发票即将到期".into(),
                }],
            ),
        ]
    }

    async fn evaluate_rule(
        &self,
        rule: &IndustryAutomationRule,
        _context: &RuleContext,
    ) -> OpcResult<bool> {
        tracing::debug!("评估规则: {}", rule.name);
        Ok(false)
    }

    async fn execute_rule_actions(
        &self,
        rule: &IndustryAutomationRule,
        _context: &RuleContext,
    ) -> OpcResult<()> {
        for action in &rule.actions {
            match action {
                AutomationAction::UpdateStatus { status } => {
                    tracing::info!("规则 [{}]: 执行 UpdateStatus → {}", rule.name, status);
                },
                AutomationAction::SendNotification { target, message } => {
                    tracing::info!("规则 [{}]: 发送通知 → {} : {}", rule.name, target, message);
                },
                AutomationAction::CreateRecord { entity_type, data } => {
                    tracing::info!(
                        "规则 [{}]: 创建关联记录 → {} (数据: {:?})",
                        rule.name,
                        entity_type,
                        data
                    );
                },
                AutomationAction::UpdateField { field, value } => {
                    tracing::info!("规则 [{}]: 更新字段 → {} = {:?}", rule.name, field, value);
                },
                AutomationAction::MarkProcessed => {
                    tracing::info!("规则 [{}]: 标记为已处理", rule.name);
                },
            }
        }
        Ok(())
    }

    async fn compute_kpis(&self, time_range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
        let data = match self.data_service() {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        let from = time_range.start;
        let to = time_range.end;

        let invoice_count = data.count_invoices(&[], from, to).await.unwrap_or(0);
        let invoice_agg = data.aggregate_invoice_amounts(&[], from, to).await.unwrap_or_default();

        Ok(vec![
            KpiValue::new("invoice_count", invoice_count as f64),
            KpiValue::new("total_revenue", invoice_agg.total),
            KpiValue::new("collection_rate", 0.85),
            KpiValue::new("avg_processing_time", 3.0),
        ])
    }

    async fn aggregate_dashboard(&self, time_range: &TimeRange) -> OpcResult<IndustryDashboard> {
        let kpis = self.compute_kpis(time_range).await?;
        Ok(IndustryDashboard {
            industry_id: Self::INDUSTRY_ID.to_string(),
            kpis,
            cards: self.dashboard_cards(),
            summary: None,
        })
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("a-create", "创建发票", "根据用户信息创建发票").with_order(1),
            WorkflowStep::new("approval", "财务审批", "财务审批（24小时超时自动拒绝）")
                .with_order(2),
            WorkflowStep::new("a-notify", "通知客户", "发票已审批通过，通知客户").with_order(3),
            WorkflowStep::new("a-report", "登记报表", "记录发票相关关键指标").with_order(4),
        ]
    }

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("invoice_count", "发票数量", "张", MetricType::Count),
            KpiDefinition::new("total_revenue", "总营收", "CNY", MetricType::Currency),
            KpiDefinition::new("collection_rate", "回款率", "%", MetricType::Percentage),
            KpiDefinition::new("avg_processing_time", "平均处理时间", "天", MetricType::Duration),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("revenue_card", "本月营收", "total_revenue", "¥--"),
            DashboardCard::new("invoice_card", "本月发票数", "invoice_count", "-- 张"),
            DashboardCard::new("collection_card", "回款率", "collection_rate", "--%"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_info() {
        let adapter = AccountingAdapter::new();
        assert_eq!(adapter.industry_id(), "accounting");
        assert_eq!(adapter.industry_name(), "会计财务流程");
        assert_eq!(adapter.version(), 1);
        assert!(adapter.enabled());
    }

    #[tokio::test]
    async fn test_validate_invoice_valid() {
        let adapter = AccountingAdapter::new();
        let data = serde_json::json!({
            "total": 1000.0,
            "customer_id": "cust-001"
        });
        let errors = adapter.validate("invoice", &data).await.unwrap();
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_validate_invoice_negative_total() {
        let adapter = AccountingAdapter::new();
        let data = serde_json::json!({
            "total": -100.0
        });
        let errors = adapter.validate("invoice", &data).await.unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "total");
    }

    #[tokio::test]
    async fn test_validate_customer_valid_email() {
        let adapter = AccountingAdapter::new();
        let data = serde_json::json!({
            "email": "test@example.com",
            "name": "Test"
        });
        let errors = adapter.validate("customer", &data).await.unwrap();
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_validate_customer_invalid_email() {
        let adapter = AccountingAdapter::new();
        let data = serde_json::json!({
            "email": "invalid-email",
            "name": "Test"
        });
        let errors = adapter.validate("customer", &data).await.unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "email");
    }

    #[test]
    fn test_workflow_steps() {
        let adapter = AccountingAdapter::new();
        let steps = adapter.workflow_steps();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0].id, "a-create");
        assert_eq!(steps[0].order, 1);
        assert_eq!(steps[3].id, "a-report");
        assert_eq!(steps[3].order, 4);
    }

    #[test]
    fn test_kpi_definitions() {
        let adapter = AccountingAdapter::new();
        let kpis = adapter.kpi_definitions();
        assert_eq!(kpis.len(), 4);
        assert_eq!(kpis[0].key, "invoice_count");
        assert_eq!(kpis[1].key, "total_revenue");
        assert_eq!(kpis[2].key, "collection_rate");
        assert_eq!(kpis[3].key, "avg_processing_time");
    }

    #[test]
    fn test_dashboard_cards() {
        let adapter = AccountingAdapter::new();
        let cards = adapter.dashboard_cards();
        assert_eq!(cards.len(), 3);
        assert_eq!(cards[0].id, "revenue_card");
        assert_eq!(cards[1].id, "invoice_card");
        assert_eq!(cards[2].id, "collection_card");
    }

    #[test]
    fn test_automation_rules() {
        let adapter = AccountingAdapter::new();
        let rules = adapter.automation_rules();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].id, "accounting_overdue_alert");
        assert_eq!(rules[1].id, "accounting_payment_reminder");
        assert!(rules[0].enabled);
        assert!(rules[1].enabled);
    }

    #[test]
    fn test_data_service_injection() {
        let adapter = AccountingAdapter::new();
        assert!(adapter.data_service().is_none());

        let mock_service = Arc::new(MockDataService::default());
        adapter.set_data_service(mock_service.clone());
        assert!(adapter.data_service().is_some());
    }

    #[tokio::test]
    async fn test_compute_kpis_with_data_service() {
        let adapter = AccountingAdapter::new();
        let mock_service = Arc::new(MockDataService::default());
        adapter.set_data_service(mock_service);

        let range = TimeRange::days(30);
        let kpis = adapter.compute_kpis(&range).await.unwrap();
        assert!(!kpis.is_empty());
    }

    #[tokio::test]
    async fn test_aggregate_dashboard() {
        let adapter = AccountingAdapter::new();
        let mock_service = Arc::new(MockDataService::default());
        adapter.set_data_service(mock_service);

        let range = TimeRange::days(30);
        let dashboard = adapter.aggregate_dashboard(&range).await.unwrap();
        assert_eq!(dashboard.industry_id, "accounting");
        assert!(!dashboard.kpis.is_empty());
        assert!(!dashboard.cards.is_empty());
    }

    #[test]
    fn test_state_transitions() {
        let adapter = AccountingAdapter::new();
        let transitions = adapter.state_transitions();
        assert!(transitions.is_empty());
    }

    #[tokio::test]
    async fn test_evaluate_rule_default() {
        let adapter = AccountingAdapter::new();
        let rule = IndustryAutomationRule::new("test_rule", "测试规则", vec![], vec![]);
        let ctx = RuleContext::new("invoice", "inv-001");
        let result = adapter.evaluate_rule(&rule, &ctx).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_execute_rule_actions() {
        let adapter = AccountingAdapter::new();
        let rule = IndustryAutomationRule::new(
            "test_rule",
            "测试规则",
            vec![],
            vec![AutomationAction::UpdateStatus { status: "paid".to_string() }],
        );
        let ctx = RuleContext::new("invoice", "inv-001");
        let result = adapter.execute_rule_actions(&rule, &ctx).await;
        assert!(result.is_ok());
    }
}
