// SPDX-License-Identifier: AGPL-3.0-only

//! 财务与会计（finance）领域工作流种子化 — 3 个工作流
//!
//! 生成的工作流：
//! - wf-fin-budget: 预算编制
//! - wf-fin-cost-analysis: 成本分析
//! - wf-fin-tax: 税务申报

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化财务与会计领域的全部工作流
pub(crate) async fn seed_domain_finance_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 预算编制
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-fin-budget",
            "预算编制",
            "编制年度预算和滚动预测",
            "💰",
            vec!["opc".to_string(), "finance".to_string()],
            "opc-cfo-cfo-financial-analyst",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-budget-review",
                    "回顾",
                    "回顾上期预算执行和差异",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-budget-review_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-budget-plan",
                    "编制",
                    "编制各部门预算方案",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-budget-plan_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-budget-approve",
                    "审批",
                    "审批预算并确定最终版本",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-budget-approve_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-budget-review", "trigger", "a-budget-review"),
                edge("e-a-budget-review-a-budget-plan", "a-budget-review", "a-budget-plan"),
                edge("e-a-budget-plan-a-budget-approve", "a-budget-plan", "a-budget-approve"),
                edge("e-a-budget-approve-end", "a-budget-approve", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 成本分析
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-fin-cost-analysis",
            "成本分析",
            "全面分析运营成本和优化空间",
            "📉",
            vec!["opc".to_string(), "finance".to_string()],
            "opc-cfo-cfo-financial-analyst",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-cost-collect",
                    "采集",
                    "采集各类成本数据",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-cost-collect_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-cost-analyze",
                    "分析",
                    "按类别、项目、客户分析成本",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-cost-analyze_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-cost-optimize",
                    "优化",
                    "制定降本方案并评估影响",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-cost-optimize_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-cost-collect", "trigger", "a-cost-collect"),
                edge("e-a-cost-collect-a-cost-analyze", "a-cost-collect", "a-cost-analyze"),
                edge("e-a-cost-analyze-a-cost-optimize", "a-cost-analyze", "a-cost-optimize"),
                edge("e-a-cost-optimize-end", "a-cost-optimize", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 3: 税务申报
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-fin-tax",
            "税务申报",
            "准备和提交税务申报材料",
            "🧾",
            vec!["opc".to_string(), "finance".to_string()],
            "opc-cfo-cfo-financial-analyst",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-tax-collect",
                    "收集",
                    "收集收入、支出、抵扣凭证",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-tax-collect_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-tax-calc",
                    "计算",
                    "计算应纳税额和抵扣项",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-tax-calc_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-tax-submit",
                    "申报",
                    "生成报表并提交申报",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-tax-submit_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-tax-collect", "trigger", "a-tax-collect"),
                edge("e-a-tax-collect-a-tax-calc", "a-tax-collect", "a-tax-calc"),
                edge("e-a-tax-calc-a-tax-submit", "a-tax-calc", "a-tax-submit"),
                edge("e-a-tax-submit-end", "a-tax-submit", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
