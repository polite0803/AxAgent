// SPDX-License-Identifier: AGPL-3.0-only

//! 内容媒体流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：content_media_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "content_media_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_content_media_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_content_media",
            "选题策划",
            "你是选题策划专家。执行「选题策划」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListBlogPosts"), td("WebSearch")],
            Some("opc-cmo-cmo-content-strategist"),
            "step_content_media",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_content_media",
            "内容创作",
            "你是内容创作专家。执行「内容创作」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateBlogPost"), td("FileWrite")],
            Some("opc-cmo-cmo-content-creator"),
            "step2_content_media",
            vec![("input", "step_content_media")],
            vec!["step_content_media"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-content_media-gate",
            "质量门",
            "根据内容创作结果判断：内容质量是否达标可进入优化打磨（是→true，否→false 重新创作）",
            "step2_content_media",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_content_media",
            "优化打磨",
            "你是优化打磨专家。执行「优化打磨」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("WebSearch"), td("FileRead")],
            Some("opc-cmo-cmo-seo-expert"),
            "step3_content_media",
            vec![("input", "step2_content_media")],
            vec!["step2_content_media"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-content_media",
            "重新创作",
            "内容质量不达标，重新创作并对照标准自检。输出 JSON：{\"rewritten\":\"\", \"qualified\":true}",
            vec![],
            None,
            "fix-content_media",
            vec![("input", "step2_content_media")],
            vec!["step2_content_media"],
            250.0,
            720.0,
        ),
        make_merge_node("m-content_media", "汇合", 0.0, 900.0),
        make_agent_node_full(
            "step4_content_media",
            "多平台发布",
            "你是多平台发布专家。执行「多平台发布」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreatePublishSchedule"), td("OpcListPublishSchedules")],
            Some("opc-cmo-cmo-social-manager"),
            "step4_content_media",
            vec![("input", "step3_content_media")],
            vec!["step3_content_media"],
            0.0,
            1080.0,
        ),
        make_agent_node_full(
            "step5_content_media",
            "IP打造",
            "你是IP打造专家。执行「IP打造」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateContentAsset")],
            None,
            "step5_content_media",
            vec![("input", "step4_content_media")],
            vec!["step4_content_media"],
            0.0,
            1260.0,
        ),
        make_approval_node(
            "ap-content_media",
            "人工审批",
            "内容媒体流程结果已生成，请内容负责人审批",
            None,
            86400,
            "ap-content_media",
            0.0,
            1440.0,
        ),
        make_end(0.0, 1620.0),
    ];

    let edges = vec![
        edge("e-trigger-step_content_media", "trigger", "step_content_media"),
        edge(
            "e-step_content_media-step2_content_media",
            "step_content_media",
            "step2_content_media",
        ),
        edge("e-step2_content_media-gate", "step2_content_media", "c-content_media-gate"),
        edge_cond(
            "e-gate-main",
            "c-content_media-gate",
            "true",
            "step3_content_media",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-content_media-gate",
            "false",
            "fix-content_media",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_content_media", "m-content_media"),
        edge("e-fix-merge", "fix-content_media", "m-content_media"),
        edge("e-m-content_media-step4_content_media", "m-content_media", "step4_content_media"),
        edge(
            "e-step4_content_media-step5_content_media",
            "step4_content_media",
            "step5_content_media",
        ),
        edge("e-step5_content_media-approval", "step5_content_media", "ap-content_media"),
        edge("e-ap-content_media-end", "ap-content_media", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "内容媒体流程".to_string(),
        description: Some(
            "选题策划 → 内容创作 → 优化打磨 → 多平台发布 → IP 打造。内容生产全流程。".to_string(),
        ),
        icon: "📝".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["opc".to_string(), "industry".to_string(), "content_media".to_string()],
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
