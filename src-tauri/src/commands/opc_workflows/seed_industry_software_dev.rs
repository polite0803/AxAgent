// SPDX-License-Identifier: AGPL-3.0-only

//! 软件开发行业工作流模板种子化（代码驱动，3步流程）。
//!
//! 流程：手动启动 → 需求分析 → 技术选型 → 性能优化 → 完成

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "software_dev_harness_workflow";
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

/// 种子化软件开发行业工作流模板。
pub async fn seed_industry_software_dev_workflow_template(
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
        // 2. 需求分析
        make_agent_node(
            "step_software_dev",
            "需求分析",
            "你是一名需求分析专家。请收集并分析项目需求，编写需求规格说明书与用户故事。输出 JSON {requirements, user_stories, acceptance_criteria, project_scope}",
            vec![td("OpcCreateProject"), td("OpcListProjects"), td("FileWrite")],
            "step_software_dev_result",
            250.0,
            150.0,
        ),
        // 3. 技术选型
        make_agent_node(
            "step2_software_dev",
            "技术选型",
            "你是一名技术架构师。请根据需求进行技术选型与架构设计，制定开发计划与里程碑。输出 JSON {tech_stack, architecture_design, development_plan, milestones}",
            vec![td("WebSearch"), td("OpcAddMilestone"), td("OpcListProjects")],
            "step2_software_dev_result",
            250.0,
            350.0,
        ),
        // 4. 性能优化
        make_agent_node(
            "step3_software_dev",
            "性能优化",
            "你是一名性能优化专家。请分析现有系统性能瓶颈，制定优化方案并实施性能改进。输出 JSON {performance_bottlenecks, optimization_plan, benchmark_results, improvement_metrics}",
            vec![td("OpcListProjects"), td("OpcAddMilestone"), td("FileRead")],
            "step3_software_dev_result",
            250.0,
            550.0,
        ),
        // 5. 结束节点
        make_end(250.0, 750.0),
    ];

    let edges = vec![
        edge("e-trigger-step", "trigger", "step_software_dev"),
        edge("e-step-step2", "step_software_dev", "step2_software_dev"),
        edge("e-step2-step3", "step2_software_dev", "step3_software_dev"),
        edge("e-step3-end", "step3_software_dev", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "软件开发流程".to_string(),
        description: Some("需求分析 → 技术选型 → 性能优化。完整的软件开发全流程。".to_string()),
        icon: "⚙️".to_string(),
        tags: vec!["opc".to_string(), "industry".to_string(), "software_dev".to_string()],
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
