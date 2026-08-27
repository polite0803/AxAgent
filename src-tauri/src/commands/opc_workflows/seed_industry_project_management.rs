// SPDX-License-Identifier: AGPL-3.0-only

//! 项目管理流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：project_management_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "project_management_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_project_management_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_project_management",
            "项目启动",
            "你是项目启动专家。执行「项目启动」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateProject"), td("OpcAddMilestone")],
            None,
            "step_project_management",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_project_management",
            "进度报告",
            "你是进度报告专家。执行「进度报告」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListProjects"), td("OpcAddMilestone")],
            None,
            "step2_project_management",
            vec![("input", "step_project_management")],
            vec!["step_project_management"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-project_management-gate",
            "质量门",
            "根据项目启动结果判断：项目启动准备是否就绪（是→true 进度报告，否→false 启动补全）",
            "step2_project_management",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_project_management",
            "项目收尾",
            "你是项目收尾专家。执行「项目收尾」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListProjects"), td("OpcSendNotification")],
            None,
            "step3_project_management",
            vec![("input", "step2_project_management")],
            vec!["step2_project_management"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-project_management",
            "启动补全",
            "启动准备不足，补齐启动项。输出 JSON：{\"completed\":[], \"ready\":true}",
            vec![],
            None,
            "fix-project_management",
            vec![("input", "step2_project_management")],
            vec!["step2_project_management"],
            250.0,
            720.0,
        ),
        make_merge_node("m-project_management", "汇合", 0.0, 900.0),
        make_approval_node(
            "ap-project_management",
            "人工审批",
            "项目流程结果已生成，请项目经理审批",
            None,
            86400,
            "ap-project_management",
            0.0,
            1080.0,
        ),
        make_end(0.0, 1260.0),
    ];

    let edges = vec![
        edge("e-trigger-step_project_management", "trigger", "step_project_management"),
        edge(
            "e-step_project_management-step2_project_management",
            "step_project_management",
            "step2_project_management",
        ),
        edge(
            "e-step2_project_management-gate",
            "step2_project_management",
            "c-project_management-gate",
        ),
        edge_cond(
            "e-gate-main",
            "c-project_management-gate",
            "true",
            "step3_project_management",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-project_management-gate",
            "false",
            "fix-project_management",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_project_management", "m-project_management"),
        edge("e-fix-merge", "fix-project_management", "m-project_management"),
        edge("e-m-project_management-approval", "m-project_management", "ap-project_management"),
        edge("e-ap-project_management-end", "ap-project_management", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "项目管理流程".to_string(),
        description: Some("项目启动 → 进度报告 → 项目收尾。项目管理全流程。".to_string()),
        icon: "📋".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["opc".to_string(), "industry".to_string(), "project_management".to_string()],
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
