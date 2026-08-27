// SPDX-License-Identifier: AGPL-3.0-only

//! 地理信息（gis）领域工作流种子化 — 4 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-gis-3d-scene:  三维场景（数据采集 → 场景搭建 → 性能判定 → 发布/优化分支）
//! - wf-gis-analysis:  空间分析（数据准备 → 分析 → 有效性判定 → 可视化/数据修正分支）
//! - wf-gis-drone:     无人机测绘（飞行规划 → 空域合规判定 → 执行/调整分支 → 处理 → 分析）
//! - wf-gis-mapping:   制图（数据准备 → 地图设计 → 逐图层校验循环 → 导出）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{
    CompareOperator, Condition, EdgeType, LogicalOperator, LoopType,
};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-ceo-ceo-business-strategist";
/// gis 领域模板版本（v4 丰富拓扑）
const GIS_TEMPLATE_VERSION: i32 = 4;

/// 种子化地理信息系统领域的全部工作流
pub(crate) async fn seed_domain_gis_workflows(db: &DatabaseConnection) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-gis-3d-scene: 三维场景 ───────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-gis-3d-scene",
            "三维场景",
            "三维场景：采集地形影像模型数据，搭建场景与光照，性能不达标自动优化后发布",
            "🏔️",
            vec!["opc".to_string(), "gis".to_string()],
            GIS_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：数据采集
                make_agent_node(
                    "a-3d-data",
                    "数据采集",
                    "采集地形、影像与模型数据：分辨率、坐标系、精度评估。\
                     输出 JSON：{\"terrain\":{\"source\":\"\", \"resolution\":0, \"crs\":\"\"}, \"imagery\":[], \"models\":[], \"data_quality\":\"good|needs_work\"}",
                    vec![td_desc("OpcSearchWiki", "检索可用的地理数据源")],
                    Some(PROFILE),
                    "a-3d-data",
                    0.0,
                    180.0,
                ),
                // Agent：场景搭建
                make_agent_node_full(
                    "a-3d-scene",
                    "场景搭建",
                    "构建三维场景与光照：地形加载、纹理映射、光源配置、相机控制。\
                     输出 JSON：{\"scene\":\"\", \"lighting\":\"\", \"camera\":\"\", \"fps_estimate\":0, \"perf_ok\":true, \"optimization_hints\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-3d-scene",
                    vec![("data", "a-3d-data")],
                    vec!["a-3d-data"],
                    0.0,
                    360.0,
                ),
                // 条件：性能是否达标
                make_condition_node(
                    "c-3d-perf",
                    "性能判定",
                    vec![Condition {
                        var_path: "a-3d-scene.perf_ok".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 不达标：场景优化
                make_agent_node_full(
                    "a-3d-optimize",
                    "场景优化",
                    "性能不达标，优化：LOD 分级、纹理压缩、遮挡剔除、实例化。\
                     输出 JSON：{\"optimizations\":[{\"technique\":\"\", \"expected_gain\":0}], \"perf_ok_after\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-3d-optimize",
                    vec![("scene", "a-3d-scene")],
                    vec!["a-3d-scene"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-3d", "汇合", 0.0, 900.0),
                // Agent：发布
                make_agent_node_full(
                    "a-3d-publish",
                    "发布",
                    "发布交互式三维场景：导出格式、平台适配、访问控制、性能基准。\
                     输出 JSON：{\"export_format\":\"\", \"platforms\":[], \"access_control\":\"\", \"perf_benchmark\":{}}",
                    vec![],
                    Some(PROFILE),
                    "a-3d-publish",
                    vec![("scene", "a-3d-scene"), ("optimize", "a-3d-optimize")],
                    vec!["a-3d-scene", "a-3d-optimize"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-data", "trigger", "a-3d-data"),
                edge("e-data-scene", "a-3d-data", "a-3d-scene"),
                edge("e-scene-perf", "a-3d-scene", "c-3d-perf"),
                edge_cond("e-slow-optimize", "c-3d-perf", "false", "a-3d-optimize", EdgeType::ConditionFalse),
                edge_cond("e-ok-merge", "c-3d-perf", "true", "m-3d", EdgeType::ConditionTrue),
                edge("e-optimize-merge", "a-3d-optimize", "m-3d"),
                edge("e-merge-publish", "m-3d", "a-3d-publish"),
                edge("e-publish-end", "a-3d-publish", "end"),
            ],
            vec![DomainInputField { key: "region", label: "区域", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-gis-analysis: 空间分析 ───────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-gis-analysis",
            "空间分析",
            "空间分析：准备地理数据，执行空间分析，结果无效自动修正数据后可视化",
            "🗺️",
            vec!["opc".to_string(), "gis".to_string()],
            GIS_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：数据准备
                make_agent_node(
                    "a-gis-data",
                    "数据准备",
                    "准备分析数据：矢量/栅格数据、坐标系统一、数据清洗与切片。\
                     输出 JSON：{\"layers\":[{\"name\":\"\", \"type\":\"vector|raster\", \"crs\":\"\", \"quality\":\"\"}], \"crs_unified\":true}",
                    vec![td_desc("OpcSearchWiki", "检索数据源")],
                    Some(PROFILE),
                    "a-gis-data",
                    0.0,
                    180.0,
                ),
                // Agent：空间分析
                make_agent_node_full(
                    "a-gis-analyze",
                    "空间分析",
                    "执行空间分析：缓冲区、叠加、邻近、密度、插值等，评估结果有效性。\
                     输出 JSON：{\"analysis\":\"\", \"results\":{}, \"valid\":true, \"issues\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-gis-analyze",
                    vec![("data", "a-gis-data")],
                    vec!["a-gis-data"],
                    0.0,
                    360.0,
                ),
                // 条件：结果有效性
                make_condition_node(
                    "c-gis-valid",
                    "有效性判定",
                    vec![Condition {
                        var_path: "a-gis-analyze.valid".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 无效分支：数据修正
                make_agent_node_full(
                    "a-gis-fix",
                    "数据修正",
                    "分析结果无效，诊断数据问题并修正：补数据、改坐标系、调整参数。\
                     输出 JSON：{\"root_cause\":\"\", \"fixes\":[], \"revalidated\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-gis-fix",
                    vec![("analyze", "a-gis-analyze"), ("data", "a-gis-data")],
                    vec!["a-gis-analyze", "a-gis-data"],
                    -250.0,
                    720.0,
                ),
                make_merge_node("m-gis", "汇合", 0.0, 900.0),
                // Agent：可视化
                make_agent_node_full(
                    "a-gis-viz",
                    "可视化",
                    "将分析结果可视化：专题图、分级设色、标注与图例。\
                     输出 JSON：{\"map_type\":\"\", \"styles\":[], \"legend\":\"\", \"annotations\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-gis-viz",
                    vec![("analyze", "a-gis-analyze"), ("fix", "a-gis-fix")],
                    vec!["a-gis-analyze", "a-gis-fix"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-data", "trigger", "a-gis-data"),
                edge("e-data-analyze", "a-gis-data", "a-gis-analyze"),
                edge("e-analyze-valid", "a-gis-analyze", "c-gis-valid"),
                edge_cond("e-invalid-fix", "c-gis-valid", "false", "a-gis-fix", EdgeType::ConditionFalse),
                edge_cond("e-valid-merge", "c-gis-valid", "true", "m-gis", EdgeType::ConditionTrue),
                edge("e-fix-merge", "a-gis-fix", "m-gis"),
                edge("e-merge-viz", "m-gis", "a-gis-viz"),
                edge("e-viz-end", "a-gis-viz", "end"),
            ],
            vec![DomainInputField { key: "analysis_goal", label: "分析目标", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-gis-drone: 无人机测绘 ────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-gis-drone",
            "无人机测绘",
            "无人机测绘：规划飞行任务，空域不合规自动调整航线，执行后处理与分析",
            "🚁",
            vec!["opc".to_string(), "gis".to_string()],
            GIS_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：飞行规划
                make_agent_node(
                    "a-drone-plan",
                    "飞行规划",
                    "规划无人机飞行任务：航线、高度、重叠率、航拍参数、电池续航。\
                     输出 JSON：{\"waypoints\":[], \"altitude\":0, \"overlap\":0, \"battery_plan\":\"\", \"airspace_compliant\":true}",
                    vec![td_desc("OpcSearchWiki", "检索空域限制信息")],
                    Some(PROFILE),
                    "a-drone-plan",
                    0.0,
                    180.0,
                ),
                // 条件：空域合规
                make_condition_node(
                    "c-drone-airspace",
                    "空域合规判定",
                    vec![Condition {
                        var_path: "a-drone-plan.airspace_compliant".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    360.0,
                ),
                // 合规：任务执行
                make_agent_node_full(
                    "a-drone-fly",
                    "任务执行",
                    "执行飞行任务：起飞、航线飞行、数据采集质量监控、返航。\
                     输出 JSON：{\"mission_status\":\"success|partial\", \"images_captured\":0, \"quality\":\"good|poor\"}",
                    vec![],
                    Some(PROFILE),
                    "a-drone-fly",
                    vec![("plan", "a-drone-plan")],
                    vec!["a-drone-plan"],
                    -250.0,
                    540.0,
                ),
                // 不合规：航线调整
                make_agent_node_full(
                    "a-drone-replan",
                    "航线调整",
                    "空域不合规，调整航线避开限制区，重新规划安全路径。\
                     输出 JSON：{\"adjusted_waypoints\":[], \"compliance_note\":\"\", \"replanned\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-drone-replan",
                    vec![("plan", "a-drone-plan")],
                    vec!["a-drone-plan"],
                    250.0,
                    540.0,
                ),
                make_merge_node("m-drone", "汇合", 0.0, 720.0),
                // Agent：数据处理
                make_agent_node_full(
                    "a-drone-process",
                    "数据处理",
                    "处理航拍数据：影像拼接、空三解算、生成正射影像与点云。\
                     输出 JSON：{\"orthophoto\":\"\", \"point_cloud\":\"\", \"gcp_accuracy\":0}",
                    vec![],
                    Some(PROFILE),
                    "a-drone-process",
                    vec![("fly", "a-drone-fly"), ("replan", "a-drone-replan")],
                    vec!["a-drone-fly", "a-drone-replan"],
                    0.0,
                    900.0,
                ),
                // Agent：成果分析
                make_agent_node_full(
                    "a-drone-analyze",
                    "成果分析",
                    "基于处理成果做测量与分析：面积/体积计算、变化检测、成果报告。\
                     输出 JSON：{\"measurements\":{}, \"report\":\"\", \"change_detected\":false}",
                    vec![],
                    Some(PROFILE),
                    "a-drone-analyze",
                    vec![("process", "a-drone-process")],
                    vec!["a-drone-process"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-plan", "trigger", "a-drone-plan"),
                edge("e-plan-airspace", "a-drone-plan", "c-drone-airspace"),
                edge_cond("e-compliant-fly", "c-drone-airspace", "true", "a-drone-fly", EdgeType::ConditionTrue),
                edge_cond(
                    "e-violation-replan",
                    "c-drone-airspace",
                    "false",
                    "a-drone-replan",
                    EdgeType::ConditionFalse,
                ),
                edge("e-fly-merge", "a-drone-fly", "m-drone"),
                edge("e-replan-merge", "a-drone-replan", "m-drone"),
                edge("e-merge-process", "m-drone", "a-drone-process"),
                edge("e-process-analyze", "a-drone-process", "a-drone-analyze"),
                edge("e-analyze-end", "a-drone-analyze", "end"),
            ],
            vec![DomainInputField { key: "survey_area", label: "测绘区域", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-gis-mapping: 制图 ────────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-gis-mapping",
            "制图",
            "制图：准备地图数据，设计地图样式，逐图层校验后导出成品",
            "🗺️",
            vec!["opc".to_string(), "gis".to_string()],
            GIS_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：数据准备
                make_agent_node(
                    "a-map-data",
                    "数据准备",
                    "准备制图数据：图层组织、要素分类、符号化基础数据。\
                     输出 JSON：{\"layers\":[{\"name\":\"\", \"features\":0, \"style_hint\":\"\"}], \"base_map\":\"\"}",
                    vec![td_desc("OpcSearchWiki", "检索制图规范")],
                    Some(PROFILE),
                    "a-map-data",
                    0.0,
                    180.0,
                ),
                // Agent：地图设计
                make_agent_node_full(
                    "a-map-design",
                    "地图设计",
                    "设计地图：比例尺、符号体系、配色、注记与图例布局。\
                     输出 JSON：{\"scale\":\"\", \"symbols\":[], \"palette\":[], \"annotations\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-map-design",
                    vec![("data", "a-map-data")],
                    vec!["a-map-data"],
                    0.0,
                    360.0,
                ),
                // Loop：逐图层校验
                make_loop_node(
                    "l-map-check",
                    "图层校验",
                    LoopType::ForEach,
                    Some("a-map-data"),
                    Some("layer_item"),
                    Some("l-map-check"),
                    Some("l-map-check__partial"),
                    Some(30),
                    vec!["a-map-layer-check".to_string()],
                    0.0,
                    540.0,
                ),
                // Loop body：单图层校验
                make_agent_node_full(
                    "a-map-layer-check",
                    "图层校验",
                    "校验当前图层：符号一致性、注记冲突、要素完整性、比例适配。\
                     输出 JSON：{\"layer\":\"\", \"issues\":[], }",
                    vec![],
                    Some(PROFILE),
                    "a-map-layer-check",
                    vec![("layer", "layer_item")],
                    vec!["a-map-data", "a-map-design"],
                    250.0,
                    540.0,
                ),
                // Agent：导出
                make_agent_node_full(
                    "a-map-export",
                    "导出",
                    "汇总校验结果，导出成品地图：格式、分辨率、发布渠道。\
                     输出 JSON：{\"format\":\"\", \"resolution\":0, \"issues_remaining\":[], \"publish_channel\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-map-export",
                    vec![("design", "a-map-design"), ("checks", "l-map-check.items")],
                    vec!["a-map-design", "l-map-check"],
                    0.0,
                    900.0,
                ),
                make_end(0.0, 1080.0),
            ],
            vec![
                edge("e-trigger-data", "trigger", "a-map-data"),
                edge("e-data-design", "a-map-data", "a-map-design"),
                edge("e-design-loop", "a-map-design", "l-map-check"),
                edge("e-loop-export", "l-map-check", "a-map-export"),
                edge("e-export-end", "a-map-export", "end"),
            ],
            vec![DomainInputField { key: "map_area", label: "制图区域", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    Ok(seeded)
}
