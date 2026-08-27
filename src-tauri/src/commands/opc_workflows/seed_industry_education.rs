// SPDX-License-Identifier: AGPL-3.0-only

//! 教育内容流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：education_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "education_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_education_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_education",
            "课程体系设计",
            "你是课程体系设计专家。执行「课程体系设计」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("WebSearch")],
            None,
            "step_education",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_education",
            "学习路径规划",
            "你是学习路径规划专家。执行「学习路径规划」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateContentAsset"), td("WebSearch")],
            None,
            "step2_education",
            vec![("input", "step_education")],
            vec!["step_education"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-education-gate",
            "质量门",
            "根据学习路径规划结果判断：路径是否覆盖完整学习目标（是→true 内容开发，否→false 路径补充）",
            "step2_education",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_education",
            "内容开发",
            "你是内容开发专家。执行「内容开发」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateLandingPage"), td("FileWrite")],
            None,
            "step3_education",
            vec![("input", "step2_education")],
            vec!["step2_education"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-education",
            "路径补充",
            "学习路径不完整，补充缺失模块与衔接。输出 JSON：{\"added\":[], \"complete\":true}",
            vec![],
            None,
            "fix-education",
            vec![("input", "step2_education")],
            vec!["step2_education"],
            250.0,
            720.0,
        ),
        make_merge_node("m-education", "汇合", 0.0, 900.0),
        make_end(0.0, 1080.0),
    ];

    let edges = vec![
        edge("e-trigger-step_education", "trigger", "step_education"),
        edge("e-step_education-step2_education", "step_education", "step2_education"),
        edge("e-step2_education-gate", "step2_education", "c-education-gate"),
        edge_cond(
            "e-gate-main",
            "c-education-gate",
            "true",
            "step3_education",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-education-gate",
            "false",
            "fix-education",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_education", "m-education"),
        edge("e-fix-merge", "fix-education", "m-education"),
        edge("e-m-education-end", "m-education", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "教育内容流程".to_string(),
        description: Some(
            "课程体系设计 → 学习路径规划 → 内容开发。教育内容生产全流程。".to_string(),
        ),
        icon: "🎓".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["opc".to_string(), "industry".to_string(), "education".to_string()],
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
