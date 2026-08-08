// SPDX-License-Identifier: AGPL-3.0-only

//! 内容媒体行业 3 个专属工作流模板种子化（代码驱动，对齐股票业务）。
//!
//! 模板列表：
//! - workflow-cm-viral-content      爆款内容生成：选题策划 → 内容创作 → 优化打磨
//! - workflow-cm-multi-platform      多平台适配：内容创作 → 平台适配 → 分发策略
//! - workflow-cm-ip-building        IP 打造方案：人设定位 → 内容规划 → 粉丝运营

use axagent_entities::workflow_template;
use axagent_harness::workflow_types::*;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

const TEMPLATE_VERSION: i32 = 2; // v2: 代码驱动版本，覆盖旧 YAML

/// 内容媒体 3 个专属工作流 ID
const CM_TEMPLATE_IDS: &[&str] =
    &["workflow-cm-viral-content", "workflow-cm-multi-platform", "workflow-cm-ip-building"];

/// 主入口：种子化内容媒体 3 个专属工作流模板。
/// 幂等：按版本判断是否需要覆盖，避免用户编辑丢失。
pub async fn seed_content_media_workflows(
    db: &sea_orm::DatabaseConnection,
) -> Result<usize, String> {
    let mut seeded_count = 0;

    for template_id in CM_TEMPLATE_IDS {
        let (nodes, edges, name, description, icon, tags) = build_template_nodes_edges(template_id);

        let template_data = WorkflowTemplateData {
            id: template_id.to_string(),
            name,
            description: Some(description),
            icon,
            tags,
            version: TEMPLATE_VERSION,
            is_preset: true,
            is_editable: true,
            is_public: true,
            trigger_config: Some(TriggerConfig {
                trigger_type: TriggerType::Manual,
                config: serde_json::json!({}),
            }),
            nodes,
            edges,
            input_schema: None,
            output_schema: None,
            variables: Vec::<Variable>::new(),
            error_config: None,
            error_workflow_id: None,
            tool_defs: Vec::<RhaiToolDef>::new(),
            mission_hash: None,
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
        };

        upsert_template_safe(db, template_data).await?;
        seeded_count += 1;
    }

    tracing::info!("[content_media_setup] 内容媒体 {} 个专属工作流已种子化", seeded_count);
    Ok(seeded_count)
}

/// 安全 upsert：版本检查 + 保留用户修改
async fn upsert_template_safe(
    db: &sea_orm::DatabaseConnection,
    data: WorkflowTemplateData,
) -> Result<(), String> {
    let id = &data.id;

    // 版本检查：若已是最新则跳过
    if let Ok(Some(existing)) = workflow_template::Entity::find_by_id(id).one(db).await {
        if existing.version >= TEMPLATE_VERSION {
            tracing::info!(
                "[content_media_setup] 模板 {} 已是最新版本 v{}，跳过",
                id,
                existing.version
            );
            return Ok(());
        }
    }

    let tags_json = serde_json::to_string(&data.tags).unwrap_or_default();
    let nodes_json = serde_json::to_string(&data.nodes).map_err(|e| format!("nodes: {e}"))?;
    let edges_json = serde_json::to_string(&data.edges).map_err(|e| format!("edges: {e}"))?;
    let trigger_json = data.trigger_config.as_ref().and_then(|t| serde_json::to_string(t).ok());

    let am = workflow_template::ActiveModel {
        id: Set(data.id.clone()),
        name: Set(data.name),
        description: Set(data.description),
        icon: Set(data.icon),
        tags: Set(Some(tags_json)),
        version: Set(data.version),
        is_preset: Set(data.is_preset),
        is_editable: Set(data.is_editable),
        is_public: Set(data.is_public),
        trigger_config: Set(trigger_json),
        nodes: Set(nodes_json),
        edges: Set(edges_json),
        input_schema: Set(None),
        output_schema: Set(None),
        variables: Set(Some("[]".to_string())),
        error_config: Set(None),
        composite_source: Set(None),
        mission_hash: Set(None),
        tool_defs: Set(Some("[]".to_string())),
        created_at: Set(data.created_at),
        updated_at: Set(data.updated_at),
    };

    // 先删再插（幂等）
    let _ = workflow_template::Entity::delete_by_id(id).exec(db).await;
    am.insert(db).await.map_err(|e| format!("写入模板 {} 失败: {e}", id))?;

    Ok(())
}

/// 构建指定模板的节点、边、名称、描述、图标、标签。
fn build_template_nodes_edges(
    template_id: &str,
) -> (Vec<WorkflowNode>, Vec<WorkflowEdge>, String, String, String, Vec<String>) {
    match template_id {
        "workflow-cm-viral-content" => build_viral_content(),
        "workflow-cm-multi-platform" => build_multi_platform(),
        "workflow-cm-ip-building" => build_ip_building(),
        _ => unreachable!("未知模板: {template_id}"),
    }
}

// ── 公共辅助函数 ──

fn make_base(id: &str, title: &str, desc: &str, x: f64, y: f64) -> WorkflowNodeBase {
    WorkflowNodeBase {
        id: id.into(),
        title: title.into(),
        description: Some(desc.into()),
        position: Position { x, y },
        retry: RetryConfig::default(),
        timeout: Some(300),
        enabled: true,
        parent_id: None,
        compensation: None,
        continue_on_fail: false,
    }
}

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
    let mut input_mapping = std::collections::HashMap::new();
    input_mapping.insert("user_input".to_string(), "trigger".to_string());

    WorkflowNode::Agent(AgentNode {
        base: make_base(id, title, prompt, x, y),
        config: AgentNodeConfig {
            system_prompt: prompt.to_string(),
            context_sources: vec!["trigger".to_string()],
            input_mapping,
            output_var: output_var.to_string(),
            model: None,
            temperature: None,
            max_tokens: None,
            tools,
            exposed_tools: Vec::new(),
            output_mode: OutputMode::Json,
            agent_profile_id: profile_id.map(|s| s.to_string()),
            max_tool_rounds: None,
            execution_mode: None,
            rag_source_ids: Vec::new(),
            model_role: None,
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
        base: make_base("trigger", "开始", "手动触发", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: make_base("end", "结束", "工作流结束", x, y),
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

fn td(name: &str, desc: &str) -> ToolDef {
    ToolDef { name: name.into(), description: Some(desc.into()), parameters: None }
}

// ── 模板 1: 爆款内容生成 ──

fn build_viral_content()
-> (Vec<WorkflowNode>, Vec<WorkflowEdge>, String, String, String, Vec<String>) {
    let profile = "opc-cmo-cmo-content-strategist";

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "vc-topic",
            "选题策划",
            "你是一名资深内容策划专家。分析当前热点和用户需求，策划具有爆款潜力的内容主题。\n\n请输出 JSON：\n{\n  \"topic\": \"选题方向\",\n  \"angle\": \"切入角度\",\n  \"target_audience\": \"目标受众\",\n  \"hook_points\": [\"钩子1\", \"钩子2\"]\n}",
            vec![],
            Some(profile),
            "vc-topic",
            200.0,
            0.0,
        ),
        make_agent_node(
            "vc-create",
            "内容创作",
            "根据选题创作高质量文章。使用 OpcCreateBlogPost 发布博客。\n\n请输出 JSON：\n{\n  \"post_id\": \"博客ID\",\n  \"title\": \"标题\",\n  \"summary\": \"摘要\",\n  \"tags\": [\"标签1\"]\n}",
            vec![td("OpcCreateBlogPost", "创建博客文章"), td("OpcListBlogPosts", "列出已有博客")],
            Some(profile),
            "vc-create",
            400.0,
            0.0,
        ),
        make_agent_node(
            "vc-optimize",
            "优化打磨",
            "对内容进行 SEO 优化和传播力增强。\n\n请输出 JSON：\n{\n  \"optimized_title\": \"优化标题\",\n  \"meta_description\": \"Meta描述\",\n  \"seo_score\": 85\n}",
            vec![td("OpcListBlogPosts", "列出已有博客")],
            Some(profile),
            "vc-optimize",
            600.0,
            0.0,
        ),
        make_end(800.0, 0.0),
    ];

    let edges = vec![
        edge("e-trigger-vc-topic", "trigger", "vc-topic"),
        edge("e-vc-topic-vc-create", "vc-topic", "vc-create"),
        edge("e-vc-create-vc-optimize", "vc-create", "vc-optimize"),
        edge("e-vc-optimize-end", "vc-optimize", "end"),
    ];

    (
        nodes,
        edges,
        "爆款内容生成".to_string(),
        "选题策划 → 内容创作 → 优化打磨。快速生成高传播潜力的爆款内容。".to_string(),
        "🔥".to_string(),
        vec!["content".to_string(), "viral".to_string(), "creation".to_string()],
    )
}

// ── 模板 2: 多平台适配 ──

fn build_multi_platform()
-> (Vec<WorkflowNode>, Vec<WorkflowEdge>, String, String, String, Vec<String>) {
    let profile = "opc-cmo-cmo-content-strategist";

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "mp-create",
            "内容创作",
            "创作一篇通用的长文内容。使用 OpcCreateBlogPost 发布博客。\n\n请输出 JSON：\n{\n  \"post_id\": \"博客ID\",\n  \"content\": \"正文内容\",\n  \"key_points\": [\"要点1\", \"要点2\"]\n}",
            vec![td("OpcCreateBlogPost", "创建博客文章")],
            Some(profile),
            "mp-create",
            200.0,
            0.0,
        ),
        make_agent_node(
            "mp-adapt",
            "平台适配",
            "将长文内容适配为不同平台格式（微博/微信/抖音/小红书）。\n\n请输出 JSON：\n{\n  \"platforms\": [\n    {\"name\": \"微博\", \"adapted_content\": \"适配内容\", \"hashtags\": [\"#标签\"]},\n    {\"name\": \"微信\", \"adapted_content\": \"适配内容\", \"hashtags\": []}\n  ]\n}",
            vec![],
            Some(profile),
            "mp-adapt",
            400.0,
            0.0,
        ),
        make_agent_node(
            "mp-distribute",
            "分发策略",
            "制定各平台的发布时间和互动策略。使用 OpcListBlogPosts 查看已有内容。\n\n请输出 JSON：\n{\n  \"schedule\": [{\"platform\": \"微博\", \"time\": \"09:00\"}],\n  \"engagement_plan\": \"互动策略说明\"\n}",
            vec![td("OpcListBlogPosts", "列出已有博客")],
            Some(profile),
            "mp-distribute",
            600.0,
            0.0,
        ),
        make_end(800.0, 0.0),
    ];

    let edges = vec![
        edge("e-trigger-mp-create", "trigger", "mp-create"),
        edge("e-mp-create-mp-adapt", "mp-create", "mp-adapt"),
        edge("e-mp-adapt-mp-distribute", "mp-adapt", "mp-distribute"),
        edge("e-mp-distribute-end", "mp-distribute", "end"),
    ];

    (
        nodes,
        edges,
        "多平台适配".to_string(),
        "内容创作 → 平台适配 → 分发策略。将内容适配到多个社交媒体平台。".to_string(),
        "🌐".to_string(),
        vec!["content".to_string(), "multi-platform".to_string(), "distribution".to_string()],
    )
}

// ── 模板 3: IP 打造方案 ──

fn build_ip_building() -> (Vec<WorkflowNode>, Vec<WorkflowEdge>, String, String, String, Vec<String>)
{
    let profile = "opc-cmo-cmo-content-strategist";

    let nodes = vec![
        make_trigger(0.0, 0.0),
        make_agent_node(
            "ip-positioning",
            "人设定位",
            "分析目标受众和竞争格局，确定 IP 人设定位和差异化价值。\n\n请输出 JSON：\n{\n  \"persona\": \"人设描述\",\n  \"niche\": \"垂直领域\",\n  \"value_proposition\": \"价值主张\",\n  \"brand_voice\": \"品牌语调\"\n}",
            vec![],
            Some(profile),
            "ip-positioning",
            200.0,
            0.0,
        ),
        make_agent_node(
            "ip-content-plan",
            "内容规划",
            "制定 30 天内容日历和核心主题。使用 OpcListLandingPages 查看现有落地页。\n\n请输出 JSON：\n{\n  \"calendar\": [{\"week\": 1, \"topics\": [\"主题1\"]}],\n  \"themes\": [\"内容主题1\"],\n  \"key_topics\": [\"关键话题1\"]\n}",
            vec![td("OpcListLandingPages", "列出落地页")],
            Some(profile),
            "ip-content-plan",
            400.0,
            0.0,
        ),
        make_agent_node(
            "ip-fans",
            "粉丝运营",
            "设计粉丝互动和增长策略。使用 OpcCreateLandingPage 创建粉丝落地页。\n\n请输出 JSON：\n{\n  \"growth_tactics\": [\"增长策略1\"],\n  \"engagement_rules\": [\"互动规则1\"],\n  \"landing_page_id\": \"落地页ID\"\n}",
            vec![td("OpcCreateLandingPage", "创建落地页")],
            Some(profile),
            "ip-fans",
            600.0,
            0.0,
        ),
        make_end(800.0, 0.0),
    ];

    let edges = vec![
        edge("e-trigger-ip-positioning", "trigger", "ip-positioning"),
        edge("e-ip-positioning-ip-content-plan", "ip-positioning", "ip-content-plan"),
        edge("e-ip-content-plan-ip-fans", "ip-content-plan", "ip-fans"),
        edge("e-ip-fans-end", "ip-fans", "end"),
    ];

    (
        nodes,
        edges,
        "IP 打造方案".to_string(),
        "人设定位 → 内容规划 → 粉丝运营。系统化打造个人 IP。".to_string(),
        "⭐".to_string(),
        vec!["ip".to_string(), "personal-brand".to_string(), "strategy".to_string()],
    )
}
