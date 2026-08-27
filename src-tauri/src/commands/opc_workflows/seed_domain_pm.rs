// SPDX-License-Identifier: AGPL-3.0-only

//! 项目管理（pm）领域工作流种子化 — 3 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-pm-risk:   风险管理（识别 → 评估 → 高风险分支 → 应对策略 → 监控计划）
//! - wf-pm-sprint: Sprint规划（待办梳理 → 冲刺规划 → 回顾 → 目标达成判定 → 改进项）
//! - wf-pm-status: 项目状态报告（数据收集 → 状态分析 → 阻塞分支 → 报告）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{CompareOperator, Condition, EdgeType, LogicalOperator};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-coo-coo-operations-manager";
/// pm 领域模板版本（v4 丰富拓扑）
const PM_TEMPLATE_VERSION: i32 = 4;

/// 种子化项目管理领域的全部工作流
pub(crate) async fn seed_domain_pm_workflows(db: &DatabaseConnection) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-pm-risk: 风险管理 ────────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-pm-risk",
            "风险管理",
            "风险管理：识别项目风险并评估影响概率，高风险自动生成应对策略与监控计划",
            "⚠️",
            vec!["opc".to_string(), "pm".to_string()],
            PM_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：风险识别
                make_agent_node(
                    "a-risk-identify",
                    "风险识别",
                    "识别项目风险：范围、进度、成本、质量、依赖、资源风险。\
                     输出 JSON：{\"risks\":[{\"id\":\"\", \"category\":\"\", \"description\":\"\", \"impact\":0, \"probability\":0}]}",
                    vec![td_desc("OpcSearchWiki", "检索同类项目风险库")],
                    Some(PROFILE),
                    "a-risk-identify",
                    0.0,
                    180.0,
                ),
                // Agent：风险评估
                make_agent_node_full(
                    "a-risk-assess",
                    "评估",
                    "评估每个风险的严重程度（影响×概率），排序并标记高风险项。\
                     输出 JSON：{\"assessed\":[{\"id\":\"\", \"severity\":\"high|medium|low\", \"score\":0}], \"high_count\":0}",
                    vec![],
                    Some(PROFILE),
                    "a-risk-assess",
                    vec![("risks", "a-risk-identify")],
                    vec!["a-risk-identify"],
                    0.0,
                    360.0,
                ),
                // 条件：是否存在高风险
                make_condition_node(
                    "c-risk-high",
                    "高风险判定",
                    vec![Condition {
                        var_path: "a-risk-assess.high_count".to_string(),
                        operator: CompareOperator::Gt,
                        value: serde_json::json!(0),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 高风险：应对策略
                make_agent_node_full(
                    "a-risk-respond",
                    "应对",
                    "为高风险项制定应对策略：规避、转移、缓解、接受，含行动项与责任人。\
                     输出 JSON：{\"responses\":[{\"risk\":\"\", \"strategy\":\"\", \"actions\":[], \"owner\":\"\", \"due\":\"\"}]}",
                    vec![td_desc("OpcSendNotification", "通知高风险项相关方")],
                    Some(PROFILE),
                    "a-risk-respond",
                    vec![("assess", "a-risk-assess")],
                    vec!["a-risk-assess"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-risk", "汇合", 0.0, 900.0),
                // Agent：监控计划
                make_agent_node_full(
                    "a-risk-monitor",
                    "监控计划",
                    "汇总风险应对与监控计划：触发器、复查频率、状态跟踪机制。\
                     输出 JSON：{\"triggers\":[], \"review_schedule\":\"\", \"tracking\":\"\", \"owners\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-risk-monitor",
                    vec![("assess", "a-risk-assess"), ("respond", "a-risk-respond")],
                    vec!["a-risk-assess", "a-risk-respond"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-identify", "trigger", "a-risk-identify"),
                edge("e-identify-assess", "a-risk-identify", "a-risk-assess"),
                edge("e-assess-high", "a-risk-assess", "c-risk-high"),
                edge_cond("e-high-respond", "c-risk-high", "true", "a-risk-respond", EdgeType::ConditionTrue),
                edge_cond("e-low-merge", "c-risk-high", "false", "m-risk", EdgeType::ConditionFalse),
                edge("e-respond-merge", "a-risk-respond", "m-risk"),
                edge("e-merge-monitor", "m-risk", "a-risk-monitor"),
                edge("e-monitor-end", "a-risk-monitor", "end"),
            ],
            vec![DomainInputField { key: "project_name", label: "项目名称", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-pm-sprint: Sprint规划 ────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-pm-sprint",
            "Sprint规划",
            "Sprint规划：梳理估算待办项，规划冲刺目标，回顾后未达标自动生成改进项",
            "🏃",
            vec!["opc".to_string(), "pm".to_string()],
            PM_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：待办梳理
                make_agent_node(
                    "a-sprint-backlog",
                    "梳理和估算待办项",
                    "梳理产品待办项：用户故事、任务拆分、复杂度估算、依赖标注。\
                     输出 JSON：{\"backlog\":[{\"id\":\"\", \"title\":\"\", \"estimate\":0, \"priority\":\"\", \"dependencies\":[]}], \"velocity_ref\":0}",
                    vec![],
                    Some(PROFILE),
                    "a-sprint-backlog",
                    0.0,
                    180.0,
                ),
                // Agent：冲刺规划
                make_agent_node_full(
                    "a-sprint-plan",
                    "冲刺规划",
                    "按团队产能选择待办项进入冲刺，设定冲刺目标与验收标准。\
                     输出 JSON：{\"sprint_goal\":\"\", \"selected\":[], \"commitment\":0, \"capacity\":0, \"risks\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-sprint-plan",
                    vec![("backlog", "a-sprint-backlog")],
                    vec!["a-sprint-backlog"],
                    0.0,
                    360.0,
                ),
                // Agent：冲刺回顾
                make_agent_node_full(
                    "a-sprint-review",
                    "冲刺回顾",
                    "回顾冲刺结果：完成情况、目标达成度、过程问题、团队反馈。\
                     输出 JSON：{\"completed\":0, \"goal_met\":true, \"issues\":[], \"feedback\":[], \"improvements\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-sprint-review",
                    vec![("plan", "a-sprint-plan")],
                    vec!["a-sprint-plan"],
                    0.0,
                    540.0,
                ),
                // 条件：目标达成判定
                make_condition_node(
                    "c-sprint-goal",
                    "目标达成判定",
                    vec![Condition {
                        var_path: "a-sprint-review.goal_met".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    720.0,
                ),
                // 未达成：改进项
                make_agent_node_full(
                    "a-sprint-improve",
                    "改进项",
                    "冲刺目标未达成，分析根因并制定可执行的改进项。\
                     输出 JSON：{\"root_causes\":[], \"improvements\":[{\"action\":\"\", \"owner\":\"\", \"next_sprint\":true}]}",
                    vec![],
                    Some(PROFILE),
                    "a-sprint-improve",
                    vec![("review", "a-sprint-review")],
                    vec!["a-sprint-review"],
                    -250.0,
                    900.0,
                ),
                make_merge_node("m-sprint", "汇合", 0.0, 1080.0),
                // Agent：总结
                make_agent_node_full(
                    "a-sprint-summary",
                    "冲刺总结",
                    "输出冲刺总结与下个冲刺建议。\
                     输出 JSON：{\"summary\":\"\", \"next_sprint_suggestions\":[], \"carryover\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-sprint-summary",
                    vec![("review", "a-sprint-review"), ("improve", "a-sprint-improve")],
                    vec!["a-sprint-review", "a-sprint-improve"],
                    0.0,
                    1260.0,
                ),
                make_end(0.0, 1440.0),
            ],
            vec![
                edge("e-trigger-backlog", "trigger", "a-sprint-backlog"),
                edge("e-backlog-plan", "a-sprint-backlog", "a-sprint-plan"),
                edge("e-plan-review", "a-sprint-plan", "a-sprint-review"),
                edge("e-review-goal", "a-sprint-review", "c-sprint-goal"),
                edge_cond("e-miss-improve", "c-sprint-goal", "false", "a-sprint-improve", EdgeType::ConditionFalse),
                edge_cond("e-hit-merge", "c-sprint-goal", "true", "m-sprint", EdgeType::ConditionTrue),
                edge("e-improve-merge", "a-sprint-improve", "m-sprint"),
                edge("e-merge-summary", "m-sprint", "a-sprint-summary"),
                edge("e-summary-end", "a-sprint-summary", "end"),
            ],
            vec![DomainInputField { key: "sprint_number", label: "冲刺编号", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-pm-status: 项目状态报告 ──────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-pm-status",
            "项目状态报告",
            "项目状态报告：收集项目数据，分析状态与偏差，存在阻塞自动生成处理建议",
            "📊",
            vec!["opc".to_string(), "pm".to_string()],
            PM_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // 工具：数据收集
                make_tool_node(
                    "t-status-collect",
                    "项目数据收集",
                    "OpcGetDashboard",
                    vec![("user_input", "trigger")],
                    "t-status-collect",
                    0.0,
                    180.0,
                ),
                // Agent：状态分析
                make_agent_node_full(
                    "a-status-analyze",
                    "状态分析",
                    "分析项目状态：进度偏差、预算消耗、里程碑达成、风险与阻塞项。\
                     输出 JSON：{\"health\":\"green|yellow|red\", \"progress\":0, \"budget_used\":0, \"blockers\":[], \"milestone_status\":{}}",
                    vec![td_desc("OpcGetDashboard", "查询项目仪表盘")],
                    Some(PROFILE),
                    "a-status-analyze",
                    vec![("dashboard", "t-status-collect.result")],
                    vec!["t-status-collect"],
                    0.0,
                    360.0,
                ),
                // 条件：是否存在阻塞
                make_condition_node(
                    "c-status-blocker",
                    "阻塞判定",
                    vec![Condition {
                        var_path: "a-status-analyze.blockers".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 有阻塞：处理建议
                make_agent_node_full(
                    "a-status-resolve",
                    "阻塞处理",
                    "为阻塞项提出处理建议：升级路径、资源调配、范围调整。\
                     输出 JSON：{\"blocker_actions\":[{\"blocker\":\"\", \"action\":\"\", \"escalate\":false, \"owner\":\"\"}]}",
                    vec![td_desc("OpcSendNotification", "通知阻塞项相关方")],
                    Some(PROFILE),
                    "a-status-resolve",
                    vec![("analyze", "a-status-analyze")],
                    vec!["a-status-analyze"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-status", "汇合", 0.0, 900.0),
                // Agent：状态报告
                make_agent_node_full(
                    "a-status-write",
                    "报告",
                    "输出项目状态报告：健康度、进度、里程碑、风险与下一步。\
                     输出 JSON：{\"summary\":\"\", \"health\":\"\", \"milestones\":[], \"risks\":[], \"next_steps\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-status-write",
                    vec![("analyze", "a-status-analyze"), ("resolve", "a-status-resolve")],
                    vec!["a-status-analyze", "a-status-resolve"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-collect", "trigger", "t-status-collect"),
                edge("e-collect-analyze", "t-status-collect", "a-status-analyze"),
                edge("e-analyze-blocker", "a-status-analyze", "c-status-blocker"),
                edge_cond(
                    "e-blocked-resolve",
                    "c-status-blocker",
                    "true",
                    "a-status-resolve",
                    EdgeType::ConditionTrue,
                ),
                edge_cond("e-clean-merge", "c-status-blocker", "false", "m-status", EdgeType::ConditionFalse),
                edge("e-resolve-merge", "a-status-resolve", "m-status"),
                edge("e-merge-write", "m-status", "a-status-write"),
                edge("e-write-end", "a-status-write", "end"),
            ],
            vec![DomainInputField { key: "project_id", label: "项目ID", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
