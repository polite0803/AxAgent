// SPDX-License-Identifier: AGPL-3.0-only

//! 地理信息行业工作流模板种子化（代码驱动，3步流程）。
//!
//! 流程：手动启动 → 空间分析 → 地图制作 → GIS应用开发 → 完成

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "geospatial_harness_workflow";
const TEMPLATE_VERSION: i32 = 3;

// ── 辅助函数 ──

fn make_agent_node(
    id: &str,
    title: &str,
    prompt: &str,
    tools: Vec<ToolDef>,
    output_var: &str,
    x: f64,
    y: f64,
) -> WorkflowNode {
    WorkflowNode::Agent(AgentNode {
        base: super::make_base(id, title, "", x, y),
        config: AgentNodeConfig {
            system_prompt: prompt.to_string(),
            context_sources: vec![],
            input_mapping: HashMap::new(),
            output_var: output_var.to_string(),
            model: None,
            temperature: None,
            max_tokens: None,
            tools,
            exposed_tools: vec![],
            output_mode: OutputMode::Json,
            agent_profile_id: None,
            max_tool_rounds: Some(10),
            execution_mode: None,
            rag_source_ids: vec![],
            model_role: Some("opc-worker".to_string()),
            consistency_check: None,
            hallucination_guard: None,
            fallback_model: None,
            task_scene: None,
            stream_chunk_timeout_secs: None,
        },
    })
}

fn make_trigger(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::Trigger(TriggerNode {
        base: super::make_base("trigger", "开始", "手动触发地理信息工作流", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: super::make_base("end", "完成", "地理信息工作流结束", x, y),
        config: EndNodeConfig { output_var: None },
    })
}

fn edge(id: &str, source: &str, target: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.into(),
        source: source.into(),
        source_handle: None,
        target: target.into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    }
}

fn td(name: &str) -> ToolDef {
    ToolDef { name: name.into(), description: None, parameters: None }
}

/// 种子化地理信息行业工作流模板。
pub async fn seed_industry_geospatial_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    // 检查版本
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        // 1. 触发节点
        make_trigger(250.0, 0.0),
        // 2. 空间分析
        make_agent_node(
            "step_geospatial",
            "空间分析",
            "你是一名 GIS 分析师。请执行地理空间数据分析与建模，提取空间特征与模式。\n\n请输出 JSON：\n{\n  \"analysis_type\": \"分析类型\",\n  \"spatial_features\": [{\"name\": \"特征名\", \"value\": \"特征值\"}],\n  \"patterns\": [\"发现的空间模式\"],\n  \"data_layers\": [\"数据图层列表\"]\n}",
            vec![td("OpcSearchWiki"), td("WebSearch")],
            "step_geospatial_result",
            250.0,
            150.0,
        ),
        // 3. 地图制作
        make_agent_node(
            "step2_geospatial",
            "地图制作",
            "你是一名制图专家。请根据空间分析结果制作专业地图与可视化产品。\n\n请输出 JSON：\n{\n  \"map_type\": \"地图类型\",\n  \"layers\": [{\"name\": \"图层名\", \"style\": \"样式\"}],\n  \"visualization_elements\": [\"标注\", \"图例\"],\n  \"output_format\": \"输出格式\"\n}",
            vec![td("OpcCreateContentAsset"), td("FileWrite")],
            "step2_geospatial_result",
            250.0,
            350.0,
        ),
        // 4. GIS应用开发
        make_agent_node(
            "step3_geospatial",
            "GIS应用开发",
            "你是一名 GIS 应用工程师。请根据需求开发地理信息系统应用，包括数据管理、空间查询与可视化功能。\n\n请输出 JSON：\n{\n  \"app_architecture\": \"应用架构说明\",\n  \"core_features\": [\"核心功能列表\"],\n  \"tech_stack\": {\"frontend\": \"前端技术\", \"backend\": \"后端技术\", \"spatial_db\": \"空间数据库\"},\n  \"deployment_plan\": \"部署方案\"\n}",
            vec![td("OpcCreateProject"), td("OpcCreateContentAsset")],
            "step3_geospatial_result",
            250.0,
            550.0,
        ),
        // 5. 结束节点
        make_end(250.0, 750.0),
    ];

    let edges = vec![
        edge("e-trigger-step_geospatial", "trigger", "step_geospatial"),
        edge("e-step_geospatial-step2_geospatial", "step_geospatial", "step2_geospatial"),
        edge("e-step2_geospatial-step3_geospatial", "step2_geospatial", "step3_geospatial"),
        edge("e-step3_geospatial-end", "step3_geospatial", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "地理信息".to_string(),
        description: Some("地理信息行业工作流：空间分析、地图制作、GIS应用开发".to_string()),
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
