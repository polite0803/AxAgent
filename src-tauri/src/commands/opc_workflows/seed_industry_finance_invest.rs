// SPDX-License-Identifier: AGPL-3.0-only

//! 金融投资流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：finance_invest_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "finance_invest_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_finance_invest_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_finance_invest",
            "市场分析",
            "你是市场分析专家。执行「市场分析」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcGetDashboard"), td("OpcListKpis")],
            Some("opc-finance_invest_lead-finance-market-analyst"),
            "step_finance_invest",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_finance_invest",
            "行业研究",
            "你是行业研究专家。执行「行业研究」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcSearchWiki"), td("OpcListProjects")],
            Some("opc-finance_invest_lead-finance-industry-researcher"),
            "step2_finance_invest",
            vec![("input", "step_finance_invest")],
            vec!["step_finance_invest"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-finance_invest-gate",
            "质量门",
            "根据行业研究结果判断：是否识别出值得配置的投资机会（是→true 资产配置，否→false 机会再挖掘）",
            "step2_finance_invest",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_finance_invest",
            "资产配置",
            "你是资产配置专家。执行「资产配置」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcGetFinancialReport"), td("OpcGetDashboard")],
            Some("opc-finance_invest_lead-finance-asset-allocator"),
            "step3_finance_invest",
            vec![("input", "step2_finance_invest")],
            vec!["step2_finance_invest"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-finance_invest",
            "机会再挖掘",
            "投资机会不足，扩大研究范围再挖掘。输出 JSON：{\"re_scanned\":[], \"found\":true}",
            vec![],
            None,
            "fix-finance_invest",
            vec![("input", "step2_finance_invest")],
            vec!["step2_finance_invest"],
            250.0,
            720.0,
        ),
        make_merge_node("m-finance_invest", "汇合", 0.0, 900.0),
        make_agent_node_full(
            "step4_finance_invest",
            "交易执行",
            "你是交易执行专家。执行「交易执行」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcSendNotification"), td("OpcGetDashboard")],
            Some("opc-finance_invest_lead-finance-trade-executor"),
            "step4_finance_invest",
            vec![("input", "step3_finance_invest")],
            vec!["step3_finance_invest"],
            0.0,
            1080.0,
        ),
        make_agent_node_full(
            "step5_finance_invest",
            "回顾复盘",
            "你是回顾复盘专家。执行「回顾复盘」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("OpcGetFinancialReport"), td("OpcRecordKpi")],
            Some("opc-finance_invest_lead-finance-portfolio-reviewer"),
            "step5_finance_invest",
            vec![("input", "step4_finance_invest")],
            vec!["step4_finance_invest"],
            0.0,
            1260.0,
        ),
        make_approval_node(
            "ap-finance_invest",
            "人工审批",
            "投资组合与交易计划已生成，请风控审批",
            None,
            86400,
            "ap-finance_invest",
            0.0,
            1440.0,
        ),
        make_end(0.0, 1620.0),
    ];

    let edges = vec![
        edge("e-trigger-step_finance_invest", "trigger", "step_finance_invest"),
        edge(
            "e-step_finance_invest-step2_finance_invest",
            "step_finance_invest",
            "step2_finance_invest",
        ),
        edge("e-step2_finance_invest-gate", "step2_finance_invest", "c-finance_invest-gate"),
        edge_cond(
            "e-gate-main",
            "c-finance_invest-gate",
            "true",
            "step3_finance_invest",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-finance_invest-gate",
            "false",
            "fix-finance_invest",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_finance_invest", "m-finance_invest"),
        edge("e-fix-merge", "fix-finance_invest", "m-finance_invest"),
        edge("e-m-finance_invest-step4_finance_invest", "m-finance_invest", "step4_finance_invest"),
        edge(
            "e-step4_finance_invest-step5_finance_invest",
            "step4_finance_invest",
            "step5_finance_invest",
        ),
        edge("e-step5_finance_invest-approval", "step5_finance_invest", "ap-finance_invest"),
        edge("e-ap-finance_invest-end", "ap-finance_invest", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "金融投资流程".to_string(),
        description: Some(
            "市场分析 → 行业研究 → 资产配置 → 交易执行 → 回顾复盘。完整投资分析与执行流程。"
                .to_string(),
        ),
        icon: "💹".to_string(),
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
