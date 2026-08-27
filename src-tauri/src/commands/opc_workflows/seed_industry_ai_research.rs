// SPDX-License-Identifier: AGPL-3.0-only

//! AI 研究流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：ai_research_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "ai_research_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_ai_research_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_ai_research",
            "需求分析",
            "你是需求分析专家。执行「需求分析」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListProjects")],
            Some("opc-ai_researcher-ai-research-director"),
            "step_ai_research",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_ai_research",
            "文献调研",
            "你是文献调研专家。执行「文献调研」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcSearchWiki"), td("WebSearch")],
            Some("opc-ai_researcher-ai-literature-analyst"),
            "step2_ai_research",
            vec![("input", "step_ai_research")],
            vec!["step_ai_research"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-ai_research-gate",
            "质量门",
            "根据文献调研结果判断：调研资料是否充分支撑研究（是→true 模型评测，否→false 补充调研）",
            "step2_ai_research",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_ai_research",
            "模型评测",
            "你是模型评测专家。执行「模型评测」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("FileRead"), td("Bash")],
            Some("opc-ai_researcher-ai-benchmark-analyst"),
            "step3_ai_research",
            vec![("input", "step2_ai_research")],
            vec!["step2_ai_research"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-ai_research",
            "补充调研",
            "调研资料不足，补充文献与数据源。输出 JSON：{\"added\":[], \"sufficient\":true}",
            vec![],
            None,
            "fix-ai_research",
            vec![("input", "step2_ai_research")],
            vec!["step2_ai_research"],
            250.0,
            720.0,
        ),
        make_merge_node("m-ai_research", "汇合", 0.0, 900.0),
        make_agent_node_full(
            "step4_ai_research",
            "报告输出",
            "你是报告输出专家。执行「报告输出」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("FileWrite")],
            None,
            "step4_ai_research",
            vec![("input", "step3_ai_research")],
            vec!["step3_ai_research"],
            0.0,
            1080.0,
        ),
        make_end(0.0, 1260.0),
    ];

    let edges = vec![
        edge("e-trigger-step_ai_research", "trigger", "step_ai_research"),
        edge("e-step_ai_research-step2_ai_research", "step_ai_research", "step2_ai_research"),
        edge("e-step2_ai_research-gate", "step2_ai_research", "c-ai_research-gate"),
        edge_cond(
            "e-gate-main",
            "c-ai_research-gate",
            "true",
            "step3_ai_research",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-ai_research-gate",
            "false",
            "fix-ai_research",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_ai_research", "m-ai_research"),
        edge("e-fix-merge", "fix-ai_research", "m-ai_research"),
        edge("e-m-ai_research-step4_ai_research", "m-ai_research", "step4_ai_research"),
        edge("e-step4_ai_research-end", "step4_ai_research", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "AI 研究流程".to_string(),
        description: Some(
            "需求分析 → 文献调研 → 模型评测 → 报告输出。AI 研究完整流程。".to_string(),
        ),
        icon: "🔬".to_string(),
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
