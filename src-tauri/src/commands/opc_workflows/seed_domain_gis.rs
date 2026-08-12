// SPDX-License-Identifier: AGPL-3.0-only

//! 地理信息系统（gis）领域工作流种子化 — 4 个工作流
//!
//! 生成的工作流：
//! - wf-gis-3d-scene: 三维场景
//! - wf-gis-analysis: 空间分析
//! - wf-gis-drone: 无人机测绘
//! - wf-gis-mapping: 地图制作

use super::seed_domain_helpers::*;
use sea_orm::DatabaseConnection;

/// 种子化地理信息系统领域的全部工作流
pub(crate) async fn seed_domain_gis_workflows(db: &DatabaseConnection) -> Result<usize, String> {
    let mut seeded = 0usize;

    // Workflow 1: 三维场景
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-gis-3d-scene",
            "三维场景",
            "构建三维地理场景和可视化",
            "🏔️",
            vec!["opc".to_string(), "gis".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-3d-data",
                    "数据采集",
                    "采集地形、影像和模型数据",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-3d-data_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-3d-scene",
                    "场景搭建",
                    "构建三维场景和光照",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-3d-scene_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-3d-publish",
                    "发布",
                    "发布交互式三维场景",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-3d-publish_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-3d-data", "trigger", "a-3d-data"),
                edge("e-a-3d-data-a-3d-scene", "a-3d-data", "a-3d-scene"),
                edge("e-a-3d-scene-a-3d-publish", "a-3d-scene", "a-3d-publish"),
                edge("e-a-3d-publish-end", "a-3d-publish", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 2: 空间分析
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-gis-analysis",
            "空间分析",
            "地理空间数据分析和可视化",
            "🗺️",
            vec!["opc".to_string(), "gis".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-gis-data",
                    "数据准备",
                    "收集和预处理空间数据",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-gis-data_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-gis-analyze",
                    "分析",
                    "执行空间分析: 缓冲、叠加、网络",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-gis-analyze_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-gis-viz",
                    "可视化",
                    "生成地图和分析报告",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-gis-viz_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-gis-data", "trigger", "a-gis-data"),
                edge("e-a-gis-data-a-gis-analyze", "a-gis-data", "a-gis-analyze"),
                edge("e-a-gis-analyze-a-gis-viz", "a-gis-analyze", "a-gis-viz"),
                edge("e-a-gis-viz-end", "a-gis-viz", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 3: 无人机测绘
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-gis-drone",
            "无人机测绘",
            "无人机航拍数据处理和分析",
            "🛸",
            vec!["opc".to_string(), "gis".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-drone-plan",
                    "飞行规划",
                    "规划飞行路线和采集参数",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-drone-plan_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-drone-process",
                    "数据处理",
                    "处理航拍影像生成正射影像和DSM",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-drone-process_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-drone-analyze",
                    "分析",
                    "从测绘数据提取信息",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-drone-analyze_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-drone-plan", "trigger", "a-drone-plan"),
                edge("e-a-drone-plan-a-drone-process", "a-drone-plan", "a-drone-process"),
                edge("e-a-drone-process-a-drone-analyze", "a-drone-process", "a-drone-analyze"),
                edge("e-a-drone-analyze-end", "a-drone-analyze", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // Workflow 4: 地图制作
    if seed_domain_template(
        db,
        build_domain_template(
            "wf-gis-mapping",
            "地图制作",
            "专业地图制图和符号设计",
            "🗺️",
            vec!["opc".to_string(), "gis".to_string()],
            "opc-ceo-ceo-business-strategist",
            vec![
                make_trigger(250.0, 0.0),
                make_agent_node(
                    "a-map-data",
                    "数据准备",
                    "准备基础地理数据和要素",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-map-data_result",
                    250.0,
                    150.0,
                ),
                make_agent_node(
                    "a-map-design",
                    "地图设计",
                    "设计地图样式、符号和标注",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-map-design_result",
                    250.0,
                    350.0,
                ),
                make_agent_node(
                    "a-map-export",
                    "输出",
                    "导出地图成品",
                    vec![],
                    Some("opc-ceo-ceo-business-strategist"),
                    "a-map-export_result",
                    250.0,
                    550.0,
                ),
                make_end(250.0, 750.0),
            ],
            vec![
                edge("e-trigger-a-map-data", "trigger", "a-map-data"),
                edge("e-a-map-data-a-map-design", "a-map-data", "a-map-design"),
                edge("e-a-map-design-a-map-export", "a-map-design", "a-map-export"),
                edge("e-a-map-export-end", "a-map-export", "end"),
            ],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
