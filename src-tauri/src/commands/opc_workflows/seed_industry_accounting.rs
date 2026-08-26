// SPDX-License-Identifier: AGPL-3.0-only

//! 会计与财务管理行业工作流模板种子化（代码驱动，4步流程）。
//!
//! 流程：手动启动 → 创建发票 → 财务审批 → 通知客户 → 登记报表 → 完成

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "accounting_harness_workflow";
const TEMPLATE_VERSION: i32 = 3;

// ── 辅助函数 ──

fn make_agent_node(
    id: &str,
    title: &str,
    prompt: &str,
    tools: Vec<ToolDef>,
    profile_id: Option<&str>,
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
            agent_profile_id: profile_id.map(|s| s.to_string()),
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
        base: super::make_base("trigger", "手动启动", "手动触发会计财务工作流", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: super::make_base("end", "完成", "会计财务工作流结束", x, y),
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

/// 种子化会计与财务管理行业工作流模板。
pub async fn seed_industry_accounting_workflow_template(
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
        // 2. 创建发票
        make_agent_node(
            "step_accounting",
            "创建发票",
            "你是一名会计专员。请根据用户提供的公司信息创建发票，检查金额与客户信息。输出 JSON {invoice_id, customer, total, due_date}",
            vec![td("OpcCreateInvoice"), td("OpcListInvoices"), td("OpcListCustomers")],
            Some("opc-accounting_lead-accounting-financial-clerk"),
            "step_accounting_result",
            250.0,
            150.0,
        ),
        // 3. 财务审批
        make_agent_node(
            "approval_accounting",
            "财务审批",
            "你是一名财务审批人。请审核发票的合规性与准确性，识别风险。输出 JSON {approved, risk_level, comments}",
            vec![td("OpcGetFinancialReport"), td("OpcListInvoices")],
            Some("opc-accounting_lead-accounting-financial-approver"),
            "approval_accounting_result",
            250.0,
            350.0,
        ),
        // 4. 通知客户
        make_agent_node(
            "step2_accounting",
            "通知客户",
            "你是一名财务助理。请向客户发送发票通知，说明金额与付款方式。输出 JSON {notified, channel, message}",
            vec![td("OpcSendNotification"), td("OpcListCustomers")],
            Some("opc-accounting_lead-accounting-financial-assistant"),
            "step2_accounting_result",
            250.0,
            550.0,
        ),
        // 5. 登记报表
        make_agent_node(
            "step3_accounting",
            "登记报表",
            "你是一名财务分析师。请将发票数据登记到财务报表，计算应收与回款指标。输出 JSON {report_updated, total_revenue, collection_rate}",
            vec![td("OpcRecordKpi"), td("OpcGetFinancialReport")],
            Some("opc-accounting_lead-accounting-financial-analyst"),
            "step3_accounting_result",
            250.0,
            750.0,
        ),
        // 6. 结束节点
        make_end(250.0, 950.0),
    ];

    let edges = vec![
        edge("e-trigger-step_accounting", "trigger", "step_accounting"),
        edge("e-step_accounting-approval_accounting", "step_accounting", "approval_accounting"),
        edge("e-approval_accounting-step2_accounting", "approval_accounting", "step2_accounting"),
        edge("e-step2_accounting-step3_accounting", "step2_accounting", "step3_accounting"),
        edge("e-step3_accounting-end", "step3_accounting", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "会计与财务管理".to_string(),
        description: Some("会计财务行业工作流：发票创建、审批、通知、报表".to_string()),
        icon: "💰".to_string(),
        tags: vec!["opc".to_string(), "industry".to_string(), "accounting".to_string()],
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
