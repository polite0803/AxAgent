// SPDX-License-Identifier: AGPL-3.0-only

//! 空间计算（spatial）领域工作流种子化 — 2 个工作流（v4 丰富拓扑）
//!
//! 生成的工作流：
//! - wf-spatial-ar:    AR 应用（概念 → UX设计 → 可行性分支 → 原型/方案调整）
//! - wf-spatial-scene: 空间场景（布局 → 构建 → 性能分支 → 优化/发布）

use super::seed_domain_helpers::*;
use axagent_harness::workflow_types::{CompareOperator, Condition, EdgeType, LogicalOperator};
use sea_orm::DatabaseConnection;

const PROFILE: &str = "opc-cto-cto-ai-engineer";
/// spatial 领域模板版本（v4 丰富拓扑）
const SPATIAL_TEMPLATE_VERSION: i32 = 4;

/// 种子化空间计算领域的全部工作流
pub(crate) async fn seed_domain_spatial_workflows(
    db: &DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded = 0usize;

    // ── wf-spatial-ar: AR 应用 ──────────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spatial-ar",
            "AR应用",
            "AR应用：概念与交互设计，技术可行性不足自动调整方案，制作可交互原型",
            "🥽",
            vec!["opc".to_string(), "spatial".to_string()],
            SPATIAL_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：概念设计
                make_agent_node(
                    "a-ar-concept",
                    "概念设计",
                    "设计 AR 应用概念：场景、交互范式、技术栈选择、目标体验。\
                     输出 JSON：{\"concept\":\"\", \"interaction_paradigm\":\"\", \"tech_stack\":\"\", \"target_experience\":\"\", \"feasibility\":{\"ok\":true, }",
                    vec![td_desc("OpcSearchWiki", "检索 AR 技术方案")],
                    Some(PROFILE),
                    "a-ar-concept",
                    0.0,
                    180.0,
                ),
                // Agent：UX 设计
                make_agent_node_full(
                    "a-ar-ux",
                    "UX设计",
                    "设计 AR 交互体验：空间布局、手势、视觉反馈、防眩晕设计。\
                     输出 JSON：{\"ux_patterns\":[], \"gestures\":[], \"feedback\":\"\", }",
                    vec![],
                    Some(PROFILE),
                    "a-ar-ux",
                    vec![("concept", "a-ar-concept")],
                    vec!["a-ar-concept"],
                    0.0,
                    360.0,
                ),
                // 条件：技术可行性
                make_condition_node(
                    "c-ar-feasible",
                    "可行性判定",
                    vec![Condition {
                        var_path: "a-ar-concept.feasibility.ok".to_string(),
                        operator: CompareOperator::Eq,
                        value: serde_json::json!(true),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 可行：原型
                make_agent_node_full(
                    "a-ar-prototype",
                    "原型",
                    "制作可交互 AR 原型：核心场景搭建、交互验证、性能基准。\
                     输出 JSON：{\"prototype\":\"\", \"test_results\":{}, \"perf\":0, \"next_steps\":[]}",
                    vec![],
                    Some(PROFILE),
                    "a-ar-prototype",
                    vec![("concept", "a-ar-concept"), ("ux", "a-ar-ux")],
                    vec!["a-ar-concept", "a-ar-ux"],
                    -250.0,
                    720.0,
                ),
                // 不可行：方案调整
                make_agent_node_full(
                    "a-ar-revise",
                    "方案调整",
                    "技术不可行，调整方案：降级交互、更换技术栈、缩小场景范围。\
                     输出 JSON：{\"revised_concept\":\"\", \"tradeoffs\":[], \"feasible\":true}",
                    vec![],
                    Some(PROFILE),
                    "a-ar-revise",
                    vec![("concept", "a-ar-concept")],
                    vec!["a-ar-concept"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-ar", "汇合", 0.0, 900.0),
                // Agent：原型结论
                make_agent_node_full(
                    "a-ar-verdict",
                    "原型结论",
                    "汇总原型或调整结果，输出结论与迭代建议。\
                     输出 JSON：{\"verdict\":\"\", \"summary\":\"\", }",
                    vec![],
                    Some(PROFILE),
                    "a-ar-verdict",
                    vec![("prototype", "a-ar-prototype"), ("revise", "a-ar-revise")],
                    vec!["a-ar-prototype", "a-ar-revise"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-concept", "trigger", "a-ar-concept"),
                edge("e-concept-ux", "a-ar-concept", "a-ar-ux"),
                edge("e-ux-feasible", "a-ar-ux", "c-ar-feasible"),
                edge_cond("e-ok-prototype", "c-ar-feasible", "true", "a-ar-prototype", EdgeType::ConditionTrue),
                edge_cond("e-no-revise", "c-ar-feasible", "false", "a-ar-revise", EdgeType::ConditionFalse),
                edge("e-prototype-merge", "a-ar-prototype", "m-ar"),
                edge("e-revise-merge", "a-ar-revise", "m-ar"),
                edge("e-merge-verdict", "m-ar", "a-ar-verdict"),
                edge("e-verdict-end", "a-ar-verdict", "end"),
            ],
            vec![DomainInputField { key: "ar_scenario", label: "AR场景", field_type: "string", required: true }],
        ),
    )
    .await?
    {
        seeded += 1;
    }

    // ── wf-spatial-scene: 空间场景 ──────────────────────────────────
    if seed_domain_template(
        db,
        build_domain_template_rich(
            "wf-spatial-scene",
            "空间场景",
            "空间场景：规划场景布局，构建空间内容，性能不达标自动优化后发布",
            "🏗️",
            vec!["opc".to_string(), "spatial".to_string()],
            SPATIAL_TEMPLATE_VERSION,
            vec![
                make_trigger(0.0, 0.0),
                // Agent：场景布局
                make_agent_node(
                    "a-scene-layout",
                    "布局规划",
                    "规划空间场景布局：空间分区、物体摆放、动线、光照方案。\
                     输出 JSON：{\"zones\":[], \"objects\":[], }",
                    vec![],
                    Some(PROFILE),
                    "a-scene-layout",
                    0.0,
                    180.0,
                ),
                // Agent：场景构建
                make_agent_node_full(
                    "a-scene-build",
                    "场景构建",
                    "构建空间场景：模型导入、材质、交互逻辑、环境光照，评估性能。\
                     输出 JSON：{\"scene\":\"\", \"interactions\":[], \"fps\":0, }",
                    vec![],
                    Some(PROFILE),
                    "a-scene-build",
                    vec![("layout", "a-scene-layout")],
                    vec!["a-scene-layout"],
                    0.0,
                    360.0,
                ),
                // 条件：性能达标
                make_condition_node(
                    "c-scene-perf",
                    "性能判定",
                    vec![Condition {
                        var_path: "a-scene-build.fps".to_string(),
                        operator: CompareOperator::Gte,
                        value: serde_json::json!(30),
                    }],
                    LogicalOperator::And,
                    0.0,
                    540.0,
                ),
                // 达标：优化发布
                make_agent_node_full(
                    "a-scene-optimize",
                    "优化发布",
                    "场景性能达标，完成发布准备：格式优化、平台打包、质量校验。\
                     输出 JSON：{\"packages\":[], \"quality_report\":\"\", }",
                    vec![td_desc("OpcSearchWiki", "检索空间内容发布规范")],
                    Some(PROFILE),
                    "a-scene-optimize",
                    vec![("build", "a-scene-build")],
                    vec!["a-scene-build"],
                    -250.0,
                    720.0,
                ),
                // 不达标：性能优化
                make_agent_node_full(
                    "a-scene-tune",
                    "性能优化",
                    "性能不达标，优化：减面、LOD、批处理、纹理压缩。\
                     输出 JSON：{\"techniques\":[], \"fps_after\":0, }",
                    vec![],
                    Some(PROFILE),
                    "a-scene-tune",
                    vec![("build", "a-scene-build")],
                    vec!["a-scene-build"],
                    250.0,
                    720.0,
                ),
                make_merge_node("m-scene", "汇合", 0.0, 900.0),
                // Agent：发布
                make_agent_node_full(
                    "a-scene-publish",
                    "发布",
                    "汇总优化结果，发布空间场景并输出验收报告。\
                     输出 JSON：{\"published\":true, \"perf_final\":0, \"report\":\"\"}",
                    vec![],
                    Some(PROFILE),
                    "a-scene-publish",
                    vec![("optimize", "a-scene-optimize"), ("tune", "a-scene-tune")],
                    vec!["a-scene-optimize", "a-scene-tune"],
                    0.0,
                    1080.0,
                ),
                make_end(0.0, 1260.0),
            ],
            vec![
                edge("e-trigger-layout", "trigger", "a-scene-layout"),
                edge("e-layout-build", "a-scene-layout", "a-scene-build"),
                edge("e-build-perf", "a-scene-build", "c-scene-perf"),
                edge_cond(
                    "e-ok-optimize",
                    "c-scene-perf",
                    "true",
                    "a-scene-optimize",
                    EdgeType::ConditionTrue,
                ),
                edge_cond(
                    "e-slow-tune",
                    "c-scene-perf",
                    "false",
                    "a-scene-tune",
                    EdgeType::ConditionFalse,
                ),
                edge("e-optimize-merge", "a-scene-optimize", "m-scene"),
                edge("e-tune-merge", "a-scene-tune", "m-scene"),
                edge("e-merge-publish", "m-scene", "a-scene-publish"),
                edge("e-publish-end", "a-scene-publish", "end"),
            ],
            vec![DomainInputField {
                key: "scene_type",
                label: "场景类型",
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
