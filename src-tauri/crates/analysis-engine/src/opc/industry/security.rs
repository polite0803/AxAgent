// 安全合规行业适配器
use std::sync::Arc;

use async_trait::async_trait;

use super::super::analytics::{KpiDefinition, KpiValue, MetricType};
use super::super::automation::{AutomationAction, AutomationCondition, IndustryAutomationRule};
use super::super::data_service::{OpcDataService, TimeRange};
use super::super::error::OpcResult;
use super::super::rules::ValidationError;
use super::super::workflow::{DashboardCardDef, KpiCalculationDef, ValidationDef, WorkflowStepDef};
use super::{
    impl_industry_base, BaseIndustryAdapter, DashboardCard, OpcIndustryAdapter, WorkflowStep,
};

pub struct SecurityIndustryAdapter {
    base: BaseIndustryAdapter,
}

impl SecurityIndustryAdapter {
    pub fn new() -> Self {
        Self { base: BaseIndustryAdapter::new("security", "安全合规") }
    }
}

impl Default for SecurityIndustryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for SecurityIndustryAdapter {
    impl_industry_base!();

    fn define_validations(&self) -> Vec<ValidationDef> {
        vec![
            ValidationDef {
                field: "policy_id".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "政策 ID 不能为空".to_string(),
            },
            ValidationDef {
                field: "severity".to_string(),
                r#type: "not_empty".to_string(),
                error_message: "风险等级不能为空".to_string(),
            },
        ]
    }

    fn define_workflow_steps(&self) -> Vec<WorkflowStepDef> {
        vec![
            WorkflowStepDef {
                name: "风险识别".to_string(),
                description: "识别和评估潜在的安全风险".to_string(),
                order: 1,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "风险分析".to_string(),
                description: "分析风险影响范围和可能性".to_string(),
                order: 2,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "风险处置".to_string(),
                description: "制定风险应对策略和控制措施".to_string(),
                order: 3,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "合规审计".to_string(),
                description: "定期进行合规性检查和审计".to_string(),
                order: 4,
                ..Default::default()
            },
            WorkflowStepDef {
                name: "事件响应".to_string(),
                description: "安全事件发生后的应急响应".to_string(),
                order: 5,
                ..Default::default()
            },
        ]
    }

    fn define_kpi_calculations(&self) -> Vec<KpiCalculationDef> {
        vec![
            KpiCalculationDef {
                key: "security_incidents".to_string(),
                name: "安全事件数".to_string(),
            },
            KpiCalculationDef {
                key: "mean_time_to_resolve".to_string(),
                name: "平均修复时间".to_string(),
            },
            KpiCalculationDef {
                key: "compliance_rate".to_string(),
                name: "合规达标率".to_string(),
            },
            KpiCalculationDef {
                key: "vulnerability_fix_rate".to_string(),
                name: "漏洞修复率".to_string(),
            },
        ]
    }

    fn define_automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "critical_vulnerability_alert",
                "严重漏洞告警",
                vec![
                    AutomationCondition::StatusIs {
                        status: "critical".to_string(),
                    },
                    AutomationCondition::EntityTypeIs {
                        entity_type: "security_vulnerability".to_string(),
                    },
                ],
                vec![AutomationAction::SendNotification {
                    target: "security_team".to_string(),
                    message: "检测到严重安全漏洞".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "compliance_check_reminder",
                "合规检查提醒",
                vec![
                    AutomationCondition::OverdueDaysGte { days: 7 },
                    AutomationCondition::EntityTypeIs {
                        entity_type: "compliance_audit".to_string(),
                    },
                ],
                vec![AutomationAction::SendNotification {
                    target: "compliance_team".to_string(),
                    message: "合规审计将在 7 天内进行".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "access_anomaly_detection",
                "访问异常检测",
                vec![
                    AutomationCondition::FieldExceeds {
                        field: "anomaly_score".to_string(),
                        threshold: 0.95,
                    },
                    AutomationCondition::EntityTypeIs {
                        entity_type: "access_log".to_string(),
                    },
                ],
                vec![AutomationAction::SendNotification {
                    target: "security_team".to_string(),
                    message: "检测到异常访问行为".to_string(),
                }],
            ),
        ]
    }

    fn define_dashboard_cards(&self) -> Vec<DashboardCardDef> {
        vec![
            DashboardCardDef {
                id: "active_incidents".to_string(),
                title: "活跃安全事件".to_string(),
                kpi_key: "active_security_incidents".to_string(),
            },
            DashboardCardDef {
                id: "compliance_score".to_string(),
                title: "合规达标率".to_string(),
                kpi_key: "compliance_rate".to_string(),
            },
            DashboardCardDef {
                id: "vulnerability_status".to_string(),
                title: "漏洞修复进度".to_string(),
                kpi_key: "vulnerability_fix_rate".to_string(),
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
            "security_policy" => {
                if entity_data.get("policy_id").is_none_or(|p| p.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("policy_id", "政策 ID 不能为空"));
                }
                if entity_data.get("title").is_none_or(|t| t.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("title", "政策标题不能为空"));
                }
            },
            "security_incident" => {
                if entity_data
                    .get("severity")
                    .is_none_or(|s| s.as_str().is_none_or(|sev| sev.is_empty()))
                {
                    errors.push(ValidationError::new("severity", "风险等级不能为空"));
                }
                if let Some(severity) = entity_data.get("severity") {
                    let valid_severities = ["low", "medium", "high", "critical"];
                    if severity.as_str().is_none_or(|s| !valid_severities.contains(&s)) {
                        errors.push(ValidationError::new("severity", "无效的风险等级"));
                    }
                }
            },
            "vulnerability" => {
                if entity_data.get("cve_id").is_none_or(|c| c.as_str().is_none_or(|s| s.is_empty()))
                {
                    errors.push(ValidationError::new("cve_id", "CVE ID 不能为空"));
                }
                if let Some(status) = entity_data.get("fix_status") {
                    let valid_statuses = ["open", "in_progress", "fixed", "accepted"];
                    if status.as_str().is_none_or(|s| !valid_statuses.contains(&s)) {
                        errors.push(ValidationError::new("fix_status", "无效的修复状态"));
                    }
                }
            },
            _ => {}
        }

        Ok(errors)
    }

    fn entity_types(&self) -> Vec<String> {
        vec![
            "security_policy".to_string(),
            "security_incident".to_string(),
            "vulnerability".to_string(),
            "compliance_audit".to_string(),
        ]
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("risk_identification", "风险识别", "识别和评估潜在的安全风险").with_order(1),
            WorkflowStep::new("risk_analysis", "风险分析", "分析风险影响范围和可能性").with_order(2),
            WorkflowStep::new("risk_treatment", "风险处置", "制定风险应对策略和控制措施").with_order(3),
            WorkflowStep::new("compliance_audit", "合规审计", "定期进行合规性检查和审计").with_order(4),
            WorkflowStep::new("incident_response", "事件响应", "安全事件发生后的应急响应").with_order(5),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        self.define_automation_rules()
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        self.define_dashboard_cards()
            .into_iter()
            .map(|c| DashboardCard::new(&c.id, &c.title, &c.kpi_key, "—"))
            .collect()
    }

    fn default_kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("security_incidents", "安全事件数", "次", MetricType::Count),
            KpiDefinition::new("mean_time_to_resolve", "平均修复时间", "小时", MetricType::Gauge),
            KpiDefinition::new("compliance_rate", "合规达标率", "%", MetricType::Percentage),
            KpiDefinition::new("vulnerability_fix_rate", "漏洞修复率", "%", MetricType::Percentage),
        ]
    }

    async fn compute_kpis(&self, time_range: &TimeRange) -> OpcResult<Vec<KpiValue>> {
        let _ = time_range;
        let now = chrono::Utc::now().timestamp();
        Ok(vec![
            KpiValue {
                key: "security_incidents".to_string(),
                value: 0.0,
                target: None,
                unit: Some("次".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "mean_time_to_resolve".to_string(),
                value: 0.0,
                target: None,
                unit: Some("小时".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "compliance_rate".to_string(),
                value: 0.0,
                target: None,
                unit: Some("%".to_string()),
                timestamp: now,
            },
            KpiValue {
                key: "vulnerability_fix_rate".to_string(),
                value: 0.0,
                target: None,
                unit: Some("%".to_string()),
                timestamp: now,
            },
        ])
    }
}
