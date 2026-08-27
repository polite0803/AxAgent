// SPDX-License-Identifier: AGPL-3.0-only

//! 付费媒体（paidmedia）领域工作流种子化 — 2 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-pm-campaign: 广告活动管理（规划 → 制作 → 素材合规判定 → 优化/修改分支）
//! - wf-pm-roi:      广告ROI分析（数据采集 → 计算 → ROI达标判定 → 报告/优化建议分支）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{CompareOperator, Condition, EdgeType, LogicalOperator};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-cmo-cmo-content-strategist";
/// paidmedia 领域模板版本（v4 丰富拓扑）
const PM_TEMPLATE_VERSION: i32 = 4;

/// 种子化付费媒体领域的全部工作流
pub(crate) async fn seed_domain_paidmedia_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-pm-campaign: 广告活动管理 ────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-pm-campaign",
            "广告活动管理",
            "广告活动管理：规划投放目标与预算，制作素材，素材不合规自动修改，持续优化投放",
            "📢",
            vec!["opc".to_string(), "paidmedia".to_string()],
            PM_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：广告规划
                make_agent_node(
                    "a-pm-plan",
                    "广告规划",
                    "规划广告活动：目标、受众、渠道、预算分配与排期。\
                     输出 JSON：{\"goal\":\"\", \"audience\":\"\", \"channels\":[{\"name\":\"\", \"budget\":0, \"schedule\":\"\"}], \"total_budget\":0}",
                    vec![td_desc("OpcSearchWiki", "检索渠道投放最佳实践")],
                    Some(PROFILE),
                    "a-pm-plan",
                    0.0,
                    180.0,
                ),
                // Agent：广告制作
                make_agent_node_full(
                    "a-pm-create",
                    "广告制作",
                    "制作广告素材与文案：标题、正文、CTA、落地页，对照平台规范检查合规。\
                     输出 JSON：{\"ads\":[{\"headline\":\"\", \"body\":\"\", \"cta\":\"\", \"creative\":\"\", \"landing\":\"\"}], \"compliant\":true, \"compliance_issues\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-pm-create",
                    vec![("plan", "a-pm-plan")],
                    vec!["a-pm-plan"],
                    0.0,
                    360.0,
                ),
                // 条件：素材合规
                make_condition_node(
                    "c-pm-compliant",
                    "合规判定",
                    vec![Condition {
                        var_path: "a-pm-create.compliant".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 不合规：素材修改
                make_agent_node_full(
                    "a-pm-rewrite",
                    "素材修改",
                    "素材不合规，对照平台广告政策逐项修改。\
                     输出 JSON：{\"fixed_issues\":[{\"issue\":\"\", \"fix\":\"\"}], \"compliant\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-pm-rewrite",
                    vec![("ads", "a-pm-create")],
                    vec!["a-pm-create"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-pm-campaign", "汇合", 0.0, 900.0),
                // Agent：投放优化
                make_agent_node_full(
                    "a-pm-optimize",
                    "投放优化",
                    "制定投放优化策略：出价调整、受众细分、素材轮换、频控规则。\
                     输出 JSON：{\"optimizations\":[{\"area\":\"\", \"action\":\"\", \"expected_effect\":\"\"}], \"monitoring\":{\"kpis\":[], \"check_frequency\":\"\"}}",
                    vec![],
                    Some(PROFILE),
                    "a-pm-optimize",
                    vec![("ads", "a-pm-create"), ("rewrite", "a-pm-rewrite")],
                    vec!["a-pm-create", "a-pm-rewrite"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-plan", "trigger", "a-pm-plan"),
                edge("e-plan-create", "a-pm-plan", "a-pm-create"),
                edge("e-create-compliant", "a-pm-create", "c-pm-compliant"),
                edge_cond(
                    "e-violation-rewrite",
                    "c-pm-compliant",
                    "false",
                    "a-pm-rewrite",
                    EdgeType::ConditionFalse,
                ),
                edge_cond("e-ok-merge", "c-pm-compliant", "true", "m-pm-campaign", EdgeType::ConditionTrue),
                edge("e-rewrite-merge", "a-pm-rewrite", "m-pm-campaign"),
                edge("e-merge-optimize", "m-pm-campaign", "a-pm-optimize"),
                edge("e-optimize-end", "a-pm-optimize", "end"),
            ],
            vec![DomainInputField { key: "campaign_goal", label: "投放目标", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-pm-roi: 广告ROI分析 ──────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-pm-roi",
            "广告ROI分析",
            "广告ROI分析：采集投放数据，计算 ROI 与效率指标，未达标自动生成优化建议",
            "📈",
            vec!["opc".to_string(), "paidmedia".to_string()],
            PM_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // 工具：投放数据采集
                make_tool_node(
                    "t-roi-collect",
                    "投放数据采集",
                    "OpcGetDashboard",
                    vec![("user_input", "trigger")],
                    "t-roi-collect",
                    0.0,
                    180.0,
                ),
                // Agent：ROI 计算
                make_agent_node_full(
                    "a-roi-calc",
                    "计算",
                    "计算广告 ROI 与效率指标：花费、收入、ROAS、CPA、点击率、转化率。\
                     输出 JSON：{\"metrics\":{\"spend\":0, \"revenue\":0, \"roas\":0, \"cpa\":0, \"ctr\":0, \"cvr\":0}, \"target_met\":true}",
                    vec![td_desc("OpcGetDashboard", "查询投放仪表盘")],
                    Some(PROFILE),
                    "a-roi-calc",
                    vec![("dashboard", "t-roi-collect.result")],
                    vec!["t-roi-collect"],
                    0.0,
                    360.0,
                ),
                // 条件：ROI 是否达标
                make_condition_node(
                    "c-roi-target",
                    "达标判定",
                    vec![Condition {
                        var_path: "a-roi-calc.target_met".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 未达标：优化建议
                make_agent_node_full(
                    "a-roi-fix",
                    "优化建议",
                    "ROI 未达标，诊断低效环节并给出优化建议：渠道调整、出价策略、素材优化、受众收窄。\
                     输出 JSON：{\"diagnosis\":\"\", \"recommendations\":[{\"action\":\"\", \"channel\":\"\", \"expected_improvement\":0}], \"priority\":\"\"}",
                    vec![td_desc("OpcSendNotification", "通知投放团队 ROI 预警")],
                    Some(PROFILE),
                    "a-roi-fix",
                    vec![("metrics", "a-roi-calc")],
                    vec!["a-roi-calc"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-roi", "汇合", 0.0, 900.0),
                // Agent：分析报告
                make_agent_node_full(
                    "a-roi-report",
                    "报告",
                    "输出 ROI 分析报告：渠道效率对比、趋势、结论与行动计划。\
                     输出 JSON：{\"channel_rank\":[], \"trend\":\"\", \"conclusion\":\"\", \"action_plan\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-roi-report",
                    vec![("metrics", "a-roi-calc"), ("fix", "a-roi-fix")],
                    vec!["a-roi-calc", "a-roi-fix"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-collect", "trigger", "t-roi-collect"),
                edge("e-collect-calc", "t-roi-collect", "a-roi-calc"),
                edge("e-calc-target", "a-roi-calc", "c-roi-target"),
                edge_cond("e-miss-fix", "c-roi-target", "false", "a-roi-fix", EdgeType::ConditionFalse),
                edge_cond("e-hit-merge", "c-roi-target", "true", "m-roi", EdgeType::ConditionTrue),
                edge("e-fix-merge", "a-roi-fix", "m-roi"),
                edge("e-merge-report", "m-roi", "a-roi-report"),
                edge("e-report-end", "a-roi-report", "end"),
            ],
            vec![DomainInputField { key: "period", label: "分析周期", field_type: "string", required: false }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
