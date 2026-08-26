// SPDX-License-Identifier: AGPL-3.0-only

//! 安全合规行业工作流模板种子化（代码驱动，3步流程）。
//!
//! 流程：手动启动 → 安全审计 → 合规检查 → 应急响应 → 完成

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "security_harness_workflow";
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

/// 种子化安全合规行业工作流模板。
pub async fn seed_industry_security_workflow_template(
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
        // 2. 安全审计
        make_agent_node(
            "step_security",
            "安全审计",
            "你是一名安全审计专家。请执行系统安全审计，识别漏洞与安全风险，生成安全审计报告。输出 JSON {audit_findings, vulnerability_list, risk_scores, remediation_plan}",
            vec![td("OpcSearchWiki"), td("FileWrite")],
            "step_security_result",
            250.0,
            150.0,
        ),
        // 3. 合规检查
        make_agent_node(
            "step2_security",
            "合规检查",
            "你是一名合规专家。请检查系统与流程的合规性，识别合规差距并提出整改建议。输出 JSON {compliance_checklist, gap_analysis, regulatory_requirements, remediation_steps}",
            vec![td("OpcSearchWiki"), td("WebSearch")],
            "step2_security_result",
            250.0,
            350.0,
        ),
        // 4. 应急响应
        make_agent_node(
            "step3_security",
            "应急响应",
            "你是一名安全应急响应专家。请制定安全事件应急响应计划，包括检测、响应、恢复与改进。输出 JSON {incident_response_plan, detection_rules, escalation_procedure, post_mortem_template}",
            vec![td("OpcSendNotification"), td("OpcCreateContentAsset")],
            "step3_security_result",
            250.0,
            550.0,
        ),
        // 5. 结束节点
        make_end(250.0, 750.0),
    ];

    let edges = vec![
        edge("e-trigger-step", "trigger", "step_security"),
        edge("e-step-step2", "step_security", "step2_security"),
        edge("e-step2-step3", "step2_security", "step3_security"),
        edge("e-step3-end", "step3_security", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "安全合规流程".to_string(),
        description: Some("安全审计 → 合规检查 → 应急响应。完整的安全合规管理流程。".to_string()),
        icon: "⚙️".to_string(),
        tags: vec!["opc".to_string(), "industry".to_string(), "security".to_string()],
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
