// SPDX-License-Identifier: AGPL-3.0-only

//! 付费媒体（paidmedia）领域工作流种子化 — 2 个工作流
//!
//! 生成的工作流：
//! - wf-pm-campaign: 广告活动管理
//! - wf-pm-roi: 广告ROI分析

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化付费媒体领域的全部工作流
pub(crate) async fn seed_domain_paidmedia_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 广告活动管理
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-pm-campaign",
            "广告活动管理",
            "规划、执行和优化付费广告活动",
            "📺",
            vec!["opc".to_string(), "paidmedia".to_string()],
            "opc-cmo-cmo-content-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-pm-plan",
                    "广告规划",
                    "确定目标受众、预算、渠道",
                    vec![],
                    Some("opc-cmo-cmo-content-strategist"),
                    "a-pm-plan_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-pm-create",
                    "广告制作",
                    "制作广告创意和落地页",
                    vec![],
                    Some("opc-cmo-cmo-content-strategist"),
                    "a-pm-create_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-pm-optimize",
                    "优化",
                    "分析表现数据并优化",
                    vec![],
                    Some("opc-cmo-cmo-content-strategist"),
                    "a-pm-optimize_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-pm-plan", "trigger", "a-pm-plan"),
                edge("e-a-pm-plan-a-pm-create", "a-pm-plan", "a-pm-create"),
                edge("e-a-pm-create-a-pm-optimize", "a-pm-create", "a-pm-optimize"),
                edge("e-a-pm-optimize-end", "a-pm-optimize", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 广告ROI分析
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-pm-roi",
            "广告ROI分析",
            "分析各渠道广告投入产出比",
            "📊",
            vec!["opc".to_string(), "paidmedia".to_string()],
            "opc-cfo-cfo-financial-analyst",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-roi-collect",
                    "数据采集",
                    "采集各渠道成本和收入",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-roi-collect_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-roi-calc",
                    "计算",
                    "计算ROI和客户获取成本",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-roi-calc_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-roi-report",
                    "报告",
                    "输出ROI报告和预算建议",
                    vec![],
                    Some("opc-cfo-cfo-financial-analyst"),
                    "a-roi-report_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-roi-collect", "trigger", "a-roi-collect"),
                edge("e-a-roi-collect-a-roi-calc", "a-roi-collect", "a-roi-calc"),
                edge("e-a-roi-calc-a-roi-report", "a-roi-calc", "a-roi-report"),
                edge("e-a-roi-report-end", "a-roi-report", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
