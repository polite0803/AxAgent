// SPDX-License-Identifier: AGPL-3.0-only

//! 内容媒体行业 4 个专属工作流模板种子化（代码驱动，对齐股票业务）。
//!
//! 模板列表：
//! - workflow-cm-viral-content      爆款内容生成：选题策划 → 内容创作 → 优化打磨
//! - workflow-cm-multi-platform      多平台适配：内容创作 → 平台适配 → 分发策略
//! - workflow-cm-ip-building        IP 打造方案：人设定位 → 内容规划 → 粉丝运营
//! - workflow-cm-literary-creation  文字创作：创作元认知 → 大纲拆章 → 逐章创作 → 评审

use axagent_entities::workflow_template;
use axagent_harness::workflow_types::*;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

/// 旧 3 模板版本号（保持 v2，不影响已有用户配置）
const LEGACY_TEMPLATE_VERSION: i32 = 2;
/// 新文学创作模板版本号（v3）
const LITERARY_TEMPLATE_VERSION: i32 = 3;

/// 获取指定模板的版本号
fn get_template_version(template_id: &str) -> i32 {
    if template_id == "workflow-cm-literary-creation" {
        LITERARY_TEMPLATE_VERSION
    } else {
        LEGACY_TEMPLATE_VERSION
    }
}

/// 内容媒体 4 个专属工作流 ID
const CM_TEMPLATE_IDS: &[&str] = &[
    "workflow-cm-viral-content",
    "workflow-cm-multi-platform",
    "workflow-cm-ip-building",
    "workflow-cm-literary-creation",
];

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
            version: get_template_version(template_id),
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
        if existing.version >= data.version {
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
        "workflow-cm-literary-creation" => build_literary_creation(),
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

/// 支持自定义 input_mapping 的 Agent 节点（用于节点间数据传递）
fn make_agent_node_with_inputs(
    id: &str,
    title: &str,
    prompt: &str,
    tools: Vec<ToolDef>,
    profile_id: Option<&str>,
    output_var: &str,
    inputs: Vec<(&str, &str)>,
    x: f64,
    y: f64,
) -> WorkflowNode {
    let mut node = make_agent_node(id, title, prompt, tools, profile_id, output_var, x, y);
    if let WorkflowNode::Agent(ref mut a) = node {
        a.config.input_mapping = inputs.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
    }
    node
}

/// 支持自定义 input_mapping + context_sources 的 Agent 节点。
///
/// context_sources 是软依赖：get_context_source_results 按节点 ID 从
/// workflow.results 取输出注入 ctx.variables（Loop body 节点 / 无 edges
/// 直接上游的节点引用跨级输出时必须用这个）。
fn make_agent_node_full(
    id: &str,
    title: &str,
    prompt: &str,
    tools: Vec<ToolDef>,
    profile_id: Option<&str>,
    output_var: &str,
    inputs: Vec<(&str, &str)>,
    context_sources: Vec<&str>,
    x: f64,
    y: f64,
) -> WorkflowNode {
    let mut node = make_agent_node(id, title, prompt, tools, profile_id, output_var, x, y);
    if let WorkflowNode::Agent(ref mut a) = node {
        a.config.input_mapping = inputs.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
        let mut sources = context_sources.into_iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // 保留默认 trigger 源（除非显式排除）
        if !sources.iter().any(|s| s == "trigger") {
            sources.insert(0, "trigger".to_string());
        }
        a.config.context_sources = sources;
    }
    node
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

/// 条件边（用于 ConditionNode 的 true/false 分支）
fn edge_cond(id: &str, source: &str, handle: &str, target: &str, etype: EdgeType) -> WorkflowEdge {
    WorkflowEdge {
        id: id.into(),
        source: source.into(),
        source_handle: Some(handle.into()),
        target: target.into(),
        target_handle: None,
        edge_type: etype,
        label: None,
    }
}

fn td(name: &str, desc: &str) -> ToolDef {
    ToolDef { name: name.into(), description: Some(desc.into()), parameters: None }
}

fn make_condition_node(
    id: &str,
    title: &str,
    conditions: Vec<Condition>,
    logical_op: LogicalOperator,
    x: f64,
    y: f64,
) -> WorkflowNode {
    WorkflowNode::Condition(ConditionNode {
        base: make_base(id, title, "条件判断", x, y),
        config: ConditionNodeConfig {
            conditions,
            logical_op,
            judge_by_llm: None,
            routing_prompt: None,
            routing_model: None,
            confidence_threshold: None,
        },
    })
}

fn make_loop_node(
    id: &str,
    title: &str,
    loop_type: LoopType,
    iter_input_var: Option<&str>,
    iteratee_var: Option<&str>,
    iter_output_var: Option<&str>,
    partial_result_var: Option<&str>,
    max_iterations: Option<u32>,
    body_steps: Vec<String>,
    x: f64,
    y: f64,
) -> WorkflowNode {
    WorkflowNode::Loop(LoopNode {
        base: make_base(id, title, "循环执行", x, y),
        config: LoopNodeConfig {
            loop_type,
            items_var: None,
            iter_input_var: iter_input_var.map(|s| s.to_string()),
            iteratee_var: iteratee_var.map(|s| s.to_string()),
            iter_output_var: iter_output_var.map(|s| s.to_string()),
            partial_result_var: partial_result_var.map(|s| s.to_string()),
            max_iterations,
            continue_condition: None,
            continue_on_error: false,
            body_steps,
            interrupt_after_each: false,
            interrupt_nodes: vec![],
            sub_graph: None,
        },
    })
}

fn make_file_operation_node(
    id: &str,
    title: &str,
    operation: &str,
    file_path: &str,
    output_var: &str,
    x: f64,
    y: f64,
) -> WorkflowNode {
    WorkflowNode::FileOperation(FileOperationNode {
        base: make_base(id, title, "文件操作", x, y),
        config: FileOperationNodeConfig {
            operation: operation.into(),
            file_path: file_path.into(),
            content: None,
            output_var: output_var.into(),
        },
    })
}

fn make_approval_node(
    id: &str,
    title: &str,
    message: &str,
    approver: Option<&str>,
    timeout_secs: u64,
    output_var: &str,
    x: f64,
    y: f64,
) -> WorkflowNode {
    WorkflowNode::Approval(ApprovalNode {
        base: make_base(id, title, "人工审批", x, y),
        config: ApprovalNodeConfig {
            message: message.into(),
            approver: approver.map(|s| s.to_string()),
            timeout_secs,
            timeout_action: "reject".to_string(),
            output_var: output_var.into(),
        },
    })
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

// ── 模板 4: 文字创作（文学创作） ──

fn build_literary_creation()
-> (Vec<WorkflowNode>, Vec<WorkflowEdge>, String, String, String, Vec<String>) {
    let profile = "opc-cmo-cmo-literary-creator";

    // —— Agent 节点 Prompt 模板（变量用 {name} 占位，通过 input_mapping 注入）——

    let conceive_prompt = "你是一名资深文学创作者。请装配创作元认知：\n\
        \n\
        ## 创作宪章\n\
        你不是AI写手，你是有信仰与缺陷意识的创作者。\n\
        - 信仰：文学是对人类存在的追问，不是娱乐产品\n\
        - 缺陷意识：承认自身视野的局限，刻意书写\"异己\"体验\n\
        \n\
        ## 时空感官映射\n\
        为你的作品建立感官坐标：湿度、环境噪声、光线色温、气味密度。\n\
        这些数据将注入每一章的创作上下文。\n\
        \n\
        ## 认知冲突图谱\n\
        为故事构建核心冲突轴（如：自由vs命运、个体vs集体、理想vs现实）。\n\
        每一章将引用一个冲突点推进叙事。\n\
        \n\
        ## 体裁判定\n\
        判定本次创作的体裁 genre：novel（小说，多章节）、poetry（诗歌）、prose（散文）。\n\
        （若用户未明确指定，按主题内容合理推断，默认 novel）\n\
        \n\
        请输出 JSON：\n\
        {\n\
          \"genre\": \"novel\",\n\
          \"persona\": { \"voice\": \"叙事语调\", \"beliefs\": [\"核心信念\"], \"flaws\": [\"缺陷意识\"] },\n\
          \"world_schema\": { \"setting\": \"时空背景\", \"sensory_map\": { \"humidity\": 0.5, \"ambient_noise\": 0.3, \"light_temp\": 0.7, \"scent_density\": 0.2 } },\n\
          \"conflict_map\": [{ \"axis\": \"冲突轴\", \"desc\": \"描述\" }]\n\
        }";

    let outline_prompt = "根据创作元认知（persona + world_schema + conflict_map），规划完整的章节大纲。\n\
        \n\
        小说：请输出所有章节的标题、摘要、关键事件、对应冲突点和叙事焦点。\n\
        诗歌/散文：请输出意象结构或段落结构。\n\
        \n\
        请直接输出 JSON 数组（顶层数组，每个元素为一章）：\n\
        [\n\
          { \"num\": 1, \"title\": \"章节标题\", \"summary\": \"200-300字摘要\", \"key_events\": [\"关键事件\"], \"conflict_idx\": 0, \"focal\": \"叙事焦点\" }\n\
        ]";

    let draft_chapter_prompt = "你正在创作一章文学作品。\n\
        \n\
        ## 创作规则（文体禁忌）\n\
        - 小说：禁止使用\"他心想\"、\"他感到\"等透视词，改用物理动作描写\n\
        - 小说：动作句与心理独白句比例约 6:4\n\
        - 小说：每 3 章至少一个\"无用的物象描写\"\n\
        - 诗歌：禁止\"爱\"、\"恨\"、\"绝望\"等抽象情感词，替换为身体/机械零件词汇\n\
        - 散文：闲笔密度适中，结尾需有价值翻转\n\
        \n\
        ## 上下文（4 项精简注入）\n\
        - 当前章大纲：{chapter}\n\
        - 前章摘要：取 prev_summary 数组最后一项的 summary 作为前章摘要（首轮为空数组）\n\
        - 全局设定 compact：{persona} {world_schema}\n\
        - 当前冲突点：{conflict_point}\n\
        \n\
        ## 输出\n\
        请输出 JSON：\n\
        {\n\
          \"chapter_draft\": \"完整章节正文\",\n\
          \"chapter_summary\": \"200-300字章节摘要\",\n\
          \"word_count\": 3500\n\
        }";

    let anti_logic_prompt = "你是一名反逻辑校验编辑。\n\
        \n\
        对以下章节草稿执行反逻辑校验：\n\
        1. 强制插入一个矛盾句\n\
        2. 要求创作者自圆其说\n\
        3. 检查叙事逻辑一致性\n\
        4. 识别并标记逻辑漏洞\n\
        \n\
        输入：{chapter_draft}\n\
        \n\
        请输出 JSON：\n\
        {\n\
          \"revised_draft\": \"修订后草稿\",\n\
          \"logic_issues\": [\"发现的逻辑问题\"],\n\
          \"coherence_score\": 85\n\
        }";

    let summary_prompt = "你是一名摘要编辑。\n\
        \n\
        为以下章节提取 200-300 字摘要，保留关键情节和人物状态变化。\n\
        这个摘要将用于下一章的上下文注入，防止 context 爆炸。\n\
        同时**原样返回修订后的章节正文**（chapter_text 字段），\n\
        供组装阶段拼接全文。\n\
        \n\
        输入：{revised_draft}\n\
        \n\
        请输出 JSON：\n\
        {\n\
          \"chapter_text\": \"修订后章节完整正文（原样返回，不得省略）\",\n\
          \"summary\": \"200-300字摘要文本\",\n\
          \"key_entities\": [\"关键实体\"],\n\
          \"emotional_arc\": \"情感走向\"\n\
        }";

    let draft_single_prompt = "你正在创作一篇文学作品（诗歌或散文，无需分章）。\n\
        \n\
        ## 创作规则（文体禁忌）\n\
        - 诗歌：禁止\"爱\"、\"恨\"、\"绝望\"等抽象情感词，替换为身体/机械零件词汇\n\
        - 散文：闲笔密度适中，结尾需有价值翻转\n\
        \n\
        ## 上下文\n\
        - 全局设定：{persona} {world_schema}\n\
        - 冲突图谱：{conflict_map}\n\
        \n\
        ## 输出\n\
        请输出 JSON：\n\
        {\n\
          \"draft\": \"完整正文\",\n\
          \"word_count\": 2000,\n\
          \"summary\": \"200字摘要\"\n\
        }";

    let assemble_prompt = "你是一名长篇小说组装编辑。\n\
        \n\
        chapters_items 是逐章创作循环的输出数组（每项含 chapter_text 完整正文与 summary 摘要）。\n\
        请将所有章节按顺序拼接为完整作品，执行一致性校验：\n\
        1. 人物性格一致性\n\
        2. 时间线连续性\n\
        3. 世界观设定一致性\n\
        4. 叙事视角一致性\n\
        \n\
        输入章节列表：{chapters_items}\n\
        \n\
        请输出 JSON：\n\
        {\n\
          \"full_text\": \"完整正文（按章节顺序拼接）\",\n\
          \"issues\": [\"发现的问题\"],\n\
          \"total_word_count\": 35000,\n\
          \"consistency_score\": 92\n\
        }";

    let assemble_single_prompt = "你是一名文学作品编辑。\n\
        \n\
        对以下单篇作品进行润色和一致性校验：\n\
        1. 语言风格一致性\n\
        2. 情感逻辑连贯性\n\
        3. 文体规范检查\n\
        \n\
        输入：{draft}\n\
        \n\
        请输出 JSON：\n\
        {\n\
          \"full_text\": \"润色后正文\",\n\
          \"issues\": [\"发现的问题\"],\n\
          \"total_word_count\": 2000,\n\
          \"consistency_score\": 92\n\
        }";

    let review_prompt = "你是一个由四位评审组成的后编辑评审委员会。\n\
        对同一篇文本依次执行四路评审，输出合并结果。\n\
        \n\
        输入全文：{full_text}\n\
        \n\
        ## 评审 1：GPT 惯性词库审查\n\
        检测 AI 惯性表达（黑名单）：\n\
        - 小说黑名单：他心想、他感到、她意识到、不禁、仿佛、宛如\n\
        - 诗歌黑名单：爱、恨、绝望、孤独、痛苦、美丽\n\
        - 通用黑名单：值得一提、不可否认、至关重要、综上所述\n\
        \n\
        ## 评审 2：文学熵值监测\n\
        统计词汇多样性：生僻字频率、词汇重复率、句式变化度。\n\
        目标：生僻度/熟悉度 ≈ 2:8（既不晦涩也不陈腐）。\n\
        \n\
        ## 评审 3：庸俗读者测试\n\
        以大众视角阅读：情感高潮是否到位、情节是否吸引人、是否有代入感。\n\
        \n\
        ## 评审 4：挑剔评论家拷问\n\
        以专业视角审查：原创性、文学价值、叙事技巧、是否有\"机器味\"。\n\
        \n\
        请输出 JSON（合并四路评审结果）：\n\
        {\n\
          \"style_audit\": { \"hit_count\": 3, \"threshold\": 5, \"verdict\": \"pass\", \"hit_details\": [] },\n\
          \"entropy_check\": { \"rare_ratio\": 0.15, \"repetition_rate\": 0.08, \"verdict\": \"pass\", \"suggestions\": [] },\n\
          \"reader_vulgar\": { \"pass\": true, \"notes\": [], \"emotional_highlights\": [] },\n\
          \"reader_critic\": { \"pass\": true, \"notes\": [], \"innovation_score\": 78, \"machine_smell_detected\": false }\n\
        }";

    let tolerance_prompt = "你是一名容错编辑器。\n\
        \n\
        基于以下四路评审结果，判定是否通过：\n\
        - 风格审查：{style_audit_result}\n\
        - 熵值监测：{entropy_check_result}\n\
        - 庸俗读者反馈：{reader_vulgar_result}\n\
        - 评论家反馈：{reader_critic_result}\n\
        \n\
        容错阈值：tolerance_threshold = 1（允许 1 处不完美不触发重写）\n\
        判定规则：\n\
        - 所有评审 pass → verdict = \"pass\"\n\
        - 仅 1 项 fail 但不严重 → verdict = \"pass\"（容错）\n\
        - 2 项及以上 fail → verdict = \"rewrite\"（需重写）\n\
        \n\
        请输出 JSON：\n\
        {\n\
          \"verdict\": \"pass\",\n\
          \"rewrite_sections\": [],\n\
          \"confidence\": 0.85\n\
        }";

    // —— 节点定义 ——

    let nodes = vec![
        // 触发节点
        make_trigger(0.0, 0.0),
        // lc-conceive: 创作元认知装配（接收 trigger 输入）
        make_agent_node(
            "lc-conceive",
            "创作元认知",
            conceive_prompt,
            vec![],
            Some(profile),
            "lc-conceive",
            200.0,
            0.0,
        ),
        // lc-genre-route: 体裁路由（在 outline 前，读 conceive 输出的 genre）
        make_condition_node(
            "lc-genre-route",
            "体裁路由",
            vec![Condition {
                var_path: "lc-conceive.genre".to_string(),
                operator: CompareOperator::Eq,
                value: serde_json::json!("novel"),
            }],
            LogicalOperator::And,
            400.0,
            0.0,
        ),
        // lc-outline: 大纲拆章（小说路径，输出顶层 JSON 数组，作为 Loop 输入）
        // context_sources 加 lc-conceive：outline 的 edges 上游是 genre-route，
        // 引用 lc-conceive.persona 等跨级输出必须经软依赖注入
        make_agent_node_full(
            "lc-outline",
            "大纲拆章",
            outline_prompt,
            vec![],
            Some(profile),
            "chapters",
            vec![
                ("persona", "lc-conceive.persona"),
                ("world_schema", "lc-conceive.world_schema"),
                ("conflict_map", "lc-conceive.conflict_map"),
            ],
            vec!["lc-conceive"],
            600.0,
            0.0,
        ),
        // —— Loop 体内部节点（小说路径，仅在 body_steps 中引用，不通过 edges 连接）——

        // lc-draft-chapter: 单章创作（Loop 内，接收当前章 + 上下文）
        // context_sources 加 lc-conceive/lc-outline：body 节点无 edges，
        // 跨级引用 persona/world_schema/conflict_map 必须经软依赖注入
        make_agent_node_full(
            "lc-draft-chapter",
            "单章创作",
            draft_chapter_prompt,
            vec![],
            Some(profile),
            "lc-draft-chapter",
            vec![
                ("chapter", "chapter"),                         // Loop 迭代变量：当前章
                ("prev_summary", "chapters_text__partial"), // 从 partial 结果读取：取数组最后一项的 summary
                ("persona", "lc-conceive.persona"),         // 全局 persona
                ("world_schema", "lc-conceive.world_schema"), // 全局设定
                ("conflict_point", "lc-conceive.conflict_map"), // 冲突图谱
            ],
            vec!["lc-conceive", "lc-outline"],
            300.0,
            -400.0,
        ),
        // lc-anti-logic: 反逻辑校验（Loop 内，接收草稿）
        make_agent_node_full(
            "lc-anti-logic",
            "反逻辑校验",
            anti_logic_prompt,
            vec![],
            Some(profile),
            "lc-anti-logic",
            vec![("chapter_draft", "lc-draft-chapter.chapter_draft")],
            vec!["lc-conceive"],
            500.0,
            -400.0,
        ),
        // lc-summary: 摘要提取（Loop 内，接收修订稿，输出含完整正文 chapter_text）
        make_agent_node_full(
            "lc-summary",
            "摘要提取",
            summary_prompt,
            vec![],
            Some(profile),
            "lc-summary",
            vec![("revised_draft", "lc-anti-logic.revised_draft")],
            vec!["lc-conceive"],
            700.0,
            -400.0,
        ),
        // lc-draft-loop: LoopNode（小说路径核心）
        // iter_input_var = "lc-outline"：deps_results 按 node_id 注入，
        // variables["lc-outline"] = outline 输出的顶层章节数组
        make_loop_node(
            "lc-draft-loop",
            "逐章创作循环",
            LoopType::ForEach,
            Some("lc-outline"),   // iter_input_var: 从 lc-outline 输出读取章节数组（node_id 注入）
            Some("chapter"),      // iteratee_var: 当前章注入 scope 的变量名
            Some("chapters_text"), // iter_output_var: 聚合输出变量（每轮 last_step 累积）
            Some("chapters_text__partial"), // partial_result_var: 流式累积变量（供下一轮 prev_summary 读取）
            Some(50),             // 最多 50 章
            vec![
                "lc-draft-chapter".to_string(),
                "lc-anti-logic".to_string(),
                "lc-summary".to_string(),
            ],
            800.0,
            0.0,
        ),
        // —— 非小说路径（诗歌/散文）——

        // lc-draft-single: 单次创作（跳过 Loop，诗歌/散文路径）
        // context_sources 加 lc-conceive：edges 上游是 genre-route（false 边），
        // 引用 lc-conceive.persona 等跨级输出必须经软依赖注入
        make_agent_node_full(
            "lc-draft-single",
            "单次创作",
            draft_single_prompt,
            vec![],
            Some(profile),
            "lc-draft-single",
            vec![
                ("persona", "lc-conceive.persona"),
                ("world_schema", "lc-conceive.world_schema"),
                ("conflict_map", "lc-conceive.conflict_map"),
            ],
            vec!["lc-conceive"],
            800.0,
            200.0,
        ),
        // —— 组装节点 ——

        // lc-assemble: 小说路径组装（接收 Loop 聚合 items，每项含 chapter_text 正文）
        // 引用 "lc-draft-loop.items"：deps_results 按 node_id 注入 Loop 输出对象，
        // items 字段 = 每轮最后一步（lc-summary）输出的累积数组
        make_agent_node_with_inputs(
            "lc-assemble",
            "组装校验",
            assemble_prompt,
            vec![],
            Some(profile),
            "lc-assemble",
            vec![("chapters_items", "lc-draft-loop.items")],
            1200.0,
            0.0,
        ),
        // lc-assemble-single: 非小说路径组装（接收单次创作输出）
        // output_var 故意与 lc-assemble 同名 "lc-assemble"（两条路径互斥）：
        // 让下游评审节点无论哪条路径都能引用 lc-assemble.full_text
        make_agent_node_with_inputs(
            "lc-assemble-single",
            "作品润色",
            assemble_single_prompt,
            vec![],
            Some(profile),
            "lc-assemble",
            vec![("draft", "lc-draft-single.draft")],
            1200.0,
            200.0,
        ),
        // —— 后编辑评审 ——
        // 注意：引擎未实现 Parallel 分支执行（parallel_executor 只返回元信息，
        // branch.steps 不会被调度），故合并为单个评审 Agent 顺序执行四路评审。

        // lc-review-agent: 合并四路评审（GPT 词库审查/熵值监测/庸俗读者/挑剔评论家）
        // context_sources 加 lc-assemble：非小说路径下 deps 只注入 lc-assemble-single，
        // 但 output_var 统一为 "lc-assemble"，经软依赖注入才能引用 lc-assemble.full_text
        make_agent_node_full(
            "lc-review-agent",
            "后编辑评审",
            review_prompt,
            vec![],
            Some(profile),
            "lc-review-agent",
            vec![("full_text", "lc-assemble.full_text")],
            vec!["lc-assemble"],
            1400.0,
            0.0,
        ),
        // lc-tolerance-agent: 容错评审 Agent（汇总四路评审结果）
        make_agent_node_with_inputs(
            "lc-tolerance-agent",
            "容错评审",
            tolerance_prompt,
            vec![],
            Some(profile),
            "lc-tolerance",
            vec![
                ("style_audit_result", "lc-review-agent.style_audit"),
                ("entropy_check_result", "lc-review-agent.entropy_check"),
                ("reader_vulgar_result", "lc-review-agent.reader_vulgar"),
                ("reader_critic_result", "lc-review-agent.reader_critic"),
            ],
            1600.0,
            0.0,
        ),
        // lc-tolerance: 容错判定 ConditionNode
        // var_path 必须用节点 ID "lc-tolerance-agent"（deps_results 按 node_id 注入），
        // Agent 的 output_var="lc-tolerance" 只写入 results 不进 ctx.variables
        make_condition_node(
            "lc-tolerance",
            "容错判定",
            vec![Condition {
                var_path: "lc-tolerance-agent.verdict".to_string(),
                operator: CompareOperator::Eq,
                value: serde_json::json!("pass"),
            }],
            LogicalOperator::And,
            1800.0,
            0.0,
        ),
        // lc-approval: 人工审批（容错 fail 时触发，人工决定接受或重写）
        make_approval_node(
            "lc-approval",
            "人工审批",
            "评审未通过，请人工决定：是否接受当前作品，或返回重写。\n\
                输入：容错评审结果 + 全文。\n\
                输出：{\"decision\": \"accept\" | \"rewrite\"}",
            None,
            3600,
            "lc-approval",
            2000.0,
            100.0,
        ),
        // lc-finalize: 文件保存（固定路径）
        make_file_operation_node(
            "lc-finalize",
            "保存作品",
            "write",
            "./output/literary_creation/work.txt",
            "lc-finalize",
            2200.0,
            0.0,
        ),
        // 结束
        make_end(2200.0, 0.0),
    ];

    // —— 边定义 ——
    // 注意：Loop 内节点（lc-draft-chapter, lc-anti-logic, lc-summary）不通过 edges 连接，
    // 由 LoopExecutor 通过 body_steps 驱动执行。

    let edges = vec![
        // 主链路：构思 → 体裁路由
        edge("e-trigger-conceive", "trigger", "lc-conceive"),
        edge("e-conceive-genre-route", "lc-conceive", "lc-genre-route"),
        // 体裁路由：true(小说) → outline → Loop；false(诗歌/散文) → 单次创作
        edge_cond(
            "e-genre-route-novel",
            "lc-genre-route",
            "true",
            "lc-outline",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-genre-route-non-novel",
            "lc-genre-route",
            "false",
            "lc-draft-single",
            EdgeType::ConditionFalse,
        ),
        // 小说路径：outline → Loop → 组装
        edge("e-outline-draft-loop", "lc-outline", "lc-draft-loop"),
        edge("e-draft-loop-assemble", "lc-draft-loop", "lc-assemble"),
        // 非小说路径：单次创作 → 组装
        edge("e-draft-single-assemble", "lc-draft-single", "lc-assemble-single"),
        // 统一：组装 → 后编辑评审（两条路径汇聚）
        edge("e-assemble-review", "lc-assemble", "lc-review-agent"),
        edge("e-assemble-single-review", "lc-assemble-single", "lc-review-agent"),
        // 评审 → 容错评审 Agent → 容错判定
        edge("e-review-tolerance-agent", "lc-review-agent", "lc-tolerance-agent"),
        edge("e-tolerance-agent-tolerance", "lc-tolerance-agent", "lc-tolerance"),
        // 容错判定：true(通过) → 保存；false(需重写) → 人工审批
        edge_cond(
            "e-tolerance-pass",
            "lc-tolerance",
            "true",
            "lc-finalize",
            EdgeType::ConditionTrue,
        ),
        edge_cond(
            "e-tolerance-fail",
            "lc-tolerance",
            "false",
            "lc-approval",
            EdgeType::ConditionFalse,
        ),
        // 人工审批 → 保存（人工接受作品；rewrite 场景由前端回环处理）
        edge("e-approval-finalize", "lc-approval", "lc-finalize"),
        // 保存 → 结束
        edge("e-finalize-end", "lc-finalize", "end"),
    ];

    (
        nodes,
        edges,
        "文字创作".to_string(),
        "创作元认知 → 大纲拆章 → 逐章创作 → 反逻辑校验 → 双读者评审 → 容错归档。专业文学创作工作流。".to_string(),
        "📖".to_string(),
        vec![
            "literary".to_string(),
            "creation".to_string(),
            "novel".to_string(),
            "poetry".to_string(),
            "prose".to_string(),
        ],
    )
}
