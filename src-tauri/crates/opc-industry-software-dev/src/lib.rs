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
                if let Some(title) = entity_data.get("title").and_then(|v| v.as_str())
                    && title.trim().is_empty()
                {
                    errors.push(ValidationError::field("title", "项目标题不能为空"));
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
            // ── 通用软件开发规则 ──
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
            // ── 大型项目重构专属规则 ──
            IndustryAutomationRule::new(
                "refactor_complexity_warn",
                "圈复杂度预警",
                vec![
                    AutomationCondition::FieldExceeds {
                        field: "high_complexity_functions".into(),
                        threshold: 10.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "refactor_batch".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "tech_lead".into(),
                    message: "检测到超过10个圈复杂度>20的函数，建议优先重构".into(),
                }],
            ),
            IndustryAutomationRule::new(
                "refactor_coverage_check",
                "测试覆盖率门禁",
                vec![
                    AutomationCondition::FieldBelow {
                        field: "test_coverage".into(),
                        threshold: 80.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "refactor_batch".into() },
                ],
                vec![
                    AutomationAction::UpdateStatus { status: "blocked".into() },
                    AutomationAction::SendNotification {
                        target: "assignee".into(),
                        message: "测试覆盖率低于80%，阻断进入下一批次".into(),
                    },
                ],
            ),
            IndustryAutomationRule::new(
                "refactor_deadline_alert",
                "重构批次超时预警",
                vec![
                    AutomationCondition::CreatedDaysGte { days: 14 },
                    AutomationCondition::EntityTypeIs { entity_type: "refactor_batch".into() },
                    AutomationCondition::StatusIs { status: "in_progress".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "project_manager".into(),
                    message: "重构批次停留时间超过14天，需要关注".into(),
                }],
            ),
            IndustryAutomationRule::new(
                "refactor_regression_fail",
                "回归测试失败触发回滚",
                vec![
                    AutomationCondition::StatusIs { status: "regression_failed".into() },
                    AutomationCondition::EntityTypeIs { entity_type: "refactor_batch".into() },
                ],
                vec![
                    AutomationAction::CreateRecord {
                        entity_type: "rollback_trigger".into(),
                        data: serde_json::json!({
                            "reason": "regression_test_failed",
                            "auto_triggered": true,
                        }),
                    },
                    AutomationAction::SendNotification {
                        target: "on_call".into(),
                        message: "重构回归测试失败，已自动触发回滚流程".into(),
                    },
                ],
            ),
            // ── 行为保真度专属规则 ──
            IndustryAutomationRule::new(
                "refactor_golden_test_fail",
                "黄金测试失败阻断",
                vec![
                    AutomationCondition::FieldBelow {
                        field: "golden_test_pass_rate".into(),
                        threshold: 95.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "refactor_batch".into() },
                ],
                vec![
                    AutomationAction::UpdateStatus { status: "blocked".into() },
                    AutomationAction::SendNotification {
                        target: "tech_lead".into(),
                        message: "黄金测试通过率低于95%，行为等价性验证失败，需人工介入".into(),
                    },
                ],
            ),
            IndustryAutomationRule::new(
                "refactor_tacit_knowledge_violation",
                "隐式知识违反预警",
                vec![
                    AutomationCondition::FieldBelow {
                        field: "tacit_knowledge_retention".into(),
                        threshold: 100.0,
                    },
                    AutomationCondition::EntityTypeIs { entity_type: "refactor_batch".into() },
                ],
                vec![AutomationAction::SendNotification {
                    target: "tech_lead".into(),
                    message: "隐式知识保留率未达100%，可能存在契约违反或行为遗漏".into(),
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
            // ── SDLC 软件开发流程（18 步） ──
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
            // ── 大型项目重构流程（18 步） ──
            WorkflowStep::new("r-asset-scan", "代码资产盘点", "统计文件数/行数/模块结构")
                .with_order(101),
            WorkflowStep::new("r-dep-graph", "依赖关系分析", "构建模块依赖图、识别循环依赖")
                .with_order(102),
            WorkflowStep::new("r-complexity", "复杂度扫描", "圈复杂度/认知复杂度热点识别")
                .with_order(103),
            WorkflowStep::new("r-smell-detect", "坏味道检测", "长方法/上帝类/重复代码检测")
                .with_order(104),
            WorkflowStep::new("r-coupling-analyze", "耦合度分析", "高耦合模块识别与内聚性评估")
                .with_order(105),
            WorkflowStep::new("r-risk-assess", "风险评估", "变更影响范围与回归风险评分")
                .with_order(106),
            WorkflowStep::new("r-strategy", "重构策略制定", "渐进式/大爆炸/绞杀者模式选择")
                .with_order(107),
            WorkflowStep::new("r-batch-plan", "分批计划", "按耦合度排序的模块分批").with_order(108),
            WorkflowStep::new("r-quality-baseline", "质量基线建立", "测试覆盖/性能基准/代码规范")
                .with_order(109),
            WorkflowStep::new("r-rollback", "回滚方案", "分支策略/特性开关/数据回滚")
                .with_order(110),
            WorkflowStep::new("r-pre-review", "预审查", "重构前代码审查").with_order(111),
            WorkflowStep::new("r-execute", "分批执行", "逐模块重构+持续集成").with_order(112),
            WorkflowStep::new("r-regression", "回归验证", "全量回归测试/性能对比").with_order(113),
            WorkflowStep::new("r-integration", "集成验证", "跨模块集成/端到端测试").with_order(114),
            WorkflowStep::new("r-quality-gate", "质量门禁", "代码覆盖/复杂度/性能达标")
                .with_order(115),
            WorkflowStep::new("r-doc-update", "文档更新", "架构文档/API文档/迁移指南")
                .with_order(116),
            WorkflowStep::new("r-handoff", "运维交接", "运行手册/监控告警/回滚预案")
                .with_order(117),
            WorkflowStep::new("r-post-review", "事后复盘", "经验总结/改进建议").with_order(118),
        ]
    }

    fn kpi_definitions(&self) -> Vec<KpiDefinition> {
        vec![
            // ── 通用软件开发 KPI ──
            KpiDefinition::new("sprint_count", "迭代数量", "次", MetricType::Count),
            KpiDefinition::new("code_coverage", "代码覆盖率", "%", MetricType::Percentage),
            KpiDefinition::new("bug_fix_rate", "缺陷修复率", "%", MetricType::Percentage),
            KpiDefinition::new("deploy_frequency", "部署频率", "次/月", MetricType::Ratio),
            // ── 大型项目重构专属 KPI ──
            KpiDefinition::new("refactor_total_lines", "代码总量", "行", MetricType::Count),
            KpiDefinition::new("refactor_total_files", "文件总数", "个", MetricType::Count),
            KpiDefinition::new("refactor_avg_complexity", "平均圈复杂度", "分", MetricType::Ratio),
            KpiDefinition::new(
                "refactor_duplicate_ratio",
                "重复代码率",
                "%",
                MetricType::Percentage,
            ),
            KpiDefinition::new("refactor_module_coupling", "模块耦合度", "分", MetricType::Ratio),
            KpiDefinition::new("refactor_test_coverage", "测试覆盖率", "%", MetricType::Percentage),
            KpiDefinition::new("refactor_batches_completed", "完成批次数", "次", MetricType::Count),
            KpiDefinition::new("refactor_quality_score", "质量评分", "分", MetricType::Ratio),
            // ── 行为保真度专属 KPI ──
            KpiDefinition::new(
                "refactor_golden_test_pass_rate",
                "黄金测试通过率",
                "%",
                MetricType::Percentage,
            ),
            KpiDefinition::new(
                "refactor_tacit_knowledge_retention",
                "隐式知识保留率",
                "%",
                MetricType::Percentage,
            ),
            KpiDefinition::new(
                "refactor_side_effect_equivalence",
                "副作用等价率",
                "%",
                MetricType::Percentage,
            ),
            KpiDefinition::new(
                "refactor_edge_case_retention",
                "边界条件保留率",
                "%",
                MetricType::Percentage,
            ),
        ]
    }

    fn dashboard_cards(&self) -> Vec<DashboardCard> {
        vec![
            // ── 通用软件开发卡片 ──
            DashboardCard::new("sprint_card", "迭代数", "sprint_count", "-- 次"),
            DashboardCard::new("coverage_card", "代码覆盖", "code_coverage", "--%"),
            DashboardCard::new("deploy_card", "部署频率", "deploy_frequency", "-- 次/月"),
            // ── 大型项目重构专属卡片 ──
            DashboardCard::new("code_scale_card", "代码规模", "refactor_total_lines", "-- 行"),
            DashboardCard::new("complexity_card", "复杂度", "refactor_avg_complexity", "-- 分"),
            DashboardCard::new("progress_card", "重构进度", "refactor_batches_completed", "-- 批"),
            DashboardCard::new("quality_card", "质量评分", "refactor_quality_score", "-- 分"),
            DashboardCard::new("coupling_card", "耦合度", "refactor_module_coupling", "-- 分"),
            // ── 行为保真度专属卡片 ──
            DashboardCard::new(
                "fidelity_card",
                "行为保真度",
                "refactor_golden_test_pass_rate",
                "--%",
            ),
        ]
    }
}
