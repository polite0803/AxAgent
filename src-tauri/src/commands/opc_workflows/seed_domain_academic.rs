// SPDX-License-Identifier: AGPL-3.0-only

//! 学术研究（academic）领域工作流种子化 — 2 个工作流
//!
//! 生成的工作流：
//! - wf-acd-literature: 文献综述
//! - wf-acd-research: 研究方案

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化学术研究领域的全部工作流
pub(crate) async fn seed_domain_academic_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 文献综述
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-acd-literature",
            "文献综述",
            "系统性地综述学术文献",
            "📚",
            vec!["opc".to_string(), "academic".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-lit-search",
                    "文献搜索",
                    "搜索目标领域的关键文献",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-lit-search_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-lit-review",
                    "文献阅读",
                    "阅读文献并提取关键信息",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-lit-review_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-lit-synthesize",
                    "综述撰写",
                    "撰写文献综述和发现",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-lit-synthesize_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-lit-search", "trigger", "a-lit-search"),
                edge("e-a-lit-search-a-lit-review", "a-lit-search", "a-lit-review"),
                edge("e-a-lit-review-a-lit-synthesize", "a-lit-review", "a-lit-synthesize"),
                edge("e-a-lit-synthesize-end", "a-lit-synthesize", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 研究方案
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-acd-research",
            "研究方案",
            "设计学术研究方案和方法论",
            "🔬",
            vec!["opc".to_string(), "academic".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-research-question",
                    "研究问题",
                    "定义研究问题和假设",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-research-question_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-research-method",
                    "方法论",
                    "设计研究方法和数据采集方案",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-research-method_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-research-plan",
                    "研究计划",
                    "制定时间表和资源计划",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-research-plan_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-research-question", "trigger", "a-research-question"),
                edge(
                    "e-a-research-question-a-research-method",
                    "a-research-question",
                    "a-research-method",
                ),
                edge("e-a-research-method-a-research-plan", "a-research-method", "a-research-plan"),
                edge("e-a-research-plan-end", "a-research-plan", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
