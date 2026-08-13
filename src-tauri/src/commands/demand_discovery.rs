// SPDX-License-Identifier: AGPL-3.0-only

//! 需求发现（Demand Discovery）领域 Tauri 命令层
//!
//! 暴露能力扫描、市场线索发现、需求确认、交付工作流执行等核心命令。

use agent_macro::agent_command;
use sea_orm::sea_query::Expr;
use tauri::State;

use crate::AppState;
use crate::commands::error::ErrorResponse;

// ── 能力扫描 ──────────────────────────────────────────────────

/// 扫描系统当前可用的能力清单（工具/技能/MCP/工作流）并落库到 opc_capability
#[agent_command(domain = "opc", safety = Safe, call_mode = StateOnly, description = "扫描系统能力清单")]
#[tauri::command]
pub async fn opc_scan_capabilities(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use axagent_analysis_engine::opc::capability::{
        CapabilityEntry, CapabilityInventory, CapabilitySource,
    };
    use axagent_entities::opc_capability;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    // 1) 工具 + MCP 工具（来自 UnifiedToolRegistry）
    let mut tools: Vec<CapabilityEntry> = Vec::new();
    let mut mcp_tools: Vec<CapabilityEntry> = Vec::new();
    {
        let registry = state.local_tool_registry.lock().await;
        for info in registry.tools.list_all() {
            tools.push(CapabilityEntry {
                id: format!("tool:{}", info.name),
                name: info.name.clone(),
                description: info.description.clone(),
                source: CapabilitySource::Tool,
                source_id: info.name.clone(),
                capability_type: format!("{:?}", info.category).to_lowercase(),
                applicable_scenarios: Vec::new(),
                example_deliverables: Vec::new(),
                metadata: serde_json::json!({
                    "enabled": info.enabled,
                    "domain": format!("{:?}", info.domain),
                }),
            });
        }
        for (key, cfg) in registry.mcp.mcp_tools.iter() {
            let desc = cfg.description.as_deref().unwrap_or("").to_string();
            mcp_tools.push(CapabilityEntry {
                id: format!("mcp:{}", key),
                name: cfg.tool_name.clone(),
                description: desc,
                source: CapabilitySource::McpTool,
                source_id: key.clone(),
                capability_type: "mcp_tool".to_string(),
                applicable_scenarios: Vec::new(),
                example_deliverables: Vec::new(),
                metadata: serde_json::json!({
                    "server_id": cfg.server_id,
                    "server_name": cfg.server_name,
                }),
            });
        }
    }

    // 2) 已启用技能（skill_states 表）
    let mut skills: Vec<CapabilityEntry> = Vec::new();
    if let Ok(enabled) = axagent_dao::repo::skill::get_enabled_skills(db).await {
        for name in enabled {
            skills.push(CapabilityEntry {
                id: format!("skill:{}", name),
                name: name.clone(),
                description: format!("技能: {name}（已启用）"),
                source: CapabilitySource::Skill,
                source_id: name,
                capability_type: "skill".to_string(),
                applicable_scenarios: Vec::new(),
                example_deliverables: Vec::new(),
                metadata: serde_json::json!({}),
            });
        }
    }

    // 3) 工作流模板（workflow_template 表）
    let mut workflows: Vec<CapabilityEntry> = Vec::new();
    if let Ok(templates) =
        axagent_dao::repo::workflow_template::list_workflow_templates(db, None).await
    {
        for tmpl in templates {
            workflows.push(CapabilityEntry {
                id: format!("workflow:{}", tmpl.id),
                name: tmpl.name.clone(),
                description: tmpl.description.unwrap_or_default(),
                source: CapabilitySource::Workflow,
                source_id: tmpl.id.clone(),
                capability_type: "workflow".to_string(),
                applicable_scenarios: Vec::new(),
                example_deliverables: Vec::new(),
                metadata: serde_json::json!({
                    "version": tmpl.version,
                    "is_preset": tmpl.is_preset,
                }),
            });
        }
    }

    let mut inv = CapabilityInventory {
        tools,
        skills,
        mcp_tools,
        workflows,
        scanned_at: now,
        total_count: 0,
    };
    inv.recalc_count();

    // 4) 落库到 opc_capability（先软删旧快照，再写入新快照）
    let _ = opc_capability::Entity::update_many()
        .col_expr(opc_capability::Column::IsActive, Expr::value(0))
        .exec(db)
        .await;

    for entry in inv.all_entries() {
        let _ = opc_capability::ActiveModel {
            id: Set(entry.id.clone()),
            source_type: Set(entry.source.as_str().to_string()),
            source_id: Set(entry.source_id.clone()),
            name: Set(entry.name.clone()),
            description: Set(entry.description.clone()),
            capability_type: Set(entry.capability_type.clone()),
            applicable_scenarios_json: Set(
                serde_json::to_string(&entry.applicable_scenarios).unwrap_or_default()
            ),
            example_deliverables_json: Set(
                serde_json::to_string(&entry.example_deliverables).unwrap_or_default()
            ),
            metadata_json: Set(serde_json::to_string(&entry.metadata).unwrap_or_default()),
            is_active: Set(1),
            scanned_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await;
    }

    serde_json::to_value(&inv).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

// ── 市场需求发现 ──────────────────────────────────────────────

/// 按关键词搜索市场平台需求线索（闲鱼、猪八戒等）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "搜索市场需求线索")]
#[tauri::command]
pub async fn opc_discover_leads(
    state: State<'_, AppState>,
    query: String,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_market_platform;
    use axagent_tools::tools::marketplace_scanner::AggregateMarketplaceScanner;
    use sea_orm::*;

    let db = state.harness.db();

    // 从平台配置加载已启用的平台连接器
    let mut scanner = AggregateMarketplaceScanner::new();
    let platforms = opc_market_platform::Entity::find()
        .filter(opc_market_platform::Column::Enabled.eq(1))
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?;

    for p in platforms {
        let config: serde_json::Value =
            serde_json::from_str(&p.config_json).unwrap_or(serde_json::json!({}));
        scanner.add_platform(&p.name, &p.platform_type, p.base_url.as_deref(), &config);
    }

    let leads = scanner.search_all(&query).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    // 记录平台最近同步时间
    let now = chrono::Utc::now().timestamp();
    let _ = opc_market_platform::Entity::update_many()
        .col_expr(opc_market_platform::Column::LastSyncAt, Expr::value(now))
        .col_expr(opc_market_platform::Column::Status, Expr::value("synced"))
        .col_expr(opc_market_platform::Column::UpdatedAt, Expr::value(now))
        .exec(db)
        .await;

    serde_json::to_value(&leads).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 一体化需求发现：扫描 → 智能评估 → 入库
///
/// 扫描多平台需求线索，自动进行价值评估（规则引擎 + 可选 LLM），
/// 将评估结果直接写入 opc_demand_lead 表。
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "扫描并评估需求线索")]
#[tauri::command]
pub async fn opc_discover_and_evaluate_leads(
    state: State<'_, AppState>,
    query: String,
    min_score: Option<f64>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_demand_lead;
    use axagent_entities::opc_market_platform;
    use axagent_tools::tools::marketplace_scanner::AggregateMarketplaceScanner;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    // 1) 从平台配置加载已启用的平台连接器
    let mut scanner = AggregateMarketplaceScanner::new();
    let platforms = opc_market_platform::Entity::find()
        .filter(opc_market_platform::Column::Enabled.eq(1))
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?;

    for p in platforms {
        let config: serde_json::Value =
            serde_json::from_str(&p.config_json).unwrap_or(serde_json::json!({}));
        scanner.add_platform(&p.name, &p.platform_type, p.base_url.as_deref(), &config);
    }

    // 2) 执行「扫描 + 评估」一体化流水线
    let evaluated = scanner.search_and_evaluate(&query, None).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    // 3) 可选：按价值分阈值筛选
    let min_threshold = min_score.unwrap_or(0.0);
    let filtered: Vec<_> =
        evaluated.into_iter().filter(|e| e.value_score() >= min_threshold).collect();

    // 4) 将评估结果入库
    let mut saved_leads: Vec<opc_demand_lead::Model> = Vec::new();
    for el in &filtered {
        let demand_type_str = el.evaluation.demand_type.as_str().to_string();
        let entity = opc_demand_lead::ActiveModel {
            id: Set(el.lead.id.clone()),
            platform: Set(el.lead.platform.clone()),
            title: Set(el.lead.title.clone()),
            description: Set(el.lead.description.clone()),
            budget_min: Set(el.lead.budget_min),
            budget_max: Set(el.lead.budget_max),
            budget_currency: Set(el.lead.budget_currency.clone()),
            contact_name: Set(el.lead.contact_name.clone()),
            contact_email: Set(el.lead.contact_email.clone()),
            contact_phone: Set(el.lead.contact_phone.clone()),
            source_url: Set(el.lead.source_url.clone()),
            raw_snapshot_json: Set(serde_json::to_string(&el.lead.raw_snapshot).unwrap_or_default()),
            matched_capabilities_json: Set("[]".to_string()),
            ai_analysis_json: Set(serde_json::to_string(&el.evaluation).unwrap_or_default()),
            recommended_workflow_id: Set(None),
            status: Set("new".to_string()),
            priority: Set(3),
            confidence: Set(el.evaluation.confidence),
            notes: Set(String::new()),
            project_id: Set(None),
            customer_id: Set(None),
            expires_at: Set(None),
            claimed_by: Set(None),
            // 需求价值评估字段
            pain_score: Set(el.evaluation.pain_score),
            market_gap_score: Set(el.evaluation.market_gap_score),
            commercial_value_score: Set(el.evaluation.commercial_value_score),
            opportunity_level: Set(el.evaluation.opportunity_level.clone()),
            demand_type: Set(demand_type_str),
            evaluated_at: Set(Some(now)),
            created_at: Set(now),
            updated_at: Set(now),
        };

        match entity.insert(db).await {
            Ok(model) => saved_leads.push(model),
            Err(e) => {
                tracing::warn!("[opc_discover_and_evaluate_leads] 入库失败 {}: {}", el.lead.id, e);
            },
        }
    }

    // 5) 记录平台最近同步时间
    let _ = opc_market_platform::Entity::update_many()
        .col_expr(opc_market_platform::Column::LastSyncAt, Expr::value(now))
        .col_expr(opc_market_platform::Column::Status, Expr::value("synced"))
        .col_expr(opc_market_platform::Column::UpdatedAt, Expr::value(now))
        .exec(db)
        .await;

    // 6) 返回结果（含统计信息）
    let total_scanned = filtered.len();
    let high_value_count = saved_leads.iter().filter(|l| l.commercial_value_score >= 70.0).count();

    let result = serde_json::json!({
        "total_scanned": total_scanned,
        "saved_count": saved_leads.len(),
        "high_value_count": high_value_count,
        "leads": saved_leads,
    });

    serde_json::to_value(&result).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

// ── Cron 路由辅助函数 ───────────────────────────────────────────

/// 需求发现定时任务执行函数
///
/// 供 CronExecutor 调用，执行「扫描 → 评估 → 入库」完整流水线。
/// 与 `opc_discover_and_evaluate_leads` 命令共享核心逻辑，
/// 但不依赖 Tauri State，可在任意上下文中执行。
///
/// # 参数
/// - `db`: 数据库连接
/// - `query`: 搜索关键词
/// - `app_handle`: Tauri AppHandle（用于发送桌面通知，可选）
pub async fn run_demand_discovery_cron(
    db: &sea_orm::DatabaseConnection,
    query: &str,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<String, String> {
    use axagent_entities::opc_demand_lead;
    use axagent_entities::opc_market_platform;
    use axagent_tools::tools::marketplace_scanner::AggregateMarketplaceScanner;
    use sea_orm::*;

    let now = chrono::Utc::now().timestamp();

    // 1) 加载已启用的平台连接器
    let mut scanner = AggregateMarketplaceScanner::new();
    let platforms = opc_market_platform::Entity::find()
        .filter(opc_market_platform::Column::Enabled.eq(1))
        .all(db)
        .await
        .map_err(|e| format!("加载平台配置失败: {e}"))?;

    for p in platforms {
        let config: serde_json::Value =
            serde_json::from_str(&p.config_json).unwrap_or(serde_json::json!({}));
        scanner.add_platform(&p.name, &p.platform_type, p.base_url.as_deref(), &config);
    }

    // 2) 执行「扫描 + 评估」
    let evaluated =
        scanner.search_and_evaluate(query, None).await.map_err(|e| format!("需求扫描失败: {e}"))?;

    // 3) 入库 + 收集高价值需求信息
    let mut saved_count = 0usize;
    let mut high_value_count = 0usize;
    let mut high_value_leads: Vec<(String, f64, String)> = Vec::new(); // (id, score, title)

    for el in &evaluated {
        let demand_type_str = el.evaluation.demand_type.as_str().to_string();
        let is_high_value = el.evaluation.commercial_value_score >= 70.0;

        let entity = opc_demand_lead::ActiveModel {
            id: Set(el.lead.id.clone()),
            platform: Set(el.lead.platform.clone()),
            title: Set(el.lead.title.clone()),
            description: Set(el.lead.description.clone()),
            budget_min: Set(el.lead.budget_min),
            budget_max: Set(el.lead.budget_max),
            budget_currency: Set(el.lead.budget_currency.clone()),
            contact_name: Set(el.lead.contact_name.clone()),
            contact_email: Set(el.lead.contact_email.clone()),
            contact_phone: Set(el.lead.contact_phone.clone()),
            source_url: Set(el.lead.source_url.clone()),
            raw_snapshot_json: Set(serde_json::to_string(&el.lead.raw_snapshot).unwrap_or_default()),
            matched_capabilities_json: Set("[]".to_string()),
            ai_analysis_json: Set(serde_json::to_string(&el.evaluation).unwrap_or_default()),
            recommended_workflow_id: Set(None),
            status: Set(if is_high_value { "high_value" } else { "new" }.to_string()),
            priority: Set(if is_high_value { 1 } else { 3 }),
            confidence: Set(el.evaluation.confidence),
            notes: Set(String::new()),
            project_id: Set(None),
            customer_id: Set(None),
            expires_at: Set(None),
            claimed_by: Set(None),
            pain_score: Set(el.evaluation.pain_score),
            market_gap_score: Set(el.evaluation.market_gap_score),
            commercial_value_score: Set(el.evaluation.commercial_value_score),
            opportunity_level: Set(el.evaluation.opportunity_level.clone()),
            demand_type: Set(demand_type_str),
            evaluated_at: Set(Some(now)),
            created_at: Set(now),
            updated_at: Set(now),
        };

        match entity.insert(db).await {
            Ok(_) => {
                saved_count += 1;
                if is_high_value {
                    high_value_count += 1;
                    high_value_leads.push((
                        el.lead.id.clone(),
                        el.evaluation.commercial_value_score,
                        el.lead.title.clone(),
                    ));
                }
            },
            Err(e) => {
                tracing::warn!("[run_demand_discovery_cron] 入库失败 {}: {}", el.lead.id, e);
            },
        }
    }

    // 4) 更新平台同步时间
    let _ = opc_market_platform::Entity::update_many()
        .col_expr(opc_market_platform::Column::LastSyncAt, Expr::value(now))
        .col_expr(opc_market_platform::Column::Status, Expr::value("synced"))
        .col_expr(opc_market_platform::Column::UpdatedAt, Expr::value(now))
        .exec(db)
        .await;

    // 5) 发送高价值需求通知
    if high_value_count > 0 {
        send_high_value_notification(app_handle, &high_value_leads).await;
    }

    Ok(format!(
        "需求发现完成: 扫描 {} 条, 入库 {} 条, 高价值 {} 条",
        evaluated.len(),
        saved_count,
        high_value_count
    ))
}

/// 发送高价值需求通知
///
/// 通过 Tauri 桌面通知 + 前端事件推送，提醒用户关注高价值需求。
async fn send_high_value_notification(
    app_handle: Option<&tauri::AppHandle>,
    high_value_leads: &[(String, f64, String)],
) {
    if high_value_leads.is_empty() {
        return;
    }

    let count = high_value_leads.len();
    let titles: Vec<String> = high_value_leads
        .iter()
        .take(3)
        .map(|(_, score, title)| format!("{} (评分: {:.1})", title, score))
        .collect();

    let body = if count > 3 {
        format!("{} 条高价值需求: {} ...等", count, titles.join(", "))
    } else {
        format!("{} 条高价值需求: {}", count, titles.join(", "))
    };

    // 发送 Tauri 桌面通知
    if let Some(app) = app_handle {
        if let Err(e) = crate::commands::desktop::send_desktop_notification(
            app.clone(),
            "🔔 OPC 需求发现：发现高价值需求".to_string(),
            body.clone(),
        )
        .await
        {
            tracing::warn!("[DemandDiscovery] 桌面通知发送失败: {}", e);
        }
    }

    // 同时通过日志记录，便于排查
    tracing::info!("[DemandDiscovery] 高价值需求通知: 发现 {} 条高价值需求", count);
    for (id, score, title) in high_value_leads {
        tracing::info!(
            "[DemandDiscovery] 高价值需求详情: id={}, score={:.1}, title={}",
            id,
            score,
            title
        );
    }
}

// ── 需求线索 CRUD ──────────────────────────────────────────────

/// 创建需求线索（手动补录或从平台线索转化）
#[agent_command(domain = "opc", safety = Caution, call_mode = StateInput, description = "创建需求线索")]
#[tauri::command]
pub async fn opc_create_lead(
    state: State<'_, AppState>,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_demand_lead;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    let id = input
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(&format!("dl-{}", uuid::Uuid::new_v4().simple()))
        .to_string();

    let title = input.get("title").and_then(|v| v.as_str()).unwrap_or("未命名需求").to_string();

    let description = input.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let platform = input.get("platform").and_then(|v| v.as_str()).unwrap_or("manual").to_string();

    let status = input.get("status").and_then(|v| v.as_str()).unwrap_or("new").to_string();

    let raw_snapshot = input.get("raw_snapshot").cloned().unwrap_or(serde_json::json!({}));

    let ai_analysis = input.get("ai_analysis").cloned().unwrap_or(serde_json::json!({}));

    let matched_capabilities =
        input.get("matched_capabilities").cloned().unwrap_or(serde_json::json!([]));

    let entity = opc_demand_lead::ActiveModel {
        id: Set(id),
        platform: Set(platform),
        title: Set(title),
        description: Set(description),
        budget_min: Set(input.get("budget_min").and_then(|v| v.as_f64())),
        budget_max: Set(input.get("budget_max").and_then(|v| v.as_f64())),
        budget_currency: Set(input
            .get("budget_currency")
            .and_then(|v| v.as_str())
            .unwrap_or("CNY")
            .to_string()),
        contact_name: Set(input
            .get("contact_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())),
        contact_email: Set(input
            .get("contact_email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())),
        contact_phone: Set(input
            .get("contact_phone")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())),
        source_url: Set(input.get("source_url").and_then(|v| v.as_str()).map(|s| s.to_string())),
        raw_snapshot_json: Set(serde_json::to_string(&raw_snapshot).unwrap_or_default()),
        matched_capabilities_json: Set(
            serde_json::to_string(&matched_capabilities).unwrap_or_default()
        ),
        ai_analysis_json: Set(serde_json::to_string(&ai_analysis).unwrap_or_default()),
        recommended_workflow_id: Set(input
            .get("recommended_workflow_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())),
        status: Set(status),
        priority: Set(input.get("priority").and_then(|v| v.as_i64()).unwrap_or(3) as i32),
        confidence: Set(input.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0)),
        notes: Set(input.get("notes").and_then(|v| v.as_str()).unwrap_or("").to_string()),
        project_id: Set(input.get("project_id").and_then(|v| v.as_str()).map(|s| s.to_string())),
        customer_id: Set(input.get("customer_id").and_then(|v| v.as_str()).map(|s| s.to_string())),
        expires_at: Set(input.get("expires_at").and_then(|v| v.as_i64())),
        claimed_by: Set(input.get("claimed_by").and_then(|v| v.as_str()).map(|s| s.to_string())),
        // 需求价值评估字段（v222 新增）
        pain_score: Set(input.get("pain_score").and_then(|v| v.as_f64()).unwrap_or(0.0)),
        market_gap_score: Set(input
            .get("market_gap_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)),
        commercial_value_score: Set(input
            .get("commercial_value_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)),
        opportunity_level: Set(input
            .get("opportunity_level")
            .and_then(|v| v.as_str())
            .unwrap_or("low")
            .to_string()),
        demand_type: Set(input
            .get("demand_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string()),
        evaluated_at: Set(input.get("evaluated_at").and_then(|v| v.as_i64())),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved = entity.insert(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    serde_json::to_value(&saved).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 列出所有需求线索（支持按状态/平台过滤）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "列出需求线索")]
#[tauri::command]
pub async fn opc_list_leads(
    state: State<'_, AppState>,
    status: Option<String>,
    platform: Option<String>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_demand_lead;
    use sea_orm::*;

    let db = state.harness.db();
    let mut qs = opc_demand_lead::Entity::find();

    if let Some(ref s) = status {
        qs = qs.filter(opc_demand_lead::Column::Status.eq(s));
    }
    if let Some(ref p) = platform {
        qs = qs.filter(opc_demand_lead::Column::Platform.eq(p));
    }

    let results =
        qs.order_by_desc(opc_demand_lead::Column::CreatedAt).all(db).await.map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?;

    serde_json::to_value(&results).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 确认需求线索（标记为 qualified，进入执行管道）
#[agent_command(domain = "opc", safety = Caution, call_mode = StateInput, description = "确认需求线索")]
#[tauri::command]
pub async fn opc_confirm_lead(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_demand_lead;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    let result = opc_demand_lead::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("需求线索不存在: {id}"))?;

    let mut am: opc_demand_lead::ActiveModel = result.into();
    am.status = Set("qualified".to_string());
    am.updated_at = Set(now);

    let saved = am.update(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    serde_json::to_value(&saved).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 为需求线索匹配能力（调用 AnalysisEngine 进行匹配）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "为需求匹配能力")]
#[tauri::command]
pub async fn opc_match_lead_capabilities(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_demand_lead;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    let result = opc_demand_lead::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("需求线索不存在: {id}"))?;

    let inv =
        axagent_analysis_engine::opc::capability::CapabilityService::scan_capabilities().await;

    // 优先使用落库的能力快照（opc_capability 表），避免扫描空实现
    let mut pool: Vec<axagent_analysis_engine::opc::capability::CapabilityEntry> = Vec::new();
    if let Ok(rows) = axagent_entities::opc_capability::Entity::find()
        .filter(axagent_entities::opc_capability::Column::IsActive.eq(1))
        .all(db)
        .await
    {
        for row in rows {
            pool.push(axagent_analysis_engine::opc::capability::CapabilityEntry {
                id: row.id.clone(),
                name: row.name.clone(),
                description: row.description.clone(),
                source: match row.source_type.as_str() {
                    "skill" => axagent_analysis_engine::opc::capability::CapabilitySource::Skill,
                    "mcp_tool" => {
                        axagent_analysis_engine::opc::capability::CapabilitySource::McpTool
                    },
                    "workflow" => {
                        axagent_analysis_engine::opc::capability::CapabilitySource::Workflow
                    },
                    _ => axagent_analysis_engine::opc::capability::CapabilitySource::Tool,
                },
                source_id: row.source_id.clone(),
                capability_type: row.capability_type.clone(),
                applicable_scenarios: Vec::new(),
                example_deliverables: Vec::new(),
                metadata: serde_json::from_str(&row.metadata_json).unwrap_or(serde_json::json!({})),
            });
        }
    }
    if pool.is_empty() {
        pool = inv.all_entries().into_iter().cloned().collect();
    }

    let description_lower = result.description.to_lowercase();
    let title_lower = result.title.to_lowercase();

    let mut matched: Vec<serde_json::Value> = Vec::new();
    let mut score_sum: f64 = 0.0;
    let mut hit_count: usize = 0;

    for entry in &pool {
        let entry_text = format!("{} {} {}", entry.name, entry.description, entry.capability_type)
            .to_lowercase();
        let mut score: f64 = 0.0;

        for word in description_lower.split_whitespace().chain(title_lower.split_whitespace()) {
            if word.len() >= 2 && entry_text.contains(word) {
                score += 0.1;
            }
        }

        if score > 0.0 {
            score = (score).min(1.0);
            matched.push(serde_json::json!({
                "id": entry.id,
                "name": entry.name,
                "source": entry.source.as_str(),
                "score": score,
            }));
            score_sum += score;
            hit_count += 1;
        }
    }

    let confidence = if hit_count > 0 {
        (score_sum / hit_count as f64).min(1.0)
    } else {
        0.0
    };

    matched.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["score"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 能力缺口落库：热门/高价值需求但匹配能力不足时，记录缺口
    if hit_count == 0 {
        let gap_id = format!("gap-{}", uuid::Uuid::new_v4().simple());
        let _ = axagent_entities::opc_capability_gap::ActiveModel {
            id: Set(gap_id),
            lead_id: Set(Some(id.clone())),
            title: Set(format!("能力缺口: {}", result.title)),
            description: Set(format!("需求『{}』未匹配到任何现有能力", result.title)),
            missing_capability: Set(result.title.clone()),
            gap_type: Set("capability".to_string()),
            suggested_action: Set("为该需求新增对应工具/技能或工作流模板，再重新匹配".to_string()),
            priority: Set(result.priority),
            status: Set("open".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            closed_at: Set(None),
        }
        .insert(db)
        .await;
    }

    let mut am: opc_demand_lead::ActiveModel = result.into();
    am.matched_capabilities_json = Set(serde_json::to_string(&matched).unwrap_or_default());
    am.confidence = Set(confidence);
    am.updated_at = Set(now);

    let saved = am.update(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    serde_json::to_value(&saved).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

// ── 平台配置 CRUD ──────────────────────────────────────────────

/// 列出所有平台连接器配置（自动确保预置平台存在）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateOnly, description = "列出市场平台配置")]
#[tauri::command]
pub async fn opc_list_platforms(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_market_platform;
    use sea_orm::*;

    let db = state.harness.db();

    // 确保预置平台种子数据存在
    let _ = axagent_dao::repo::market_platform::ensure_preset_platforms(db).await;

    let results = opc_market_platform::Entity::find()
        .order_by_desc(opc_market_platform::Column::Enabled)
        .order_by_asc(opc_market_platform::Column::Name)
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?;

    serde_json::to_value(&results).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 保存（新增或更新）平台连接器配置
#[agent_command(domain = "opc", safety = Caution, call_mode = StateInput, description = "保存平台配置")]
#[tauri::command]
pub async fn opc_save_platform(
    state: State<'_, AppState>,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_market_platform;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    let id = input
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(&format!("mp-{}", uuid::Uuid::new_v4().simple()))
        .to_string();

    let existing = opc_market_platform::Entity::find_by_id(&id).one(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    let name = input.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let platform_type =
        input.get("platform_type").and_then(|v| v.as_str()).unwrap_or("manual").to_string();
    let enabled = input.get("enabled").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    let base_url = input.get("base_url").and_then(|v| v.as_str()).map(|s| s.to_string());
    let config = input.get("config").cloned().unwrap_or(serde_json::json!({}));

    if let Some(existing) = existing {
        let mut am: opc_market_platform::ActiveModel = existing.into();
        am.name = Set(name);
        am.platform_type = Set(platform_type);
        am.enabled = Set(enabled);
        am.base_url = Set(base_url);
        am.config_json = Set(serde_json::to_string(&config).unwrap_or_default());
        am.updated_at = Set(now);
        let saved = am.update(db).await.map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?;
        return serde_json::to_value(&saved).map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        });
    }

    let entity = opc_market_platform::ActiveModel {
        id: Set(id),
        name: Set(name),
        platform_type: Set(platform_type),
        enabled: Set(enabled),
        base_url: Set(base_url),
        config_json: Set(serde_json::to_string(&config).unwrap_or_default()),
        last_sync_at: Set(None),
        status: Set("idle".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved = entity.insert(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    serde_json::to_value(&saved).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 删除平台连接器配置
#[agent_command(domain = "opc", safety = Caution, call_mode = StateInput, description = "删除平台配置")]
#[tauri::command]
pub async fn opc_delete_platform(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_market_platform;
    use sea_orm::*;

    let db = state.harness.db();
    let result = opc_market_platform::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("平台配置不存在: {id}"))?;

    result.delete(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    serde_json::to_value(serde_json::json!({ "deleted": true, "id": id })).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

// ── 能力缺口 ──────────────────────────────────────────────────

/// 列出能力缺口记录（可按状态过滤）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateOnly, description = "列出能力缺口")]
#[tauri::command]
pub async fn opc_list_capability_gaps(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_capability_gap;
    use sea_orm::*;

    let db = state.harness.db();
    let mut qs = opc_capability_gap::Entity::find();
    if let Some(ref s) = status {
        qs = qs.filter(opc_capability_gap::Column::Status.eq(s));
    }

    let results =
        qs.order_by_desc(opc_capability_gap::Column::CreatedAt).all(db).await.map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?;

    serde_json::to_value(&results).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 关闭能力缺口（能力建设完成后标记 resolved）
#[agent_command(domain = "opc", safety = Caution, call_mode = StateInput, description = "关闭能力缺口")]
#[tauri::command]
pub async fn opc_close_capability_gap(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_capability_gap;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    let result = opc_capability_gap::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("能力缺口不存在: {id}"))?;

    let mut am: opc_capability_gap::ActiveModel = result.into();
    am.status = Set("resolved".to_string());
    am.closed_at = Set(Some(now));
    am.updated_at = Set(now);

    let saved = am.update(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    serde_json::to_value(&saved).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

// ── 状态标记 ──────────────────────────────────────────────────

/// 标记需求线索状态（expired 过期 / claimed 他人承接 / cancelled 取消等）
#[agent_command(domain = "opc", safety = Caution, call_mode = StateInput, description = "标记需求线索状态")]
#[tauri::command]
pub async fn opc_mark_lead_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_demand_lead;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    let result = opc_demand_lead::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("需求线索不存在: {id}"))?;

    let mut am: opc_demand_lead::ActiveModel = result.into();
    am.status = Set(status.clone());
    if status == "claimed" {
        am.claimed_by = Set(Some("other".to_string()));
    }
    am.updated_at = Set(now);

    let saved = am.update(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    serde_json::to_value(&saved).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 执行需求交付工作流（为 confirmed 需求创建交付记录并触发工作流）
#[agent_command(domain = "opc", safety = Caution, call_mode = StateInput, description = "执行需求交付")]
#[tauri::command]
pub async fn opc_execute_demand_workflow(
    state: State<'_, AppState>,
    lead_id: String,
    workflow_template_id: Option<String>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_delivery;
    use axagent_entities::opc_demand_lead;
    use sea_orm::*;

    let db_owned = state.harness.db().clone();
    let db = &db_owned;
    let now = chrono::Utc::now().timestamp();

    let lead = opc_demand_lead::Entity::find_by_id(&lead_id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("需求线索不存在: {lead_id}"))?;

    let delivery_id = format!("dv-{}", uuid::Uuid::new_v4().simple());

    let template_id = workflow_template_id.unwrap_or_else(|| {
        lead.recommended_workflow_id
            .clone()
            .unwrap_or_else(|| "default_demand_delivery".to_string())
    });

    let entity = opc_delivery::ActiveModel {
        id: Set(delivery_id.clone()),
        lead_id: Set(Some(lead_id.clone())),
        project_id: Set(lead.project_id.clone()),
        customer_id: Set(lead.customer_id.clone()),
        title: Set(format!("交付: {}", lead.title)),
        workflow_template_id: Set(template_id.clone()),
        description: Set(lead.description.clone()),
        status: Set("pending".to_string()),
        progress: Set(0.0),
        started_at: Set(Some(now)),
        completed_at: Set(None),
        result_summary: Set(None),
        deliverables_json: Set("[]".to_string()),
        errors_json: Set("[]".to_string()),
        metadata_json: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved = entity.insert(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    // 将 lead 状态置为 executing
    let mut lead_am: opc_demand_lead::ActiveModel = lead.into();
    lead_am.status = Set("executing".to_string());
    lead_am.updated_at = Set(now);
    let _ = lead_am.update(db).await;

    // 真正调用工作流引擎下发（异步后台执行，不阻塞交付记录创建）
    let engine = std::sync::Arc::clone(&state.work_engine);
    let delivery_id_for_task = delivery_id.clone();
    let lead_title = saved.title.clone();
    let lead_desc = saved.description.clone();
    let lead_id_for_task = lead_id.clone();

    use axagent_harness::workflow_types::Variable;
    use axagent_rt_workflow::work_engine::RunOptions;

    let variables = vec![
        Variable {
            name: "lead_id".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(lead_id_for_task.clone()),
            description: Some("需求线索 ID".into()),
            is_secret: false,
        },
        Variable {
            name: "delivery_id".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(delivery_id_for_task.clone()),
            description: Some("交付记录 ID".into()),
            is_secret: false,
        },
        Variable {
            name: "demand_title".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(lead_title.clone()),
            description: Some("需求标题".into()),
            is_secret: false,
        },
        Variable {
            name: "demand_description".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(lead_desc.clone()),
            description: Some("需求描述".into()),
            is_secret: false,
        },
    ];

    let opts = RunOptions {
        max_concurrent: 2,
        step_timeout: std::time::Duration::from_secs(300),
        tool_timeout: std::time::Duration::from_secs(60),
        variables: Some(variables),
        progress_callback: None,
        ..Default::default()
    };

    let wf_id = template_id;
    let db_for_task = db_owned.clone();
    tauri::async_runtime::spawn(async move {
        let result = engine.run_workflow(&wf_id, opts).await;
        let task_now = chrono::Utc::now().timestamp();
        match result {
            Ok(wf) => {
                use axagent_entities::opc_delivery;
                use sea_orm::*;
                let _ = opc_delivery::Entity::update_many()
                    .col_expr(opc_delivery::Column::Status, Expr::value("completed"))
                    .col_expr(opc_delivery::Column::Progress, Expr::value(1.0))
                    .col_expr(opc_delivery::Column::CompletedAt, Expr::value(task_now))
                    .col_expr(opc_delivery::Column::UpdatedAt, Expr::value(task_now))
                    .col_expr(
                        opc_delivery::Column::ResultSummary,
                        Expr::value(format!("工作流已完成，节点数: {}", wf.nodes.len())),
                    )
                    .filter(opc_delivery::Column::Id.eq(&delivery_id_for_task))
                    .exec(&db_for_task)
                    .await
                    .ok();
                // lead 状态同步 delivered
                let _ = axagent_entities::opc_demand_lead::Entity::update_many()
                    .col_expr(
                        axagent_entities::opc_demand_lead::Column::Status,
                        Expr::value("delivered"),
                    )
                    .col_expr(
                        axagent_entities::opc_demand_lead::Column::UpdatedAt,
                        Expr::value(task_now),
                    )
                    .filter(axagent_entities::opc_demand_lead::Column::Id.eq(&lead_id_for_task))
                    .exec(&db_for_task)
                    .await
                    .ok();
            },
            Err(e) => {
                use axagent_entities::opc_delivery;
                use sea_orm::*;
                let _ = opc_delivery::Entity::update_many()
                    .col_expr(opc_delivery::Column::Status, Expr::value("failed"))
                    .col_expr(opc_delivery::Column::CompletedAt, Expr::value(task_now))
                    .col_expr(opc_delivery::Column::UpdatedAt, Expr::value(task_now))
                    .col_expr(
                        opc_delivery::Column::ErrorsJson,
                        Expr::value(serde_json::json!([{ "workflow": e.to_string() }]).to_string()),
                    )
                    .filter(opc_delivery::Column::Id.eq(&delivery_id_for_task))
                    .exec(&db_for_task)
                    .await
                    .ok();
                // lead 状态同步 failed
                let _ = axagent_entities::opc_demand_lead::Entity::update_many()
                    .col_expr(
                        axagent_entities::opc_demand_lead::Column::Status,
                        Expr::value("failed"),
                    )
                    .col_expr(
                        axagent_entities::opc_demand_lead::Column::UpdatedAt,
                        Expr::value(task_now),
                    )
                    .filter(axagent_entities::opc_demand_lead::Column::Id.eq(&lead_id_for_task))
                    .exec(&db_for_task)
                    .await
                    .ok();
            },
        }
    });

    serde_json::to_value(&saved).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 列出交付记录（支持按状态/线索ID过滤）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "列出交付记录")]
#[tauri::command]
pub async fn opc_list_deliveries(
    state: State<'_, AppState>,
    status: Option<String>,
    lead_id: Option<String>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_delivery;
    use sea_orm::*;

    let db = state.harness.db();
    let mut qs = opc_delivery::Entity::find();

    if let Some(ref s) = status {
        qs = qs.filter(opc_delivery::Column::Status.eq(s));
    }
    if let Some(ref l) = lead_id {
        qs = qs.filter(opc_delivery::Column::LeadId.eq(l));
    }

    let results = qs.order_by_desc(opc_delivery::Column::CreatedAt).all(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    serde_json::to_value(&results).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 获取单个交付详情
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "获取交付详情")]
#[tauri::command]
pub async fn opc_get_delivery(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_delivery;
    use sea_orm::*;

    let db = state.harness.db();
    let result = opc_delivery::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("交付记录不存在: {id}"))?;

    serde_json::to_value(&result).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 更新交付状态（工作流执行完成后回调）
#[agent_command(domain = "opc", safety = Caution, call_mode = StateInput, description = "更新交付状态")]
#[tauri::command]
pub async fn opc_update_delivery(
    state: State<'_, AppState>,
    id: String,
    status: String,
    progress: Option<f64>,
    result_summary: Option<String>,
    deliverables: Option<serde_json::Value>,
    errors: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_delivery;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    let result = opc_delivery::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("交付记录不存在: {id}"))?;

    let mut am: opc_delivery::ActiveModel = result.into();
    am.status = Set(status.clone());
    if let Some(p) = progress {
        am.progress = Set(p);
    }
    if let Some(ref summary) = result_summary {
        am.result_summary = Set(Some(summary.clone()));
    }
    if let Some(d) = deliverables {
        am.deliverables_json = Set(serde_json::to_string(&d).unwrap_or_default());
    }
    if let Some(e) = errors {
        am.errors_json = Set(serde_json::to_string(&e).unwrap_or_default());
    }
    if status == "completed" || status == "failed" {
        am.completed_at = Set(Some(now));
    }
    am.updated_at = Set(now);

    let saved = am.update(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    // 同步更新 lead 状态
    if let Some(ref lead_id) = saved.lead_id {
        let lead_result = axagent_entities::opc_demand_lead::Entity::find_by_id(lead_id)
            .one(db)
            .await
            .ok()
            .flatten();
        if let Some(lead) = lead_result {
            let mut lead_am: axagent_entities::opc_demand_lead::ActiveModel = lead.into();
            lead_am.status = Set(if status == "completed" {
                "delivered".to_string()
            } else {
                status.clone()
            });
            lead_am.updated_at = Set(now);
            let _ = lead_am.update(db).await;
        }
    }

    serde_json::to_value(&saved).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 重试失败的交付任务
#[agent_command(domain = "opc", safety = Caution, call_mode = StateInput, description = "重试交付任务")]
#[tauri::command]
pub async fn opc_retry_delivery(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_delivery;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    let result = opc_delivery::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("交付记录不存在: {id}"))?;

    let mut am: opc_delivery::ActiveModel = result.into();
    am.status = Set("pending".to_string());
    am.progress = Set(0.0);
    am.result_summary = Set(None);
    am.completed_at = Set(None);
    am.errors_json = Set("[]".to_string());
    am.updated_at = Set(now);

    let saved = am.update(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    serde_json::to_value(&saved).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 取消进行中的交付任务
#[agent_command(domain = "opc", safety = Caution, call_mode = StateInput, description = "取消交付任务")]
#[tauri::command]
pub async fn opc_cancel_delivery(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_delivery;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    let result = opc_delivery::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("交付记录不存在: {id}"))?;

    let mut am: opc_delivery::ActiveModel = result.into();
    am.status = Set("cancelled".to_string());
    am.completed_at = Set(Some(now));
    am.updated_at = Set(now);

    let saved = am.update(db).await.map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })?;

    // 同步更新 lead 状态
    if let Some(ref lead_id) = saved.lead_id {
        let lead_result = axagent_entities::opc_demand_lead::Entity::find_by_id(lead_id)
            .one(db)
            .await
            .ok()
            .flatten();
        if let Some(lead) = lead_result {
            let mut lead_am: axagent_entities::opc_demand_lead::ActiveModel = lead.into();
            lead_am.status = Set("cancelled".to_string());
            lead_am.updated_at = Set(now);
            let _ = lead_am.update(db).await;
        }
    }

    serde_json::to_value(&saved).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 测试平台连接器连接（验证 API Token 和认证是否有效）
#[agent_command(domain = "opc", safety = Safe, call_mode = StateInput, description = "测试平台连接")]
#[tauri::command]
pub async fn opc_test_platform_connection(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_market_platform;
    use axagent_tools::tools::marketplace_scanner::AggregateMarketplaceScanner;
    use sea_orm::*;

    let db = state.harness.db();

    let platform = opc_market_platform::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?
        .ok_or_else(|| format!("平台配置不存在: {id}"))?;

    let config: serde_json::Value =
        serde_json::from_str(&platform.config_json).unwrap_or(serde_json::json!({}));

    let mut scanner = AggregateMarketplaceScanner::new();
    scanner.add_platform(
        &platform.name,
        &platform.platform_type,
        platform.base_url.as_deref(),
        &config,
    );

    let test_query = "test";
    match scanner.search_all(test_query).await {
        Ok(leads) => {
            let now = chrono::Utc::now().timestamp();
            let _ = opc_market_platform::Entity::update_many()
                .col_expr(opc_market_platform::Column::Status, Expr::value("connected"))
                .col_expr(opc_market_platform::Column::LastSyncAt, Expr::value(now))
                .col_expr(opc_market_platform::Column::UpdatedAt, Expr::value(now))
                .filter(opc_market_platform::Column::Id.eq(&id))
                .exec(db)
                .await;

            Ok(serde_json::json!({
                "success": true,
                "platform_id": id,
                "message": format!("连接成功，测试查询返回 {} 条结果", leads.len()),
                "lead_count": leads.len(),
            }))
        },
        Err(e) => {
            let now = chrono::Utc::now().timestamp();
            let _ = opc_market_platform::Entity::update_many()
                .col_expr(opc_market_platform::Column::Status, Expr::value("error"))
                .col_expr(opc_market_platform::Column::UpdatedAt, Expr::value(now))
                .filter(opc_market_platform::Column::Id.eq(&id))
                .exec(db)
                .await;

            Ok(serde_json::json!({
                "success": false,
                "platform_id": id,
                "message": format!("连接失败: {}", e),
                "error": e,
            }))
        },
    }
}
