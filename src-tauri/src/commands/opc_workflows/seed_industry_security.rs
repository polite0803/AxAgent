// SPDX-License-Identifier: AGPL-3.0-only

//! 安全运营流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：security_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "security_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_security_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_security",
            "安全审计",
            "你是安全审计专家。执行「安全审计」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcSearchWiki"), td("WebSearch")],
            None,
            "step_security",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_security",
            "合规检查",
            "你是合规检查专家。执行「合规检查」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcSearchWiki"), td("FileWrite")],
            None,
            "step2_security",
            vec![("input", "step_security")],
            vec!["step_security"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-security-gate",
            "质量门",
            "根据合规检查结果判断：是否发现需要应急响应的高危问题（是→true 应急响应，否→false 常规整改）",
            "step2_security",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_security",
            "应急响应",
            "你是应急响应专家。执行「应急响应」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcSendNotification"), td("WebSearch")],
            None,
            "step3_security",
            vec![("input", "step2_security")],
            vec!["step2_security"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-security",
            "常规整改",
            "无应急需求，输出常规整改建议。输出 JSON：{\"recommendations\":[], \"priority\":\"\"}",
            vec![],
            None,
            "fix-security",
            vec![("input", "step2_security")],
            vec!["step2_security"],
            250.0,
            720.0,
        ),
        make_merge_node("m-security", "汇合", 0.0, 900.0),
        make_end(0.0, 1080.0),
    ];

    let edges = vec![
        edge("e-trigger-step_security", "trigger", "step_security"),
        edge("e-step_security-step2_security", "step_security", "step2_security"),
        edge("e-step2_security-gate", "step2_security", "c-security-gate"),
        edge_cond(
            "e-gate-main",
            "c-security-gate",
            "true",
            "step3_security",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-security-gate",
            "false",
            "fix-security",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_security", "m-security"),
        edge("e-fix-merge", "fix-security", "m-security"),
        edge("e-m-security-end", "m-security", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "安全运营流程".to_string(),
        description: Some("安全审计 → 合规检查 → 应急响应。安全运营全流程。".to_string()),
        icon: "🛡️".to_string(),
        cluster_id: None,
        route_path: None,
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
