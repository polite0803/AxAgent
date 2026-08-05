// SPDX-License-Identifier: AGPL-3.0-only

//! 软件开发完整流程 行业适配器
//!
//! 从 YAML 配置迁移而来：config/opc/industries/software_dev/

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axagent_opc_types::*;

/// 软件开发完整流程 行业适配器
pub struct SoftwareDevAdapter {
    data_service: Mutex<Option<Arc<dyn OpcDataService>>>,
}

impl SoftwareDevAdapter {
    pub const INDUSTRY_ID: &'static str = "software_dev";
    pub const INDUSTRY_NAME: &'static str = "软件开发完整流程";

    pub fn new() -> Self {
        Self { data_service: Mutex::new(None) }
    }
}

impl Default for SoftwareDevAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpcIndustryAdapter for SoftwareDevAdapter {
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
            "project" => {
                if let Some(title) = entity_data.get("title").and_then(|v| v.as_str()) {
                    if title.trim().is_empty() {
                        errors.push(ValidationError::field("title", "项目标题不能为空"));
                    }
                }
            },
            "sprint" => {
                if entity_data.get("start_date").is_none() {
                    errors.push(ValidationError::field("start_date", "迭代必须包含开始日期"));
                }
                if entity_data.get("end_date").is_none() {
                    errors.push(ValidationError::field("end_date", "迭代必须包含结束日期"));
                }
            },
            _ => {},
        }

        Ok(errors)
    }

    fn automation_rules(&self) -> Vec<IndustryAutomationRule> {
        vec![
            IndustryAutomationRule::new(
                "software_deadline_warning",
                "任务截止日期预警",
                vec![
                    AutomationCondition::CreatedDaysGte { days: 7 },
                    AutomationCondition::EntityTypeIs { entity_type: "task".into() },
                    AutomationCondition::StatusIs { status: "in_progress".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "assignee".into(),
                    message: "任务已进行7天，请注意截止日期".into(),
                }],
            ),
            IndustryAutomationRule::new(
                "software_code_review_reminder",
                "代码审查提醒",
                vec![
                    AutomationCondition::FieldExceeds {
                        field: "review_comments".into(),
                        threshold: 5.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "pull_request".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "author".into(),
                    message: "代码审查有超过5条评论需要处理".into(),
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
            WorkflowStep::new("a-req", "需求分析", "分析用户需求").with_order(1),
            WorkflowStep::new("a-feasibility", "可行性评审", "评估技术可行性、资源和风险")
                .with_order(2),
            WorkflowStep::new("a-feasibility-approval", "可行性审批", "批准项目进入架构设计阶段")
                .with_order(3),
            WorkflowStep::new("a-arch", "架构设计", "设计系统架构和技术栈").with_order(4),
            WorkflowStep::new("a-data", "数据模型设计", "设计数据库实体、关系和索引").with_order(5),
            WorkflowStep::new("a-api", "API设计", "设计RESTful API接口").with_order(6),
            WorkflowStep::new("a-setup", "项目环境搭建", "初始化开发环境").with_order(7),
            WorkflowStep::new("a-code", "编码实现", "按设计实现代码").with_order(8),
            WorkflowStep::new("a-cr", "代码审查", "审查代码质量").with_order(9),
            WorkflowStep::new("a-cr-approval", "代码审查审批", "代码审查是否通过").with_order(10),
            WorkflowStep::new("a-fix", "缺陷修复", "根据代码审查意见修复代码缺陷").with_order(11),
            WorkflowStep::new("a-doc", "文档编写", "生成设计文档、API文档和开发指南")
                .with_order(12),
            WorkflowStep::new("a-unit-test", "单元测试", "为核心模块编写单元测试").with_order(13),
            WorkflowStep::new("a-integration-test", "集成测试", "执行模块间集成测试和端到端测试")
                .with_order(14),
            WorkflowStep::new("a-security", "安全审查", "执行安全审计").with_order(15),
            WorkflowStep::new("a-deploy-approval", "部署审批", "批准部署到目标环境").with_order(16),
            WorkflowStep::new("a-deploy", "部署上线", "执行部署、构建、数据库迁移").with_order(17),
            WorkflowStep::new("a-handoff", "运维交接", "生成运维文档和交接包").with_order(18),
        ]
    }

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            KpiDefinition::new("sprint_count", "迭代数量", "次", MetricType::Count),
            KpiDefinition::new("code_coverage", "代码覆盖率", "%", MetricType::Percentage),
            KpiDefinition::new("bug_fix_rate", "缺陷修复率", "%", MetricType::Percentage),
            KpiDefinition::new("deploy_frequency", "部署频率", "次/月", MetricType::Ratio),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            DashboardCard::new("sprint_card", "迭代数", "sprint_count", "-- 次"),
            DashboardCard::new("coverage_card", "代码覆盖", "code_coverage", "--%"),
            DashboardCard::new("deploy_card", "部署频率", "deploy_frequency", "-- 次/月"),
        ]
    }
}
