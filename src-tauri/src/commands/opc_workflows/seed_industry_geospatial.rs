// SPDX-License-Identifier: AGPL-3.0-only

//! 地理空间流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：geospatial_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "geospatial_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_geospatial_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_geospatial",
            "空间分析",
            "你是空间分析专家。执行「空间分析」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcSearchWiki"), td("WebSearch")],
            None,
            "step_geospatial",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_geospatial",
            "地图制作",
            "你是地图制作专家。执行「地图制作」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateContentAsset"), td("WebSearch")],
            None,
            "step2_geospatial",
            vec![("input", "step_geospatial")],
            vec!["step_geospatial"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-geospatial-gate",
            "质量门",
            "根据地图制作结果判断：地图数据与制作是否有效可用（是→true GIS应用开发，否→false 数据修正）",
            "step2_geospatial",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_geospatial",
            "GIS应用开发",
            "你是GIS应用开发专家。执行「GIS应用开发」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateProject"), td("FileWrite")],
            None,
            "step3_geospatial",
            vec![("input", "step2_geospatial")],
            vec!["step2_geospatial"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-geospatial",
            "数据修正",
            "地图数据无效，修正数据源与处理流程。输出 JSON：{\"fixes\":[], \"valid\":true}",
            vec![],
            None,
            "fix-geospatial",
            vec![("input", "step2_geospatial")],
            vec!["step2_geospatial"],
            250.0,
            720.0,
        ),
        make_merge_node("m-geospatial", "汇合", 0.0, 900.0),
        make_end(0.0, 1080.0),
    ];

    let edges = vec![
        edge("e-trigger-step_geospatial", "trigger", "step_geospatial"),
        edge("e-step_geospatial-step2_geospatial", "step_geospatial", "step2_geospatial"),
        edge("e-step2_geospatial-gate", "step2_geospatial", "c-geospatial-gate"),
        edge_cond(
            "e-gate-main",
            "c-geospatial-gate",
            "true",
            "step3_geospatial",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-geospatial-gate",
            "false",
            "fix-geospatial",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_geospatial", "m-geospatial"),
        edge("e-fix-merge", "fix-geospatial", "m-geospatial"),
        edge("e-m-geospatial-end", "m-geospatial", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "地理空间流程".to_string(),
        description: Some("空间分析 → 地图制作 → GIS 应用开发。地理信息服务全流程。".to_string()),
        icon: "🗺️".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["opc".to_string(), "industry".to_string(), "geospatial".to_string()],
        version: TEMPLATE_VERSION,
        is_preset: true,
        is_editable: true,
        is_public: false,
        visibility: Visibility::Public,
        trigger_config: Some(TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        }),
        nodes,
        edges,
        input_schema: None,
        output_schema: None,
        variables: vec![],
        error_config: None,
        error_workflow_id: None,
        tool_defs: vec![],
        mission_hash: None,
        created_at: now,
        updated_at: now,
    };

    super::upsert_template(db, template_data).await
}
