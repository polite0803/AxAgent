// SPDX-License-Identifier: AGPL-3.0-only

//! 软件开发流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：software_dev_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "software_dev_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_software_dev_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_software_dev",
            "需求分析",
            "你是需求分析专家。执行「需求分析」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListProjects"), td("OpcCreateProject")],
            None,
            "step_software_dev",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_software_dev",
            "技术选型",
            "你是技术选型专家。执行「技术选型」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListProjects"), td("FileWrite")],
            None,
            "step2_software_dev",
            vec![("input", "step_software_dev")],
            vec!["step_software_dev"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-software_dev-gate",
            "质量门",
            "根据技术选型结果判断：技术选型是否满足需求约束（是→true 性能优化，否→false 选型调整）",
            "step2_software_dev",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_software_dev",
            "性能优化",
            "你是性能优化专家。执行「性能优化」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("WebSearch"), td("OpcAddMilestone")],
            None,
            "step3_software_dev",
            vec![("input", "step2_software_dev")],
            vec!["step2_software_dev"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-software_dev",
            "选型调整",
            "选型不满足约束，调整技术选型。输出 JSON：{\"alternatives\":[], \"satisfied\":true}",
            vec![],
            None,
            "fix-software_dev",
            vec![("input", "step2_software_dev")],
            vec!["step2_software_dev"],
            250.0,
            720.0,
        ),
        make_merge_node("m-software_dev", "汇合", 0.0, 900.0),
        make_end(0.0, 1080.0),
    ];

    let edges = vec![
        edge("e-trigger-step_software_dev", "trigger", "step_software_dev"),
        edge("e-step_software_dev-step2_software_dev", "step_software_dev", "step2_software_dev"),
        edge("e-step2_software_dev-gate", "step2_software_dev", "c-software_dev-gate"),
        edge_cond(
            "e-gate-main",
            "c-software_dev-gate",
            "true",
            "step3_software_dev",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-software_dev-gate",
            "false",
            "fix-software_dev",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_software_dev", "m-software_dev"),
        edge("e-fix-merge", "fix-software_dev", "m-software_dev"),
        edge("e-m-software_dev-end", "m-software_dev", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "软件开发流程".to_string(),
        description: Some("需求分析 → 技术选型 → 性能优化。软件开发全流程。".to_string()),
        icon: "💻".to_string(),
        cluster_id: None,
        route_path: None,
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
