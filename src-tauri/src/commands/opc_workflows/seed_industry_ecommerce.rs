// SPDX-License-Identifier: AGPL-3.0-only

//! 电子商务行业工作流模板种子化（代码驱动，3步流程）。
//!
//! 流程：手动启动 → 爆品挖掘 → 竞品监控 → 营销策划 → 完成

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "ecommerce_harness_workflow";
const TEMPLATE_VERSION: i32 = 2;

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
        base: super::make_base("trigger", "开始", "手动触发电子商务工作流", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: super::make_base("end", "完成", "电子商务工作流结束", x, y),
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

/// 种子化电子商务行业工作流模板。
pub async fn seed_industry_ecommerce_workflow_template(
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
        // 2. 爆品挖掘
        make_agent_node(
            "step_ecommerce",
            "爆品挖掘",
            "你是一名电商选品专家。请分析市场趋势与用户需求，挖掘具有爆款潜力的商品。\n\n请输出 JSON：\n{\n  \"product_ideas\": [{\"name\": \"商品名\", \"reason\": \"爆款理由\", \"estimated_demand\": \"高\"}],\n  \"market_trends\": [\"市场趋势1\"],\n  \"target_audience\": \"目标人群\"\n}",
            vec![td("OpcListCustomers"), td("WebSearch")],
            "step_ecommerce_result",
            250.0,
            150.0,
        ),
        // 3. 竞品监控
        make_agent_node(
            "step2_ecommerce",
            "竞品监控",
            "你是一名电商竞品分析师。请分析竞争对手的产品策略、定价与市场表现。\n\n请输出 JSON：\n{\n  \"competitors\": [{\"name\": \"竞品名\", \"strength\": \"优势\", \"weakness\": \"劣势\"}],\n  \"market_positioning\": \"市场定位建议\",\n  \"differentiation\": \"差异化策略\"\n}",
            vec![td("WebSearch"), td("OpcSearchWiki")],
            "step2_ecommerce_result",
            250.0,
            350.0,
        ),
        // 4. 营销策划
        make_agent_node(
            "step3_ecommerce",
            "营销策划",
            "你是一名电商营销专家。请制定电商营销推广方案，包含渠道策略与预算分配。\n\n请输出 JSON：\n{\n  \"marketing_plan\": [{\"channel\": \"渠道\", \"budget\": \"预算\", \"kpi\": \"关键指标\"}],\n  \"campaign_schedule\": \"活动时间表\",\n  \"expected_roi\": \"预期回报率\"\n}",
            vec![
                td("OpcCreateContentAsset"),
                td("OpcCreateLandingPage"),
                td("OpcSendNotification"),
            ],
            "step3_ecommerce_result",
            250.0,
            550.0,
        ),
        // 5. 结束节点
        make_end(250.0, 750.0),
    ];

    let edges = vec![
        edge("e-trigger-step_ecommerce", "trigger", "step_ecommerce"),
        edge("e-step_ecommerce-step2_ecommerce", "step_ecommerce", "step2_ecommerce"),
        edge("e-step2_ecommerce-step3_ecommerce", "step2_ecommerce", "step3_ecommerce"),
        edge("e-step3_ecommerce-end", "step3_ecommerce", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "电子商务".to_string(),
        description: Some("电子商务行业工作流：爆品挖掘、竞品监控、营销策划".to_string()),
        icon: "🛒".to_string(),
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
