// SPDX-License-Identifier: AGPL-3.0-only

//! 会计财务流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：accounting_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "accounting_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_accounting_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_accounting",
            "创建发票",
            "你是创建发票专家。执行「创建发票」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListInvoices"), td("OpcCreateInvoice")],
            Some("opc-accounting_lead-accounting-financial-clerk"),
            "step_accounting",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_accounting",
            "财务审批",
            "你是财务审批专家。执行「财务审批」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListInvoices"), td("OpcGetFinancialReport")],
            Some("opc-accounting_lead-accounting-financial-analyst"),
            "step2_accounting",
            vec![("input", "step_accounting")],
            vec!["step_accounting"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-accounting-gate",
            "质量门",
            "根据财务审批结果判断：发票金额是否超过 10 万需要升级审批（是→true 走通知客户，否→false 常规处理）",
            "step2_accounting",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_accounting",
            "通知客户",
            "你是通知客户专家。执行「通知客户」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcListCustomers"), td("OpcSendNotification")],
            Some("opc-accounting_lead-accounting-financial-approver"),
            "step3_accounting",
            vec![("input", "step2_accounting")],
            vec!["step2_accounting"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-accounting",
            "补充财务材料",
            "财务数据不完整，补充材料后继续。输出 JSON：{\"supplemented\":[], \"ready\":true}",
            vec![],
            None,
            "fix-accounting",
            vec![("input", "step2_accounting")],
            vec!["step2_accounting"],
            250.0,
            720.0,
        ),
        make_merge_node("m-accounting", "汇合", 0.0, 900.0),
        make_agent_node_full(
            "step4_accounting",
            "登记报表",
            "你是登记报表专家。执行「登记报表」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcRecordKpi"), td("OpcGetFinancialReport")],
            None,
            "step4_accounting",
            vec![("input", "step3_accounting")],
            vec!["step3_accounting"],
            0.0,
            1080.0,
        ),
        make_approval_node(
            "ap-accounting",
            "人工审批",
            "会计流程结果已生成，请财务负责人审批",
            None,
            86400,
            "ap-accounting",
            0.0,
            1260.0,
        ),
        make_end(0.0, 1440.0),
    ];

    let edges = vec![
        edge("e-trigger-step_accounting", "trigger", "step_accounting"),
        edge("e-step_accounting-step2_accounting", "step_accounting", "step2_accounting"),
        edge("e-step2_accounting-gate", "step2_accounting", "c-accounting-gate"),
        edge_cond(
            "e-gate-main",
            "c-accounting-gate",
            "true",
            "step3_accounting",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-accounting-gate",
            "false",
            "fix-accounting",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_accounting", "m-accounting"),
        edge("e-fix-merge", "fix-accounting", "m-accounting"),
        edge("e-m-accounting-step4_accounting", "m-accounting", "step4_accounting"),
        edge("e-step4_accounting-approval", "step4_accounting", "ap-accounting"),
        edge("e-ap-accounting-end", "ap-accounting", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "会计财务流程".to_string(),
        description: Some(
            "发票创建 → 财务审批 → 客户通知 → 报表登记。完整会计流程闭环。".to_string(),
        ),
        icon: "🧾".to_string(),
        cluster_id: None,
        route_path: None,
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
