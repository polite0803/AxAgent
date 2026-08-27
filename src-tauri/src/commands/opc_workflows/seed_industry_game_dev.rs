// SPDX-License-Identifier: AGPL-3.0-only

//! 游戏开发流程行业工作流模板种子化（v4 丰富拓扑：LLM 条件门 + 修正分支 + 汇合）。
//! 模板 ID：game_dev_harness_workflow

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{EdgeType, TriggerConfig, TriggerType, WorkflowTemplateData};
use sea_orm::DatabaseConnection;

use super::seed_domain_helpers::*;

const TEMPLATE_ID: &str = "game_dev_harness_workflow";
const TEMPLATE_VERSION: i32 = 4;

pub async fn seed_industry_game_dev_workflow_template(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let should_seed = super::check_template_version(db, TEMPLATE_ID, TEMPLATE_VERSION).await?;
    if !should_seed {
        return Ok(());
    }

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "step_game_dev",
            "概念设计",
            "你是概念设计专家。执行「概念设计」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("WebSearch")],
            Some("opc-game_dev_lead-game-concept-designer"),
            "step_game_dev",
            0.0,
            180.0,
        ),
        make_agent_node_full(
            "step2_game_dev",
            "原型开发",
            "你是原型开发专家。执行「原型开发」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("FileWrite"), td("WebSearch")],
            Some("opc-game_dev_lead-game-prototype-developer"),
            "step2_game_dev",
            vec![("input", "step_game_dev")],
            vec!["step_game_dev"],
            0.0,
            360.0,
        ),
        make_condition_node_llm(
            "c-game_dev-gate",
            "质量门",
            "根据原型开发结果判断：原型核心玩法是否可玩达标（是→true 内容生产，否→false 原型迭代）",
            "step2_game_dev",
            0.0,
            540.0,
        ),
        make_agent_node_full(
            "step3_game_dev",
            "内容生产",
            "你是内容生产专家。执行「内容生产」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("FileWrite")],
            Some("opc-game_dev_lead-game-content-designer"),
            "step3_game_dev",
            vec![("input", "step2_game_dev")],
            vec!["step2_game_dev"],
            -250.0,
            720.0,
        ),
        make_agent_node_full(
            "fix-game_dev",
            "原型迭代",
            "原型可玩性不足，迭代核心机制。输出 JSON：{\"iterations\":[], \"playable\":true}",
            vec![],
            None,
            "fix-game_dev",
            vec![("input", "step2_game_dev")],
            vec!["step2_game_dev"],
            250.0,
            720.0,
        ),
        make_merge_node("m-game_dev", "汇合", 0.0, 900.0),
        make_agent_node_full(
            "step4_game_dev",
            "测试优化",
            "你是测试优化专家。执行「测试优化」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("FileRead"), td("WebSearch")],
            Some("opc-game_dev_lead-game-qa-expert"),
            "step4_game_dev",
            vec![("input", "step3_game_dev")],
            vec!["step3_game_dev"],
            0.0,
            1080.0,
        ),
        make_agent_node_full(
            "step5_game_dev",
            "上线运营",
            "你是上线运营专家。执行「上线运营」：结合上游输入，输出结构化 JSON 结果（含关键指标、结论与建议）。",
            vec![td("FileWrite")],
            None,
            "step5_game_dev",
            vec![("input", "step4_game_dev")],
            vec!["step4_game_dev"],
            0.0,
            1260.0,
        ),
        make_approval_node(
            "ap-game_dev",
            "人工审批",
            "游戏内容与运营计划已生成，请主创审批",
            None,
            86400,
            "ap-game_dev",
            0.0,
            1440.0,
        ),
        make_end(0.0, 1620.0),
    ];

    let edges = vec![
        edge("e-trigger-step_game_dev", "trigger", "step_game_dev"),
        edge("e-step_game_dev-step2_game_dev", "step_game_dev", "step2_game_dev"),
        edge("e-step2_game_dev-gate", "step2_game_dev", "c-game_dev-gate"),
        edge_cond(
            "e-gate-main",
            "c-game_dev-gate",
            "true",
            "step3_game_dev",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-gate-fix",
            "c-game_dev-gate",
            "false",
            "fix-game_dev",
            EdgeType::ConditionFalse,
        ),
        edge("e-main-merge", "step3_game_dev", "m-game_dev"),
        edge("e-fix-merge", "fix-game_dev", "m-game_dev"),
        edge("e-m-game_dev-step4_game_dev", "m-game_dev", "step4_game_dev"),
        edge("e-step4_game_dev-step5_game_dev", "step4_game_dev", "step5_game_dev"),
        edge("e-step5_game_dev-approval", "step5_game_dev", "ap-game_dev"),
        edge("e-ap-game_dev-end", "ap-game_dev", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();
    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "游戏开发流程".to_string(),
        description: Some(
            "概念设计 → 原型开发 → 内容生产 → 测试优化 → 上线运营。游戏研发全流程。".to_string(),
        ),
        icon: "🎮".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["opc".to_string(), "industry".to_string(), "game_dev".to_string()],
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
