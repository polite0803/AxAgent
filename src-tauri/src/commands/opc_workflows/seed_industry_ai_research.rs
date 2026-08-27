// SPDX-License-Identifier: AGPL-3.0-only

//! AI 研究与咨询行业工作流模板种子化（代码驱动，4步流程）。
//!
//! 流程：手动启动 → 需求分析 → 文献调研 → 模型评测 → 报告输出 → 完成

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "ai_research_harness_workflow";
const TEMPLATE_VERSION: i32 = 3;

// ── 辅助函数 ──

fn make_agent_node(
    id: &str,
    title: &str,
    prompt: &str,
    tools: Vec<ToolDef>,
    profile_id: Option<&str>,
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
            agent_profile_id: profile_id.map(|s| s.to_string()),
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
        base: super::make_base("trigger", "开始", "手动触发 AI 研究工作流", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: super::make_base("end", "完成", "AI 研究工作流结束", x, y),
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

/// 种子化 AI 研究与咨询行业工作流模板。
pub async fn seed_industry_ai_research_workflow_template(
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
            "step_ai_research",
            "需求分析",
            "你是一名 AI 研究负责人。请将用户的研究需求拆解为明确的范围、方法、交付物与评估标准。输出 JSON {topic, scope, deliverables, success_criteria}",
            vec![td("OpcListProjects"), td("OpcCreateProject"), td("OpcSearchWiki")],
            Some("opc-ai_researcher-ai-research-director"),
            "step_ai_research_result",
            250.0,
            150.0,
        ),
        // 3. 文献调研
        make_agent_node(
            "step2_ai_research",
            "文献调研",
            "你是一名 AI 文献分析师。请基于网络搜索与内部知识库，调研目标方向的最新论文与技术资料，提取关键突破并评估可信度。输出 JSON {key_findings, source_references, confidence}",
            vec![td("WebSearch"), td("FileRead"), td("OpcSearchWiki")],
            Some("opc-ai_researcher-ai-literature-analyst"),
            "step2_ai_research_result",
            250.0,
            350.0,
        ),
        // 4. 模型评测
        make_agent_node(
            "step3_ai_research",
            "模型评测",
            "你是一名 AI 模型评测专家。请对比主流大模型在该场景下的能力边界、性能与成本，给出选型建议。输出 JSON {model_scores, tradeoffs, recommendation}",
            vec![td("Bash"), td("FileRead"), td("FileWrite")],
            Some("opc-ai_researcher-ai-benchmark-analyst"),
            "step3_ai_research_result",
            250.0,
            550.0,
        ),
        // 5. 报告输出
        make_agent_node(
            "step4_ai_research",
            "报告输出",
            "你是一名 AI 报告分析师。请整合前序研究成果，撰写结构化研究报告，输出结论与后续建议。输出 JSON {summary, conclusion, next_steps}",
            vec![td("FileWrite"), td("OpcListKpis"), td("OpcRecordKpi"), td("OpcSendNotification")],
            Some("opc-ai_researcher-ai-report-analyst"),
            "step4_ai_research_result",
            250.0,
            750.0,
        ),
        // 6. 结束节点
        make_end(250.0, 950.0),
    ];

    let edges = vec![
        edge("e-trigger-step_ai_research", "trigger", "step_ai_research"),
        edge("e-step_ai_research-step2_ai_research", "step_ai_research", "step2_ai_research"),
        edge("e-step2_ai_research-step3_ai_research", "step2_ai_research", "step3_ai_research"),
        edge("e-step3_ai_research-step4_ai_research", "step3_ai_research", "step4_ai_research"),
        edge("e-step4_ai_research-end", "step4_ai_research", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "AI 研究与咨询".to_string(),
        description: Some("AI 研究行业工作流：需求分析、文献调研、模型评测、报告输出".to_string()),
        icon: "🤖".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["opc".to_string(), "industry".to_string(), "ai_research".to_string()],
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
