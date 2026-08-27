// SPDX-License-Identifier: AGPL-3.0-only

//! 电商运营流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：ecommerce_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "ecommerce_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_ecommerce_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_ecommerce",
            "爆品挖掘",
            "你是爆品挖掘专家。执行「爆品挖掘」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("WebSearch"), td("OpcSearchWiki")],
            None,
            "step_ecommerce",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_ecommerce",
            "竞品监控",
            "你是竞品监控专家。执行「竞品监控」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListCustomers"), td("WebSearch")],
            None,
            "step2_ecommerce",
            vec![("input", "step_ecommerce")],
            vec!["step_ecommerce"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-ecommerce-gate",
            "质量门",
            "根据竞品监控结果判断：是否发现可跟进的高潜力爆品机会（是→true 营销策划，否→false 扩展挖掘）",
            "step2_ecommerce",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_ecommerce",
            "营销策划",
            "你是营销策划专家。执行「营销策划」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateContentAsset"), td("OpcCreateLandingPage")],
            None,
            "step3_ecommerce",
            vec![("input", "step2_ecommerce")],
            vec!["step2_ecommerce"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-ecommerce",
            "扩展挖掘",
            "爆品机会不足，扩展挖掘渠道与候选。输出 JSON：{\"expanded\":[], \"found\":true}",
            vec![],
            None,
            "fix-ecommerce",
            vec![("input", "step2_ecommerce")],
            vec!["step2_ecommerce"],
            250.0,
            720.0,
        ),
        make_merge_node("m-ecommerce", "汇合", 0.0, 900.0),
        make_end(0.0, 1080.0),
    ];

    let edges = vec![
        edge("e-trigger-step_ecommerce", "trigger", "step_ecommerce"),
        edge("e-step_ecommerce-step2_ecommerce", "step_ecommerce", "step2_ecommerce"),
        edge("e-step2_ecommerce-gate", "step2_ecommerce", "c-ecommerce-gate"),
        edge_cond(
            "e-gate-main",
            "c-ecommerce-gate",
            "true",
            "step3_ecommerce",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-ecommerce-gate",
            "false",
            "fix-ecommerce",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_ecommerce", "m-ecommerce"),
        edge("e-fix-merge", "fix-ecommerce", "m-ecommerce"),
        edge("e-m-ecommerce-end", "m-ecommerce", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "电商运营流程".to_string(),
        description: Some("爆品挖掘 → 竞品监控 → 营销策划。电商运营全流程。".to_string()),
        icon: "🛒".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["opc".to_string(), "industry".to_string(), "ecommerce".to_string()],
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
