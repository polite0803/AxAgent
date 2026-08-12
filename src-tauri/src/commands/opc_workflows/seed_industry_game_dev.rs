// SPDX-License-Identifier: AGPL-3.0-only

//! 游戏开发行业工作流模板种子化（代码驱动，5步流程）。
//!
//! 流程：手动启动 → 概念设计 → 原型开发 → 内容生产 → 测试优化 → 上线运营 → 完成

use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "game_dev_harness_workflow";
const TEMPLATE_VERSION: i32 = 2;

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

/// 种子化游戏开发行业工作流模板。
pub async fn seed_industry_game_dev_workflow_template(
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
        // 2. 概念设计
        make_agent_node(
            "step_game_dev",
            "概念设计",
            "你是一名资深游戏概念设计师。请进行游戏概念设计，包括核心玩法、世界观、美术风格与目标受众。输出 JSON {game_concept, core_mechanics, target_audience, art_style}",
            vec![td("WebSearch")],
            Some("opc-game_dev_lead-game-concept-designer".to_string()),
            "step_game_dev_result",
            250.0,
            150.0,
        ),
        // 3. 原型开发
        make_agent_node(
            "step2_game_dev",
            "原型开发",
            "你是一名游戏原型开发专家。请根据概念设计开发可玩原型，包括核心机制实现与技术选型。输出 JSON {prototype_features, tech_stack, development_plan, key_risks}",
            vec![td("FileWrite"), td("WebSearch")],
            Some("opc-game_dev_lead-game-prototype-developer".to_string()),
            "step2_game_dev_result",
            250.0,
            350.0,
        ),
        // 4. 内容生产
        make_agent_node(
            "step3_game_dev",
            "内容生产",
            "你是一名游戏内容设计师。请规划并生产游戏内容，包括关卡设计、角色美术、音效与剧情。输出 JSON {level_design, character_assets, sound_plan, narrative_outline}",
            vec![td("FileWrite")],
            Some("opc-game_dev_lead-game-content-designer".to_string()),
            "step3_game_dev_result",
            250.0,
            550.0,
        ),
        // 5. 测试优化
        make_agent_node(
            "step4_game_dev",
            "测试优化",
            "你是一名游戏 QA 专家。请制定测试计划并执行质量保证，识别性能瓶颈与体验问题。输出 JSON {test_plan, bug_report, performance_issues, optimization_suggestions}",
            vec![td("FileRead"), td("WebSearch")],
            Some("opc-game_dev_lead-game-qa-expert".to_string()),
            "step4_game_dev_result",
            250.0,
            750.0,
        ),
        // 6. 上线运营
        make_agent_node(
            "step5_game_dev",
            "上线运营",
            "你是一名游戏运营专家。请制定上线发布计划与运营策略，包括渠道分发、社区运营与数据分析。输出 JSON {launch_plan, channel_strategy, community_plan, kpi_targets}",
            vec![td("WebSearch")],
            Some("opc-game_dev_lead-game-operations-expert".to_string()),
            "step5_game_dev_result",
            250.0,
            950.0,
        ),
        // 7. 结束节点
        make_end(250.0, 1150.0),
    ];

    let edges = vec![
        edge("e-trigger-step", "trigger", "step_game_dev"),
        edge("e-step-step2", "step_game_dev", "step2_game_dev"),
        edge("e-step2-step3", "step2_game_dev", "step3_game_dev"),
        edge("e-step3-step4", "step3_game_dev", "step4_game_dev"),
        edge("e-step4-step5", "step4_game_dev", "step5_game_dev"),
        edge("e-step5-end", "step5_game_dev", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "游戏开发流程".to_string(),
        description: Some(
            "概念设计 → 原型开发 → 内容生产 → 测试优化 → 上线运营。完整的游戏开发全流程。"
                .to_string(),
        ),
        icon: "⚙️".to_string(),
        tags: vec!["opc".to_string(), "industry".to_string(), "game_dev".to_string()],
        version: TEMPLATE_VERSION,
        is_preset: true,
        is_editable: true,
        is_public: false,
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
