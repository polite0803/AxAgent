// SPDX-License-Identifier: AGPL-3.0-only

//! 设计流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：design_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "design_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_design_workflow_template(db: &DatabaseConnection) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_design",
            "产品UI设计",
            "你是产品UI设计专家。执行「产品UI设计」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("WebSearch"), td("FileWrite")],
            None,
            "step_design",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_design",
            "品牌视觉设计",
            "你是品牌视觉设计专家。执行「品牌视觉设计」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateContentAsset"), td("WebSearch")],
            None,
            "step2_design",
            vec![("input", "step_design")],
            vec!["step_design"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-design-gate",
            "质量门",
            "根据品牌视觉设计结果判断：设计风格与品牌规范是否一致（是→true 构建设计系统，否→false 修正规范）",
            "step2_design",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_design",
            "设计系统构建",
            "你是设计系统构建专家。执行「设计系统构建」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateContentAsset"), td("OpcCreateLandingPage")],
            None,
            "step3_design",
            vec![("input", "step2_design")],
            vec!["step2_design"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-design",
            "规范修正",
            "设计不一致，修正视觉规范与品牌对齐。输出 JSON：{\"fixes\":[], \"consistent\":true}",
            vec![],
            None,
            "fix-design",
            vec![("input", "step2_design")],
            vec!["step2_design"],
            250.0,
            720.0,
        ),
        make_merge_node("m-design", "汇合", 0.0, 900.0),
        make_end(0.0, 1080.0),
    ];

    let edges = vec![
        edge("e-trigger-step_design", "trigger", "step_design"),
        edge("e-step_design-step2_design", "step_design", "step2_design"),
        edge("e-step2_design-gate", "step2_design", "c-design-gate"),
        edge_cond("e-gate-main", "c-design-gate", "true", "step3_design", EdgeType::ConditionTrue),
        edge_cond("e-gate-fix", "c-design-gate", "false", "fix-design", EdgeType::ConditionFalse),
        edge("e-main-merge", "step3_design", "m-design"),
        edge("e-fix-merge", "fix-design", "m-design"),
        edge("e-m-design-end", "m-design", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "设计流程".to_string(),
        description: Some(
            "产品 UI 设计 → 品牌视觉设计 → 设计系统构建。设计交付全流程。".to_string(),
        ),
        icon: "🎨".to_string(),
        cluster_id: None,
        route_path: None,
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
