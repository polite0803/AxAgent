// SPDX-License-Identifier: AGPL-3.0-only

//! 项目管理行业工作流模板种子化（代码驱动，3步流程）。
//!
//! 流程：手动启动 → 项目启动 → 进度报告 → 项目收尾 → 完成

use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "project_management_harness_workflow";
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

/// 种子化项目管理行业工作流模板。
pub async fn seed_industry_project_management_workflow_template(
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
        // 2. 项目启动
        make_agent_node(
            "step_project_management",
            "项目启动",
            "你是一名项目经理。请制定项目章程与启动计划，明确项目范围、目标、里程碑与团队角色。输出 JSON {charter, milestones, team_roles, resource_plan}",
            vec![td("OpcCreateProject"), td("OpcAddMilestone")],
            "step_project_management_result",
            250.0,
            150.0,
        ),
        // 3. 进度报告
        make_agent_node(
            "step2_project_management",
            "进度报告",
            "你是一名项目进度管理员。请生成进度报告并识别风险，跟踪里程碑完成情况。输出 JSON {progress, blockers, risk_register, next_actions}",
            vec![td("OpcListProjects"), td("OpcAddMilestone"), td("OpcSendNotification")],
            "step2_project_management_result",
            250.0,
            350.0,
        ),
        // 4. 项目收尾
        make_agent_node(
            "step3_project_management",
            "项目收尾",
            "你是一名项目收尾经理。请完成项目收尾工作，包括验收、总结与绩效评估。输出 JSON {acceptance, lessons_learned, performance_review, kpi_results}",
            vec![td("OpcListProjects"), td("OpcRecordKpi")],
            "step3_project_management_result",
            250.0,
            550.0,
        ),
        // 5. 结束节点
        make_end(250.0, 750.0),
    ];

    let edges = vec![
        edge("e-trigger-step", "trigger", "step_project_management"),
        edge("e-step-step2", "step_project_management", "step2_project_management"),
        edge("e-step2-step3", "step2_project_management", "step3_project_management"),
        edge("e-step3-end", "step3_project_management", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "项目管理流程".to_string(),
        description: Some("项目启动 → 进度报告 → 项目收尾。标准项目管理全流程。".to_string()),
        icon: "⚙️".to_string(),
        tags: vec!["opc".to_string(), "industry".to_string(), "project_management".to_string()],
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
