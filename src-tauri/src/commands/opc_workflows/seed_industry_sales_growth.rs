// SPDX-License-Identifier: AGPL-3.0-only

//! 销售增长与营销行业工作流模板种子化（代码驱动，3步流程）。
//!
//! 流程：手动启动 → 获客策略 → 转化优化 → 留存提升 → 完成

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "sales_growth_harness_workflow";
const TEMPLATE_VERSION: i32 = 3;

// ── 辅助函数 ──

fn make_agent_node(
    id: &str,
    title: &str,
    prompt: &str,
    tools: Vec<ToolDef>,
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
            agent_profile_id: None,
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
        base: super::make_base("trigger", "开始", "手动触发", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: super::make_base("end", "完成", "工作流结束", x, y),
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

/// 种子化销售增长与营销行业工作流模板。
pub async fn seed_industry_sales_growth_workflow_template(
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
        // 2. 获客策略
        make_agent_node(
            "step_sales_growth",
            "获客策略",
            "你是一名营销获客专家。请制定多渠道获客策略，创建客户画像与落地页，规划获客渠道组合。输出 JSON {customer_persona, channel_strategy, landing_page_ids, budget_allocation}",
            vec![td("OpcCreateCustomer"), td("OpcCreateLandingPage"), td("WebSearch")],
            "step_sales_growth_result",
            250.0,
            150.0,
        ),
        // 3. 转化优化
        make_agent_node(
            "step2_sales_growth",
            "转化优化",
            "你是一名转化率优化专家。请分析现有客户数据与落地页效果，优化转化漏斗与发布排期。输出 JSON {conversion_funnel, optimization_plan, publish_schedule, ab_test_plan}",
            vec![td("OpcListCustomers"), td("OpcCreatePublishSchedule"), td("OpcListLandingPages")],
            "step2_sales_growth_result",
            250.0,
            350.0,
        ),
        // 4. 留存提升
        make_agent_node(
            "step3_sales_growth",
            "留存提升",
            "你是一名客户留存专家。请设计客户留存策略与忠诚度计划，通过内容营销与精准通知提升复购率。输出 JSON {retention_strategy, loyalty_program, content_plan, notification_plan}",
            vec![td("OpcSendNotification"), td("OpcListCustomers"), td("OpcCreateContentAsset")],
            "step3_sales_growth_result",
            250.0,
            550.0,
        ),
        // 5. 结束节点
        make_end(250.0, 750.0),
    ];

    let edges = vec![
        edge("e-trigger-step", "trigger", "step_sales_growth"),
        edge("e-step-step2", "step_sales_growth", "step2_sales_growth"),
        edge("e-step2-step3", "step2_sales_growth", "step3_sales_growth"),
        edge("e-step3-end", "step3_sales_growth", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "销售增长与营销流程".to_string(),
        description: Some(
            "获客策略 → 转化优化 → 留存提升。完整的销售增长与营销全流程。".to_string(),
        ),
        icon: "⚙️".to_string(),
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
