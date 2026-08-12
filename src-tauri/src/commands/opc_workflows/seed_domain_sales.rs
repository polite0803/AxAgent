// SPDX-License-Identifier: AGPL-3.0-only

//! 销售与商务（sales）领域工作流种子化 — 5 个工作流
//!
//! 生成的工作流：
//! - wf-sal-account-plan: 客户规划
//! - wf-sal-deal-strategy: 交易策略
//! - wf-sal-outbound: 外呼获客
//! - wf-sal-pipeline-review: 商机复盘
//! - wf-sal-proposal: 方案建议书

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-cmo-cmo-content-strategist";

/// 种子化销售与商务领域的全部工作流
pub(crate) async fn seed_domain_sales_workflows(db: &DatabaseConnection) -> Result<usize, String> {
    let mut seeded = 0usize;

    // wf-sal-account-plan: 客户规划
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sal-account-plan",
            "客户规划",
            "回顾合作历史、满意度、收入 → 制定年度目标、策略、里程碑 → 内部审核计划可行性",
            "🤝",
            vec!["opc".to_string(), "sales".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-account-review",
                    "客户回顾",
                    "回顾合作历史、满意度、收入",
                    vec![],
                    Some(PROFILE),
                    "a-account-review_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-account-plan",
                    "计划制定",
                    "制定年度目标、策略、里程碑",
                    vec![],
                    Some(PROFILE),
                    "a-account-plan_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-account-review-plan",
                    "计划审核",
                    "内部审核计划可行性",
                    vec![],
                    Some(PROFILE),
                    "a-account-review-plan_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-account-review", "trigger", "a-account-review"),
                edge("e-a-account-review-a-account-plan", "a-account-review", "a-account-plan"),
                edge(
                    "e-a-account-plan-a-account-review-plan",
                    "a-account-plan",
                    "a-account-review-plan",
                ),
                edge("e-a-account-review-plan-end", "a-account-review-plan", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-sal-deal-strategy: 交易策略
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sal-deal-strategy",
            "交易策略",
            "分析客户需求、决策链、预算 → 制定赢单策略和行动计划 → 执行策略并跟踪进展",
            "🏆",
            vec!["opc".to_string(), "sales".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-deal-analyze",
                    "需求分析",
                    "分析客户需求、决策链、预算",
                    vec![],
                    Some(PROFILE),
                    "a-deal-analyze_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-deal-strategy",
                    "策略制定",
                    "制定赢单策略和行动计划",
                    vec![],
                    Some(PROFILE),
                    "a-deal-strategy_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-deal-execute",
                    "策略执行",
                    "执行策略并跟踪进展",
                    vec![],
                    Some(PROFILE),
                    "a-deal-execute_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-deal-analyze", "trigger", "a-deal-analyze"),
                edge("e-a-deal-analyze-a-deal-strategy", "a-deal-analyze", "a-deal-strategy"),
                edge("e-a-deal-strategy-a-deal-execute", "a-deal-strategy", "a-deal-execute"),
                edge("e-a-deal-execute-end", "a-deal-execute", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-sal-outbound: 外呼获客
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sal-outbound",
            "外呼获客",
            "定义理想客户画像和名单 → 准备外呼话术和常见问题 → 执行外呼并记录反馈",
            "📞",
            vec!["opc".to_string(), "sales".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-outbound-target",
                    "客户画像",
                    "定义理想客户画像和名单",
                    vec![],
                    Some(PROFILE),
                    "a-outbound-target_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-outbound-script",
                    "话术准备",
                    "准备外呼话术和常见问题",
                    vec![],
                    Some(PROFILE),
                    "a-outbound-script_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-outbound-execute",
                    "外呼执行",
                    "执行外呼并记录反馈",
                    vec![],
                    Some(PROFILE),
                    "a-outbound-execute_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-outbound-target", "trigger", "a-outbound-target"),
                edge(
                    "e-a-outbound-target-a-outbound-script",
                    "a-outbound-target",
                    "a-outbound-script",
                ),
                edge(
                    "e-a-outbound-script-a-outbound-execute",
                    "a-outbound-script",
                    "a-outbound-execute",
                ),
                edge("e-a-outbound-execute-end", "a-outbound-execute", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-sal-pipeline-review: 商机复盘
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sal-pipeline-review",
            "商机复盘",
            "列出所有活跃商机和阶段 → 分析瓶颈、预计收入、风险 → 制定下周跟进计划",
            "📊",
            vec!["opc".to_string(), "sales".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-pipe-list",
                    "商机清单",
                    "列出所有活跃商机和阶段",
                    vec![],
                    Some(PROFILE),
                    "a-pipe-list_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-pipe-analyze",
                    "管道分析",
                    "分析瓶颈、预计收入、风险",
                    vec![],
                    Some(PROFILE),
                    "a-pipe-analyze_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-pipe-plan",
                    "跟进计划",
                    "制定下周跟进计划",
                    vec![],
                    Some(PROFILE),
                    "a-pipe-plan_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-pipe-list", "trigger", "a-pipe-list"),
                edge("e-a-pipe-list-a-pipe-analyze", "a-pipe-list", "a-pipe-analyze"),
                edge("e-a-pipe-analyze-a-pipe-plan", "a-pipe-analyze", "a-pipe-plan"),
                edge("e-a-pipe-plan-end", "a-pipe-plan", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // wf-sal-proposal: 方案建议书
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sal-proposal",
            "方案建议书",
            "确认客户需求和决策标准 → 撰写方案建议书: 方案、价值、报价 → 审查方案质量和竞品定位",
            "📄",
            vec!["opc".to_string(), "sales".to_string()],
            PROFILE,
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-prop-needs",
                    "需求确认",
                    "确认客户需求和决策标准",
                    vec![],
                    Some(PROFILE),
                    "a-prop-needs_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-prop-write",
                    "方案撰写",
                    "撰写方案建议书: 方案、价值、报价",
                    vec![],
                    Some(PROFILE),
                    "a-prop-write_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-prop-review",
                    "方案审查",
                    "审查方案质量和竞品定位",
                    vec![],
                    Some(PROFILE),
                    "a-prop-review_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-prop-needs", "trigger", "a-prop-needs"),
                edge("e-a-prop-needs-a-prop-write", "a-prop-needs", "a-prop-write"),
                edge("e-a-prop-write-a-prop-review", "a-prop-write", "a-prop-review"),
                edge("e-a-prop-review-end", "a-prop-review", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
