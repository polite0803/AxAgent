// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业命令 — 命令直读行业 adapter（与股票业务同架构，无运行时容器）
//!
//! 宏观要求：OPC 行业与股票业务**同架构**。股票业务 = 引擎 + 命令直调
//! （`search_stock` / `get_stock_quote` 直接调 astock 引擎）；OPC 行业同理 =
//! 内建手写 adapter（Rust 硬编码）+ 命令直读。**没有 opc-runtime / registry / adapter 注册表**。
//!
//! 每个命令：`IndustryAdapterFactory::create` 创建内建手写 adapter（校验/KPI/规则逻辑
//! 硬编码在 Rust，对齐股票「配置硬编码在 Rust」）→ 注入 `DefaultDataService` → 执行。
//! 行业包 yaml（`config/opc/industries/*/`）仅用于工作流模板 seed 与 Phase 1 数据源配置，
//! **不驱动业务逻辑**（v3.0 已废弃 DataDrivenIndustryAdapter / runtime.yaml 数据驱动）。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_macro::agent_command;
use tauri::State;

use axagent_analysis_engine::opc::industry::IndustryAdapterFactory;
use axagent_analysis_engine::opc::workflow::IndustryWorkflowManager;
use axagent_analysis_engine::opc::*;
use axagent_dao::db::DatabaseConnection;

use crate::AppState;
use crate::commands::opc_industry_logic;

// ── 行业适配器构造（工厂统一创建内建手写 adapter，脱离 yaml） ──

/// 定位行业包目录：`{industries_dir}/{industry_id}`（app_dir 优先 → 仓库根 fallback）
///
/// 仅用于 Phase 1 数据接入（读取行业包 `analysis.yaml` 数据源配置）。
/// 业务逻辑（validate / KPI / 工作流 / 规则 / 仪表盘）一律走内建 adapter，不依赖本目录。
fn industry_dir(app_dir: Option<&Path>, industry_id: &str) -> Result<PathBuf, String> {
    let base = crate::commands::opc_workflows::resolve_industries_dir(app_dir);
    let dir = base.join(industry_id);
    if dir.is_dir() {
        Ok(dir)
    } else {
        Err(format!("行业包不存在: {industry_id}"))
    }
}

/// 通过工厂创建行业适配器（内建手写逻辑）并注入数据服务
fn load_adapter(
    db: &DatabaseConnection,
    industry_id: &str,
) -> Result<Arc<dyn OpcIndustryAdapter>, String> {
    let adapter = IndustryAdapterFactory::create(industry_id)
        .ok_or_else(|| format!("行业适配器不存在: {industry_id}"))?;
    adapter.set_data_service(Arc::new(DefaultDataService::new(db.clone())));
    Ok(adapter)
}

// ── 公共 API（内部函数，命令直读行业包） ───────────────────────

/// 验证行业实体
pub async fn validate_entity(
    db: &DatabaseConnection,
    industry_id: &str,
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> Result<Vec<ValidationError>, String> {
    let adapter = load_adapter(db, industry_id)?;
    adapter.validate(entity_type, entity_data).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 批量验证行业实体
pub async fn validate_batch(
    db: &DatabaseConnection,
    industry_id: &str,
    entities: &[(String, serde_json::Value)],
) -> Result<Vec<(String, Vec<ValidationError>)>, String> {
    let adapter = load_adapter(db, industry_id)?;
    adapter.validate_batch(entities).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 计算行业 KPI 指标
pub async fn compute_kpis(
    db: &DatabaseConnection,
    industry_id: &str,
    time_range: TimeRange,
) -> Result<Vec<KpiValue>, String> {
    let adapter = load_adapter(db, industry_id)?;
    adapter.compute_kpis(&time_range).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取行业 KPI 定义列表
pub fn get_kpi_definitions(industry_id: &str) -> Result<Vec<KpiDefinition>, String> {
    let adapter = IndustryAdapterFactory::create(industry_id)
        .ok_or_else(|| format!("行业适配器不存在: {industry_id}"))?;
    Ok(adapter.kpi_definitions())
}

/// 获取行业工作流步骤
pub fn get_workflow_steps(industry_id: &str) -> Result<Vec<WorkflowStep>, String> {
    let adapter = IndustryAdapterFactory::create(industry_id)
        .ok_or_else(|| format!("行业适配器不存在: {industry_id}"))?;
    let mut steps = adapter.workflow_steps();
    steps.sort_by_key(|s| s.order);
    Ok(steps)
}

/// 获取行业启用的自动化规则
pub fn get_enabled_rules(industry_id: &str) -> Result<Vec<IndustryAutomationRule>, String> {
    let adapter = IndustryAdapterFactory::create(industry_id)
        .ok_or_else(|| format!("行业适配器不存在: {industry_id}"))?;
    Ok(adapter.automation_rules().into_iter().filter(|r| r.enabled).collect())
}

/// 运行行业自动化规则（通用条件求值 + 动作执行）
pub async fn run_automation_rules(
    db: &DatabaseConnection,
    industry_id: &str,
    context: RuleContext,
) -> Result<Vec<String>, String> {
    let adapter = load_adapter(db, industry_id)?;
    let rules = adapter.automation_rules().into_iter().filter(|r| r.enabled).collect::<Vec<_>>();
    let ctx_map = opc_industry_logic::context_to_hashmap(&context);
    let ds = adapter.data_service();
    let mut triggered = Vec::new();
    for rule in &rules {
        if opc_industry_logic::evaluate_conditions(&rule.conditions, &ctx_map) {
            opc_industry_logic::execute_rule_actions(ds.as_ref(), rule, &context).await.map_err(
                |e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                },
            )?;
            triggered.push(rule.id.clone());
        }
    }
    Ok(triggered)
}

/// 获取行业仪表盘数据
pub async fn get_dashboard(
    db: &DatabaseConnection,
    industry_id: &str,
    time_range: TimeRange,
) -> Result<IndustryDashboard, String> {
    let adapter = load_adapter(db, industry_id)?;
    adapter.aggregate_dashboard(&time_range).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 执行行业动态工作流（DB 模板优先，无则一次性种子化后执行）
///
/// 所有 DAG 均来自 DB（种子化 + 用户可编辑），不再运行时动态生成。
pub async fn execute_dynamic_workflow(
    db: &DatabaseConnection,
    engine: &Arc<axagent_runtime::work_engine::WorkEngine>,
    industry_id: &str,
    days: u32,
    user_input: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let industry_id_normalized = industry_id.replace('-', "_");
    let harness_template_id = format!("{industry_id_normalized}_harness_workflow");

    // 1. 首选：行业 harness 模板（seed 时写入 DB，用户可在编辑器修改）
    if let Some(result) = crate::commands::opc_industry_actions::run_template_via_engine(
        db,
        engine,
        industry_id,
        &harness_template_id,
        days,
        user_input.clone(),
    )
    .await?
    {
        return Ok(result);
    }

    // 2. 兜底：从 adapter 一次性种子化到 DB（用户之后可编辑），再走 rt-workflow
    tracing::warn!("[opc-dynamic] 模板 {} 不存在，从 adapter 种子化后执行", harness_template_id);
    let adapter = load_adapter(db, industry_id)?;
    let mut manager = IndustryWorkflowManager::new();
    let workflow = manager.create_or_update(&industry_id_normalized, adapter.as_ref()).clone();
    let template_data = workflow.to_template_data();
    crate::commands::opc_workflows::upsert_template(db, template_data).await?;

    crate::commands::opc_industry_actions::run_template_via_engine(
        db,
        engine,
        industry_id,
        &harness_template_id,
        days,
        user_input,
    )
    .await?
    .ok_or_else(|| format!("工作流种子化失败: {harness_template_id}"))
}

/// 列出全部内建行业（工厂注册）
pub fn list_industries() -> Vec<(String, String)> {
    IndustryAdapterFactory::list_all()
        .into_iter()
        .map(|(id, name)| (id.to_string(), name.to_string()))
        .collect()
}

/// 检查行业是否存在（工厂注册）
pub fn has_industry(industry_id: &str) -> bool {
    IndustryAdapterFactory::create(industry_id).is_some()
}

// ── Tauri 命令（签名保持前端契约；app_state 由 Tauri 自动注入） ──

/// 验证行业实体（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "验证行业实体")]
#[tauri::command]
pub async fn opc_validate_entity(
    app_state: State<'_, AppState>,
    industry_id: String,
    entity_type: String,
    entity_data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let db = app_state.harness.db();
    let errors = validate_entity(db, &industry_id, &entity_type, &entity_data).await?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "entityType": entity_type,
        "valid": errors.is_empty(),
        "errors": errors,
    }))
}

/// 计算行业 KPI（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "计算行业KPI")]
#[tauri::command]
pub async fn opc_compute_kpis(
    app_state: State<'_, AppState>,
    industry_id: String,
    days: Option<i64>,
) -> Result<serde_json::Value, String> {
    let range = match days {
        Some(d) => TimeRange::days(d),
        None => TimeRange::days(30),
    };
    let db = app_state.harness.db();
    let kpis = compute_kpis(db, &industry_id, range).await?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "kpis": kpis,
    }))
}

/// 运行行业自动化规则（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "运行行业自动化规则")]
#[tauri::command]
pub async fn opc_run_automation_rules(
    app_state: State<'_, AppState>,
    industry_id: String,
    entity_type: String,
    entity_id: String,
    status: Option<String>,
    overdue_days: Option<u32>,
    created_days: Option<u32>,
    fields: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let mut ctx = RuleContext::new(&entity_type, &entity_id);
    if let Some(s) = status {
        ctx = ctx.with_status(s);
    }
    if let Some(d) = overdue_days {
        ctx = ctx.with_overdue_days(d);
    }
    if let Some(d) = created_days {
        ctx = ctx.with_created_days(d);
    }
    if let Some(f) = fields {
        ctx.fields = f;
    }
    let db = app_state.harness.db();
    let triggered = run_automation_rules(db, &industry_id, ctx).await?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "triggeredRules": triggered,
        "triggeredCount": triggered.len(),
    }))
}

/// 获取行业仪表盘（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "获取行业仪表盘")]
#[tauri::command]
pub async fn opc_get_industry_dashboard(
    app_state: State<'_, AppState>,
    industry_id: String,
    days: Option<i64>,
) -> Result<serde_json::Value, String> {
    let range = match days {
        Some(d) => TimeRange::days(d),
        None => TimeRange::days(30),
    };
    let db = app_state.harness.db();
    let dashboard = get_dashboard(db, &industry_id, range).await?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "dashboard": dashboard,
    }))
}

/// 列出全部行业包（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateOnly, description = "列出全部行业包")]
#[tauri::command]
pub async fn opc_list_runtime_industries(
    _app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let industries = list_industries();
    Ok(serde_json::json!({
        "count": industries.len(),
        "industries": industries.into_iter().map(|(id, name)| {
            serde_json::json!({ "id": id, "name": name })
        }).collect::<Vec<_>>(),
    }))
}

/// 获取行业工作流步骤（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "获取行业工作流步骤")]
#[tauri::command]
pub async fn opc_get_industry_workflow_steps(
    _app_state: State<'_, AppState>,
    industry_id: String,
) -> Result<serde_json::Value, String> {
    let steps = get_workflow_steps(&industry_id)?;
    let step_infos: Vec<serde_json::Value> = steps
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "description": s.description,
                "order": s.order,
                "status": "pending",
            })
        })
        .collect();
    Ok(serde_json::json!({
        "industryId": industry_id,
        "steps": step_infos,
    }))
}

/// 获取行业自动化规则（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "获取行业自动化规则")]
#[tauri::command]
pub async fn opc_get_industry_automation_rules(
    _app_state: State<'_, AppState>,
    industry_id: String,
) -> Result<serde_json::Value, String> {
    let rules = get_enabled_rules(&industry_id)?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "rules": rules,
    }))
}

/// 批量验证行业实体（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "批量验证行业实体")]
#[tauri::command]
pub async fn opc_batch_validate_entities(
    app_state: State<'_, AppState>,
    industry_id: String,
    entities: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let pairs: Vec<(String, serde_json::Value)> = entities
        .into_iter()
        .filter_map(|e| {
            let t = e.get("entityType").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let d = e.get("data").cloned().unwrap_or(serde_json::Value::Null);
            if t.is_empty() { None } else { Some((t, d)) }
        })
        .collect();
    let db = app_state.harness.db();
    let results = validate_batch(db, &industry_id, &pairs).await?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "results": results.into_iter().map(|(t, errs)| {
            serde_json::json!({ "entityType": t, "valid": errs.is_empty(), "errors": errs })
        }).collect::<Vec<_>>(),
    }))
}

/// 获取行业 KPI 定义（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "获取行业KPI定义")]
#[tauri::command]
pub async fn opc_get_kpi_definitions(
    _app_state: State<'_, AppState>,
    industry_id: String,
) -> Result<serde_json::Value, String> {
    let definitions = get_kpi_definitions(&industry_id)?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "definitions": definitions,
    }))
}

/// 检查行业包是否存在（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "检查行业包是否存在")]
#[tauri::command]
pub async fn opc_has_industry(
    _app_state: State<'_, AppState>,
    industry_id: String,
) -> Result<serde_json::Value, String> {
    let exists = has_industry(&industry_id);
    Ok(serde_json::json!({
        "industryId": industry_id,
        "exists": exists,
    }))
}

// ── Phase 1 数据接入命令（OpIndustryClient 直读行业包 analysis.yaml） ──

/// 构造行业数据客户端（db/cache/web/file 内建 vendor，无容器）
fn build_data_client(app_state: &AppState, industry_id: &str) -> Result<OpIndustryClient, String> {
    let dir = industry_dir(Some(&app_state.app_data_dir), industry_id)?;
    let config = crate::commands::opc_data::load_analysis_config(&dir)?;
    let db = app_state.harness.db();
    let mut vendors: std::collections::HashMap<String, std::sync::Arc<dyn OpIndustryVendor>> =
        std::collections::HashMap::new();
    let db_vendor = DbVendor::new(std::sync::Arc::new(DefaultDataService::new(db.clone())));
    vendors.insert("db".to_string(), std::sync::Arc::new(db_vendor));
    let cache_vendor = CacheVendor::new(app_state.app_data_dir.join("opc-cache"));
    vendors.insert("cache".to_string(), std::sync::Arc::new(cache_vendor));
    vendors.insert("web".to_string(), std::sync::Arc::new(WebVendor));
    vendors.insert("file".to_string(), std::sync::Arc::new(FileVendor));

    let sources: Vec<AnalysisDataSource> = config
        .data_sources
        .iter()
        .map(|s| AnalysisDataSource {
            id: s.id.clone(),
            chain: s.chain.clone(),
            quality_precheck: s.quality_precheck,
        })
        .collect();

    Ok(OpIndustryClient::new(industry_id.to_string(), sources, vendors))
}

/// 获取行业数据（Phase 1：按 analysis.yaml data_sources 路由 + 降级）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "获取行业数据")]
#[tauri::command]
pub async fn opc_get_industry_data(
    app_state: State<'_, AppState>,
    industry_id: String,
    source_id: String,
    data_domain: String,
    query: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let client = build_data_client(&app_state, &industry_id)?;
    let data = client
        .fetch(&source_id, &data_domain, &query.unwrap_or(serde_json::json!({})))
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "sourceId": source_id,
        "data": data,
    }))
}

/// 获取行业数据质量预检（Phase 1：quality_precheck 源清单探测）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "获取行业数据质量预检")]
#[tauri::command]
pub async fn opc_get_industry_precheck(
    app_state: State<'_, AppState>,
    industry_id: String,
) -> Result<serde_json::Value, String> {
    let client = build_data_client(&app_state, &industry_id)?;
    let precheck = client.precheck().await;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "precheck": precheck,
    }))
}

/// 获取行业数据源健康状态（Phase 1：vendor 降级可观测）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "获取行业数据源健康状态")]
#[tauri::command]
pub async fn opc_get_industry_health(
    app_state: State<'_, AppState>,
    industry_id: String,
) -> Result<serde_json::Value, String> {
    let client = build_data_client(&app_state, &industry_id)?;
    let health = client.health_snapshot();
    Ok(serde_json::json!({
        "industryId": industry_id,
        "health": health,
    }))
}

/// 执行行业动态工作流（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "执行行业动态工作流")]
#[tauri::command]
pub async fn opc_execute_dynamic_workflow(
    app_state: State<'_, AppState>,
    industry_id: String,
    days: Option<i64>,
) -> Result<serde_json::Value, String> {
    let days = days.unwrap_or(30) as u32;
    let db = app_state.harness.db();
    let engine = Arc::clone(&app_state.work_engine);
    let result = execute_dynamic_workflow(db, &engine, &industry_id, days, None).await?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "workflowExecution": result,
    }))
}
