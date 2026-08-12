// SPDX-License-Identifier: AGPL-3.0-only

//! 战略规划（strategy）领域工作流种子化 — 2 个工作流
//!
//! 生成的工作流：
//! - wf-strat-biz-plan: 商业计划书
//! - wf-strat-market-entry: 市场进入策略

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化战略规划领域的全部工作流
pub(crate) async fn seed_domain_strategy_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 商业计划书
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-strat-biz-plan",
            "商业计划书",
            "编写完整商业计划书",
            "📄",
            vec!["opc".to_string(), "strategy".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-bp-summary",
                    "执行摘要",
                    "撰写执行摘要和公司概述",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-bp-summary_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-bp-market",
                    "市场分析",
                    "市场分析、竞争分析、SWOT",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-bp-market_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-bp-financial",
                    "财务预测",
                    "收入模型、成本、现金流预测",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-bp-financial_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-bp-summary", "trigger", "a-bp-summary"),
                edge("e-a-bp-summary-a-bp-market", "a-bp-summary", "a-bp-market"),
                edge("e-a-bp-market-a-bp-financial", "a-bp-market", "a-bp-financial"),
                edge("e-a-bp-financial-end", "a-bp-financial", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 市场进入策略
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-strat-market-entry",
            "市场进入策略",
            "制定新市场进入策略和计划",
            "🎯",
            vec!["opc".to_string(), "strategy".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-market-size",
                    "市场分析",
                    "分析市场规模、竞争、进入壁垒",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-market-size_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-market-strategy",
                    "策略制定",
                    "制定进入策略: 渠道、定价、定位",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-market-strategy_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-market-plan",
                    "行动计划",
                    "制定执行计划和预算",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-market-plan_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-market-size", "trigger", "a-market-size"),
                edge("e-a-market-size-a-market-strategy", "a-market-size", "a-market-strategy"),
                edge("e-a-market-strategy-a-market-plan", "a-market-strategy", "a-market-plan"),
                edge("e-a-market-plan-end", "a-market-plan", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
