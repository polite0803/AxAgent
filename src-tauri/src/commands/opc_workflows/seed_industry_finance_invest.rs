// SPDX-License-Identifier: AGPL-3.0-only

//! 金融投资行业工作流模板种子化（代码驱动，5步流程）。
//!
//! 流程：手动启动 → 市场分析 → 行业研究 → 资产配置 → 交易执行 → 回顾复盘 → 完成
//! requires_approval = true

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "finance_invest_harness_workflow";
const TEMPLATE_VERSION: i32 = 3;

// ── 辅助函数 ──

fn make_agent_node(
    id: &str,
    title: &str,
    prompt: &str,
    tools: Vec<ToolDef>,
    profile_id: Option<String>,
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
            agent_profile_id: profile_id,
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

/// 种子化金融投资行业工作流模板。
pub async fn seed_industry_finance_invest_workflow_template(
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
        // 2. 市场分析
        make_agent_node(
            "step_finance_invest",
            "市场分析",
            "你是一名资深投资分析师。请分析宏观经济与市场趋势，识别投资机会。输出 JSON {market_view, key_sectors, risk_factors}",
            vec![td("OpcGetDashboard"), td("OpcListKpis"), td("OpcListCustomers")],
            Some("opc-finance_invest_lead-finance-market-analyst".to_string()),
            "step_finance_invest_result",
            250.0,
            150.0,
        ),
        // 3. 行业研究
        make_agent_node(
            "step2_finance_invest",
            "行业研究",
            "你是一名行业研究专家。请深入研究目标行业与个股。输出 JSON {industry_outlook, stock_analysis, valuation}",
            vec![td("OpcSearchWiki"), td("OpcListProjects")],
            Some("opc-finance_invest_lead-finance-industry-researcher".to_string()),
            "step2_finance_invest_result",
            250.0,
            350.0,
        ),
        // 4. 资产配置
        make_agent_node(
            "step3_finance_invest",
            "资产配置",
            "你是一名资产配置专家。请根据分析结果构建最优投资组合。输出 JSON {allocation, positions, rebalance_plan}",
            vec![td("OpcGetFinancialReport"), td("OpcGetDashboard")],
            Some("opc-finance_invest_lead-finance-asset-allocator".to_string()),
            "step3_finance_invest_result",
            250.0,
            550.0,
        ),
        // 5. 交易执行
        make_agent_node(
            "step4_finance_invest",
            "交易执行",
            "你是一名交易执行专家。请执行交易并实时监控市场。输出 JSON {executed_trades, pnl, alerts}",
            vec![td("OpcSendNotification"), td("OpcGetDashboard")],
            Some("opc-finance_invest_lead-finance-trade-executor".to_string()),
            "step4_finance_invest_result",
            250.0,
            750.0,
        ),
        // 6. 回顾复盘
        make_agent_node(
            "step5_finance_invest",
            "回顾复盘",
            "你是一名投资回顾专家。请分析组合表现并提出再平衡建议。输出 JSON {performance_attribution, rebalance_recommendation, lessons_learned}",
            vec![td("OpcGetFinancialReport"), td("OpcRecordKpi")],
            Some("opc-finance_invest_lead-finance-portfolio-reviewer".to_string()),
            "step5_finance_invest_result",
            250.0,
            950.0,
        ),
        // 7. 结束节点
        make_end(250.0, 1150.0),
    ];

    let edges = vec![
        edge("e-trigger-step", "trigger", "step_finance_invest"),
        edge("e-step-step2", "step_finance_invest", "step2_finance_invest"),
        edge("e-step2-step3", "step2_finance_invest", "step3_finance_invest"),
        edge("e-step3-step4", "step3_finance_invest", "step4_finance_invest"),
        edge("e-step4-step5", "step4_finance_invest", "step5_finance_invest"),
        edge("e-step5-end", "step5_finance_invest", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "金融投资流程".to_string(),
        description: Some(
            "市场分析 → 行业研究 → 资产配置 → 交易执行 → 回顾复盘。完整的金融投资分析与执行流程。"
                .to_string(),
        ),
        icon: "⚙️".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["opc".to_string(), "industry".to_string(), "finance_invest".to_string()],
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
