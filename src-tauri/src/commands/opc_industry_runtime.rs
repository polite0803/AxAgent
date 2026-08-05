// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业运行时 — 基于行业适配器的动态业务逻辑
//!
//! 本模块提供行业运行时的初始化和业务操作封装。
//! 采用 `OnceLock + RwLock` 模式存储全局运行时实例，
//! 可被 `opc_industry_bridge` 等模块调用。
//!
//! # 架构
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │          opc_industry_runtime               │
//! │  (全局 OnceLock<RwLock<Option<OpcRuntime>>>) │
//! └──────────┬──────────────────────────────────┘
//!            │ Arc<OpcRuntime>
//!            ▼
//! ┌─────────────────────────────────────────────┐
//! │  OpcRuntime                                 │
//! │  ├── IndustryAdapterRegistry (9 个行业)     │
//! │  └── DefaultDataService (数据库访问)         │
//! └─────────────────────────────────────────────┘
//! ```

use std::sync::Arc;
use std::sync::OnceLock;

use agent_macro::agent_command;
use tauri::State;
use tokio::sync::RwLock;

use axagent_opc_dao::DefaultDataService;
use axagent_opc_industries::register_all_industries;
use axagent_opc_runtime::{IndustryAdapterRegistry, OpcRuntime};
use axagent_opc_types::{
    IndustryAutomationRule, IndustryDashboard, KpiDefinition, KpiValue, OpcDataService,
    RuleContext, TimeRange, ValidationError, WorkflowStep,
};

// ── 全局运行时存储 ──────────────────────────────────────────────

/// 全局行业运行时状态
///
/// 使用 `OnceLock` 保证全局唯一，`RwLock` 支持异步读取/写入。
/// 初始化前为 `None`，调用 `init_runtime()` 后变为 `Some`。
static RUNTIME: OnceLock<RwLock<Option<Arc<OpcRuntime>>>> = OnceLock::new();

/// 获取运行时读写锁引用
fn runtime_lock() -> &'static RwLock<Option<Arc<OpcRuntime>>> {
    RUNTIME.get_or_init(|| RwLock::new(None))
}

// ── 初始化 ──────────────────────────────────────────────────

/// 初始化行业运行时
///
/// 执行以下步骤：
/// 1. 创建 `IndustryAdapterRegistry`
/// 2. 调用 `register_all_industries` 注册全部 9 个行业适配器
/// 3. 创建 `DefaultDataService` 并注入运行时
/// 4. 存储到全局状态
///
/// # 幂等
///
/// 多次调用仅首次生效，后续调用直接返回已有实例。
pub async fn init_runtime(db: sea_orm::DatabaseConnection) -> Result<Arc<OpcRuntime>, String> {
    {
        let lock = runtime_lock().read().await;
        if let Some(existing) = lock.as_ref() {
            tracing::debug!(
                "[opc-industry-runtime] 运行时已初始化，跳过 (行业数={})",
                existing.registry().list_ids().await.len()
            );
            return Ok(existing.clone());
        }
    }

    let registry = Arc::new(IndustryAdapterRegistry::new());

    register_all_industries(&registry).await;

    let data_service: Arc<dyn OpcDataService> = Arc::new(DefaultDataService::new(db));

    let runtime = Arc::new(OpcRuntime::new(registry).with_data_service(data_service).await);

    {
        let mut lock = runtime_lock().write().await;
        *lock = Some(runtime.clone());
    }

    let ids = runtime.registry().list_ids().await;
    tracing::info!("[opc-industry-runtime] 行业运行时初始化完成: {} 个行业已注册", ids.len());

    Ok(runtime)
}

// ── 内部工具 ──────────────────────────────────────────────────

/// 获取已初始化的运行时实例
///
/// # Errors
///
/// 返回 `Err` 当运行时尚未初始化。
async fn get_runtime() -> Result<Arc<OpcRuntime>, String> {
    let lock = runtime_lock().read().await;
    lock.clone().ok_or_else(|| "行业运行时未初始化，请先调用 opc_init_industry_runtime".to_string())
}

// ── 公共 API ──────────────────────────────────────────────────

/// 验证行业实体
///
/// 根据 `industry_id` 找到对应适配器，执行行业特有校验逻辑。
pub async fn validate_entity(
    industry_id: &str,
    entity_type: &str,
    entity_data: &serde_json::Value,
) -> Result<Vec<ValidationError>, String> {
    let runtime = get_runtime().await?;
    runtime.validate_entity(industry_id, entity_type, entity_data).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 批量验证行业实体
pub async fn validate_batch(
    industry_id: &str,
    entities: &[(String, serde_json::Value)],
) -> Result<Vec<(String, Vec<ValidationError>)>, String> {
    let runtime = get_runtime().await?;
    runtime.validate_batch(industry_id, entities).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 计算行业 KPI 指标
pub async fn compute_kpis(
    industry_id: &str,
    time_range: TimeRange,
) -> Result<Vec<KpiValue>, String> {
    let runtime = get_runtime().await?;
    runtime.compute_kpis(industry_id, &time_range).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取行业 KPI 定义列表
pub async fn get_kpi_definitions(industry_id: &str) -> Result<Vec<KpiDefinition>, String> {
    let runtime = get_runtime().await?;
    runtime.get_kpi_definitions(industry_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取行业工作流步骤
pub async fn get_workflow_steps(industry_id: &str) -> Result<Vec<WorkflowStep>, String> {
    let runtime = get_runtime().await?;
    runtime.get_workflow_steps(industry_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取行业启用的自动化规则
pub async fn get_enabled_rules(industry_id: &str) -> Result<Vec<IndustryAutomationRule>, String> {
    let runtime = get_runtime().await?;
    runtime.get_enabled_rules(industry_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 运行行业自动化规则
///
/// 评估并执行指定行业的所有启用规则，返回被触发的规则 ID 列表。
pub async fn run_automation_rules(
    industry_id: &str,
    context: RuleContext,
) -> Result<Vec<String>, String> {
    let runtime = get_runtime().await?;
    runtime.run_all_rules(industry_id, &context).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取行业仪表盘数据
pub async fn get_dashboard(
    industry_id: &str,
    time_range: TimeRange,
) -> Result<IndustryDashboard, String> {
    let runtime = get_runtime().await?;
    runtime.get_industry_dashboard(industry_id, &time_range).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 列出所有已注册行业
pub async fn list_industries() -> Result<Vec<(String, String)>, String> {
    let runtime = get_runtime().await?;
    Ok(runtime.registry().list_all().await)
}

/// 检查行业是否已注册
pub async fn has_industry(industry_id: &str) -> Result<bool, String> {
    let runtime = get_runtime().await?;
    Ok(runtime.registry().contains(industry_id).await)
}

/// 重置运行时（测试/调试用）
///
/// 清除全局运行时状态，下次调用 `init_runtime` 将重新初始化。
pub async fn reset_runtime() {
    let mut lock = runtime_lock().write().await;
    *lock = None;
    tracing::warn!("[opc-industry-runtime] 行业运行时已重置");
}

// ── Tauri 命令 ────────────────────────────────────────────────
//
// 以下 Tauri 命令暴露行业运行时能力给前端，
// 同时允许 Agent 通过 tool bridge 调用。

/// 初始化行业运行时（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "初始化行业运行时")]
#[tauri::command]
pub async fn opc_init_industry_runtime(
    app_state: State<'_, crate::AppState>,
) -> Result<serde_json::Value, String> {
    let db = app_state.harness.db().clone();

    let runtime = init_runtime(db).await?;

    let industries = runtime.registry().list_all().await;
    Ok(serde_json::json!({
        "status": "initialized",
        "industryCount": industries.len(),
        "industries": industries.into_iter().map(|(id, name)| {
            serde_json::json!({ "id": id, "name": name })
        }).collect::<Vec<_>>(),
    }))
}

/// 验证行业实体（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "验证行业实体")]
#[tauri::command]
pub async fn opc_validate_entity(
    industry_id: String,
    entity_type: String,
    entity_data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let errors = validate_entity(&industry_id, &entity_type, &entity_data).await?;
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
    industry_id: String,
    days: Option<i64>,
) -> Result<serde_json::Value, String> {
    let range = match days {
        Some(d) => TimeRange::days(d),
        None => TimeRange::days(30),
    };
    let kpis = compute_kpis(&industry_id, range).await?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "kpis": kpis,
    }))
}

/// 运行行业自动化规则（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "运行行业自动化规则")]
#[tauri::command]
pub async fn opc_run_automation_rules(
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

    let triggered = run_automation_rules(&industry_id, ctx).await?;
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
    industry_id: String,
    days: Option<i64>,
) -> Result<serde_json::Value, String> {
    let range = match days {
        Some(d) => TimeRange::days(d),
        None => TimeRange::days(30),
    };
    let dashboard = get_dashboard(&industry_id, range).await?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "dashboard": dashboard,
    }))
}

/// 列出所有已注册行业（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "列出行业运行时所有已注册行业")]
#[tauri::command]
pub async fn opc_list_runtime_industries() -> Result<serde_json::Value, String> {
    let industries = list_industries().await?;
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
    industry_id: String,
) -> Result<serde_json::Value, String> {
    let steps = get_workflow_steps(&industry_id).await?;
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
    industry_id: String,
) -> Result<serde_json::Value, String> {
    let rules = get_enabled_rules(&industry_id).await?;
    let rule_infos: Vec<serde_json::Value> = rules
        .into_iter()
        .map(|r| {
            let conditions: Vec<serde_json::Value> = r
                .conditions
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "type": format!("{:?}", c),
                        "config": serde_json::json!({}),
                    })
                })
                .collect();
            let actions: Vec<serde_json::Value> = r
                .actions
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "type": format!("{:?}", a),
                        "config": serde_json::json!({}),
                    })
                })
                .collect();
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "enabled": r.enabled,
                "conditions": conditions,
                "actions": actions,
            })
        })
        .collect();
    Ok(serde_json::json!({
        "industryId": industry_id,
        "rules": rule_infos,
    }))
}

/// 批量验证行业实体（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "批量验证行业实体")]
#[tauri::command]
pub async fn opc_batch_validate_entities(
    industry_id: String,
    entities: Vec<(String, serde_json::Value)>,
) -> Result<serde_json::Value, String> {
    let results = validate_batch(&industry_id, &entities).await?;
    let items: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(entity_id, errors)| {
            serde_json::json!({
                "entityId": entity_id,
                "valid": errors.is_empty(),
                "errors": errors,
            })
        })
        .collect();
    Ok(serde_json::json!({
        "industryId": industry_id,
        "results": items,
    }))
}

/// 获取行业 KPI 定义（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "获取行业KPI定义")]
#[tauri::command]
pub async fn opc_get_kpi_definitions(industry_id: String) -> Result<serde_json::Value, String> {
    let definitions = get_kpi_definitions(&industry_id).await?;
    let defs: Vec<serde_json::Value> = definitions
        .into_iter()
        .map(|d| {
            serde_json::json!({
                "key": d.key,
                "name": d.name,
                "unit": d.unit,
                "metricType": format!("{:?}", d.metric_type),
            })
        })
        .collect();
    Ok(serde_json::json!({
        "industryId": industry_id,
        "definitions": defs,
    }))
}

/// 检查行业是否已注册（Tauri 命令）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "检查行业是否已注册")]
#[tauri::command]
pub async fn opc_has_industry(industry_id: String) -> Result<serde_json::Value, String> {
    let exists = has_industry(&industry_id).await?;
    Ok(serde_json::json!({
        "industryId": industry_id,
        "registered": exists,
    }))
}

/// 重置行业运行时（Tauri 命令，仅限调试）
#[agent_command(domain = "opc", safety = Caution, call_mode = StateOnly, description = "重置行业运行时")]
#[tauri::command]
pub async fn opc_reset_industry_runtime() -> Result<serde_json::Value, String> {
    reset_runtime().await;
    Ok(serde_json::json!({
        "status": "reset",
    }))
}
