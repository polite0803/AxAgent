// SPDX-License-Identifier: AGPL-3.0-only

//! 教育培训行业工作流模板种子化（代码驱动，3步流程）。
//!
//! 流程：手动启动 → 课程体系设计 → 学习路径规划 → 内容开发 → 完成

use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "education_harness_workflow";
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
        base: super::make_base("trigger", "开始", "手动触发教育培训工作流", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: super::make_base("end", "完成", "教育培训工作流结束", x, y),
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

/// 种子化教育培训行业工作流模板。
pub async fn seed_industry_education_workflow_template(
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
        // 2. 课程体系设计
        make_agent_node(
            "step_education",
            "课程体系设计",
            "你是一名课程设计专家。请设计完整的课程体系与教学大纲，明确学习目标与评估方式。\n\n请输出 JSON：\n{\n  \"curriculum\": [{\"module\": \"模块名\", \"hours\": \"学时\", \"objectives\": [\"学习目标\"]}],\n  \"prerequisites\": \"前置要求\",\n  \"assessment_methods\": [\"评估方式1\"]\n}",
            vec![td("OpcCreateContentAsset"), td("WebSearch")],
            "step_education_result",
            250.0,
            150.0,
        ),
        // 3. 学习路径规划
        make_agent_node(
            "step2_education",
            "学习路径规划",
            "你是一名教育规划专家。请根据学员背景与目标，规划个性化学习路径与进度安排。\n\n请输出 JSON：\n{\n  \"learning_path\": [{\"phase\": \"阶段\", \"duration\": \"时长\", \"focus\": \"重点\"}],\n  \"milestones\": [{\"name\": \"里程碑\", \"criteria\": \"完成标准\"}],\n  \"recommended_resources\": [\"推荐资源\"]\n}",
            vec![td("OpcCreateLandingPage"), td("FileWrite")],
            "step2_education_result",
            250.0,
            350.0,
        ),
        // 4. 内容开发
        make_agent_node(
            "step3_education",
            "内容开发",
            "你是一名课件开发专家。请根据课程大纲开发教学课件、教材与习题等教学资源。\n\n请输出 JSON：\n{\n  \"courseware\": [{\"module\": \"模块名\", \"content\": \"课件内容概要\"}],\n  \"exercises\": [\"习题列表\"],\n  \"supplementary_materials\": [\"补充材料\"]\n}",
            vec![td("OpcCreateContentAsset"), td("FileWrite")],
            "step3_education_result",
            250.0,
            550.0,
        ),
        // 5. 结束节点
        make_end(250.0, 750.0),
    ];

    let edges = vec![
        edge("e-trigger-step_education", "trigger", "step_education"),
        edge("e-step_education-step2_education", "step_education", "step2_education"),
        edge("e-step2_education-step3_education", "step2_education", "step3_education"),
        edge("e-step3_education-end", "step3_education", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "教育培训".to_string(),
        description: Some("教育培训行业工作流：课程体系设计、学习路径规划、内容开发".to_string()),
        icon: "📚".to_string(),
        tags: vec!["opc".to_string(), "industry".to_string(), "education".to_string()],
        version: TEMPLATE_VERSION,
        is_preset: true,
        is_editable: true,
        is_public: false,
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
