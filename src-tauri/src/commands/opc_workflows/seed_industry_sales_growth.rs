// SPDX-License-Identifier: AGPL-3.0-only

//! 销售增长流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：sales_growth_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "sales_growth_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_sales_growth_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_sales_growth",
            "获客策略",
            "你是获客策略专家。执行「获客策略」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateCustomer"), td("OpcListCustomers")],
            None,
            "step_sales_growth",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_sales_growth",
            "转化优化",
            "你是转化优化专家。执行「转化优化」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcCreateLandingPage"), td("WebSearch")],
            None,
            "step2_sales_growth",
            vec![("input", "step_sales_growth")],
            vec!["step_sales_growth"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-sales_growth-gate",
            "质量门",
            "根据获客策略结果判断：获客策略是否具备落地条件（是→true 转化优化，否→false 获客调整）",
            "step2_sales_growth",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_sales_growth",
            "留存提升",
            "你是留存提升专家。执行「留存提升」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListCustomers"), td("OpcCreatePublishSchedule"), td("OpcListLandingPages")],
            None,
            "step3_sales_growth",
            vec![("input", "step2_sales_growth")],
            vec!["step2_sales_growth"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-sales_growth",
            "获客调整",
            "获客策略不落地，调整策略与渠道组合。输出 JSON：{\"adjustments\":[], \"actionable\":true}",
            vec![],
            None,
            "fix-sales_growth",
            vec![("input", "step2_sales_growth")],
            vec!["step2_sales_growth"],
            250.0,
            720.0,
        ),
        make_merge_node("m-sales_growth", "汇合", 0.0, 900.0),
        make_end(0.0, 1080.0),
    ];

    let edges = vec![
        edge("e-trigger-step_sales_growth", "trigger", "step_sales_growth"),
        edge("e-step_sales_growth-step2_sales_growth", "step_sales_growth", "step2_sales_growth"),
        edge("e-step2_sales_growth-gate", "step2_sales_growth", "c-sales_growth-gate"),
        edge_cond(
            "e-gate-main",
            "c-sales_growth-gate",
            "true",
            "step3_sales_growth",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-sales_growth-gate",
            "false",
            "fix-sales_growth",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_sales_growth", "m-sales_growth"),
        edge("e-fix-merge", "fix-sales_growth", "m-sales_growth"),
        edge("e-m-sales_growth-end", "m-sales_growth", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "销售增长流程".to_string(),
        description: Some("获客策略 → 转化优化 → 留存提升。销售增长全流程。".to_string()),
        icon: "📈".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["opc".to_string(), "industry".to_string(), "sales_growth".to_string()],
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
