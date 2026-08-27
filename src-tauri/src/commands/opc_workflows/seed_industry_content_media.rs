// SPDX-License-Identifier: AGPL-3.0-only

//! 内容与媒体行业工作流模板种子化（代码驱动，5步流程）。
//!
//! 流程：手动启动 → 选题策划 → 内容创作 → 优化打磨 → 多平台发布 → IP打造 → 完成

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, OutputMode, ToolDef,
    TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

const TEMPLATE_ID: &str = "content_media_harness_workflow";
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
        base: super::make_base("trigger", "开始", "手动触发内容媒体工作流", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: super::make_base("end", "完成", "内容媒体工作流结束", x, y),
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

/// 种子化内容与媒体行业工作流模板。
pub async fn seed_industry_content_media_workflow_template(
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
        // 2. 选题策划
        make_agent_node(
            "step_content_media",
            "选题策划",
            "你是一名资深内容策划专家。分析当前热点和用户需求，策划具有爆款潜力的内容主题。\n\n请输出 JSON：\n{\n  \"topic\": \"选题方向\",\n  \"angle\": \"切入角度\",\n  \"target_audience\": \"目标受众\",\n  \"hook_points\": [\"钩子1\", \"钩子2\"]\n}",
            vec![td("OpcListBlogPosts"), td("WebSearch")],
            Some("opc-cmo-cmo-content-strategist"),
            "step_content_media_result",
            250.0,
            150.0,
        ),
        // 3. 内容创作
        make_agent_node(
            "step2_content_media",
            "内容创作",
            "你是一名内容创作者。根据选题创作高质量文章。使用 OpcCreateBlogPost 发布博客。\n\n请输出 JSON：\n{\n  \"post_id\": \"博客ID\",\n  \"title\": \"标题\",\n  \"summary\": \"摘要\",\n  \"tags\": [\"标签1\"]\n}",
            vec![td("OpcCreateBlogPost"), td("FileWrite"), td("WebSearch")],
            Some("opc-cmo-cmo-content-creator"),
            "step2_content_media_result",
            250.0,
            350.0,
        ),
        // 4. 优化打磨
        make_agent_node(
            "step3_content_media",
            "优化打磨",
            "你是一名 SEO 优化专家。对内容进行 SEO 优化和传播力增强。\n\n请输出 JSON：\n{\n  \"optimized_title\": \"优化标题\",\n  \"meta_description\": \"Meta描述\",\n  \"seo_score\": 85\n}",
            vec![td("WebSearch"), td("FileRead")],
            Some("opc-cmo-cmo-seo-expert"),
            "step3_content_media_result",
            250.0,
            550.0,
        ),
        // 5. 多平台发布
        make_agent_node(
            "step4_content_media",
            "多平台发布",
            "你是一名社交媒体经理。制定各平台的发布计划和时间安排。\n\n请输出 JSON：\n{\n  \"schedule\": [{\"platform\": \"微博\", \"time\": \"09:00\"}],\n  \"engagement_plan\": \"互动策略说明\"\n}",
            vec![td("OpcCreatePublishSchedule"), td("OpcListPublishSchedules")],
            Some("opc-cmo-cmo-social-manager"),
            "step4_content_media_result",
            250.0,
            750.0,
        ),
        // 6. IP打造
        make_agent_node(
            "step5_content_media",
            "IP打造",
            "你是一名品牌策略师。分析目标受众和竞争格局，设计品牌 IP 人设定位和内容策略。\n\n请输出 JSON：\n{\n  \"persona\": \"人设描述\",\n  \"brand_voice\": \"品牌语调\",\n  \"content_strategy\": \"内容策略说明\"\n}",
            vec![td("OpcCreateContentAsset"), td("WebSearch"), td("OpcListCustomers")],
            Some("opc-cmo-cmo-brand-strategist"),
            "step5_content_media_result",
            250.0,
            950.0,
        ),
        // 7. 结束节点
        make_end(250.0, 1150.0),
    ];

    let edges = vec![
        edge("e-trigger-step_content_media", "trigger", "step_content_media"),
        edge(
            "e-step_content_media-step2_content_media",
            "step_content_media",
            "step2_content_media",
        ),
        edge(
            "e-step2_content_media-step3_content_media",
            "step2_content_media",
            "step3_content_media",
        ),
        edge(
            "e-step3_content_media-step4_content_media",
            "step3_content_media",
            "step4_content_media",
        ),
        edge(
            "e-step4_content_media-step5_content_media",
            "step4_content_media",
            "step5_content_media",
        ),
        edge("e-step5_content_media-end", "step5_content_media", "end"),
    ];

    let now = chrono::Utc::now().timestamp_millis();

    let template_data = WorkflowTemplateData {
        id: TEMPLATE_ID.to_string(),
        name: "内容与媒体".to_string(),
        description: Some(
            "内容媒体行业工作流：选题策划、内容创作、优化打磨、多平台发布、IP打造".to_string(),
        ),
        icon: "📱".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["opc".to_string(), "industry".to_string(), "content_media".to_string()],
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
