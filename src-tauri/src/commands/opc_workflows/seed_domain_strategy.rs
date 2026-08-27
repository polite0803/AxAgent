// SPDX-License-Identifier: AGPL-3.0-only

//! 战略规划（strategy）领域工作流种子化 — 2 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-strat-biz-plan:   商业计划（摘要 → 市场分析 → 财务规划 → 可行性分支 → 审批）
//! - wf-strat-market-entry:市场进入（市场规模 → 可行性分支 → 进入策略/备选市场 → 实施计划）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{CompareOperator, Condition, EdgeType, LogicalOperator};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-ceo-ceo-business-strategist";
/// strategy 领域模板版本（v4 丰富拓扑）
const STRAT_TEMPLATE_VERSION: i32 = 4;

/// 种子化战略规划领域的全部工作流
pub(crate) async fn seed_domain_strategy_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-strat-biz-plan: 商业计划 ─────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-strat-biz-plan",
            "商业计划",
            "商业计划：撰写执行摘要与市场分析，财务规划不可行自动调整，经审批定稿",
            "📈",
            vec!["opc".to_string(), "strategy".to_string()],
            STRAT_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：执行摘要
                make_agent_node(
                    "a-bp-summary",
                    "执行摘要",
                    "撰写商业计划执行摘要：业务定位、目标市场、商业模式、增长计划。\
                     输出 JSON：{\"business\":\"\", \"market\":\"\", \"model\":\"\", }",
                    vec![td_desc("OpcSearchWiki", "检索行业报告")],
                    Some(PROFILE),
                    "a-bp-summary",
                    0.0,
                    180.0,
                ),
                // Agent：市场分析
                make_agent_node_full(
                    "a-bp-market",
                    "市场分析",
                    "市场规模与竞争分析：TAM/SAM/SOM、竞争格局、差异化优势。\
                     输出 JSON：{\"tam\":0, \"sam\":0, \"som\":0, \"competitors\":[], }",
                    vec![],
                    Some(PROFILE),
                    "a-bp-market",
                    vec![("summary", "a-bp-summary")],
                    vec!["a-bp-summary"],
                    0.0,
                    360.0,
                ),
                // Agent：财务规划
                make_agent_node_full(
                    "a-bp-financial",
                    "财务规划",
                    "编制财务规划：收入预测、成本结构、现金流、盈亏平衡、融资需求。\
                     输出 JSON：{\"revenue\":[], \"costs\":{}, \"cashflow\":\"\", }",
                    vec![],
                    Some(PROFILE),
                    "a-bp-financial",
                    vec![("market", "a-bp-market")],
                    vec!["a-bp-market"],
                    0.0,
                    540.0,
                ),
                // 条件：财务可行性
                make_condition_node(
                    "c-bp-viable",
                    "可行性判定",
                    vec![Condition {
                        var_path: "a-bp-financial.cashflow".to_string(),
                        operator: CompareOperator::IsNotEmpty,
                        value: serde_json::json!(null),
                    }],
                    LogicalOperator::And,
                    0.0,
                    720.0,
                ),
                // 不可行：财务调整
                make_agent_node_full(
                    "a-bp-adjust",
                    "财务调整",
                    "财务规划不可行，调整：成本压缩、收入结构、融资方案。\
                     输出 JSON：{\"adjustments\":[], \"revised_plan\":\"\", \"viable\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-bp-adjust",
                    vec![("financial", "a-bp-financial")],
                    vec!["a-bp-financial"],
                    -250.0,
                    900.0,
                ),
                make_merge_node("m-bp", "汇合", 0.0, 1080.0),
                // Agent：定稿
                make_agent_node_full(
                    "a-bp-final",
                    "定稿",
                    "整合全部章节，输出完整商业计划书。\
                     输出 JSON：{\"final_plan\":\"\", }",
                    vec![],
                    Some(PROFILE),
                    "a-bp-final",
                    vec![
                        ("summary", "a-bp-summary"),
                        ("market", "a-bp-market"),
                        ("financial", "a-bp-financial"),
                        ("adjust", "a-bp-adjust"),
                    ],
                    vec!["a-bp-summary", "a-bp-market", "a-bp-financial", "a-bp-adjust"],
                    0.0,
                    1260.0,
                ),
                // 人工审批
                make_approval_node(
                    "ap-bp",
                    "计划审批",
                    "商业计划书已完成，请董事会审批",
                    Some("board"),
                    86400,
                    "ap-bp",
                    0.0,
                    1440.0,
                ),
                make_end(0.0, 1620.0),
            ],
            vec![
                edge("e-trigger-summary", "trigger", "a-bp-summary"),
                edge("e-summary-market", "a-bp-summary", "a-bp-market"),
                edge("e-market-financial", "a-bp-market", "a-bp-financial"),
                edge("e-financial-viable", "a-bp-financial", "c-bp-viable"),
                edge_cond(
                    "e-inviable-adjust",
                    "c-bp-viable",
                    "false",
                    "a-bp-adjust",
                    EdgeType::ConditionFalse,
                ),
                edge_cond("e-viable-merge", "c-bp-viable", "true", "m-bp", EdgeType::ConditionTrue),
                edge("e-adjust-merge", "a-bp-adjust", "m-bp"),
                edge("e-merge-final", "m-bp", "a-bp-final"),
                edge("e-final-approval", "a-bp-final", "ap-bp"),
                edge("e-approval-end", "ap-bp", "end"),
            ],
            vec![DomainInputField {
                key: "business_idea",
                label: "业务构想",
                field_type: "string",
                required: true,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-strat-market-entry: 市场进入 ─────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-strat-market-entry",
            "市场进入",
            "市场进入：评估目标市场规模，可行性不足自动切换备选市场，制定实施计划",
            "🌍",
            vec!["opc".to_string(), "strategy".to_string()],
            STRAT_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：市场规模评估
                make_agent_node(
                    "a-market-size",
                    "市场规模",
                    "评估目标市场：规模、增长、竞争强度、进入壁垒、监管环境。\
                     输出 JSON：{\"market_size\":0, }",
                    vec![td_desc("OpcSearchWiki", "检索目标市场报告")],
                    Some(PROFILE),
                    "a-market-size",
                    0.0,
                    180.0,
                ),
                // 条件：进入可行性
                make_condition_node(
                    "c-market-viable",
                    "可行性判定",
                    vec![Condition {
                        var_path: "a-market-size.market_size".to_string(),
                        operator: CompareOperator::Gt,
                        value: serde_json::json!(0),
                    }],
                    LogicalOperator::And,
                    0.0,
                    360.0,
                ),
                // 可行：进入策略
                make_agent_node_full(
                    "a-market-strategy",
                    "进入策略",
                    "制定市场进入策略：进入模式、渠道、定价、本地化、资源计划。\
                     输出 JSON：{\"entry_mode\":\"\", \"channels\":[], \"pricing\":\"\", }",
                    vec![],
                    Some(PROFILE),
                    "a-market-strategy",
                    vec![("size", "a-market-size")],
                    vec!["a-market-size"],
                    -250.0,
                    540.0,
                ),
                // 不可行：备选市场
                make_agent_node_full(
                    "a-market-alt",
                    "备选市场",
                    "目标市场不可行，评估备选市场并给出切换建议。\
                     输出 JSON：{\"alternatives\":[{\"market\":\"\", \"size\":0, \"fit\":\"\", }",
                    vec![td_desc("OpcSearchWiki", "检索备选市场")],
                    Some(PROFILE),
                    "a-market-alt",
                    vec![("size", "a-market-size")],
                    vec!["a-market-size"],
                    250.0,
                    540.0,
                ),
                make_merge_node("m-market", "汇合", 0.0, 720.0),
                // Agent：实施计划
                make_agent_node_full(
                    "a-market-plan",
                    "实施计划",
                    "制定市场进入实施计划：里程碑、团队、预算、风险应对。\
                     输出 JSON：{\"milestones\":[], \"team\":[], \"budget\":0, }",
                    vec![],
                    Some(PROFILE),
                    "a-market-plan",
                    vec![("strategy", "a-market-strategy"), ("alt", "a-market-alt")],
                    vec!["a-market-strategy", "a-market-alt"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-size", "trigger", "a-market-size"),
                edge("e-size-viable", "a-market-size", "c-market-viable"),
                edge_cond(
                    "e-ok-strategy",
                    "c-market-viable",
                    "true",
                    "a-market-strategy",
                    EdgeType::ConditionTrue,
                ),
                edge_cond(
                    "e-no-alt",
                    "c-market-viable",
                    "false",
                    "a-market-alt",
                    EdgeType::ConditionFalse,
                ),
                edge("e-strategy-merge", "a-market-strategy", "m-market"),
                edge("e-alt-merge", "a-market-alt", "m-market"),
                edge("e-merge-plan", "m-market", "a-market-plan"),
                edge("e-plan-end", "a-market-plan", "end"),
            ],
            vec![DomainInputField {
                key: "target_market",
                label: "目标市场",
                field_type: "string",
                required: true,
            }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
