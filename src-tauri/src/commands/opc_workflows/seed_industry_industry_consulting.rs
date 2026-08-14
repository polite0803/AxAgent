// SPDX-License-Identifier: AGPL-3.0-only

//! 行业咨询工作流模板种子化（代码驱动，3步流程）。
//!
//! 流程：手动启动 → 行业扫描 → 进入评估 → 战略制定 → 完成

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "industry_consulting_harness_workflow";
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
        base: super::make_base("trigger", "开始", "手动触发", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: super::make_base("end", "完成", "工作流结束", x, y),
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

/// 种子化行业咨询工作流模板。
pub async fn seed_industry_industry_consulting_workflow_template(
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
        // 2. 行业扫描
        make_agent_node(
            "step_industry_consulting",
            "行业扫描",
            "你是一名产业咨询顾问。请扫描目标行业的全景，分析市场规模、竞争格局与增长驱动因素。输出 JSON {industry_overview, market_size, competitive_landscape, growth_drivers}",
            vec![td("OpcSearchWiki"), td("WebSearch")],
            "step_industry_consulting_result",
            250.0,
            150.0,
        ),
        // 3. 进入评估
        make_agent_node(
            "step2_industry_consulting",
            "进入评估",
            "你是一名产业进入评估专家。请评估客户进入该行业的可行性与风险，识别关键成功因素与潜在障碍。输出 JSON {feasibility_assessment, risk_analysis, key_success_factors, entry_barriers}",
            vec![td("OpcSearchWiki"), td("OpcGetDashboard")],
            "step2_industry_consulting_result",
            250.0,
            350.0,
        ),
        // 4. 战略制定
        make_agent_node(
            "step3_industry_consulting",
            "战略制定",
            "你是一名企业战略顾问。请根据扫描与评估结果制定进入战略与实施路线图。输出 JSON {strategy_plan, roadmap, resource_requirements, roi_projection}",
            vec![td("OpcCreateContentAsset"), td("FileWrite")],
            "step3_industry_consulting_result",
            250.0,
            550.0,
        ),
        // 5. 结束节点
        make_end(250.0, 750.0),
    ];

    let edges = vec![
        edge("e-trigger-step", "trigger", "step_industry_consulting"),
        edge("e-step-step2", "step_industry_consulting", "step2_industry_consulting"),
        edge("e-step2-step3", "step2_industry_consulting", "step3_industry_consulting"),
        edge("e-step3-end", "step3_industry_consulting", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "行业咨询流程".to_string(),
        description: Some(
            "行业扫描 → 进入评估 → 战略制定。快速完成行业进入分析与战略规划。".to_string(),
        ),
        icon: "⚙️".to_string(),
        tags: vec!["opc".to_string(), "industry".to_string(), "industry_consulting".to_string()],
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
