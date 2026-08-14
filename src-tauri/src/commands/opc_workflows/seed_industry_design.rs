// SPDX-License-Identifier: AGPL-3.0-only

//! 设计行业工作流模板种子化（代码驱动，3步流程）。
//!
//! 流程：手动启动 → 产品UI设计 → 品牌视觉设计 → 设计系统构建 → 完成

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "design_harness_workflow";
const TEMPLATE_VERSION: i32 = 2;

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
        base: super::make_base("trigger", "开始", "手动触发设计工作流", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: super::make_base("end", "完成", "设计工作流结束", x, y),
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

/// 种子化设计行业工作流模板。
pub async fn seed_industry_design_workflow_template(db: &DatabaseConnection) -> Result<(), String> {
    // 检查版本
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        // 1. 触发节点
        make_trigger(250.0, 0.0),
        // 2. 产品UI设计
        make_agent_node(
            "step_design",
            "产品UI设计",
            "你是一名 UI 设计师。请根据产品需求进行界面设计，输出设计规范与视觉稿说明。\n\n请输出 JSON：\n{\n  \"design_spec\": \"设计规范说明\",\n  \"color_palette\": [\"主色\", \"辅助色\"],\n  \"typography\": \"字体方案\",\n  \"components\": [\"组件列表\"]\n}",
            vec![td("OpcCreateContentAsset"), td("FileWrite"), td("WebSearch")],
            "step_design_result",
            250.0,
            150.0,
        ),
        // 3. 品牌视觉设计
        make_agent_node(
            "step2_design",
            "品牌视觉设计",
            "你是一名品牌设计师。请进行品牌视觉识别系统设计，包括标志、色彩、应用规范。\n\n请输出 JSON：\n{\n  \"brand_guidelines\": \"品牌指南\",\n  \"logo_concepts\": [\"标志概念说明\"],\n  \"brand_colors\": [\"品牌色板\"],\n  \"application_mockups\": [\"应用场景\"]\n}",
            vec![td("OpcCreateContentAsset"), td("OpcCreateLandingPage")],
            "step2_design_result",
            250.0,
            350.0,
        ),
        // 4. 设计系统构建
        make_agent_node(
            "step3_design",
            "设计系统构建",
            "你是一名设计系统专家。请构建设计系统组件库与规范文档，确保跨产品一致性。\n\n请输出 JSON：\n{\n  \"component_library\": [\"组件列表\"],\n  \"design_tokens\": {\"color\": {}, \"spacing\": {}, \"typography\": {}},\n  \"usage_guidelines\": \"使用指南说明\"\n}",
            vec![td("OpcCreateContentAsset"), td("FileWrite")],
            "step3_design_result",
            250.0,
            550.0,
        ),
        // 5. 结束节点
        make_end(250.0, 750.0),
    ];

    let edges = vec![
        edge("e-trigger-step_design", "trigger", "step_design"),
        edge("e-step_design-step2_design", "step_design", "step2_design"),
        edge("e-step2_design-step3_design", "step2_design", "step3_design"),
        edge("e-step3_design-end", "step3_design", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "设计".to_string(),
        description: Some("设计行业工作流：产品UI设计、品牌视觉设计、设计系统构建".to_string()),
        icon: "🎨".to_string(),
        tags: vec!["opc".to_string(), "industry".to_string(), "design".to_string()],
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
