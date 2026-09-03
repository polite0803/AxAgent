// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::workflow as workflow_err;
use axagent_agent_macro::agent_command;
use axagent_dao::repo::workflow_template as db_repo;
use axagent_dao::workflow_conversions::workflow_template_response_from_model;
use axagent_harness::CapabilityIndexer;
use axagent_harness::capability::CapabilityPassport;
use axagent_harness::workflow_types::*;
use axagent_runtime::work_engine::node_executor_trait::node_type_name;
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, Set, TransactionTrait};
use serde::Deserialize;
use tauri::State;

/// 判断模板是否为系统模板（领域 = system）。
///
/// 判定规则：route_path 的 L1 段为 `system`（即 `route_path` 以 `/system/` 开头）。
/// 认知编排器由 cognitive_router_init 初始化时设置 `route_path: /system/cognitive_router/{id}`，
/// route_path 是原生字符串列，不依赖 JSON 解析或 tags，稳定可靠。
///
/// 与前端 TemplateList.getTemplateDomain 的领域解析口径一致：前端从 route_path 拆 L1 段做业务域分组，
/// system 域模板被排除在业务域之外；后端以同一维度判定系统模板，前后端口径对齐。
fn is_cognitive_router_template(model: &axagent_entities::workflow_template::Model) -> bool {
    model.route_path.as_deref().map(|p| p.starts_with("/system/")).unwrap_or(false)
}

fn model_to_active_model(
    template: &WorkflowTemplateData,
) -> axagent_entities::workflow_template::ActiveModel {
    let now = chrono::Utc::now().timestamp_millis();

    axagent_entities::workflow_template::ActiveModel {
        id: Set(template.id.clone()),
        name: Set(template.name.clone()),
        description: Set(template.description.clone()),
        icon: Set(template.icon.clone()),
        tags: Set(Some(serde_json::to_string(&template.tags).unwrap_or_default())),
        version: Set(template.version),
        is_preset: Set(template.is_preset),
        is_editable: Set(template.is_editable),
        is_public: Set(template.is_public),
        trigger_config: Set(template
            .trigger_config
            .as_ref()
            .and_then(|c| serde_json::to_string(c).ok())),
        nodes: Set(serde_json::to_string(&template.nodes).unwrap_or_default()),
        edges: Set(serde_json::to_string(&template.edges).unwrap_or_default()),
        input_schema: Set(template
            .input_schema
            .as_ref()
            .and_then(|s| serde_json::to_string(s).ok())),
        output_schema: Set(template
            .output_schema
            .as_ref()
            .and_then(|s| serde_json::to_string(s).ok())),
        variables: Set(Some(serde_json::to_string(&template.variables).unwrap_or_default())),
        error_config: Set(template
            .error_config
            .as_ref()
            .and_then(|e| serde_json::to_string(e).ok())),
        composite_source: Set(None),
        tool_defs: Set(if template.tool_defs.is_empty() {
            None
        } else {
            serde_json::to_string(&template.tool_defs).ok()
        }),
        mission_hash: Set(template.mission_hash.clone()),
        cluster_id: Set(template.cluster_id.clone()),
        route_path: Set(template.route_path.clone()),
        created_at: Set(template.created_at),
        updated_at: Set(now),
    }
}

/// 把模板护照同步进能力索引（模板写入口的统一收口）。
///
/// 能力护照只在启动时由 `register_all_capabilities` 从 `workflow_templates` 表
/// 全量重建一次；运行期新增/改名的模板若不回灌索引，本会话内路由候选集里
/// 就没有它——固化的动态工作流会变成「孤儿」，要等重启才可路由。
///
/// 索引依赖本地 embedding 服务，失败仅 warn：模板已落库，重启后会被重建。
pub(crate) async fn sync_template_passport(state: &AppState, template: &WorkflowTemplateData) {
    let passport = template.to_passport_dto();
    if let Err(e) = state.capability_indexer.index_passport(&passport).await {
        tracing::warn!(
            target: "axagent.capability.index",
            capability_id = %passport.capability_id,
            error = %e,
            "工作流模板已落库但能力索引失败（重启后自动重建）"
        );
    }
}

/// 按模板 ID 从库里读回最新模型，再回灌能力索引。
///
/// 适用于调用方手上只有 entity `ActiveModel`、拿不到完整 `WorkflowTemplateData`
/// 的写入口（技能转工作流、技能分解、AI 应用类修改等）。统一走读回再派生，
/// 避免因调用方只填了部分字段而把残缺护照写进索引。
pub(crate) async fn sync_template_index_by_id(state: &AppState, template_id: &str) {
    let db = state.harness.db();
    match db_repo::get_workflow_template(db, template_id).await {
        Ok(Some(model)) => {
            sync_template_passport(state, &db_repo::template_model_to_data(&model)).await;
        },
        Ok(None) => {},
        Err(e) => {
            tracing::warn!(
                target: "axagent.capability.index",
                template_id = %template_id,
                error = %e,
                "读回模板失败，能力索引未同步（重启后自动重建）"
            );
        },
    }
}

/// 从能力索引中移除模板护照（删除模板时调用，避免脏护照残留到重启）。
async fn remove_template_passport(state: &AppState, template_id: &str) {
    let capability_id = format!("workflow:{template_id}");
    if let Err(e) = state.capability_indexer.remove_index(&capability_id).await {
        tracing::warn!(
            target: "axagent.capability.index",
            capability_id = %capability_id,
            error = %e,
            "删除工作流模板后清理能力索引失败（重启后自动重建）"
        );
    }
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "列出工作流模板")]
#[tauri::command]
pub async fn list_workflow_templates(
    state: State<'_, AppState>,
    is_preset: Option<bool>,
    include_system: Option<bool>,
) -> Result<Vec<WorkflowTemplateResponse>, String> {
    let db = state.harness.db();
    let templates = db_repo::list_workflow_templates(db, is_preset).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 过滤认知编排器等系统模板：业务模板页对系统模板不可见（结构隔离）。
    // include_system=true（系统模板页）时返回系统模板，供工作流编辑器查看/编辑。
    let include_system = include_system.unwrap_or(false);
    Ok(templates
        .into_iter()
        .filter(|t| include_system || !is_cognitive_router_template(t))
        .map(workflow_template_response_from_model)
        .collect())
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "列出系统模板（认知编排器等）")]
#[tauri::command]
pub async fn list_system_templates(
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowTemplateResponse>, String> {
    // 系统模板页专用：只返回 is_preset + cognitive_router 标签的模板（认知编排器等）。
    // 不依赖 include_system 参数传递（该参数在部分调用路径上不可靠），后端权威过滤。
    let db = state.harness.db();
    let templates = db_repo::list_workflow_templates(db, Some(true)).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let total = templates.len();
    let cognitive_ids: Vec<_> = templates
        .iter()
        .filter(|&t| is_cognitive_router_template(t))
        .map(|t| t.id.clone())
        .collect();
    tracing::info!(
        "[list_system_templates] preset模板总数={}, cognitive_router匹配数={}, ids={:?}",
        total,
        cognitive_ids.len(),
        cognitive_ids
    );

    Ok(templates
        .into_iter()
        .filter(is_cognitive_router_template)
        .map(workflow_template_response_from_model)
        .collect())
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "获取单个工作流模板详情")]
#[tauri::command]
pub async fn get_workflow_template(
    state: State<'_, AppState>,
    id: String,
    include_system: Option<bool>,
) -> Result<Option<WorkflowTemplateResponse>, String> {
    let db = state.harness.db();
    tracing::warn!("[get_workflow_template] 开始: id={id}, include_system={:?}", include_system);

    let template = db_repo::get_workflow_template(db, &id).await.map_err(|e| {
        tracing::error!("[get_workflow_template] 数据库查询失败: {e}");
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // get_single 按 id 精准读取，不做领域过滤（list 命令才有过滤语义）。
    // include_system 参数保留签名兼容，但不再影响结果。
    let result = template.map(workflow_template_response_from_model);
    if let Some(ref r) = result {
        tracing::info!(
            "[get_workflow_template] id={} include_system={:?} -> nodes={} edges={}",
            id,
            include_system,
            r.nodes.len(),
            r.edges.len()
        );
    } else {
        tracing::warn!("[get_workflow_template] id={} -> not found", id);
    }
    Ok(result)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "创建新工作流模板")]
#[tauri::command]
pub async fn create_workflow_template(
    state: State<'_, AppState>,
    input: WorkflowTemplateInput,
) -> Result<String, String> {
    let db = state.harness.db();

    // 节点组成相似性检查
    let similar = find_similar_workflows(db, &input.nodes).await?;
    if !similar.is_empty() {
        tracing::info!(
            "[workflow_template] 新建模板 '{}' 与 {} 个已有模板节点组成相似: {:?}",
            input.name,
            similar.len(),
            similar.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    let now = chrono::Utc::now().timestamp_millis();

    let template = WorkflowTemplateData {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        description: input.description,
        icon: input.icon,
        cluster_id: input.cluster_id.clone(),
        route_path: input.route_path.clone(),
        tags: input.tags,
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        visibility: axagent_harness::capability::Visibility::Public,
        trigger_config: input.trigger_config,
        nodes: input.nodes,
        edges: input.edges,
        input_schema: input.input_schema,
        output_schema: input.output_schema,
        variables: input.variables,
        error_config: input.error_config,
        tool_defs: input.tool_defs.unwrap_or_default(),
        error_workflow_id: None,
        mission_hash: input.mission_hash,
        created_at: now,
        updated_at: now,
    };

    let active_model = model_to_active_model(&template);
    db_repo::insert_workflow_template(db, active_model).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    state.work_engine.precompile_tool_defs(&template.id, &template.tool_defs).await;

    // 回灌能力索引：否则新建的模板在本会话内不可路由（护照只在启动时全量重建）
    sync_template_passport(&state, &template).await;

    // 2.7 P1:同步运行时触发器 — DB 已持久化 trigger_config,此处把同一份配置
    // 注册到 TriggerManager,使 Schedule/Webhook/Event 触发器立即生效。
    // 失败仅 warn 日志,不阻断命令返回(下次启动恢复时会重新注册)。
    crate::init::trigger_recovery::sync_workflow_trigger(
        &state.work_engine.trigger_manager,
        &template.id,
        template.trigger_config.as_ref(),
    )
    .await;

    Ok(template.id)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "更新工作流模板")]
#[tauri::command]
pub async fn update_workflow_template(
    state: State<'_, AppState>,
    id: String,
    mut input: WorkflowTemplateInput,
) -> Result<bool, String> {
    let db = state.harness.db();

    // 保存前提取 tool_defs（确保移动后仍可引用）
    let tool_defs = input.tool_defs.take();
    // 2.7 P1:同步提取 trigger_config — 持久化后还要用于运行时触发器同步。
    // clone 一份给 sync 用,原值 move 到 db_repo。
    let trigger_config = input.trigger_config.take();
    let trigger_config_for_sync = trigger_config.clone();

    // stock-analysis 模板热更新（vendor 状态注入已随 stock_workflow 模块删除）
    // NOTE: stock_monitor 和 cross_stock_aggregator 已随下游功能清理移除
    // if id == "stock-analysis" {
    //     // P1-1: 热更新 RealtimeMonitor 的告警冷却 + 轮询间隔
    //     // P2: 热更新 CrossStockSignalAggregator 配置
    //     // ... (原实现依赖已删除的字段)
    // }

    let updated = db_repo::update_workflow_template(
        db,
        &id,
        input.name,
        input.description,
        input.icon,
        input.tags,
        trigger_config,
        input.nodes,
        input.edges,
        input.input_schema,
        input.output_schema,
        input.variables,
        input.error_config,
        tool_defs.clone(),
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 保存后立即预编译 Rhai 工具，Agent 节点可即时引用
    if let Some(ref tds) = tool_defs {
        state.work_engine.precompile_tool_defs(&id, tds).await;
    }

    // 2.7 P1:同步运行时触发器 — DB 已更新 trigger_config,此处把运行时
    // TriggerManager 的注册状态同步为新配置(先清理旧注册,再按新配置注册)。
    // 失败仅 warn 日志,不阻断命令返回。
    crate::init::trigger_recovery::sync_workflow_trigger(
        &state.work_engine.trigger_manager,
        &id,
        trigger_config_for_sync.as_ref(),
    )
    .await;

    // 索引同步：update 只接收部分字段（tool_defs/trigger_config 已被 take），
    // 直接从 DB 读回最新模型再派生护照，避免漏字段造成索引与库不一致。
    if let Ok(Some(model)) = db_repo::get_workflow_template(db, &id).await {
        sync_template_passport(&state, &db_repo::template_model_to_data(&model)).await;
    }

    Ok(updated)
}

#[agent_command(domain = workflow, safety = Dangerous, call_mode = StateInput, description = "删除工作流模板")]
#[tauri::command]
pub async fn delete_workflow_template(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let db = state.harness.db();

    // 系统模板（认知编排器等）禁止用户删除
    if let Some(t) = db_repo::get_workflow_template(db, &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })? {
        if is_cognitive_router_template(&t) {
            return Err(ErrorResponse::err_with_detail(
                workflow_err::SYSTEM_TEMPLATE_PROTECTED,
                "System template is protected",
            ));
        }
    }

    let deleted = db_repo::delete_workflow_template(db, &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 2.7 P1:清理运行时触发器注册状态 — DB 已删除模板,此处把 TriggerManager
    // 中残留的 schedule/webhook 注册一并清理(event 订阅可能残留,见文档注释)。
    crate::init::trigger_recovery::unregister_workflow_triggers(
        &state.work_engine.trigger_manager,
        &id,
    )
    .await;

    // 清理能力索引：不清理则脏护照会残留到下次启动，路由仍会命中已删除的模板
    remove_template_passport(&state, &id).await;

    Ok(deleted)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "复制工作流模板")]
#[tauri::command]
pub async fn duplicate_workflow_template(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let db = state.harness.db();

    let template = db_repo::get_workflow_template(db, &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let template = template.ok_or_else(|| {
        ErrorResponse::err_with_detail(workflow_err::NOT_FOUND, "Template not found")
    })?;

    // 系统模板（认知编排器等）禁止复制
    if is_cognitive_router_template(&template) {
        return Err(ErrorResponse::err_with_detail(
            workflow_err::SYSTEM_TEMPLATE_PROTECTED,
            "System template is protected",
        ));
    }

    let response = workflow_template_response_from_model(template);

    let now = chrono::Utc::now().timestamp_millis();
    let new_template = WorkflowTemplateData {
        id: uuid::Uuid::new_v4().to_string(),
        name: format!("{} (Copy)", response.name),
        description: response.description,
        icon: response.icon,
        cluster_id: response.cluster_id.clone(),
        route_path: response.route_path.clone(),
        tags: response.tags,
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        visibility: Default::default(),
        trigger_config: response.trigger_config,
        nodes: response.nodes,
        edges: response.edges,
        input_schema: response.input_schema,
        output_schema: response.output_schema,
        variables: response.variables,
        error_config: response.error_config,
        tool_defs: vec![],
        error_workflow_id: None,
        mission_hash: None,
        created_at: now,
        updated_at: now,
    };

    let active_model = model_to_active_model(&new_template);
    db_repo::insert_workflow_template(db, active_model).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 回灌能力索引：副本是新模板（新 ID），不索引同样本会话内不可路由
    sync_template_passport(&state, &new_template).await;

    Ok(new_template.id)
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateOnly, description = "初始化预置工作流模板")]
#[tauri::command]
pub async fn seed_preset_templates(state: State<'_, AppState>) -> Result<usize, String> {
    use axagent_kit::preset_templates::{
        convert_preset_to_workflow_template, get_preset_templates,
    };

    let db = state.harness.db();
    let presets = get_preset_templates();

    let mut items = Vec::with_capacity(presets.len());
    for preset in &presets {
        let mut template = convert_preset_to_workflow_template(preset);
        template.is_preset = true;
        template.is_editable = true;
        template.is_public = true;
        items.push(template);
    }

    // 索引回灌需要保留一份副本：items 会被 db 层消费掉。
    // 手动 seed 发生在运行期（启动期的全量重建已过），不回灌则本会话内不可路由。
    let index_items = items.clone();
    db_repo::seed_preset_templates(db, items).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    for template in &index_items {
        sync_template_passport(&state, template).await;
    }

    Ok(presets.len())
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "获取模板版本列表")]
#[tauri::command]
pub async fn get_template_versions(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<i32>, String> {
    let db = state.harness.db();

    // 系统模板（认知编排器等）对用户不可见：版本列表返回空
    if let Some(t) = db_repo::get_workflow_template(db, &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })? {
        if is_cognitive_router_template(&t) {
            return Ok(Vec::new());
        }
    }

    let versions = db_repo::get_template_versions(db, &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(versions)
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "获取指定版本的模板")]
#[tauri::command]
pub async fn get_template_by_version(
    state: State<'_, AppState>,
    id: String,
    version: i32,
) -> Result<Option<WorkflowTemplateResponse>, String> {
    let db = state.harness.db();

    // 系统模板（认知编排器等）对用户不可见：返回 None
    if let Some(t) = db_repo::get_workflow_template(db, &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })? {
        if is_cognitive_router_template(&t) {
            return Ok(None);
        }
    }

    let template = db_repo::get_template_by_version(db, &id, version).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(template.map(workflow_template_response_from_model))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateWorkflowInput {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "校验工作流模板结构")]
#[tauri::command]
pub async fn validate_workflow_template(
    _state: State<'_, AppState>,
    input: ValidateWorkflowInput,
) -> Result<ValidationResult, String> {
    let nodes = input.nodes;
    let edges = input.edges;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let node_ids: std::collections::HashSet<String> = nodes
        .iter()
        .flat_map(|n| match n {
            WorkflowNode::Trigger(t) => Some(t.base.id.clone()),
            WorkflowNode::Agent(t) => Some(t.base.id.clone()),
            WorkflowNode::Llm(t) => Some(t.base.id.clone()),
            WorkflowNode::Condition(t) => Some(t.base.id.clone()),
            WorkflowNode::Parallel(t) => Some(t.base.id.clone()),
            WorkflowNode::Loop(t) => Some(t.base.id.clone()),
            WorkflowNode::Merge(t) => Some(t.base.id.clone()),
            WorkflowNode::Delay(t) => Some(t.base.id.clone()),
            WorkflowNode::Validation(t) => Some(t.base.id.clone()),
            WorkflowNode::Tool(t) => Some(t.base.id.clone()),
            WorkflowNode::Code(t) => Some(t.base.id.clone()),
            WorkflowNode::SubWorkflow(t) => Some(t.base.id.clone()),
            WorkflowNode::DocumentParser(t) => Some(t.base.id.clone()),
            WorkflowNode::VectorRetrieve(t) => Some(t.base.id.clone()),
            WorkflowNode::HttpRequest(t) => Some(t.base.id.clone()),
            WorkflowNode::Switch(t) => Some(t.base.id.clone()),
            WorkflowNode::DatabaseQuery(t) => Some(t.base.id.clone()),
            WorkflowNode::Notification(t) => Some(t.base.id.clone()),
            WorkflowNode::Approval(t) => Some(t.base.id.clone()),
            WorkflowNode::FileOperation(t) => Some(t.base.id.clone()),
            WorkflowNode::DataTransformer(t) => Some(t.base.id.clone()),
            WorkflowNode::WebhookSend(t) => Some(t.base.id.clone()),
            WorkflowNode::Logging(t) => Some(t.base.id.clone()),
            WorkflowNode::LlmClassifier(t) => Some(t.base.id.clone()),
            WorkflowNode::Aggregator(t) => Some(t.base.id.clone()),
            WorkflowNode::Email(t) => Some(t.base.id.clone()),
            WorkflowNode::Debate(t) => Some(t.base.id.clone()),
            WorkflowNode::Swarm(t) => Some(t.base.id.clone()),
            WorkflowNode::MultiAgent(t) => Some(t.base.id.clone()),
            WorkflowNode::Storage(t) => Some(t.base.id.clone()),
            WorkflowNode::WorkflowRef(t) => Some(t.base.id.clone()),
            WorkflowNode::End(t) => Some(t.base.id.clone()),
        })
        .collect();

    if nodes.is_empty() {
        errors.push(ValidationError {
            error_type: "empty_workflow".to_string(),
            node_id: None,
            message: "Workflow must have at least one node".to_string(),
            suggestion: Some("Add a trigger node to start the workflow".to_string()),
        });
    }

    let trigger_count = nodes.iter().filter(|n| matches!(n, WorkflowNode::Trigger(_))).count();
    if trigger_count == 0 {
        errors.push(ValidationError {
            error_type: "missing_trigger".to_string(),
            node_id: None,
            message: "Workflow must have at least one trigger node".to_string(),
            suggestion: Some(
                "Add a trigger node (manual, schedule, webhook, or event)".to_string(),
            ),
        });
    } else if trigger_count > 1 {
        warnings.push(ValidationWarning {
            warning_type: "multiple_triggers".to_string(),
            node_id: None,
            message: format!("Workflow has {} trigger nodes. Consider using a single trigger with conditional branching.", trigger_count),
        });
    }

    let end_count = nodes.iter().filter(|n| matches!(n, WorkflowNode::End(_))).count();
    if end_count == 0 {
        warnings.push(ValidationWarning {
            warning_type: "missing_end".to_string(),
            node_id: None,
            message:
                "Workflow has no End node. Consider adding one for proper workflow termination."
                    .to_string(),
        });
    }

    for edge in &edges {
        if !node_ids.contains(&edge.source) {
            errors.push(ValidationError {
                error_type: "invalid_edge_source".to_string(),
                node_id: Some(edge.id.clone()),
                message: format!(
                    "Edge '{}' references non-existent source node '{}'",
                    edge.id, edge.source
                ),
                suggestion: Some("Remove this edge or create the missing source node".to_string()),
            });
        }
        if !node_ids.contains(&edge.target) {
            errors.push(ValidationError {
                error_type: "invalid_edge_target".to_string(),
                node_id: Some(edge.id.clone()),
                message: format!(
                    "Edge '{}' references non-existent target node '{}'",
                    edge.id, edge.target
                ),
                suggestion: Some("Remove this edge or create the missing target node".to_string()),
            });
        }
    }

    let mut has_cycle = false;
    let mut visited = std::collections::HashSet::new();
    let mut rec_stack = std::collections::HashSet::new();
    let mut adjacency: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for edge in &edges {
        if edge.edge_type == EdgeType::LoopBack {
            continue;
        }
        adjacency.entry(edge.source.clone()).or_default().push(edge.target.clone());
    }

    fn dfs(
        node: &str,
        adjacency: &std::collections::HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        if let Some(neighbors) = adjacency.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if dfs(neighbor, adjacency, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }
        rec_stack.remove(node);
        false
    }

    for node_id in &node_ids {
        if !visited.contains(node_id) && dfs(node_id, &adjacency, &mut visited, &mut rec_stack) {
            has_cycle = true;
            break;
        }
    }

    if has_cycle {
        errors.push(ValidationError {
            error_type: "cyclic_dependency".to_string(),
            node_id: None,
            message: "Workflow contains cyclic dependencies".to_string(),
            suggestion: Some(
                "Remove loops in the workflow graph or use a Loop node for iteration".to_string(),
            ),
        });
    }

    // ── ConditionNode 专项校验 ──
    for node in &nodes {
        if let WorkflowNode::Condition(c) = node {
            // 1. 无 conditions 且未启用 LLM 路由
            if c.config.conditions.is_empty() && !c.config.judge_by_llm.unwrap_or(false) {
                errors.push(ValidationError {
                    error_type: "empty_conditions".to_string(),
                    node_id: Some(c.base.id.clone()),
                    message: format!(
                        "Condition node '{}' has no conditions and LLM routing is not enabled",
                        c.base.id
                    ),
                    suggestion: Some(
                        "Add at least one condition or enable LLM routing".to_string(),
                    ),
                });
            }

            // 2. 检查出边类型：condition 节点的出边必须是 conditionTrue/conditionFalse
            //    不允许 direct 类型出边
            for edge in &edges {
                if edge.source != c.base.id {
                    continue;
                }
                match &edge.edge_type {
                    EdgeType::ConditionTrue | EdgeType::ConditionFalse => {},
                    _ => {
                        warnings.push(ValidationWarning {
                            warning_type: "invalid_condition_edge".to_string(),
                            node_id: Some(c.base.id.clone()),
                            message: format!(
                                "Condition node '{}' has an edge with type '{:?}'. Condition edges should be 'conditionTrue' (✓) or 'conditionFalse' (✗).",
                                c.base.id, edge.edge_type
                            ),
                        });
                    },
                }
            }

            // 3. 检查是否缺少 conditionTrue 或 conditionFalse 出边
            let has_true = edges
                .iter()
                .any(|e| e.source == c.base.id && e.edge_type == EdgeType::ConditionTrue);
            let has_false = edges
                .iter()
                .any(|e| e.source == c.base.id && e.edge_type == EdgeType::ConditionFalse);
            if !has_true {
                warnings.push(ValidationWarning {
                    warning_type: "missing_true_branch".to_string(),
                    node_id: Some(c.base.id.clone()),
                    message: format!(
                        "Condition node '{}' is missing a 'true' branch edge (✓ handle)",
                        c.base.id
                    ),
                });
            }
            if !has_false {
                warnings.push(ValidationWarning {
                    warning_type: "missing_false_branch".to_string(),
                    node_id: Some(c.base.id.clone()),
                    message: format!(
                        "Condition node '{}' is missing a 'false' branch edge (✗ handle)",
                        c.base.id
                    ),
                });
            }
        }
    }

    // ── 节点配置字段校验 ──
    for node in &nodes {
        match node {
            WorkflowNode::Agent(a) => {
                if a.config.system_prompt.is_empty() {
                    warnings.push(ValidationWarning {
                        warning_type: "invalid_config".to_string(),
                        node_id: Some(a.base.id.clone()),
                        message: format!("Agent node '{}' has an empty system_prompt", a.base.id),
                    });
                }
            },
            WorkflowNode::Llm(l) => {
                if l.config.model.is_empty() {
                    warnings.push(ValidationWarning {
                        warning_type: "invalid_config".to_string(),
                        node_id: Some(l.base.id.clone()),
                        message: format!("LLM node '{}' has an empty model field", l.base.id),
                    });
                }
                if l.config.prompt.is_empty() {
                    warnings.push(ValidationWarning {
                        warning_type: "invalid_config".to_string(),
                        node_id: Some(l.base.id.clone()),
                        message: format!("LLM node '{}' has an empty prompt field", l.base.id),
                    });
                }
            },
            WorkflowNode::Switch(s) => {
                if s.config.cases.is_empty() {
                    warnings.push(ValidationWarning {
                        warning_type: "invalid_config".to_string(),
                        node_id: Some(s.base.id.clone()),
                        message: format!("Switch node '{}' has empty cases", s.base.id),
                    });
                }
            },
            WorkflowNode::Loop(lp) => {
                if lp.config.body_steps.is_empty() {
                    warnings.push(ValidationWarning {
                        warning_type: "invalid_config".to_string(),
                        node_id: Some(lp.base.id.clone()),
                        message: format!("Loop node '{}' has empty body_steps", lp.base.id),
                    });
                }
            },
            WorkflowNode::Parallel(p) => {
                if p.config.branches.is_empty() {
                    warnings.push(ValidationWarning {
                        warning_type: "invalid_config".to_string(),
                        node_id: Some(p.base.id.clone()),
                        message: format!("Parallel node '{}' has empty branches", p.base.id),
                    });
                }
            },
            WorkflowNode::Condition(c)
                if c.config.conditions.is_empty() && c.config.judge_by_llm.unwrap_or(false) =>
            {
                warnings.push(ValidationWarning {
                    warning_type: "invalid_config".to_string(),
                    node_id: Some(c.base.id.clone()),
                    message: format!(
                        "Condition node '{}' has empty conditions (LLM routing is enabled)",
                        c.base.id
                    ),
                });
            },
            _ => {},
        }
    }

    // ── 边类型校验：特定边类型必须源自对应节点类型 ──
    let node_type_map: std::collections::HashMap<String, &str> =
        nodes.iter().map(|n| (n.base_id().to_string(), node_type_name(n))).collect();

    for edge in &edges {
        match &edge.edge_type {
            EdgeType::ConditionTrue | EdgeType::ConditionFalse => {
                if let Some(&ntype) = node_type_map.get(&edge.source) {
                    if ntype != "condition" {
                        errors.push(ValidationError {
                            error_type: "invalid_edge".to_string(),
                            node_id: Some(edge.id.clone()),
                            message: format!(
                                "{:?} edge '{}' must originate from a Condition node, but source '{}' is a '{}' node",
                                edge.edge_type, edge.id, edge.source, ntype
                            ),
                            suggestion: Some(
                                "Change the source node to a Condition node or change the edge type"
                                    .to_string(),
                            ),
                        });
                    }
                }
            },
            EdgeType::DebateRound => {
                if let Some(&ntype) = node_type_map.get(&edge.source) {
                    if ntype != "debate" {
                        errors.push(ValidationError {
                            error_type: "invalid_edge".to_string(),
                            node_id: Some(edge.id.clone()),
                            message: format!(
                                "debateRound edge '{}' must originate from a Debate node, but source '{}' is a '{}' node",
                                edge.id, edge.source, ntype
                            ),
                            suggestion: Some(
                                "Change the source node to a Debate node or change the edge type"
                                    .to_string(),
                            ),
                        });
                    }
                }
            },
            EdgeType::ParallelBranch => {
                if let Some(&ntype) = node_type_map.get(&edge.source) {
                    if ntype != "parallel" {
                        errors.push(ValidationError {
                            error_type: "invalid_edge".to_string(),
                            node_id: Some(edge.id.clone()),
                            message: format!(
                                "parallelBranch edge '{}' must originate from a Parallel node, but source '{}' is a '{}' node",
                                edge.id, edge.source, ntype
                            ),
                            suggestion: Some(
                                "Change the source node to a Parallel node or change the edge type"
                                    .to_string(),
                            ),
                        });
                    }
                }
            },
            _ => {},
        }
    }

    let is_valid = errors.is_empty();

    Ok(ValidationResult { is_valid, errors, warnings })
}

#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "导出工作流模板")]
#[tauri::command]
pub async fn export_workflow_template(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let db = state.harness.db();
    let template = db_repo::get_workflow_template(db, &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let template = template.ok_or_else(|| {
        ErrorResponse::err_with_detail(workflow_err::NOT_FOUND, "Template not found")
    })?;

    // 系统模板（认知编排器等）对用户不可见：与「不存在」等价，禁止导出
    if is_cognitive_router_template(&template) {
        return Err(ErrorResponse::err_with_detail(workflow_err::NOT_FOUND, "Template not found"));
    }

    let response = workflow_template_response_from_model(template);

    serde_json::to_string_pretty(&response).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 判断节点类型是否属于 n8n 家族（内置 n8n-nodes-base、LangChain @n8n 节点、社区 n8n-nodes-*）
fn is_n8n_node_type(t: &str) -> bool {
    t.starts_with("n8n-nodes-base.") || t.starts_with("n8n-nodes-") || t.starts_with("@n8n/")
}

/// 检测是否为 n8n 格式（存在 n8n 家族类型节点）
fn is_n8n_format(json: &serde_json::Value) -> bool {
    json.get("nodes")
        .and_then(|n| n.as_array())
        .map(|nodes| {
            nodes.iter().any(|n| {
                n.get("type").and_then(|t| t.as_str()).map(is_n8n_node_type).unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// 解析 n8n 节点 position —— n8n 官方导出为 `[x, y]` 数组，部分格式为 `{x, y}` 对象，两者兼容
fn parse_n8n_position(v: &serde_json::Value) -> Position {
    if let Some(arr) = v.as_array() {
        Position {
            x: arr.first().and_then(|e| e.as_f64()).unwrap_or(0.0),
            y: arr.get(1).and_then(|e| e.as_f64()).unwrap_or(0.0),
        }
    } else {
        Position {
            x: v.get("x").and_then(|e| e.as_f64()).unwrap_or(0.0),
            y: v.get("y").and_then(|e| e.as_f64()).unwrap_or(0.0),
        }
    }
}

/// n8n 触发器类型集合（作为工作流入口节点）
fn is_n8n_trigger_type(t: &str) -> bool {
    matches!(
        t,
        "n8n-nodes-base.webhook"
            | "n8n-nodes-base.scheduleTrigger"
            | "n8n-nodes-base.cronTrigger"
            | "n8n-nodes-base.intervalTrigger"
            | "n8n-nodes-base.manualTrigger"
            | "n8n-nodes-base.chatTrigger"
            | "n8n-nodes-base.formTrigger"
            | "n8n-nodes-base.emailTrigger"
            | "n8n-nodes-base.gmailTrigger"
            | "n8n-nodes-base.executeWorkflowTrigger"
            | "n8n-nodes-base.errorTrigger"
    )
}

/// 将 n8n 触发器类型映射为 AxAgent TriggerType
fn map_n8n_trigger_type(t: &str) -> TriggerType {
    match t {
        "n8n-nodes-base.webhook" => TriggerType::Webhook,
        "n8n-nodes-base.scheduleTrigger"
        | "n8n-nodes-base.cronTrigger"
        | "n8n-nodes-base.intervalTrigger" => TriggerType::Schedule,
        _ => TriggerType::Manual,
    }
}

/// 去掉 n8n 表达式包装 `={{ $json.foo }}` → `$json.foo`；无包装时原样返回
fn unwrap_n8n_expression(s: &str) -> String {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("={{") {
        inner.trim().trim_end_matches("}}").trim().to_string()
    } else if let Some(inner) = s.strip_prefix("{{") {
        inner.trim().trim_end_matches("}}").trim().to_string()
    } else {
        s.to_string()
    }
}

/// n8n IF 操作符 → AxAgent CompareOperator
fn n8n_operator_to_compare(op: &str) -> CompareOperator {
    match op {
        "equals" => CompareOperator::Eq,
        "notEquals" => CompareOperator::Ne,
        "gt" => CompareOperator::Gt,
        "lt" => CompareOperator::Lt,
        "gte" => CompareOperator::Gte,
        "lte" => CompareOperator::Lte,
        "contains" => CompareOperator::Contains,
        "notContains" => CompareOperator::NotContains,
        "startsWith" => CompareOperator::StartsWith,
        "endsWith" => CompareOperator::EndsWith,
        "regex" => CompareOperator::RegexMatch,
        "isEmpty" => CompareOperator::IsEmpty,
        "isNotEmpty" => CompareOperator::IsNotEmpty,
        _ => CompareOperator::Eq,
    }
}

/// 解析 n8n IF 节点的 conditions → AxAgent Condition 列表
/// 解析失败（conditions 为空）时由调用方启用 LLM 路由兜底，避免分支静默失效
fn parse_n8n_if_conditions(
    params: Option<&serde_json::Value>,
) -> (Vec<Condition>, LogicalOperator) {
    let mut conditions: Vec<Condition> = Vec::new();
    let mut logical_op = LogicalOperator::And;
    if let Some(p) = params
        && let Some(conds_obj) = p.get("conditions")
    {
        if conds_obj.get("combinator").and_then(|v| v.as_str()) == Some("or") {
            logical_op = LogicalOperator::Or;
        }
        if let Some(arr) = conds_obj.get("conditions").and_then(|v| v.as_array()) {
            for c in arr {
                let left = c.get("leftValue").and_then(|v| v.as_str()).unwrap_or("");
                let right = c.get("rightValue").and_then(|v| v.as_str()).unwrap_or("");
                let op = c
                    .get("operator")
                    .and_then(|o| o.get("operation"))
                    .and_then(|v| v.as_str())
                    .or_else(|| c.get("operator").and_then(|v| v.as_str()))
                    .unwrap_or("equals");
                // 左侧表达式去壳为变量路径；右侧保留原值（字面量或变量表达式）
                let var_path = unwrap_n8n_expression(left);
                let value = serde_json::Value::String(right.to_string());
                conditions.push(Condition {
                    var_path,
                    operator: n8n_operator_to_compare(op),
                    value,
                });
            }
        }
    }
    (conditions, logical_op)
}

/// n8n 节点 kind —— 决定源端口（handle）的语义
#[derive(Debug, Clone, Copy, PartialEq)]
enum N8nNodeKind {
    /// 触发器入口
    Trigger,
    /// IF 条件节点（true/false 两端口）
    Condition,
    /// Switch 多分支节点（branch-N 端口）
    Switch,
    /// 普通单/多输出节点
    Other,
}

/// 由 n8n 节点类型推导其端口语义（用于 edges 的 source_handle 映射）
fn n8n_node_kind(n8n_type: &str) -> N8nNodeKind {
    if is_n8n_trigger_type(n8n_type) {
        N8nNodeKind::Trigger
    } else if n8n_type == "n8n-nodes-base.if" {
        N8nNodeKind::Condition
    } else if n8n_type == "n8n-nodes-base.switch" {
        N8nNodeKind::Switch
    } else {
        N8nNodeKind::Other
    }
}

/// n8n 连接 main 组索引 → AxAgent 源端口（source_handle + edge_type）。
/// n8n 中 main 数组的每个元素即源节点的一个输出端口：
/// - IF 节点：索引 0 = true 分支、1 = false 分支
/// - Switch 节点：索引 N = 第 N 个分支
/// - 其他节点：仅主输出（默认端口，不填 handle）
fn n8n_source_handle(kind: N8nNodeKind, main_index: usize) -> (Option<String>, EdgeType) {
    match kind {
        N8nNodeKind::Condition => match main_index {
            0 => (Some("true".to_string()), EdgeType::ConditionTrue),
            1 => (Some("false".to_string()), EdgeType::ConditionFalse),
            _ => (Some(format!("branch-{}", main_index)), EdgeType::ParallelBranch),
        },
        N8nNodeKind::Switch => (Some(format!("branch-{}", main_index)), EdgeType::ParallelBranch),
        _ => (None, EdgeType::Direct),
    }
}

/// n8n 节点 → AxAgent 节点变体（精确 type 匹配）。
/// 无法精确映射的节点返回 None，由调用方走 Agent 兜底。
fn map_n8n_node(
    base: WorkflowNodeBase,
    n8n_node: &serde_json::Value,
    n8n_type: &str,
) -> Option<WorkflowNode> {
    use axagent_harness::workflow_types::*;
    let node_id = base.id.clone();
    let params = n8n_node.get("parameters");

    // ── 触发器 ──
    if is_n8n_trigger_type(n8n_type) {
        let config = params.cloned().unwrap_or(serde_json::Value::Null);
        return Some(WorkflowNode::Trigger(TriggerNode {
            base,
            config: TriggerConfig { trigger_type: map_n8n_trigger_type(n8n_type), config },
        }));
    }

    // ── IF 条件 ──
    if n8n_type == "n8n-nodes-base.if" {
        let (conditions, logical_op) = parse_n8n_if_conditions(params);
        // 静态条件解析失败时退化为 LLM 路由，保证分支语义不静默失效
        let judge_by_llm = if conditions.is_empty() {
            Some(true)
        } else {
            None
        };
        let routing_prompt = judge_by_llm
            .map(|_| format!("按节点语义判断进入 true 还是 false 分支：{}", base.title));
        return Some(WorkflowNode::Condition(ConditionNode {
            base,
            config: ConditionNodeConfig {
                conditions,
                logical_op,
                judge_by_llm,
                routing_prompt,
                routing_model: None,
                confidence_threshold: None,
            },
        }));
    }

    // ── Switch 多分支 ──
    if n8n_type == "n8n-nodes-base.switch" {
        let input_var = params
            .and_then(|p| p.get("input1"))
            .and_then(|v| v.as_str())
            .map(unwrap_n8n_expression)
            .or_else(|| {
                params
                    .and_then(|p| p.get("inputValue"))
                    .and_then(|v| v.as_str())
                    .map(unwrap_n8n_expression)
            })
            .unwrap_or_default();
        let mut cases: Vec<SwitchCase> = Vec::new();
        let mut default_case: Option<String> = None;
        if let Some(rules) = params.and_then(|p| p.get("rules")) {
            if let Some(values) = rules.get("values").and_then(|v| v.as_array()) {
                let mut by_index: std::collections::BTreeMap<usize, String> =
                    std::collections::BTreeMap::new();
                for r in values {
                    let idx = r.get("outputIndex").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let val = r.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    by_index.insert(idx, val);
                }
                cases = by_index
                    .into_values()
                    .map(|v| SwitchCase { value: v.clone(), label: v })
                    .collect();
            }
            if let Some(fb) = rules.get("fallbackOutputIndex").and_then(|v| v.as_u64()) {
                default_case =
                    cases.iter().find(|c| c.value == fb.to_string()).map(|c| c.value.clone());
                // 兜底端口未在 cases 中时，用占位表达式标记（前端可编辑）
                if default_case.is_none() {
                    default_case = Some(format!("__default_{}", fb));
                }
            }
        }
        return Some(WorkflowNode::Switch(SwitchNode {
            base,
            config: SwitchNodeConfig {
                input_var,
                cases,
                default_case,
                match_mode: "exact".to_string(),
                use_llm: None,
                llm_prompt: None,
                llm_model: None,
                output_var: format!("{}_output", node_id),
            },
        }));
    }

    // ── Merge 合并 ──
    if n8n_type == "n8n-nodes-base.merge" {
        let merge_type = match params.and_then(|p| p.get("mode")).and_then(|v| v.as_str()) {
            Some(m)
                if m.contains("combineByKey")
                    || m.contains("combineByPosition")
                    || m.contains("combine") =>
            {
                MergeStrategy::All
            },
            Some(m) if m.contains("append") => MergeStrategy::All,
            _ => MergeStrategy::All,
        };
        return Some(WorkflowNode::Merge(MergeNode {
            base,
            config: MergeNodeConfig {
                merge_type,
                inputs: Vec::new(),
                auto_inputs_from_branches: true,
            },
        }));
    }

    // ── Wait 延时 ──
    if n8n_type == "n8n-nodes-base.wait" {
        let amount = params.and_then(|p| p.get("amount")).and_then(|v| v.as_u64()).unwrap_or(0);
        let seconds = match params.and_then(|p| p.get("unit")).and_then(|v| v.as_str()) {
            Some("minutes") => amount * 60,
            Some("hours") => amount * 3600,
            Some("days") => amount * 86400,
            _ => amount,
        };
        return Some(WorkflowNode::Delay(DelayNode {
            base,
            config: DelayNodeConfig { delay_type: "seconds".to_string(), seconds, until: None },
        }));
    }

    // ── 循环（分批/遍历）──
    if n8n_type == "n8n-nodes-base.splitInBatches" || n8n_type == "n8n-nodes-base.loopOverItems" {
        return Some(WorkflowNode::Loop(LoopNode {
            base,
            config: LoopNodeConfig {
                loop_type: LoopType::ForEach,
                items_var: None,
                iter_input_var: None,
                iteratee_var: Some("item".to_string()),
                iter_output_var: None,
                partial_result_var: None,
                max_iterations: Some(1000),
                continue_condition: None,
                continue_on_error: false,
                body_steps: Vec::new(),
                interrupt_after_each: false,
                interrupt_nodes: Vec::new(),
                sub_graph: None,
            },
        }));
    }

    // ── noOp：分支结束占位 → 透传空操作 Code 节点以保持拓扑 ──
    if n8n_type == "n8n-nodes-base.noOp" {
        return Some(WorkflowNode::Code(CodeNode {
            base,
            config: CodeNodeConfig {
                language: "javascript".to_string(),
                code: "// no-op: 保持 n8n 分支拓扑的透传节点".to_string(),
                output_var: format!("{}_output", node_id),
                tool_name: None,
                execute_directly: true,
                input_mapping: std::collections::HashMap::new(),
            },
        }));
    }

    // ── HTTP Request ──
    if n8n_type == "n8n-nodes-base.httpRequest" {
        let mut headers: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        if params.and_then(|p| p.get("sendHeaders")).and_then(|v| v.as_bool()).unwrap_or(false) {
            if let Some(hp) = params
                .and_then(|p| p.get("headerParameters"))
                .and_then(|v| v.get("parameters"))
                .and_then(|v| v.as_array())
            {
                for h in hp {
                    if let (Some(k), Some(v)) = (
                        h.get("name").and_then(|x| x.as_str()),
                        h.get("value").and_then(|x| x.as_str()),
                    ) {
                        headers.insert(k.to_string(), v.to_string());
                    }
                }
            }
        }
        let mut body: Option<String> = None;
        if params.and_then(|p| p.get("sendBody")).and_then(|v| v.as_bool()).unwrap_or(false) {
            if let Some(jb) = params.and_then(|p| p.get("jsonBody")).and_then(|v| v.as_str()) {
                body = Some(jb.to_string());
            } else if let Some(bp) = params
                .and_then(|p| p.get("bodyParameters"))
                .and_then(|v| v.get("parameters"))
                .and_then(|v| v.as_array())
            {
                let parts: Vec<String> = bp
                    .iter()
                    .filter_map(|x| {
                        let n = x.get("name").and_then(|v| v.as_str())?;
                        let v = x.get("value").and_then(|v| v.as_str()).unwrap_or("");
                        Some(format!("{}={}", n, v))
                    })
                    .collect();
                if !parts.is_empty() {
                    body = Some(parts.join("&"));
                }
            }
        }
        return Some(WorkflowNode::HttpRequest(HttpRequestNode {
            base,
            config: HttpRequestNodeConfig {
                url: params
                    .and_then(|p| p.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                method: params
                    .and_then(|p| p.get("method"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("GET")
                    .to_uppercase(),
                headers,
                body,
                body_type: "json".to_string(),
                timeout_secs: 30,
                output_var: format!("{}_output", node_id),
                credential_id: None,
            },
        }));
    }

    // ── Code / 函数 ──
    if n8n_type == "n8n-nodes-base.code"
        || n8n_type == "n8n-nodes-base.function"
        || n8n_type == "n8n-nodes-base.functionItem"
    {
        let code = params
            .and_then(|p| p.get("jsCode"))
            .or_else(|| params.and_then(|p| p.get("code")))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return Some(WorkflowNode::Code(CodeNode {
            base,
            config: CodeNodeConfig {
                language: "javascript".to_string(),
                code,
                output_var: format!("{}_output", node_id),
                tool_name: None,
                execute_directly: true,
                input_mapping: std::collections::HashMap::new(),
            },
        }));
    }

    // ── 数据库 ──
    if matches!(
        n8n_type,
        "n8n-nodes-base.postgres"
            | "n8n-nodes-base.mysql"
            | "n8n-nodes-base.sqlite"
            | "n8n-nodes-base.mssql"
            | "n8n-nodes-base.mongoDb"
            | "n8n-nodes-base.redis"
            | "n8n-nodes-base.snowflake"
            | "n8n-nodes-base.oracle"
    ) {
        let operation = params
            .and_then(|p| p.get("operation"))
            .and_then(|v| v.as_str())
            .unwrap_or("executeQuery");
        let query = params
            .and_then(|p| p.get("query"))
            .and_then(|v| v.as_str())
            .filter(|q| !q.is_empty())
            .map(|q| q.to_string())
            .unwrap_or_else(|| format!("-- n8n 节点原操作: {}", operation));
        return Some(WorkflowNode::DatabaseQuery(DatabaseQueryNode {
            base,
            config: DatabaseQueryNodeConfig {
                query,
                params: Vec::new(),
                connection_name: None,
                timeout_secs: 30,
                output_var: format!("{}_output", node_id),
                credential_id: None,
            },
        }));
    }

    // ── 邮件 ──
    if n8n_type == "n8n-nodes-base.emailSend" {
        let to: Vec<String> = params
            .and_then(|p| p.get("toEmail"))
            .and_then(|v| v.as_str())
            .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
            .unwrap_or_default();
        let subject = params
            .and_then(|p| p.get("subject"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let body = params
            .and_then(|p| p.get("text"))
            .or_else(|| params.and_then(|p| p.get("body")))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return Some(WorkflowNode::Email(EmailNode {
            base,
            config: EmailNodeConfig {
                to,
                subject,
                body,
                smtp_host: None,
                smtp_port: None,
                smtp_user: None,
                smtp_pass: None,
                output_var: format!("{}_output", node_id),
                credential_id: None,
            },
        }));
    }

    // ── Set / 字段编辑 → 数据转换 ──
    if n8n_type == "n8n-nodes-base.set" || n8n_type == "n8n-nodes-base.editFields" {
        let mut parts: Vec<String> = Vec::new();
        if let Some(assign) = params
            .and_then(|p| p.get("assignments"))
            .and_then(|v| v.get("assignments"))
            .and_then(|v| v.as_array())
        {
            for a in assign {
                if let (Some(name), Some(value)) = (
                    a.get("name").and_then(|v| v.as_str()),
                    a.get("value").and_then(|v| v.as_str()),
                ) {
                    parts.push(format!("{}={}", name, value));
                }
            }
        }
        let expression = if parts.is_empty() {
            "({ json }) => json".to_string()
        } else {
            parts.join(";")
        };
        return Some(WorkflowNode::DataTransformer(DataTransformerNode {
            base,
            config: DataTransformerNodeConfig {
                input_var: "input".to_string(),
                expression,
                output_var: format!("{}_output", node_id),
            },
        }));
    }

    None
}

/// n8n 节点类型 → (agent_profile_id, agent_role, expert_id, expert_system_prompt)
// i18n-exempt: Expert role descriptions are LLM system prompts — model interaction data, not UI
fn infer_agent_from_n8n(
    node_type: &str,
    node_name: &str,
) -> (&'static str, &'static str, &'static str, &'static str) {
    let t = node_type.to_lowercase();
    let n = node_name.to_lowercase();

    // Node name takes priority — handles generic n8n types (e.g. "n8n-nodes-base.noOp")
    if n.contains("review") || n.contains("check") || n.contains("validate") || n.contains("audit")
    {
        return (
            "code-reviewer",
            "reviewer",
            "code-reviewer",
            "Code Review Expert: Review code for correctness, security, performance, and maintainability. Provide specific improvement suggestions.",
        );
    }
    if n.contains("debug") || n.contains("fix") || n.contains("troubleshoot") {
        return (
            "debug-expert",
            "developer",
            "debug-expert",
            "Debug Expert: Systematically analyze error logs, identify root causes. Verify fix solutions.",
        );
    }
    if n.contains("test") || n.contains("qa") || n.contains("quality") {
        return (
            "debug-expert",
            "reviewer",
            "debug-expert",
            "Test Engineer: Write and execute test cases, verify functional correctness.",
        );
    }
    if n.contains("doc") || n.contains("report") || n.contains("summary") || n.contains("write") {
        return (
            "tech-writer",
            "synthesizer",
            "tech-writer",
            "Technical Writer: Write clear and accurate technical documentation and reports.",
        );
    }
    if n.contains("plan") || n.contains("design") || n.contains("architect") {
        return (
            "architect",
            "planner",
            "architect",
            "System Architect: Responsible for system design, technology selection, and architecture review.",
        );
    }
    if n.contains("monitor") || n.contains("alert") || n.contains("watch") {
        return (
            "devops-engineer",
            "executor",
            "devops-engineer",
            "DevOps Engineer: Monitor system status, handle alerts, and automate operations.",
        );
    }
    if n.contains("analyze") || n.contains("insight") || n.contains("report") {
        return (
            "data-analyst",
            "researcher",
            "data-analyst",
            "Data Analyst: Data cleaning, statistical analysis, and visualization.",
        );
    }

    if t.contains("http")
        || t.contains("api")
        || t.contains("rest")
        || t.contains("webhook")
        || t.contains("graphql")
        || t.contains("request")
    {
        (
            "devops-engineer",
            "executor",
            "devops-engineer",
            "DevOps Engineer: Responsible for API integration, CI/CD pipelines, HTTP request automation. Ensure reliability and error handling of interface calls.",
        )
    } else if t.contains("database")
        || t.contains("sql")
        || t.contains("postgres")
        || t.contains("mysql")
        || t.contains("mongo")
        || t.contains("redis")
    {
        (
            "sql-expert",
            "researcher",
            "sql-expert",
            "SQL Expert: Proficient in database query optimization, data modeling, and SQL writing. Consider indexing strategies and concurrency control.",
        )
    } else if t.contains("code")
        || t.contains("function")
        || t.contains("python")
        || t.contains("javascript")
        || t.contains("typescript")
    {
        (
            "senior-developer",
            "developer",
            "senior-developer",
            "Senior Developer: Proficient in multiple languages and frameworks, following best practices. Write clear, efficient, and maintainable code.",
        )
    } else if t.contains("email")
        || t.contains("slack")
        || t.contains("notify")
        || t.contains("telegram")
        || t.contains("discord")
    {
        (
            "product-manager",
            "coordinator",
            "product-manager",
            "Product Manager: Communication coordination, requirements analysis, and notification management.",
        )
    } else if t.contains("ai")
        || t.contains("llm")
        || t.contains("openai")
        || t.contains("anthropic")
        || t.contains("chat")
    {
        (
            "general-assistant",
            "coordinator",
            "general-assistant",
            "General AI Assistant: Versatile assistant handling various tasks and questions.",
        )
    } else if t.contains("file")
        || t.contains("csv")
        || t.contains("spreadsheet")
        || t.contains("xml")
        || t.contains("json")
        || t.contains("excel")
    {
        (
            "data-analyst",
            "researcher",
            "data-analyst",
            "Data Analyst: Data cleaning, statistical analysis, and visualization, skilled at extracting insights from data.",
        )
    } else if t.contains("security") || t.contains("auth") || t.contains("oauth") {
        (
            "security-auditor",
            "reviewer",
            "security-auditor",
            "Security Auditor: OWASP Top 10 review, authentication/authorization checks, data encryption, and privacy protection.",
        )
    } else if t.contains("transform")
        || t.contains("convert")
        || t.contains("merge")
        || t.contains("sort")
        || t.contains("filter")
    {
        (
            "tech-writer",
            "synthesizer",
            "tech-writer",
            "Technical Writer: Organize, transform, and aggregate data, output structured results.",
        )
    } else {
        (
            "debug-expert",
            "executor",
            "debug-expert",
            "Debug Expert: Systematic analysis, identify root causes, verify fix solutions.",
        )
    }
}

/// 确保 AgentRole 存在，不存在则创建
async fn ensure_agent_role<C: ConnectionTrait>(db: &C, role_name: &str) -> Result<(), String> {
    use axagent_entities::agent_roles;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // 按 name 字段查重，避免仅按主键匹配导致同名字段创建重复记录
    let existing = agent_roles::Entity::find()
        .filter(agent_roles::Column::Name.eq(role_name))
        .one(db)
        .await
        .map_err(|e: sea_orm::DbErr| e.to_string())?;

    if existing.is_none() {
        let now = chrono::Utc::now().timestamp_millis();
        let am = agent_roles::ActiveModel {
            id: Set(role_name.to_string()),
            name: Set(role_name.to_string()),
            description: Set(Some(format!("Auto-created from n8n import: {}", role_name))),
            system_prompt: Set(String::new()),
            default_tools: Set(None),
            active_domains: Set(None),
            max_concurrent: Set(3),
            timeout_seconds: Set(600),
            responsibilities: Set(None),
            decision_authority: Set(None),
            reports_to: Set(None),
            managed_expert_ids: Set(None),
            required_certifications: Set(None),
            icon: Set(None),
            color: Set(None),
            is_enabled: Set(1),
            source: Set("imported".to_string()),
            sort_order: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
        };
        agent_roles::Entity::insert(am)
            .exec(db)
            .await
            .map_err(|e| format!("Failed to create AgentRole {}: {}", role_name, e))?;
    }
    Ok(())
}

/// 从 n8n 导入时创建 Expert（技能）+ AgentRole（岗位）+ AgentProfile（组装体）。
/// profile 名称收敛为 `n8n-{role}` 固定格式（而非节点名），避免同名节点产生冗余技能。
async fn ensure_agent_profile<C: ConnectionTrait>(
    db: &C,
    profile_id: &str,
    agent_role: &str,
    expert_id: &str,
    expert_prompt: &str,
) -> Result<(), String> {
    use axagent_entities::{agency_experts, agent_profiles};
    use sea_orm::Set;

    // profile 名称按角色收敛为稳定格式，重复导入不会产生新技能记录
    let profile_name = format!("n8n-{}", agent_role);

    let now = chrono::Utc::now().timestamp_millis();

    // 1. 确保 Expert（技能）存在
    let expert_exists = agency_experts::Entity::find_by_id(expert_id)
        .one(db)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .is_some();
    if !expert_exists {
        let expert_am = agency_experts::ActiveModel {
            id: Set(expert_id.to_string()),
            name: Set(profile_name.to_string()),
            description: Set(Some(format!("Auto-created from n8n import: {}", profile_name))),
            category: Set("general".to_string()),
            system_prompt: Set(expert_prompt.to_string()),
            color: Set(None),
            source_dir: Set("n8n-import".to_string()),
            is_enabled: Set(1),
            imported_at: Set(now),
            recommended_workflows: Set(None),
            recommended_tools: Set(None),
            active_domains: Set(None),
            seniority: Set(None),
            specialties: Set(None),
            parent_role_id: Set(None),
            success_rate: Set(None),
            avg_latency_ms: Set(None),
            avg_token_cost: Set(None),
        };
        agency_experts::Entity::insert(expert_am)
            .exec(db)
            .await
            .map_err(|e| format!("Failed to create Expert {}: {}", expert_id, e))?;
    }

    // 2. 确保 AgentRole（岗位）存在
    ensure_agent_role(db, agent_role).await?;

    // 3. 确保 AgentProfile（组装体）存在并绑定 Expert
    let profile_exists = agent_profiles::Entity::find_by_id(profile_id)
        .one(db)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .is_some();
    if !profile_exists {
        let profile_am = agent_profiles::ActiveModel {
            id: Set(profile_id.to_string()),
            name: Set(profile_name.to_string()),
            description: Set(Some(format!("{} + {}", profile_name, agent_role))),
            category: Set("general".to_string()),
            icon: Set("🤖".to_string()),
            agent_role: Set(Some(agent_role.to_string())),
            source: Set("imported".to_string()),
            sort_order: Set(0),
            is_enabled: Set(1),
            expert_id: Set(Some(expert_id.to_string())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        agent_profiles::Entity::insert(profile_am)
            .exec(db)
            .await
            .map_err(|e| format!("Failed to create AgentProfile {}: {}", profile_id, e))?;
    }
    Ok(())
}

/// 相似工作流信息
#[derive(Debug, serde::Serialize)]
pub struct SimilarWorkflow {
    pub workflow_id: String,
    pub name: String,
    pub similarity: f64,
    pub overlapping_nodes: usize,
    pub total_nodes: usize,
}

/// 基于节点类型组成查找相似工作流（Jaccard 相似度 ≥ 0.6 视为相似）。
/// 用于创建、导入工作流时检测是否与已有模板高度重合。
pub async fn find_similar_workflows(
    db: &DatabaseConnection,
    nodes: &[WorkflowNode],
) -> Result<Vec<SimilarWorkflow>, String> {
    let input_types: std::collections::HashSet<String> =
        nodes.iter().map(|n| node_type_name(n).to_string()).collect();

    if input_types.is_empty() {
        return Ok(Vec::new());
    }

    let all = axagent_entities::workflow_template::Entity::find().all(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let mut results = Vec::new();
    for tmpl in &all {
        let existing_nodes: Vec<WorkflowNode> =
            serde_json::from_str(&tmpl.nodes).unwrap_or_default();
        let existing_types: std::collections::HashSet<String> =
            existing_nodes.iter().map(|n| node_type_name(n).to_string()).collect();

        let intersection = input_types.intersection(&existing_types).count();
        let union = input_types.union(&existing_types).count();
        let similarity = if union > 0 {
            intersection as f64 / union as f64
        } else {
            0.0
        };

        if similarity >= 0.6 {
            results.push(SimilarWorkflow {
                workflow_id: tmpl.id.clone(),
                name: tmpl.name.clone(),
                similarity,
                overlapping_nodes: intersection,
                total_nodes: input_types.len(),
            });
        }
    }

    Ok(results)
}

/// 语义重复检查：Jaccard 相似度 ≥ 0.6 视为重复
/// 注意：此函数当前全表扫描已导入模板进行字符级相似度比较。
/// 本地客户端模板数量有限（通常 < 1000），性能影响可接受。
/// 若未来支持云端同步或大规模模板库，应改为数据库模糊查询或向量索引。
async fn check_workflow_duplicate(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<String>, String> {
    use axagent_entities::workflow_template;

    let input_tokens: std::collections::HashSet<String> = name
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 1)
        .map(|s| s.to_string())
        .collect();

    if input_tokens.is_empty() {
        return Ok(None);
    }

    let all = workflow_template::Entity::find().all(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    for tmpl in &all {
        let existing_tokens: std::collections::HashSet<String> = tmpl
            .name
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 1)
            .map(|s| s.to_string())
            .collect();

        let intersection = input_tokens.intersection(&existing_tokens).count();
        let union = input_tokens.union(&existing_tokens).count();
        let similarity = if union > 0 {
            (intersection as f64) / (union as f64)
        } else {
            0.0
        };

        if similarity >= 0.95 {
            return Ok(Some(tmpl.name.clone()));
        }
    }
    Ok(None)
}

/// 从 n8n 节点参数提取有意义的 goal 描述
fn extract_goal_from_n8n(node: &serde_json::Value) -> String {
    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let params = node.get("parameters");

    if let Some(p) = params {
        if node_type.contains("http") || node_type.contains("api") {
            let method = p.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
            let url = p.get("url").and_then(|v| v.as_str()).unwrap_or("(no URL)");
            return format!("HTTP {} {}", method, url);
        }
        if node_type.contains("database") || node_type.contains("sql") {
            let op = p.get("operation").and_then(|v| v.as_str()).unwrap_or("query");
            let table = p.get("table").and_then(|v| v.as_str()).unwrap_or("");
            return format!("SQL {} {}", op, table);
        }
        if node_type.contains("email") {
            let subj = p.get("subject").and_then(|v| v.as_str()).unwrap_or("");
            return format!("Send email: {}", subj);
        }
        if node_type.contains("code") || node_type.contains("function") {
            let lang = node_type.rsplit('.').next().unwrap_or("code");
            return format!("Execute {} function", lang);
        }
    }
    let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed");
    format!("{} ({})", name, node_type.rsplit('.').next().unwrap_or(node_type))
}

/// 从 n8n 节点参数提取 AxAgent AgentNodeConfig 配置
fn extract_config_from_n8n(n8n_node: &serde_json::Value, node_id: &str) -> AgentNodeConfig {
    let node_type = n8n_node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let node_name = n8n_node.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed");
    let params = n8n_node.get("parameters");
    let goal = extract_goal_from_n8n(n8n_node);

    // ── 构建 system_prompt：n8n 节点参数 → 自然语言任务描述 ──
    let mut prompt_parts: Vec<String> = Vec::new();
    prompt_parts.push(format!("任务目标：{goal}"));

    if let Some(p) = params {
        // HTTP / API 节点
        if node_type.contains("http") || node_type.contains("api") || node_type.contains("webhook")
        {
            let method = p.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
            let url = p.get("url").and_then(|v| v.as_str()).unwrap_or("");
            if !url.is_empty() {
                prompt_parts.push(format!("调用方式：{method} {url}"));
            }
            if let Some(body) = p.get("body") {
                if let Some(s) = body.as_str() {
                    prompt_parts.push(format!("请求体参数：{s}"));
                }
            }
            if let Some(headers) = p.get("headers") {
                prompt_parts.push(format!("请求头：{headers}"));
            }
            if let Some(auth) = p.get("authentication") {
                prompt_parts.push(format!("认证方式：{auth}"));
            }
        }
        // 数据库节点
        else if node_type.contains("database")
            || node_type.contains("sql")
            || node_type.contains("postgres")
        {
            let op = p.get("operation").and_then(|v| v.as_str()).unwrap_or("query");
            prompt_parts.push(format!("操作类型：{op}"));
            if let Some(query) = p.get("query").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("SQL 语句：{query}"));
            }
            if let Some(table) = p.get("table").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("目标表：{table}"));
            }
        }
        // 邮件节点
        else if node_type.contains("email") {
            if let Some(subj) = p.get("subject").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("邮件主题：{subj}"));
            }
            if let Some(to) = p.get("toEmail").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("收件人：{to}"));
            }
            if let Some(text) = p.get("text").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("邮件内容：{text}"));
            }
        }
        // 代码/函数节点
        else if node_type.contains("code") || node_type.contains("function") {
            let lang = node_type.rsplit('.').next().unwrap_or("javascript");
            prompt_parts.push(format!("执行语言：{lang}"));
            if let Some(code) = p.get("jsCode").or(p.get("code")).and_then(|v| v.as_str()) {
                let code_preview = if code.len() > 500 {
                    format!("{}…(截断)", &code[..500])
                } else {
                    code.to_string()
                };
                prompt_parts.push(format!("代码片段：\n```{lang}\n{code_preview}\n```"));
            }
        }
        // AI / LLM 节点
        else if node_type.contains("ai")
            || node_type.contains("llm")
            || node_type.contains("openai")
            || node_type.contains("openAi")
        {
            if let Some(prompt) = p.get("prompt").or(p.get("text")).and_then(|v| v.as_str()) {
                prompt_parts.push(format!("AI 提示词：{prompt}"));
            }
            if let Some(model) = p.get("model").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("使用模型：{model}"));
            }
        }
        // 通用节点：提取所有参数
        else {
            prompt_parts
                .push(format!("节点类型：{}", node_type.rsplit('.').next().unwrap_or(node_type)));
            if let Some(obj) = p.as_object() {
                let params_desc: Vec<String> = obj
                    .iter()
                    .filter_map(|(k, v)| {
                        if k == "options" || k == "additionalFields" {
                            None
                        } else if let Some(s) = v.as_str() {
                            Some(format!("  {k}: {s}"))
                        } else {
                            Some(format!("  {k}: {v}"))
                        }
                    })
                    .collect();
                if !params_desc.is_empty() {
                    prompt_parts.push(format!("参数配置：\n{}", params_desc.join("\n")));
                }
            }
        }
    }

    // 追加节点描述（n8n 节点的 notes）
    if let Some(notes) = n8n_node.get("notes").and_then(|v| v.as_str()) {
        if !notes.is_empty() {
            prompt_parts.push(format!("备注说明：{notes}"));
        }
    }

    let system_prompt = prompt_parts.join("\n\n");

    // ── 构建 tools：根据 n8n 节点类型生成 ToolDef ──
    let mut tools: Vec<ToolDef> = Vec::new();

    let (tool_name, tool_desc) =
        if node_type.contains("http") || node_type.contains("api") || node_type.contains("webhook")
        {
            ("http_request", "发送 HTTP 请求并获取响应数据".to_string())
        } else if node_type.contains("database")
            || node_type.contains("sql")
            || node_type.contains("postgres")
        {
            ("database_query", "执行数据库查询或操作".to_string())
        } else if node_type.contains("email") {
            ("send_email", "发送电子邮件".to_string())
        } else if node_type.contains("code") || node_type.contains("function") {
            let lang = node_type.rsplit('.').next().unwrap_or("javascript");
            ("execute_code", format!("执行 {lang} 代码"))
        } else if node_type.contains("file")
            || node_type.contains("spreadsheet")
            || node_type.contains("csv")
        {
            ("file_operation", "读写文件或电子表格".to_string())
        } else {
            ("process_data", "处理数据或执行业务逻辑".to_string())
        };

    tools.push(ToolDef {
        name: format!("{tool_name}_{node_id}"),
        description: Some(format!(
            "{tool_desc}。原始节点: {node_name} ({n8n_type})",
            tool_desc = tool_desc,
            node_name = node_name,
            n8n_type = node_type.rsplit('.').next().unwrap_or(node_type)
        )),
        parameters: None,
    });

    // ── 提取模型设置（如果 n8n AI 节点有） ──
    let (model, temperature, max_tokens) = if let Some(p) = params {
        let model = if node_type.contains("ai")
            || node_type.contains("openai")
            || node_type.contains("openAi")
        {
            p.get("model").and_then(|v| v.as_str()).map(|s| s.to_string())
        } else {
            None
        };
        let temperature = p.get("temperature").and_then(|v| v.as_f64()).map(|t| t as f32);
        let max_tokens = p.get("maxTokens").and_then(|v| v.as_u64()).map(|t| t as u32);
        (model, temperature, max_tokens)
    } else {
        (None, None, None)
    };

    AgentNodeConfig {
        system_prompt,
        context_sources: Vec::new(),
        output_var: format!("{}_output", node_id),
        model,
        temperature,
        max_tokens,
        tools,
        exposed_tools: vec![],
        output_mode: OutputMode::Text,
        agent_profile_id: None,
        max_tool_rounds: None,
        execution_mode: None,
        rag_source_ids: vec![],
        model_role: None,
        consistency_check: None,
        hallucination_guard: None,
        task_scene: None,
        input_mapping: std::collections::HashMap::new(),
        fallback_model: None,
        stream_chunk_timeout_secs: None,
    }
}

/// 生成 n8n 导入占位工具的 Rhai 脚本：纯字符串返回，保证可通过 Rhai 编译；
/// 运行时若被调用，显式提示"待配置"，而非静默失败或报"工具未注册"。
fn n8n_placeholder_tool_script(tool_name: &str, node_name: &str) -> String {
    format!(
        "// 占位工具 {tool_name}（来源 n8n 节点 {node_name}）\n\"工具待配置：由 n8n 工作流导入生成，请在工具面板补全实现并启用后使用\""
    )
}

/// 将 n8n JSON 转换为 AxAgent Workflow — 两阶段：先 DB 准备，再组装
async fn convert_n8n_to_axagent<C: ConnectionTrait>(
    db: &C,
    json: &serde_json::Value,
) -> Result<axagent_harness::workflow_types::WorkflowTemplateData, String> {
    use axagent_harness::workflow_types::*;

    let name =
        json.get("name").and_then(|v| v.as_str()).unwrap_or("Imported n8n Workflow").to_string();

    let n8n_nodes = json
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing 'nodes' array".to_string())?;

    let n8n_connections = json.get("connections").cloned();

    let mut ax_nodes: Vec<WorkflowNode> = Vec::new();
    let mut ax_edges: Vec<WorkflowEdge> = Vec::new();
    let mut edge_id_counter = 0u32;
    // 降级 Agent 节点声明的工具 → 占位 RhaiToolDef（补全 tool_defs，避免运行时"工具未注册"）
    let mut tool_defs: Vec<RhaiToolDef> = Vec::new();
    let mut tool_def_name_seen: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    // 降级 Agent 节点在 ax_nodes 中的下标（edges 定型后回填 context_sources）
    let mut agent_node_indices: Vec<usize> = Vec::new();
    let mut name_to_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // 节点 id → 端口语义（供 edges 的 source_handle 映射）
    let mut node_kinds: std::collections::HashMap<String, N8nNodeKind> =
        std::collections::HashMap::new();
    // 已 ensure 过的 profile_id —— 同一工作流内多个节点映射到同一 profile 时避免重复查库
    let mut ensured_profiles: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 入口节点：n8n 自带触发器（webhook/schedule 等）时以其为入口，避免叠加固定 Trigger 造成
    // 双入口歧义；否则补充一个固定的 Manual 入口。
    let native_trigger_id: Option<String> = n8n_nodes.iter().find_map(|n| {
        let is_trigger =
            n.get("type").and_then(|t| t.as_str()).map(is_n8n_trigger_type).unwrap_or(false);
        is_trigger.then(|| {
            n.get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        })
    });
    let entry_node_id = native_trigger_id.unwrap_or_else(|| {
        let trigger_node = WorkflowNode::Trigger(TriggerNode {
            base: WorkflowNodeBase {
                continue_on_fail: false,
                compensation: None,
                id: "trigger_imported".to_string(),
                title: "Trigger".to_string(),
                description: Some("Auto-created trigger from n8n import".to_string()),
                position: Position { x: 0.0, y: 0.0 },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
            },
            config: TriggerConfig {
                trigger_type: TriggerType::Manual,
                config: serde_json::Value::Null,
            },
        });
        ax_nodes.push(trigger_node);
        "trigger_imported".to_string()
    });

    for n8n_node in n8n_nodes {
        let node_id = n8n_node
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let node_name =
            n8n_node.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed").to_string();

        let n8n_type =
            n8n_node.get("type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();

        name_to_id.insert(node_name.clone(), node_id.clone());

        let kind = n8n_node_kind(&n8n_type);

        // n8n 官方导出 position 为 `[x, y]` 数组（部分为 `{x, y}` 对象），统一解析
        let position =
            n8n_node.get("position").map(parse_n8n_position).unwrap_or(Position { x: 0.0, y: 0.0 });

        let base = WorkflowNodeBase {
            continue_on_fail: false,
            compensation: None,
            id: node_id.clone(),
            title: node_name.clone(),
            description: None,
            position: position.clone(),
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
        };

        // 优先精确映射为对应节点类型（HTTP/Code/Database/IF/Switch/Merge 等）
        if let Some(mapped) = map_n8n_node(base, n8n_node, &n8n_type) {
            node_kinds.insert(node_id.clone(), kind);
            ax_nodes.push(mapped);
            continue;
        }

        // 无法精确映射 → Agent 兜底：按节点名称/类型推断角色与专家，生成配置
        let (agent_profile_id, agent_role, expert_id, expert_prompt) =
            infer_agent_from_n8n(&n8n_type, &node_name);

        // 收敛 Profile 生成：同一工作流内相同 profile 只 ensure 一次，避免重复查库
        if ensured_profiles.insert(agent_profile_id.to_string()) {
            ensure_agent_profile(db, agent_profile_id, agent_role, expert_id, expert_prompt)
                .await?;
        }

        let goal = extract_goal_from_n8n(n8n_node);

        // 从 n8n 节点提取配置（system_prompt、tools、model 等）
        let mut agent_config = extract_config_from_n8n(n8n_node, &node_id);
        agent_config.agent_profile_id = Some(agent_profile_id.to_string());
        // context_sources 在 edges 定型后统一回填，此处保持空
        agent_config.context_sources = Vec::new();

        // 为降级 Agent 声明的工具生成占位 RhaiToolDef，保证运行时能命中 handler
        for tool in &agent_config.tools {
            if tool_def_name_seen.insert(tool.name.clone()) {
                tool_defs.push(RhaiToolDef {
                    tool_name: tool.name.clone(),
                    description: tool.description.clone(),
                    code: n8n_placeholder_tool_script(&tool.name, &node_name),
                });
            }
        }

        let base = WorkflowNodeBase {
            continue_on_fail: false,
            compensation: None,
            id: node_id.clone(),
            title: node_name,
            description: Some(goal),
            position,
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
        };

        let agent_node = WorkflowNode::Agent(AgentNode { base, config: agent_config });

        node_kinds.insert(node_id.clone(), N8nNodeKind::Other);
        agent_node_indices.push(ax_nodes.len());
        ax_nodes.push(agent_node);
    }

    let last_position = ax_nodes
        .iter()
        .map(|n| match n {
            WorkflowNode::Trigger(t) => t.base.position.clone(),
            WorkflowNode::Agent(t) => t.base.position.clone(),
            WorkflowNode::Llm(t) => t.base.position.clone(),
            WorkflowNode::Condition(t) => t.base.position.clone(),
            WorkflowNode::Parallel(t) => t.base.position.clone(),
            WorkflowNode::Loop(t) => t.base.position.clone(),
            WorkflowNode::Merge(t) => t.base.position.clone(),
            WorkflowNode::Delay(t) => t.base.position.clone(),
            WorkflowNode::Validation(t) => t.base.position.clone(),
            WorkflowNode::Tool(t) => t.base.position.clone(),
            WorkflowNode::Code(t) => t.base.position.clone(),
            WorkflowNode::SubWorkflow(t) => t.base.position.clone(),
            WorkflowNode::DocumentParser(t) => t.base.position.clone(),
            WorkflowNode::VectorRetrieve(t) => t.base.position.clone(),
            WorkflowNode::HttpRequest(t) => t.base.position.clone(),
            WorkflowNode::Switch(t) => t.base.position.clone(),
            WorkflowNode::DatabaseQuery(t) => t.base.position.clone(),
            WorkflowNode::Notification(t) => t.base.position.clone(),
            WorkflowNode::Approval(t) => t.base.position.clone(),
            WorkflowNode::FileOperation(t) => t.base.position.clone(),
            WorkflowNode::DataTransformer(t) => t.base.position.clone(),
            WorkflowNode::WebhookSend(t) => t.base.position.clone(),
            WorkflowNode::Logging(t) => t.base.position.clone(),
            WorkflowNode::LlmClassifier(t) => t.base.position.clone(),
            WorkflowNode::Aggregator(t) => t.base.position.clone(),
            WorkflowNode::Email(t) => t.base.position.clone(),
            WorkflowNode::Debate(t) => t.base.position.clone(),
            WorkflowNode::Swarm(t) => t.base.position.clone(),
            WorkflowNode::MultiAgent(t) => t.base.position.clone(),
            WorkflowNode::Storage(t) => t.base.position.clone(),
            WorkflowNode::WorkflowRef(t) => t.base.position.clone(),
            WorkflowNode::End(t) => t.base.position.clone(),
        })
        .next_back()
        .unwrap_or(Position { x: 250.0, y: 0.0 });

    let end_node = WorkflowNode::End(EndNode {
        base: WorkflowNodeBase {
            continue_on_fail: false,
            compensation: None,
            id: "end_imported".to_string(),
            title: "End".to_string(),
            description: Some("Auto-created end node from n8n import".to_string()),
            position: Position { x: last_position.x + 250.0, y: last_position.y },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
        },
        config: EndNodeConfig { output_var: Some("final_output".to_string()) },
    });
    ax_nodes.push(end_node);

    // Convert n8n connections → edges
    // n8n 连接 `main` 数组的每个元素即源节点的第 N 个输出端口：
    // IF 节点 main[0]=true / main[1]=false；Switch 节点 main[N]=第 N 分支。
    // 这里据此映射 source_handle + edge_type，还原分支拓扑。
    if let Some(connections) = n8n_connections {
        if let Some(conn_map) = connections.as_object() {
            for (source_name, conn_val) in conn_map {
                let source_id = match name_to_id.get(source_name) {
                    Some(id) => id.clone(),
                    None => continue,
                };
                let source_kind = node_kinds.get(&source_id).copied().unwrap_or(N8nNodeKind::Other);
                if let Some(main_arr) = conn_val.get("main").and_then(|v| v.as_array()) {
                    for (main_index, main_group) in main_arr.iter().enumerate() {
                        if let Some(entries) = main_group.as_array() {
                            for entry in entries {
                                let target_name = entry.get("node").and_then(|v| v.as_str());
                                let target_id = match target_name.and_then(|n| name_to_id.get(n)) {
                                    Some(id) => id.clone(),
                                    None => continue,
                                };
                                let (source_handle, edge_type) =
                                    n8n_source_handle(source_kind, main_index);
                                ax_edges.push(WorkflowEdge {
                                    id: format!("edge_{}", edge_id_counter),
                                    source: source_id.clone(),
                                    source_handle,
                                    target: target_id,
                                    // 目标输入端口索引（Merge 多输入等），普通节点用默认输入端口
                                    target_handle: None,
                                    edge_type,
                                    label: None,
                                });
                                edge_id_counter += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // If no edges, create sequential flow
    if ax_edges.is_empty() && ax_nodes.len() > 1 {
        for i in 1..ax_nodes.len() {
            ax_edges.push(WorkflowEdge {
                id: format!("edge_{}", edge_id_counter),
                source: ax_nodes[i - 1].base_id().to_string(),
                source_handle: None,
                target: ax_nodes[i].base_id().to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            });
            edge_id_counter += 1;
        }
    } else if !ax_edges.is_empty() {
        let targets_with_incoming: std::collections::HashSet<String> =
            ax_edges.iter().map(|e| e.target.clone()).collect();
        for node in &ax_nodes {
            let nid = node.base_id();
            if nid != entry_node_id && nid != "end_imported" && !targets_with_incoming.contains(nid)
            {
                ax_edges.push(WorkflowEdge {
                    id: format!("edge_{}", edge_id_counter),
                    source: entry_node_id.clone(),
                    source_handle: None,
                    target: nid.to_string(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
                edge_id_counter += 1;
            }
        }
        let sources_with_outgoing: std::collections::HashSet<String> =
            ax_edges.iter().map(|e| e.source.clone()).collect();
        for node in &ax_nodes {
            let nid = node.base_id();
            if nid != entry_node_id && nid != "end_imported" && !sources_with_outgoing.contains(nid)
            {
                ax_edges.push(WorkflowEdge {
                    id: format!("edge_{}", edge_id_counter),
                    source: nid.to_string(),
                    source_handle: None,
                    target: "end_imported".to_string(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
                edge_id_counter += 1;
            }
        }
    }

    // 回填降级 Agent 节点的 context_sources：按最终 edges 反推其直接上游节点 id，
    // 运行时按 id 从 workflow.results 取值注入（见 engine::get_context_source_results）。
    for idx in &agent_node_indices {
        let target_id = ax_nodes[*idx].base_id().to_string();
        let mut sources: Vec<String> = Vec::new();
        for e in &ax_edges {
            if e.target == target_id && e.source != target_id && !sources.contains(&e.source) {
                sources.push(e.source.clone());
            }
        }
        if let WorkflowNode::Agent(a) = &mut ax_nodes[*idx] {
            a.config.context_sources = sources;
        }
    }

    let now = chrono::Utc::now().timestamp_millis();
    Ok(WorkflowTemplateData {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        description: Some("Imported from n8n workflow".to_string()),
        icon: "🔧".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec!["n8n".to_string(), "imported".to_string()],
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        visibility: Default::default(),
        trigger_config: None,
        nodes: ax_nodes,
        edges: ax_edges,
        input_schema: None,
        output_schema: None,
        variables: Vec::new(),
        error_config: None,
        tool_defs,
        error_workflow_id: None,
        mission_hash: None,
        created_at: now,
        updated_at: now,
    })
}

async fn do_import_workflow(
    state: &AppState,
    db: &DatabaseConnection,
    json_data: String,
) -> Result<serde_json::Value, String> {
    let raw_json: serde_json::Value =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON: {}", e))?;

    let workflow_name =
        raw_json.get("name").and_then(|v| v.as_str()).unwrap_or("Imported Workflow").to_string();

    // n8n 转换会写 AgentRole/Expert/Profile 等副作用，须与模板插入同事务，失败整体回滚
    let mut tx_holder: Option<sea_orm::DatabaseTransaction> = None;

    let mut new_template = if is_n8n_format(&raw_json) {
        let tx = db.begin().await.map_err(|e| format!("Begin transaction: {}", e))?;
        let t = convert_n8n_to_axagent(&tx, &raw_json).await?;
        tx_holder = Some(tx);
        t
    } else {
        let template: WorkflowTemplateResponse = serde_json::from_value(raw_json)
            .map_err(|e| format!("Invalid AxAgent format: {}", e))?;

        let nodes = template.nodes.clone();

        let now = chrono::Utc::now().timestamp_millis();
        WorkflowTemplateData {
            id: uuid::Uuid::new_v4().to_string(),
            name: template.name,
            description: template.description,
            icon: template.icon,
            cluster_id: template.cluster_id.clone(),
            route_path: template.route_path.clone(),
            tags: template.tags,
            version: 1,
            is_preset: false,
            is_editable: true,
            is_public: false,
            visibility: Default::default(),
            trigger_config: template.trigger_config,
            nodes,
            edges: template.edges,
            input_schema: template.input_schema,
            output_schema: template.output_schema,
            variables: template.variables,
            error_config: template.error_config,
            tool_defs: vec![],
            error_workflow_id: None,
            mission_hash: None,
            created_at: now,
            updated_at: now,
        }
    };

    let mut warnings: Vec<String> = Vec::new();

    // 名称相似性检查
    if let Some(_existing) = check_workflow_duplicate(db, &workflow_name).await? {
        let new_name = format!("{} (Imported)", workflow_name);
        warnings.push(format!(
            "Workflow renamed from '{}' to '{}' due to name similarity with existing workflow",
            workflow_name, new_name
        ));
        new_template.name = new_name;
    }

    // 节点组成相似性检查
    let node_similar = find_similar_workflows(db, &new_template.nodes).await?;
    if !node_similar.is_empty() {
        warnings.push(format!(
            "Node composition {}% similar to existing workflow(s): {}",
            (node_similar[0].similarity * 100.0) as u32,
            node_similar.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }

    let active_model = model_to_active_model(&new_template);
    if let Some(tx) = tx_holder.take() {
        db_repo::insert_workflow_template(&tx, active_model).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        // n8n 分支：Role/Expert/Profile + 模板 整体提交，任一失败即回滚
        tx.commit().await.map_err(|e| format!("Commit transaction: {}", e))?;
    } else {
        db_repo::insert_workflow_template(db, active_model).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }

    // 预编译导入模板的 tool_defs：保证降级 Agent 节点声明的工具本会话内即可命中 handler
    state.work_engine.precompile_tool_defs(&new_template.id, &new_template.tool_defs).await;

    // 回灌能力索引：导入生成的是全新模板（新 ID），不索引则本会话内不可路由。
    // 两个分支（事务提交 / 直接插入）都在此汇合，放分支外保证不漏。
    sync_template_passport(state, &new_template).await;

    let mut errors: Vec<String> = Vec::new();

    if new_template.nodes.is_empty() {
        errors.push("Workflow has no nodes".to_string());
    }

    let has_trigger = new_template.nodes.iter().any(|n| matches!(n, WorkflowNode::Trigger(_)));
    if !has_trigger {
        warnings.push("Workflow has no trigger node".to_string());
    }

    let node_ids: std::collections::HashSet<String> =
        new_template.nodes.iter().map(|n| n.base_id().to_string()).collect();
    for edge in &new_template.edges {
        if !node_ids.contains(&edge.source) {
            errors.push(format!(
                "Edge '{}' references non-existent source node '{}'",
                edge.id, edge.source
            ));
        }
        if !node_ids.contains(&edge.target) {
            errors.push(format!(
                "Edge '{}' references non-existent target node '{}'",
                edge.id, edge.target
            ));
        }
    }

    if !warnings.is_empty() {
        tracing::warn!("Import validation warnings for {}: {:?}", new_template.id, warnings);
    }
    if !errors.is_empty() {
        tracing::warn!("Import validation errors for {}: {:?}", new_template.id, errors);
    }

    // 能力补全管线：为降级 Agent 声明的工具写入 workflow_tools（pending 待确认），
    // 并把关联的 Expert/Profile 登记为 Agent 能力护照，实现"导入即能力发现"。幂等。
    let capability_notes = complete_imported_capabilities(state, &new_template).await;
    warnings.extend(capability_notes);

    Ok(serde_json::json!({
        "id": new_template.id,
        "warnings": warnings,
        "errors": errors,
    }))
}

/// 能力补全管线（导入尾端）：把导入工作流中降级 Agent 节点声明的工具按 pending
/// 写入 `workflow_tools` 表（待人工确认启用），并把关联的 Expert/Profile 作为
/// `CapabilityKind::Agent` 护照登记进能力索引，实现"导入即能力发现"闭环。
///
/// 幂等约束：工具以 `(workflow_id, tool_name)` 为业务键先查后写（`upsert`），
/// 护照以 `capability_id` 去重，重复导入不会产生重复记录。
async fn complete_imported_capabilities(
    state: &AppState,
    template: &WorkflowTemplateData,
) -> Vec<String> {
    use axagent_harness::capability::{CapabilityDomain, CapabilityKind, CapabilityPassportDto};
    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp_millis();
    let mut notes: Vec<String> = Vec::new();
    let mut tool_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut profile_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut passports: Vec<CapabilityPassportDto> = Vec::new();

    for node in &template.nodes {
        let WorkflowNode::Agent(a) = node else { continue };

        // 1) 工具按 pending 落入 workflow_tools，供前端确认后启用（安全红线：不自动 active）
        for tool in &a.config.tools {
            if !tool_seen.insert(tool.name.clone()) {
                continue;
            }
            let exists =
                axagent_dao::repo::workflow_tool::get_by_name(db, &template.id, &tool.name)
                    .await
                    .ok()
                    .flatten()
                    .is_some();
            if exists {
                continue;
            }
            let ok = axagent_dao::repo::workflow_tool::upsert(
                db,
                &uuid::Uuid::new_v4().to_string(),
                &template.id,
                &tool.name,
                axagent_dao::repo::workflow_tool::TYPE_RHAI_SCRIPT,
                tool.description.as_deref(),
                Some(&n8n_placeholder_tool_script(&tool.name, &template.name)),
                None,
                "n8n-import",
                axagent_dao::repo::workflow_tool::STATUS_PENDING,
                now,
            )
            .await
            .is_ok();
            if ok {
                notes.push(format!("工具待配置（pending，需确认启用）：{}", tool.name));
            }
        }

        // 2) 关联的 Expert/Profile → Agent 能力护照，使 capability_discover 可检索
        if let Some(pid) = &a.config.agent_profile_id {
            if profile_seen.insert(pid.clone()) {
                let cap_id = format!("agent:{pid}");
                passports.push(CapabilityPassportDto {
                    capability_id: cap_id.clone(),
                    name: format!("n8n 导入专家：{pid}"),
                    description: format!(
                        "由 n8n 工作流 {} 导入自动注册的专家/角色能力",
                        template.name
                    ),
                    kind: CapabilityKind::Agent,
                    domain: CapabilityDomain::General,
                    created_at: Some(now),
                    updated_at: Some(now),
                    agent_profile_id: Some(pid.clone()),
                    tags: vec!["n8n".to_string(), "imported".to_string()],
                    ..Default::default()
                });
                notes.push(format!("专家能力已登记：{cap_id}"));
            }
        }
    }

    if !passports.is_empty() {
        state.capability_indexer.index_batch(&passports).await;
    }
    notes
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "导入工作流模板JSON")]
#[tauri::command]
pub async fn import_workflow_template(
    state: State<'_, AppState>,
    json_data: String,
) -> Result<serde_json::Value, String> {
    do_import_workflow(&state, state.harness.db(), json_data).await
}

/// 批量导入 n8n 目录中的所有工作流 JSON 文件
fn collect_json_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_json_files(&path, files);
            } else if path.extension().is_some_and(|e| e == "json") {
                files.push(path);
            }
        }
    }
}

#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "批量导入n8n目录中的工作流")]
#[tauri::command]
pub async fn import_n8n_directory(
    state: State<'_, AppState>,
    path: String,
) -> Result<serde_json::Value, String> {
    use std::fs;
    use std::path::Path;

    let db = state.harness.db();
    let dir = Path::new(&path);
    if !dir.is_dir() {
        return Err(ErrorResponse::new(workflow_err::NOT_FOUND)
            .with_detail(format!("Path does not exist or is not a directory: {}", path))
            .into());
    }

    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    let mut errors = Vec::new();
    let mut capability_notes: Vec<String> = Vec::new();

    let mut json_files: Vec<std::path::PathBuf> = Vec::new();
    collect_json_files(dir, &mut json_files);

    for file_path in json_files {
        // 单个文件读取失败只记录错误，不中断整个批量导入
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("{}: read error: {}", file_path.display(), e));
                continue;
            },
        };
        let raw_json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("{}: JSON parse error: {}", file_path.display(), e));
                continue;
            },
        };

        if !is_n8n_format(&raw_json) {
            skipped.push(file_path.display().to_string());
            continue;
        }

        let workflow_name = raw_json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Imported n8n Workflow")
            .to_string();

        if let Ok(Some(existing)) = check_workflow_duplicate(db, &workflow_name).await {
            skipped.push(format!(
                "{} (semantically similar to '{}')",
                file_path.display(),
                existing
            ));
            continue;
        }

        match convert_n8n_to_axagent(db, &raw_json).await {
            Ok(template) => {
                let am = model_to_active_model(&template);
                if let Err(e) = db_repo::insert_workflow_template(db, am).await {
                    errors.push(format!("{}: save error: {}", file_path.display(), e));
                } else {
                    // 与单文件导入 do_import_workflow 对齐：预编译工具 + 回灌能力索引 + 能力补全。
                    // 三者均借用 template，必须在 push 前调用——push 会移动 template.name。
                    state.work_engine.precompile_tool_defs(&template.id, &template.tool_defs).await;
                    sync_template_passport(&state, &template).await;
                    capability_notes
                        .extend(complete_imported_capabilities(state.inner(), &template).await);
                    imported.push(template.name);
                }
            },
            Err(e) => errors.push(format!("{}: conversion error: {}", file_path.display(), e)),
        }
    }

    Ok(serde_json::json!({
        "imported": imported.len(),
        "imported_names": imported,
        "skipped": skipped.len(),
        "skipped_reasons": skipped,
        "errors": errors.len(),
        "error_details": errors,
        "capabilities": capability_notes,
    }))
}

/// 批量导入目录下所有 JSON 工作流模板文件
#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "批量导入目录中的工作流模板")]
#[tauri::command]
pub async fn import_workflow_directory(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<serde_json::Value, String> {
    use std::fs;
    use std::path::Path;

    let db = state.harness.db();
    let dir = Path::new(&path);
    if !dir.is_dir() {
        return Err(ErrorResponse::new(workflow_err::NOT_FOUND)
            .with_detail(format!("Path does not exist or is not a directory: {}", path))
            .into());
    }

    let mut imported = Vec::new();
    let mut errors = Vec::new();

    let mut json_files: Vec<std::path::PathBuf> = Vec::new();
    collect_json_files(dir, &mut json_files);

    for file_path in json_files {
        // 单个文件读取失败只记录错误，不中断整个批量导入
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("{}: read error: {}", file_path.display(), e));
                continue;
            },
        };
        if serde_json::from_str::<serde_json::Value>(&content).is_err() {
            errors.push(format!("{}: Invalid JSON format", file_path.display()));
            continue;
        }

        match do_import_workflow(&state, db, content).await {
            Ok(val) => {
                if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                    imported.push(id.to_string());
                }
            },
            Err(e) => {
                errors.push(format!("{}: {}", file_path.display(), e));
            },
        }
    }

    Ok(serde_json::json!({
        "imported": imported.len(),
        "errors": errors.len(),
        "error_details": errors,
    }))
}

// ── P1-1: 从会话状态自动组装并持久化工作流 ────────────────────────────────────
//
// 前端/外部触发入口：用户在对话界面点"保存为工作流"按钮时调用。
// 与 SaveAsWorkflow Tool（Agent 自驱动）的区别：
// - 本命令由前端直接调用，传入 conversation_id + 可选 capability_ids
// - SaveAsWorkflow Tool 由 Agent 在会话中通过 LLM 间接调用
// 二者共享同一套 AssemblyBuilder 组装逻辑 + WorkflowTemplateRepository 持久化链路。

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDynamicWorkflowInput {
    pub conversation_id: String,
    pub name: String,
    #[serde(default)]
    pub capability_ids: Option<Vec<String>>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDynamicWorkflowResult {
    pub template_id: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub capability_count: usize,
}

#[tauri::command]
pub async fn save_dynamic_workflow(
    state: State<'_, AppState>,
    input: SaveDynamicWorkflowInput,
) -> Result<SaveDynamicWorkflowResult, String> {
    use axagent_harness::assembly_builder::{AssemblyBuilder, DefaultAssemblyBuilder};
    use axagent_harness::session_state::{NS_SKILL_LOADED, StateScope, namespace_prefix};

    let name = input.name.trim();
    if name.is_empty() {
        return Err("name 为必填参数且不可为空".to_string());
    }

    let store = &state.session_state_store;
    let indexer = &state.capability_indexer;

    // 1. 确定 capability_ids
    let capability_ids: Vec<String> = match input.capability_ids {
        Some(ids) if !ids.is_empty() => ids,
        _ => {
            let prefix =
                namespace_prefix(StateScope::Temp, NS_SKILL_LOADED, &input.conversation_id, None);
            let entries = store
                .list_by_prefix(&prefix)
                .await
                .map_err(|e| format!("读取会话状态失败: {e}"))?;

            if entries.is_empty() {
                return Err("本会话没有已加载的能力，请先通过能力加载工具加载能力。".to_string());
            }

            entries
                .iter()
                .filter_map(|e| {
                    serde_json::from_str::<serde_json::Value>(&e.value)
                        .ok()
                        .and_then(|v| v["capabilityId"].as_str().map(str::to_string))
                })
                .collect()
        },
    };

    if capability_ids.is_empty() {
        return Err("没有有效的 capability_id".to_string());
    }

    // 2. 逐个取护照
    let mut passports = Vec::new();
    for cap_id in &capability_ids {
        if let Some(passport) = indexer.get_passport(cap_id).await {
            passports.push(passport);
        }
    }

    if passports.is_empty() {
        return Err("所有 capability_id 均未找到对应能力".to_string());
    }

    // 3. 组装
    let builder = DefaultAssemblyBuilder::new().with_prefix("auto");
    let result = builder.assemble_linear(&passports);

    if result.nodes.is_empty() {
        return Err("选定的能力无法生成工作流节点".to_string());
    }

    // 4. 构造模板并持久化（复用 create_workflow_template 的数据库链路）
    let now = chrono::Utc::now().timestamp_millis();
    let template_id = uuid::Uuid::new_v4().to_string();

    let template = WorkflowTemplateData {
        id: template_id.clone(),
        name: name.to_string(),
        description: Some(format!(
            "由 save_dynamic_workflow 自动组装，包含 {} 个能力",
            passports.len()
        )),
        icon: input.icon.unwrap_or_else(|| "🧩".to_string()),
        tags: input
            .tags
            .unwrap_or_else(|| vec!["capability-assembly".to_string(), "auto-saved".to_string()]),
        cluster_id: None,
        route_path: None,
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        visibility: axagent_harness::capability::Visibility::Public,
        trigger_config: None,
        nodes: result.nodes.clone(),
        edges: result.edges.clone(),
        input_schema: None,
        output_schema: None,
        variables: Vec::new(),
        error_config: None,
        tool_defs: Vec::new(),
        error_workflow_id: None,
        mission_hash: None,
        created_at: now,
        updated_at: now,
    };

    let active_model = model_to_active_model(&template);
    let db = state.harness.db();
    db_repo::insert_workflow_template(db, active_model).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    state.work_engine.precompile_tool_defs(&template.id, &template.tool_defs).await;

    // 回灌能力索引：这是整条「能力组装 → 固化为动态工作流」闭环的落点。
    // 若不同步，固化产物在本会话内不可路由，用户固化完立刻问同样的问题，
    // 编排器仍会走原路径重新组装一遍——固化等于白做，要等重启才见效。
    sync_template_passport(&state, &template).await;

    Ok(SaveDynamicWorkflowResult {
        template_id,
        node_count: result.nodes.len(),
        edge_count: result.edges.len(),
        capability_count: passports.len(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── is_n8n_format ──────────────────────────────────────

    #[test]
    fn test_is_n8n_format_true() {
        let json = json!({
            "nodes": [
                { "type": "n8n-nodes-base.httpRequest" },
                { "type": "n8n-nodes-base.code" }
            ]
        });
        assert!(is_n8n_format(&json));
    }

    #[test]
    fn test_is_n8n_format_false_for_axagent() {
        let json = json!({
            "nodes": [
                { "type": "Agent", "id": "1" },
                { "type": "Code", "id": "2" }
            ]
        });
        assert!(!is_n8n_format(&json));
    }

    #[test]
    fn test_is_n8n_format_empty_nodes() {
        let json = json!({ "nodes": [] });
        assert!(!is_n8n_format(&json));
    }

    #[test]
    fn test_is_n8n_format_no_nodes_key() {
        let json = json!({ "other": "value" });
        assert!(!is_n8n_format(&json));
    }

    // ── infer_agent_from_n8n ────────────────────────────────

    #[test]
    fn test_infer_by_node_name_review() {
        let (profile, role, expert, prompt) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Review PR changes");
        assert_eq!(profile, "code-reviewer");
        assert_eq!(role, "reviewer");
        assert_eq!(expert, "code-reviewer");
        assert!(prompt.contains("Review"));
    }

    #[test]
    fn test_infer_by_node_name_debug() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Debug login error");
        assert_eq!(profile, "debug-expert");
        assert_eq!(role, "developer");
    }

    #[test]
    fn test_infer_by_node_name_test() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Test API endpoints");
        assert_eq!(profile, "debug-expert");
        assert_eq!(role, "reviewer");
    }

    #[test]
    fn test_infer_by_node_name_doc() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Write documentation");
        assert_eq!(profile, "tech-writer");
        assert_eq!(role, "synthesizer");
    }

    #[test]
    fn test_infer_by_node_name_plan() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Plan architecture");
        assert_eq!(profile, "architect");
        assert_eq!(role, "planner");
    }

    #[test]
    fn test_infer_by_node_name_monitor() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Monitor server health");
        assert_eq!(profile, "devops-engineer");
        assert_eq!(role, "executor");
    }

    #[test]
    fn test_infer_by_node_name_analyze() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Analyze user data");
        assert_eq!(profile, "data-analyst");
        assert_eq!(role, "researcher");
    }

    #[test]
    fn test_infer_by_node_type_http() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.httpRequest", "do something generic");
        assert_eq!(profile, "devops-engineer");
        assert_eq!(role, "executor");
    }

    #[test]
    fn test_infer_by_node_type_database() {
        let (profile, role, _, _) = infer_agent_from_n8n("n8n-nodes-base.postgres", "generic node");
        assert_eq!(profile, "sql-expert");
        assert_eq!(role, "researcher");
    }

    #[test]
    fn test_infer_by_node_type_code() {
        let (profile, role, _, _) = infer_agent_from_n8n("n8n-nodes-base.code", "generic node");
        assert_eq!(profile, "senior-developer");
        assert_eq!(role, "developer");
    }

    #[test]
    fn test_infer_by_node_type_ai() {
        let (profile, role, _, _) = infer_agent_from_n8n("n8n-nodes-base.openAi", "generic node");
        assert_eq!(profile, "general-assistant");
        assert_eq!(role, "coordinator");
    }

    #[test]
    fn test_infer_by_node_type_email() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.emailSend", "generic node");
        assert_eq!(profile, "product-manager");
        assert_eq!(role, "coordinator");
    }

    #[test]
    fn test_infer_by_node_type_file() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.spreadsheetFile", "generic node");
        assert_eq!(profile, "data-analyst");
        assert_eq!(role, "researcher");
    }

    #[test]
    fn test_infer_by_node_type_security() {
        let (profile, role, _, _) = infer_agent_from_n8n("n8n-nodes-base.oauth2", "generic node");
        assert_eq!(profile, "security-auditor");
        assert_eq!(role, "reviewer");
    }

    #[test]
    fn test_infer_by_node_type_transform() {
        let (profile, role, _, _) = infer_agent_from_n8n("n8n-nodes-base.merge", "generic node");
        assert_eq!(profile, "tech-writer");
        assert_eq!(role, "synthesizer");
    }

    #[test]
    fn test_infer_fallback_to_debug_expert() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.somethingUnknown", "unknown node");
        assert_eq!(profile, "debug-expert");
        assert_eq!(role, "executor");
    }

    #[test]
    fn test_infer_name_has_priority_over_type() {
        // node name "review" should match before node type "http"
        let (profile, _role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.httpRequest", "review API response");
        assert_eq!(profile, "code-reviewer");
        // 确认名称关键词 "review" 优先级高于节点类型 "http" — 映射到 code-reviewer 而非 devops-engineer
    }

    // ── extract_goal_from_n8n ───────────────────────────────

    #[test]
    fn test_extract_goal_http_node() {
        let node = json!({
            "type": "n8n-nodes-base.httpRequest",
            "parameters": {
                "method": "GET",
                "url": "https://api.example.com/users"
            }
        });
        let goal = extract_goal_from_n8n(&node);
        assert!(goal.contains("GET"));
        assert!(goal.contains("api.example.com"));
    }

    #[test]
    fn test_extract_goal_database_node() {
        let node = json!({
            "type": "n8n-nodes-base.sqlite",
            "parameters": {
                "operation": "SELECT",
                "table": "orders"
            }
        });
        let goal = extract_goal_from_n8n(&node);
        assert!(goal.contains("SELECT"));
        assert!(goal.contains("orders"));
    }

    #[test]
    fn test_extract_goal_email_node() {
        let node = json!({
            "type": "n8n-nodes-base.emailSend",
            "parameters": {
                "subject": "Weekly Report"
            }
        });
        let goal = extract_goal_from_n8n(&node);
        assert!(goal.contains("Weekly Report"));
    }

    #[test]
    fn test_extract_goal_empty_node() {
        let node = json!({});
        let goal = extract_goal_from_n8n(&node);
        // 无任何字段时返回 "Unnamed ()"
        assert!(!goal.is_empty());
        assert!(goal.starts_with("Unnamed"));
    }

    // ── n8n_node_kind / n8n_source_handle ──────────────────

    #[test]
    fn test_n8n_node_kind_classification() {
        assert_eq!(n8n_node_kind("n8n-nodes-base.webhook"), N8nNodeKind::Trigger);
        assert_eq!(n8n_node_kind("n8n-nodes-base.if"), N8nNodeKind::Condition);
        assert_eq!(n8n_node_kind("n8n-nodes-base.switch"), N8nNodeKind::Switch);
        assert_eq!(n8n_node_kind("n8n-nodes-base.httpRequest"), N8nNodeKind::Other);
    }

    #[test]
    fn test_n8n_source_handle_if_true_false() {
        // IF 节点：main[0]=true / main[1]=false
        let (h0, t0) = n8n_source_handle(N8nNodeKind::Condition, 0);
        assert_eq!(h0.as_deref(), Some("true"));
        assert!(matches!(t0, EdgeType::ConditionTrue));
        let (h1, t1) = n8n_source_handle(N8nNodeKind::Condition, 1);
        assert_eq!(h1.as_deref(), Some("false"));
        assert!(matches!(t1, EdgeType::ConditionFalse));
    }

    #[test]
    fn test_n8n_source_handle_switch_branch() {
        let (h, t) = n8n_source_handle(N8nNodeKind::Switch, 2);
        assert_eq!(h.as_deref(), Some("branch-2"));
        assert!(matches!(t, EdgeType::ParallelBranch));
    }

    #[test]
    fn test_n8n_source_handle_other_none() {
        let (h, t) = n8n_source_handle(N8nNodeKind::Other, 0);
        assert!(h.is_none());
        assert!(matches!(t, EdgeType::Direct));
    }

    // ── parse_n8n_position ─────────────────────────────────

    #[test]
    fn test_parse_n8n_position_array() {
        let pos = json!([120.0, 340.0]);
        let p = parse_n8n_position(&pos);
        assert_eq!(p.x, 120.0);
        assert_eq!(p.y, 340.0);
    }

    #[test]
    fn test_parse_n8n_position_object() {
        let pos = json!({ "x": 12.0, "y": 34.0 });
        let p = parse_n8n_position(&pos);
        assert_eq!(p.x, 12.0);
        assert_eq!(p.y, 34.0);
    }

    // ── map_n8n_node 精确映射 ─────────────────────────────

    fn test_base(id: &str) -> WorkflowNodeBase {
        WorkflowNodeBase {
            continue_on_fail: false,
            compensation: None,
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            position: Position { x: 0.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
        }
    }

    #[test]
    fn test_map_n8n_http_request() {
        let node = json!({
            "type": "n8n-nodes-base.httpRequest",
            "parameters": {
                "url": "https://api.example.com/data",
                "method": "POST",
                "sendHeaders": true,
                "headerParameters": { "parameters": [ { "name": "Authorization", "value": "Bearer x" } ] }
            }
        });
        let mapped = map_n8n_node(test_base("n1"), &node, "n8n-nodes-base.httpRequest").unwrap();
        match mapped {
            WorkflowNode::HttpRequest(t) => {
                assert_eq!(t.config.url, "https://api.example.com/data");
                assert_eq!(t.config.method, "POST");
                assert_eq!(
                    t.config.headers.get("Authorization").map(String::as_str),
                    Some("Bearer x")
                );
            },
            _ => panic!("expected HttpRequest"),
        }
    }

    #[test]
    fn test_map_n8n_if_conditions() {
        let node = json!({
            "type": "n8n-nodes-base.if",
            "parameters": {
                "conditions": {
                    "combinator": "and",
                    "conditions": [
                        {
                            "leftValue": "={{ $json.status }}",
                            "rightValue": "active",
                            "operator": { "type": "string", "operation": "equals" }
                        }
                    ]
                }
            }
        });
        let mapped = map_n8n_node(test_base("n1"), &node, "n8n-nodes-base.if").unwrap();
        match mapped {
            WorkflowNode::Condition(t) => {
                assert_eq!(t.config.conditions.len(), 1);
                assert_eq!(t.config.conditions[0].var_path, "$json.status");
                assert_eq!(
                    t.config.conditions[0].value,
                    serde_json::Value::String("active".to_string())
                );
                assert!(matches!(t.config.conditions[0].operator, CompareOperator::Eq));
            },
            _ => panic!("expected Condition"),
        }
    }

    #[test]
    fn test_map_n8n_switch_cases() {
        let node = json!({
            "type": "n8n-nodes-base.switch",
            "parameters": {
                "input1": "={{ $json.type }}",
                "rules": {
                    "values": [
                        { "outputIndex": 0, "value": "a" },
                        { "outputIndex": 1, "value": "b" }
                    ]
                }
            }
        });
        let mapped = map_n8n_node(test_base("n1"), &node, "n8n-nodes-base.switch").unwrap();
        match mapped {
            WorkflowNode::Switch(t) => {
                assert_eq!(t.config.input_var, "$json.type");
                assert_eq!(t.config.cases.len(), 2);
                assert_eq!(t.config.cases[0].value, "a");
                assert_eq!(t.config.cases[1].value, "b");
            },
            _ => panic!("expected Switch"),
        }
    }

    #[test]
    fn test_map_n8n_unknown_returns_none() {
        let node = json!({ "type": "n8n-nodes-base.unknownNode" });
        assert!(map_n8n_node(test_base("n1"), &node, "n8n-nodes-base.unknownNode").is_none());
    }
}
