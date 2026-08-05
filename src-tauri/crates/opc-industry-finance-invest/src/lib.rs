// SPDX-License-Identifier: AGPL-3.0-only

//! 金融投资分析 行业适配器
//!
//! 从 YAML 配置迁移而来：config/opc/industries/finance_invest/

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axagent_opc_types::*;

/// 金融投资分析 行业适配器
pub struct FinanceInvestAdapter {
    data_service: Mutex<Option<Arc<dyn OpcDataService>>>,
}

impl FinanceInvestAdapter {
    pub const INDUSTRY_ID: &'static str = "finance_invest";
    pub const INDUSTRY_NAME: &'static str = "金融投资分析";

    pub fn new() -> Self {
        Self { data_service: Mutex::new(None) }
    }
}

impl Default for FinanceInvestAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for FinanceInvestAdapter {
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
                if let Some(amount) = entity_data.get("amount").and_then(|v| v.as_f64())
                    && amount <= 0.0
                {
                    errors.push(ValidationError::field("amount", "发票金额必须大于0"));
                }
            },
            "project" if entity_data.get("budget").is_none() => {
                errors.push(ValidationError::field("budget", "项目必须包含预算信息"));
            },
            _ => {},
        }

        if let Some(investment) = entity_data.get("investment_amount").and_then(|v| v.as_f64())
            && investment <= 0.0
        {
            errors.push(ValidationError::field("investment_amount", "投资金额必须为正数"));
        }

        Ok(errors)
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "finance_overdue_escalation",
                "逾期发票升级处理",
                vec![
                    AutomationCondition::OverdueDaysGte { days: 30 },
                    AutomationCondition::EntityTypeIs { entity_type: "invoice".into() },
                ],
                vec![
                    AutomationAction::UpdateStatus { status: "high_risk".into() },
                    AutomationAction::SendNotification {
                        target: "finance_team".into(),
                        message: "逾期30天以上的发票需要关注".into(),
                    },
                ],
            ),
            IndustryAutomationRule::new(
                "finance_risk_alert",
                "风险敞口预警",
                vec![AutomationCondition::FieldExceeds {
                    field: "risk_exposure".into(),
                    threshold: 1_000_000.0,
                }],
                vec![AutomationAction::SendNotification {
                    target: "risk_team".into(),
                    message: "风险敞口超过阈值".into(),
                }],
            ),
            IndustryAutomationRule::new(
                "finance_investment_review",
                "大额投资审核",
                vec![
                    AutomationCondition::FieldExceeds {
                        field: "amount".into(),
                        threshold: 500_000.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "project".into() },
                ],
                vec![AutomationAction::CreateRecord {
                    entity_type: "review".into(),
                    data: serde_json::json!({
                        "reason": "大额投资自动触发审核",
                    }),
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

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("a-report", "财务数据获取", "获取财务报表和运营数据").with_order(1),
            WorkflowStep::new("a-validate", "数据验证", "校验财务数据的完整性和合理性")
                .with_order(2),
            WorkflowStep::new("a-risk", "风险评估", "基于财务数据计算风险指标").with_order(3),
            WorkflowStep::new("a-advice", "投资建议", "基于财务分析和风险评估生成投资建议")
                .with_order(4),
        ]
    }

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("total_assets", "总资产", "CNY", MetricType::Currency),
            KpiDefinition::new("revenue_growth", "营收增长率", "%", MetricType::Percentage),
            KpiDefinition::new("risk_level", "风险等级", "级", MetricType::Count),
            KpiDefinition::new("roi", "投资回报率", "%", MetricType::Percentage),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("assets_card", "总资产", "total_assets", "¥--"),
            DashboardCard::new("growth_card", "营收增长", "revenue_growth", "--%"),
            DashboardCard::new("roi_card", "投资回报", "roi", "--%"),
        ]
    }
}
