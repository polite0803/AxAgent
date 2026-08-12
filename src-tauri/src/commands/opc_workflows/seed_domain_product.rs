// SPDX-License-Identifier: AGPL-3.0-only

//! 产品管理（product）领域工作流种子化 — 3 个工作流
//!
//! 生成的工作流：
//! - wf-prod-launch: 产品发布
//! - wf-prod-roadmap: 产品路线图
//! - wf-prod-spec: 产品规格书

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化产品管理领域的全部工作流
pub(crate) async fn seed_domain_product_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 产品发布
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-prod-launch",
            "产品发布",
            "新产品/功能发布流程",
            "🚀",
            vec!["opc".to_string(), "product".to_string()],
            "opc-cpo-cpo-product-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-launch-plan",
                    "发布计划",
                    "制定发布计划和时间线",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-launch-plan_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-launch-prep",
                    "发布准备",
                    "准备发布说明、营销材料",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-launch-prep_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-launch-exec",
                    "执行发布",
                    "执行发布并监控指标",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-launch-exec_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-launch-plan", "trigger", "a-launch-plan"),
                edge("e-a-launch-plan-a-launch-prep", "a-launch-plan", "a-launch-prep"),
                edge("e-a-launch-prep-a-launch-exec", "a-launch-prep", "a-launch-exec"),
                edge("e-a-launch-exec-end", "a-launch-exec", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 产品路线图
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-prod-roadmap",
            "产品路线图",
            "制定季度产品路线图",
            "🗺️",
            vec!["opc".to_string(), "product".to_string()],
            "opc-cpo-cpo-product-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-road-collect",
                    "需求收集",
                    "收集用户反馈、数据分析、市场趋势",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-road-collect_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-road-prioritize",
                    "优先级排序",
                    "按影响和资源排序功能",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-road-prioritize_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-road-publish",
                    "发布",
                    "输出产品路线图并同步团队",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-road-publish_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-road-collect", "trigger", "a-road-collect"),
                edge("e-a-road-collect-a-road-prioritize", "a-road-collect", "a-road-prioritize"),
                edge("e-a-road-prioritize-a-road-publish", "a-road-prioritize", "a-road-publish"),
                edge("e-a-road-publish-end", "a-road-publish", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 3: 产品规格书
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-prod-spec",
            "产品规格书",
            "编写功能规格和验收标准",
            "📄",
            vec!["opc".to_string(), "product".to_string()],
            "opc-cpo-cpo-product-manager",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-spec-req",
                    "需求分析",
                    "分析用户故事和功能需求",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-spec-req_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-spec-write",
                    "编写",
                    "编写功能规格和验收标准",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-spec-write_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-spec-review",
                    "评审",
                    "与技术团队评审可行性",
                    vec![],
                    Some("opc-cpo-cpo-product-manager"),
                    "a-spec-review_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-spec-req", "trigger", "a-spec-req"),
                edge("e-a-spec-req-a-spec-write", "a-spec-req", "a-spec-write"),
                edge("e-a-spec-write-a-spec-review", "a-spec-write", "a-spec-review"),
                edge("e-a-spec-review-end", "a-spec-review", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
