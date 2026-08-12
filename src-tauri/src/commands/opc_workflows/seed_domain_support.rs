// SPDX-License-Identifier: AGPL-3.0-only

//! 客户支持（support）领域工作流种子化 — 3 个工作流
//!
//! 生成的工作流：
//! - wf-sup-faq: FAQ知识库
//! - wf-sup-satisfaction: 客户满意度调查
//! - wf-sup-ticket: 客户工单处理

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化客户支持领域的全部工作流
pub(crate) async fn seed_domain_support_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: FAQ知识库
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sup-faq",
            "FAQ知识库",
            "从客户问题提取和更新知识库",
            "📚",
            vec!["opc".to_string(), "support".to_string()],
            "opc-coo-coo-operations-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-faq-collect",
                    "收集",
                    "采集高频客户问题和解决方案",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-faq-collect_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-faq-write",
                    "编写",
                    "编写清晰的FAQ文档",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-faq-write_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-faq-publish",
                    "发布",
                    "审核并发布到知识库",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-faq-publish_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-faq-collect", "trigger", "a-faq-collect"),
                edge("e-a-faq-collect-a-faq-write", "a-faq-collect", "a-faq-write"),
                edge("e-a-faq-write-a-faq-publish", "a-faq-write", "a-faq-publish"),
                edge("e-a-faq-publish-end", "a-faq-publish", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 客户满意度调查
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sup-satisfaction",
            "客户满意度调查",
            "设计、发送和分析满意度调查",
            "📊",
            vec!["opc".to_string(), "support".to_string()],
            "opc-coo-coo-operations-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-sat-design",
                    "设计",
                    "设计调查问卷和评分体系",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-sat-design_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-sat-send",
                    "发送",
                    "选择样本并发送调查",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-sat-send_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-sat-analyze",
                    "分析",
                    "分析结果并制定改进计划",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-sat-analyze_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-sat-design", "trigger", "a-sat-design"),
                edge("e-a-sat-design-a-sat-send", "a-sat-design", "a-sat-send"),
                edge("e-a-sat-send-a-sat-analyze", "a-sat-send", "a-sat-analyze"),
                edge("e-a-sat-analyze-end", "a-sat-analyze", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 3: 客户工单处理
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-sup-ticket",
            "客户工单处理",
            "接收、分类、处理和关闭客户工单",
            "🎫",
            vec!["opc".to_string(), "support".to_string()],
            "opc-coo-coo-operations-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-ticket-categorize",
                    "分类",
                    "分类工单类型和紧急程度",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-ticket-categorize_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-ticket-solve",
                    "解决",
                    "排查问题并给出解决方案",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-ticket-solve_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-ticket-follow",
                    "跟进",
                    "确认客户满意并关闭工单",
                    vec![],
                    Some("opc-coo-coo-operations-manager"),
                    "a-ticket-follow_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-ticket-categorize", "trigger", "a-ticket-categorize"),
                edge(
                    "e-a-ticket-categorize-a-ticket-solve",
                    "a-ticket-categorize",
                    "a-ticket-solve",
                ),
                edge("e-a-ticket-solve-a-ticket-follow", "a-ticket-solve", "a-ticket-follow"),
                edge("e-a-ticket-follow-end", "a-ticket-follow", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
