// SPDX-License-Identifier: AGPL-3.0-only

//! 空间计算（spatial）领域工作流种子化 — 2 个工作流
//!
//! 生成的工作流：
//! - wf-spatial-ar: AR应用设计
//! - wf-spatial-scene: 空间场景

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化空间计算领域的全部工作流
pub(crate) async fn seed_domain_spatial_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: AR应用设计
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spatial-ar",
            "AR应用设计",
            "增强现实应用概念和交互设计",
            "🥽",
            vec!["opc".to_string(), "spatial".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-ar-concept",
                    "概念设计",
                    "设计AR应用核心交互模式",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-ar-concept_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-ar-ux",
                    "空间UI设计",
                    "设计3D空间用户界面和手势",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-ar-ux_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-ar-prototype",
                    "原型验证",
                    "搭建AR原型验证可行性",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-ar-prototype_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-ar-concept", "trigger", "a-ar-concept"),
                edge("e-a-ar-concept-a-ar-ux", "a-ar-concept", "a-ar-ux"),
                edge("e-a-ar-ux-a-ar-prototype", "a-ar-ux", "a-ar-prototype"),
                edge("e-a-ar-prototype-end", "a-ar-prototype", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 空间场景
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-spatial-scene",
            "空间场景",
            "构建沉浸式3D空间场景",
            "🏠",
            vec!["opc".to_string(), "spatial".to_string()],
            "opc-cto-cto-ai-engineer",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-scene-layout",
                    "场景规划",
                    "规划空间布局和交互区域",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-scene-layout_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-scene-build",
                    "场景构建",
                    "构建3D场景和光照",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-scene-build_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-scene-optimize",
                    "优化",
                    "优化性能和用户体验",
                    vec![],
                    Some("opc-cto-cto-ai-engineer"),
                    "a-scene-optimize_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-scene-layout", "trigger", "a-scene-layout"),
                edge("e-a-scene-layout-a-scene-build", "a-scene-layout", "a-scene-build"),
                edge("e-a-scene-build-a-scene-optimize", "a-scene-build", "a-scene-optimize"),
                edge("e-a-scene-optimize-end", "a-scene-optimize", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
