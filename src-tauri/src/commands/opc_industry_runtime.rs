// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业命令 — 直读行业配置和服务（与股票业务同架构）
//!
//! 所有业务逻辑通过独立的 Service 实现，不再依赖行业适配器。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axagent_agent_macro::agent_command;
use tauri::State;

use axagent_analysis_engine::opc::*;
use axagent_dao::db::DatabaseConnection;

use crate::AppState;
use crate::commands::opc_industry_logic;

/// 定位行业包目录：`{industries_dir}/{industry_id}`
///
/// 仅用于 Phase 1 数据接入（读取行业包 `analysis.yaml` 数据源配置）。
fn industry_dir(app_dir: Option<&Path>, industry_id: &str) -> Result<PathBuf, String> {
    let base = crate::commands::opc_workflows::resolve_industries_dir(app_dir);
    let dir = base.join(industry_id);
    if dir.is_dir() {
        Ok(dir)
    } else {
        Err(format!("行业包不存在: {industry_id}"))
    }
}

// ── 公共 API（内部函数） ───────────────────────────────────────

/// 验证行业实体
pub async fn validate_entity(
    db: &DatabaseConnection,
    industry_id: &str,
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> Result<Vec<ValidationError>, String> {
    let errors = industry_validator::validate_entity(industry_id, entity_type, entity_data)
        .await
        .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let _ = db; // 参数保留，未来可能用于数据库验证
    Ok(errors)
}

/// 批量验证行业实体
pub async fn validate_batch(
    db: &DatabaseConnection,
    industry_id: &str,
    entities: &[(String, serde_json::Value)],
) -> Result<Vec<(String, Vec<ValidationError>)>, String> {
    let results = industry_validator::validate_batch(industry_id, entities).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let _ = db;
    Ok(results)
}

/// 计算行业 KPI 指标
pub async fn compute_kpis(
    db: &DatabaseConnection,
    industry_id: &str,
    time_range: TimeRange,
) -> Result<Vec<KpiValue>, String> {
    let data_service: Arc<dyn OpcDataService> = Arc::new(DefaultDataService::new(db.clone()));
    industry_kpi_service::compute_kpis(industry_id, &data_service, &time_range).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取行业 KPI 定义列表
pub fn get_kpi_definitions(industry_id: &str) -> Result<Vec<KpiDefinition>, String> {
    let config = industry_config::get_config(industry_id)
        .ok_or_else(|| format!("行业配置不存在: {}", industry_id))?;
    Ok(config.kpi_definitions)
}

/// 获取行业工作流步骤（从配置获取）
pub fn get_workflow_steps(industry_id: &str) -> Result<Vec<WorkflowStep>, String> {
    // 从 seed 文件定义的工作流节点获取，不再使用适配器
    // 这里返回空列表，实际工作流步骤由前端从 template 表加载
    let _ = industry_id;
    Ok(Vec::new())
}

/// 获取行业启用的自动化规则
pub fn get_enabled_rules(industry_id: &str) -> Result<Vec<IndustryAutomationRule>, String> {
    let config = industry_config::get_config(industry_id)
        .ok_or_else(|| format!("行业配置不存在: {}", industry_id))?;
    Ok(config.automation_rules.into_iter().filter(|r| r.enabled).collect())
}

/// 运行行业自动化规则（通用条件求值 + 动作执行）
pub async fn run_automation_rules(
    db: &DatabaseConnection,
    industry_id: &str,
    context: RuleContext,
) -> Result<Vec<String>, String> {
    let config = industry_config::get_config(industry_id)
        .ok_or_else(|| format!("行业配置不存在: {}", industry_id))?;
    let rules = config.automation_rules.into_iter().filter(|r| r.enabled).collect::<Vec<_>>();
    let ctx_map = opc_industry_logic::context_to_hashmap(&context);
    let data_service: Arc<dyn OpcDataService> = Arc::new(DefaultDataService::new(db.clone()));
    let mut triggered = Vec::new();
    for rule in &rules {
        if opc_industry_logic::evaluate_conditions(&rule.conditions, &ctx_map) {
            opc_industry_logic::execute_rule_actions(Some(&data_service), rule, &context)
                .await
                .map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
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
    let config = industry_config::get_config(industry_id)
        .ok_or_else(|| format!("行业配置不存在: {}", industry_id))?;
    let kpis = compute_kpis(db, industry_id, time_range).await?;

    let cards = config
        .dashboard_cards
        .into_iter()
        .map(|c| DashboardCard::new(&c.id, &c.title, &c.kpi_key, ""))
        .collect();

    Ok(IndustryDashboard { industry_id: industry_id.to_string(), kpis, cards, summary: None })
}

/// 列出全部内建行业（从配置获取）
pub fn list_industries() -> Vec<(String, String)> {
    industry_config::list_industries()
}

/// 检查行业是否存在（从配置获取）
pub fn has_industry(industry_id: &str) -> bool {
    industry_config::get_config(industry_id).is_some()
}

// ── Tauri 命令（签名保持前端契约；app_state 由 Tauri 自动注入） ──

/// 验证行业实体（Tauri 命令）
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "验证行业实体")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "计算行业KPI")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "运行行业自动化规则")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取行业仪表盘")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateOnly, description = "列出全部行业包")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取行业工作流步骤")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取行业自动化规则")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "批量验证行业实体")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取行业KPI定义")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "检查行业包是否存在")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取行业数据")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取行业数据质量预检")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取行业数据源健康状态")]
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

/// 动态工作流执行（兼容旧接口）
///
/// 新架构下所有工作流均通过种子化到 DB → WorkEngine 执行，
/// 此函数将旧的动态执行请求转发到标准执行通道。
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "动态工作流执行（兼容旧接口）")]
#[tauri::command]
pub async fn opc_execute_dynamic_workflow(
    app_state: State<'_, AppState>,
    industry_id: String,
    workflow_id: Option<String>,
    days: Option<u32>,
    user_input: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    crate::commands::opc_industry_actions::opc_execute_workflow(
        app_state,
        industry_id,
        workflow_id,
        days,
        user_input,
    )
    .await
}
