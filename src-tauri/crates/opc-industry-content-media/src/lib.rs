// SPDX-License-Identifier: AGPL-3.0-only

//! 内容营销流程 行业适配器
//!
//! 从 YAML 配置迁移而来：config/opc/industries/content_media/

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axagent_opc_types::*;

/// 内容营销流程 行业适配器
pub struct ContentMediaAdapter {
    data_service: Mutex<Option<Arc<dyn OpcDataService>>>,
}

impl ContentMediaAdapter {
    pub const INDUSTRY_ID: &'static str = "content_media";
    pub const INDUSTRY_NAME: &'static str = "内容营销流程";

    pub fn new() -> Self {
        Self { data_service: Mutex::new(None) }
    }
}

impl Default for ContentMediaAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for ContentMediaAdapter {
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
            "post" => {
                if entity_data.get("title").and_then(|v| v.as_str()).is_none_or(|s| s.is_empty()) {
                    errors.push(ValidationError::field("title", "文章标题不能为空"));
                }
                if let Some(content) = entity_data.get("content").and_then(|v| v.as_str()) {
                    if content.len() < 50 {
                        errors.push(ValidationError::field("content", "文章内容至少需要50个字符"));
                    }
                } else {
                    errors.push(ValidationError::field("content", "文章内容不能为空"));
                }
            },
            "landing_page" => {
                if let Some(url) = entity_data.get("url").and_then(|v| v.as_str()) {
                    if !url.starts_with("http://") && !url.starts_with("https://") {
                        errors.push(ValidationError::field(
                            "url",
                            "落地页URL格式无效，需以http://或https://开头",
                        ));
                    }
                } else {
                    errors.push(ValidationError::field("url", "落地页URL不能为空"));
                }
            },
            _ => {},
        }

        Ok(errors)
    }

    fn workflow_steps(&self) -> Vec<WorkflowStep> {
        vec![
            WorkflowStep::new("a-topic", "选题策划", "分析市场和客户数据，策划内容主题")
                .with_order(1),
            WorkflowStep::new("a-create", "内容创作", "根据选题创作内容，发布博客").with_order(2),
            WorkflowStep::new("a-landing", "创建落地页", "为内容创建落地页").with_order(3),
        ]
    }

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("content_count", "内容数量", "篇", MetricType::Count),
            KpiDefinition::new("page_views", "页面浏览量", "次", MetricType::Count),
            KpiDefinition::new("conversion_rate", "转化率", "%", MetricType::Percentage),
            KpiDefinition::new("content_engagement", "内容互动率", "%", MetricType::Percentage),
        ]
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "content_publish_schedule",
                "内容发布提醒",
                vec![
                    AutomationCondition::CreatedDaysGte { days: 7 },
                    AutomationCondition::EntityTypeIs { entity_type: "post".to_string() },
                    AutomationCondition::StatusIs { status: "draft".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "editor".to_string(),
                    message: "草稿已创建7天，建议尽快发布".to_string(),
                }],
            ),
            IndustryAutomationRule::new(
                "content_performance_alert",
                "内容绩效预警",
                vec![
                    AutomationCondition::FieldBelow {
                        field: "engagement_rate".to_string(),
                        threshold: 2.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "post".to_string() },
                    AutomationCondition::StatusIs { status: "published".to_string() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "content_team".to_string(),
                    message: "文章互动率低于2%，需要优化".to_string(),
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
            DashboardCard::new("content_card", "本月内容数", "content_count", "-- 篇"),
            DashboardCard::new("views_card", "页面浏览", "page_views", "-- 次"),
            DashboardCard::new("conversion_card", "转化率", "conversion_rate", "--%"),
        ]
    }
}
