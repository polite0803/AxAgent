// SPDX-License-Identifier: AGPL-3.0-only

//! 需求发现（Demand Discovery）领域 Tauri 命令层
//!
//! 暴露能力扫描、市场线索发现、需求确认、交付工作流执行等核心命令。

use axagent_agent_macro::agent_command;
use sea_orm::sea_query::Expr;
use tauri::State;

use crate::AppState;
use crate::commands::error::ErrorResponse;

// ── 能力扫描 ──────────────────────────────────────────────────

/// 扫描系统当前可用的能力清单（工具/技能/MCP/工作流）
///
/// 复用上游能力发现索引（`capability_indexer`）的能力护照，按能力类型分组组装
/// `CapabilityInventory`。不再重复扫描注册表并落库到 `opc_capability` 表，
/// 避免与上游 `register_all_capabilities` 的能力基座重复收集。
#[agent_command(domain = "automation", safety = Safe, call_mode = StateOnly, description = "扫描系统能力清单")]
#[tauri::command]
pub async fn opc_scan_capabilities(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use axagent_analysis_engine::opc::capability::{
        CapabilityEntry, CapabilityInventory, CapabilitySource,
    };
    use axagent_harness::CapabilityKind;

    let now = chrono::Utc::now().timestamp();

    // 从上游能力索引读取全部护照，按类型分组（来源与 kind 保持一致）
    let mut tools: Vec<CapabilityEntry> = Vec::new();
    let mut skills: Vec<CapabilityEntry> = Vec::new();
    let mut mcp_tools: Vec<CapabilityEntry> = Vec::new();
    let mut workflows: Vec<CapabilityEntry> = Vec::new();
    let mut agents: Vec<CapabilityEntry> = Vec::new();

    let ids = state.capability_indexer.list_capability_ids().await;
    for id in ids {
        if let Some(p) = state.capability_indexer.get_passport(&id).await {
            // 系统专用护照（如认知编排器）不进入业务能力清单
            if p.visibility.is_system_only() {
                continue;
            }
            let source = match p.kind {
                CapabilityKind::Skill => CapabilitySource::Skill,
                CapabilityKind::Workflow => CapabilitySource::Workflow,
                CapabilityKind::Tool if p.capability_id.starts_with("mcp:") => {
                    CapabilitySource::McpTool
                },
                CapabilityKind::Tool => CapabilitySource::Tool,
                CapabilityKind::Agent => CapabilitySource::Agent,
                _ => continue,
            };
            let entry = CapabilityEntry {
                id: p.capability_id.clone(),
                name: p.name.clone(),
                description: p.description.clone(),
                source: source.clone(),
                source_id: p.capability_id.clone(),
                capability_type: p.kind.as_str().to_string(),
                applicable_scenarios: Vec::new(),
                example_deliverables: Vec::new(),
                metadata: serde_json::json!({
                    "enabled": p.enabled,
                    "domain": p.domain.as_str(),
                    "sub_category": p.sub_category,
                }),
            };
            match source {
                CapabilitySource::Tool => tools.push(entry),
                CapabilitySource::Skill => skills.push(entry),
                CapabilitySource::McpTool => mcp_tools.push(entry),
                CapabilitySource::Workflow => workflows.push(entry),
                CapabilitySource::Agent => agents.push(entry),
            }
        }
    }

    let mut inv = CapabilityInventory {
        tools,
        skills,
        mcp_tools,
        workflows,
        agents,
        scanned_at: now,
        total_count: 0,
    };
    inv.recalc_count();

    serde_json::to_value(&inv).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

// ── 市场需求发现 ──────────────────────────────────────────────

/// 从配置中提取领域关键词，生成主动扫描的查询列表
///
/// 读取 workflow_template(id="demand-discovery") 中的 domain_* 变量，
/// 将每个领域的关键词展开为独立的搜索查询。
async fn extract_domain_queries(db: &sea_orm::DatabaseConnection) -> Result<Vec<String>, String> {
    use axagent_entities::workflow_template;
    use sea_orm::*;

    let template = workflow_template::Entity::find_by_id("demand-discovery")
        .one(db)
        .await
        .map_err(|e| format!("读取需求发现配置失败: {e}"))?;

    let config_json = template
        .and_then(|t| t.variables)
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let mut queries = Vec::new();

    // 提取所有 domain_* 开头的变量
    if let Some(vars) = config_json.get("variables").and_then(|v| v.as_array()) {
        for var in vars {
            let name = var.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if name.starts_with("domain_") {
                if let Some(value) = var.get("value").and_then(|v| v.as_str()) {
                    // 将 "科技/AI/软件" 拆分为独立关键词
                    for kw in value.split('/') {
                        let trimmed = kw.trim();
                        if !trimmed.is_empty() {
                            queries.push(trimmed.to_string());
                        }
                    }
                }
            }
        }
    }

    // 如果没有配置任何领域关键词，使用默认种子
    if queries.is_empty() {
        queries = vec![
            "AI".to_string(),
            "软件".to_string(),
            "设计".to_string(),
            "营销".to_string(),
            "写作".to_string(),
            "翻译".to_string(),
        ];
    }

    Ok(queries)
}

/// 主动需求发现：基于配置的领域关键词自动扫描市场
///
/// 无需用户输入关键词，系统自动从配置中提取 domain_* 关键词，
/// 依次扫描各平台，聚合所有发现的需求线索。
#[agent_command(domain = "automation", safety = Safe, call_mode = StateOnly, description = "主动需求发现")]
#[tauri::command]
pub async fn opc_proactive_discover_leads(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_market_platform;
    use axagent_tools::tools::marketplace_scanner::AggregateMarketplaceScanner;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    // 1) 从配置提取领域关键词
    let queries = extract_domain_queries(db).await?;

    // 2) 加载已启用的平台连接器
    let mut scanner = AggregateMarketplaceScanner::new();
    let platforms = opc_market_platform::Entity::find()
        .filter(opc_market_platform::Column::Enabled.eq(1))
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?;

    for p in &platforms {
        let config: serde_json::Value =
            serde_json::from_str(&p.config_json).unwrap_or(serde_json::json!({}));
        scanner.add_platform(&p.name, &p.platform_type, p.base_url.as_deref(), &config);
    }

    // 3) 遍历所有领域关键词，聚合扫描结果
    let mut all_leads: Vec<serde_json::Value> = Vec::new();
    let mut query_stats = Vec::new();

    for query in &queries {
        match scanner.search_all(query).await {
            Ok(leads) => {
                let count = leads.len();
                query_stats.push(serde_json::json!({
                    "query": query,
                    "found": count,
                }));
                for lead in leads {
                    all_leads.push(serde_json::to_value(&lead).unwrap_or_default());
                }
            },
            Err(e) => {
                tracing::warn!("[opc_proactive_discover_leads] 关键词 '{}' 扫描失败: {}", query, e);
                query_stats.push(serde_json::json!({
                    "query": query,
                    "found": 0,
                    "error": e.to_string(),
                }));
            },
        }
    }

    // 4) 记录平台同步时间
    let _ = opc_market_platform::Entity::update_many()
        .col_expr(opc_market_platform::Column::LastSyncAt, Expr::value(now))
        .col_expr(opc_market_platform::Column::Status, Expr::value("synced"))
        .col_expr(opc_market_platform::Column::UpdatedAt, Expr::value(now))
        .exec(db)
        .await;

    // 5) 返回统计信息
    let result = serde_json::json!({
        "total_queries": queries.len(),
        "total_found": all_leads.len(),
        "query_stats": query_stats,
        "queries": queries,
        "leads": all_leads,
    });

    serde_json::to_value(&result).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 按关键词搜索市场平台需求线索（闲鱼、猪八戒等）—— 保留用于精确检索场景
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "搜索市场需求线索")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "扫描并评估需求线索")]
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

/// 主动评估入库：基于配置的领域关键词自动扫描、评估并入库
///
/// 无需用户输入关键词，系统自动从配置中提取 domain_* 关键词，
/// 对每个领域执行「扫描 + 评估 + 入库」完整流水线。
#[agent_command(domain = "automation", safety = Safe, call_mode = StateOnly, description = "主动评估并入库需求")]
#[tauri::command]
pub async fn opc_proactive_evaluate_and_save_leads(
    state: State<'_, AppState>,
    min_score: Option<f64>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_demand_lead;
    use axagent_entities::opc_market_platform;
    use axagent_tools::tools::marketplace_scanner::AggregateMarketplaceScanner;
    use sea_orm::*;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    // 1) 从配置提取领域关键词
    let queries = extract_domain_queries(db).await?;

    // 2) 加载已启用的平台连接器
    let mut scanner = AggregateMarketplaceScanner::new();
    let platforms = opc_market_platform::Entity::find()
        .filter(opc_market_platform::Column::Enabled.eq(1))
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?;

    for p in &platforms {
        let config: serde_json::Value =
            serde_json::from_str(&p.config_json).unwrap_or(serde_json::json!({}));
        scanner.add_platform(&p.name, &p.platform_type, p.base_url.as_deref(), &config);
    }

    // 3) 遍历所有领域关键词，执行「扫描 + 评估 + 入库」
    let min_threshold = min_score.unwrap_or(0.0);
    let mut total_scanned = 0usize;
    let mut total_saved = 0usize;
    let mut high_value_count = 0usize;
    let mut query_stats = Vec::new();

    for query in &queries {
        match scanner.search_and_evaluate(query, None).await {
            Ok(evaluated) => {
                let count = evaluated.len();
                total_scanned += count;

                let filtered: Vec<_> =
                    evaluated.into_iter().filter(|e| e.value_score() >= min_threshold).collect();

                for el in &filtered {
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
                        raw_snapshot_json: Set(
                            serde_json::to_string(&el.lead.raw_snapshot).unwrap_or_default()
                        ),
                        matched_capabilities_json: Set("[]".to_string()),
                        ai_analysis_json: Set(
                            serde_json::to_string(&el.evaluation).unwrap_or_default()
                        ),
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
                            total_saved += 1;
                            if is_high_value {
                                high_value_count += 1;
                            }
                        },
                        Err(e) => {
                            tracing::warn!(
                                "[opc_proactive_evaluate_and_save_leads] 入库失败 {}: {}",
                                el.lead.id,
                                e
                            );
                        },
                    }
                }

                query_stats.push(serde_json::json!({
                    "query": query,
                    "scanned": count,
                    "saved": filtered.len(),
                }));
            },
            Err(e) => {
                tracing::warn!(
                    "[opc_proactive_evaluate_and_save_leads] 关键词 '{}' 评估失败: {}",
                    query,
                    e
                );
                query_stats.push(serde_json::json!({
                    "query": query,
                    "scanned": 0,
                    "saved": 0,
                    "error": e.to_string(),
                }));
            },
        }
    }

    // 4) 记录平台同步时间
    let _ = opc_market_platform::Entity::update_many()
        .col_expr(opc_market_platform::Column::LastSyncAt, Expr::value(now))
        .col_expr(opc_market_platform::Column::Status, Expr::value("synced"))
        .col_expr(opc_market_platform::Column::UpdatedAt, Expr::value(now))
        .exec(db)
        .await;

    // 5) 返回统计信息
    let result = serde_json::json!({
        "total_queries": queries.len(),
        "total_scanned": total_scanned,
        "total_saved": total_saved,
        "high_value_count": high_value_count,
        "query_stats": query_stats,
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
/// 当 query 为 None 或空字符串时，自动从配置中提取领域关键词进行主动扫描。
///
/// # 参数
/// - `db`: 数据库连接
/// - `query`: 搜索关键词（None 或空则自动从配置提取）
/// - `app_handle`: Tauri AppHandle（用于发送桌面通知，可选）
pub async fn run_demand_discovery_cron(
    db: &sea_orm::DatabaseConnection,
    query: Option<&str>,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<String, String> {
    use axagent_entities::opc_demand_lead;
    use axagent_entities::opc_market_platform;
    use axagent_tools::tools::marketplace_scanner::AggregateMarketplaceScanner;
    use sea_orm::*;

    let now = chrono::Utc::now().timestamp();

    // 1) 确定查询关键词列表
    let queries = if let Some(q) = query {
        if !q.trim().is_empty() {
            vec![q.to_string()]
        } else {
            extract_domain_queries(db).await?
        }
    } else {
        extract_domain_queries(db).await?
    };

    // 2) 加载已启用的平台连接器
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

    // 3) 遍历所有关键词，执行「扫描 + 评估 + 入库」
    let mut total_scanned = 0usize;
    let mut total_saved = 0usize;
    let mut high_value_count = 0usize;
    let mut high_value_leads: Vec<(String, f64, String)> = Vec::new();

    for query in &queries {
        match scanner.search_and_evaluate(query, None).await {
            Ok(evaluated) => {
                let count = evaluated.len();
                total_scanned += count;

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
                        raw_snapshot_json: Set(
                            serde_json::to_string(&el.lead.raw_snapshot).unwrap_or_default()
                        ),
                        matched_capabilities_json: Set("[]".to_string()),
                        ai_analysis_json: Set(
                            serde_json::to_string(&el.evaluation).unwrap_or_default()
                        ),
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
                            total_saved += 1;
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
                            tracing::warn!(
                                "[run_demand_discovery_cron] 入库失败 {}: {}",
                                el.lead.id,
                                e
                            );
                        },
                    }
                }
            },
            Err(e) => {
                tracing::warn!("[run_demand_discovery_cron] 关键词 '{}' 扫描失败: {}", query, e);
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
        "主动需求发现完成: {} 个关键词, 扫描 {} 条, 入库 {} 条, 高价值 {} 条",
        queries.len(),
        total_scanned,
        total_saved,
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
    // 移动端无桌面通知通道，app_handle 仅用于桌面端，避免 unused 警告
    #[cfg(mobile)]
    let _ = app_handle;

    if high_value_leads.is_empty() {
        return;
    }

    let count = high_value_leads.len();
    let titles: Vec<String> = high_value_leads
        .iter()
        .take(3)
        .map(|(_, score, title)| format!("{} (评分: {:.1})", title, score))
        .collect();

    #[allow(unused_variables)]
    #[cfg(not(mobile))]
    let body = if count > 3 {
        format!("{} 条高价值需求: {} ...等", count, titles.join(", "))
    } else {
        format!("{} 条高价值需求: {}", count, titles.join(", "))
    };

    // 发送 Tauri 桌面通知（仅桌面端；移动端无桌面通知通道，仅靠下方日志记录）
    #[cfg(not(mobile))]
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
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "创建需求线索")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出需求线索")]
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
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "确认需求线索")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "为需求匹配能力")]
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

    // 复用上游能力发现管线（RAR 语义匹配），以需求描述为查询输入
    let user_input = format!("{} {}", result.title, result.description);
    let query =
        axagent_harness::CapabilityQuery { user_input: user_input.clone(), ..Default::default() };
    let discovery_request = axagent_harness::CapabilityDiscoveryRequest {
        user_input,
        filter_context: axagent_harness::FilterContext::default(),
        query,
        weights: axagent_harness::DiscoveryWeights::default(),
        budget: axagent_harness::SessionBudget::default(),
        enable_completion: false,
        enable_circuit_breaker: true,
        enable_rar: true,
        rar_top_k: 10,
        task_shape: None,
    };

    let discovery_result = axagent_harness::CapabilityRouter::discover(
        state.capability_router.as_ref(),
        &discovery_request,
    )
    .await
    .map_err(|e| format!("能力匹配失败: {e}"))?;

    // 将上游排序结果映射为前端期望的 {id, name, source, score}
    let mut matched: Vec<serde_json::Value> = Vec::new();
    if let Some(primary) = &discovery_result.primary_match {
        matched.push(serde_json::json!({
            "id": primary.passport.capability_id,
            "name": primary.passport.name,
            "source": primary.passport.kind.as_str(),
            "score": primary.final_score,
        }));
    }
    for alt in &discovery_result.alternatives {
        matched.push(serde_json::json!({
            "id": alt.passport.capability_id,
            "name": alt.passport.name,
            "source": alt.passport.kind.as_str(),
            "score": alt.final_score,
        }));
    }

    let hit_count = matched.len();
    let score_sum: f64 = matched.iter().filter_map(|m| m["score"].as_f64()).sum();
    let confidence = if hit_count > 0 {
        (score_sum / hit_count as f64).min(1.0)
    } else {
        0.0
    };

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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateOnly, description = "列出市场平台配置")]
#[tauri::command]
pub async fn opc_list_platforms(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_market_platform;
    use sea_orm::*;

    let db = state.harness.db();

    // 确保预置平台种子数据存在（如果失败则返回错误）
    axagent_dao::repo::market_platform::ensure_preset_platforms(db).await.map_err(|e| {
        tracing::error!("[opc_list_platforms] 种子数据初始化失败: {}", e);
        e
    })?;

    let results = opc_market_platform::Entity::find()
        .order_by_desc(opc_market_platform::Column::Enabled)
        .order_by_asc(opc_market_platform::Column::Name)
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
                .to_string()
        })?;

    tracing::info!("[opc_list_platforms] 返回 {} 个平台", results.len());

    serde_json::to_value(&results).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

/// 保存（新增或更新）平台连接器配置
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "保存平台配置")]
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
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "删除平台配置")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateOnly, description = "列出能力缺口")]
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
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "关闭能力缺口")]
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

/// 主动分析能力缺口：基于已有需求线索统计高频缺失能力
///
/// 与被动"匹配失败即缺口"不同，此命令主动分析：
/// 1. 统计高价值需求中未匹配能力的高频关键词
/// 2. 分析领域需求趋势与现有能力库的覆盖差距
/// 3. 基于配置的领域关键词对比能力库覆盖
#[agent_command(domain = "automation", safety = Safe, call_mode = StateOnly, description = "主动分析能力缺口")]
#[tauri::command]
pub async fn opc_analyze_capability_gaps(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_capability_gap;
    use axagent_entities::opc_demand_lead;
    use sea_orm::*;
    use std::collections::HashMap;

    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp();

    // 1) 扫描现有能力库（复用上游能力索引的能力护照）
    let capability_keywords: Vec<String> = {
        let mut keywords: Vec<String> = Vec::new();
        for id in state.capability_indexer.list_capability_ids().await {
            if let Some(p) = state.capability_indexer.get_passport(&id).await {
                if p.visibility.is_system_only() {
                    continue;
                }
                keywords.push(
                    format!("{} {} {}", p.name, p.description, p.kind.as_str()).to_lowercase(),
                );
            }
        }
        keywords
    };

    // 2) 统计高价值需求中的高频关键词
    let leads = opc_demand_lead::Entity::find()
        .filter(opc_demand_lead::Column::CommercialValueScore.gte(50.0))
        .filter(opc_demand_lead::Column::Status.ne("delivered"))
        .filter(opc_demand_lead::Column::Status.ne("failed"))
        .all(db)
        .await
        .map_err(|e| format!("读取需求线索失败: {e}"))?;

    let mut keyword_freq: HashMap<String, (usize, f64)> = HashMap::new(); // (出现次数, 累计评分)
    for lead in &leads {
        let text = format!("{} {}", lead.title, lead.description).to_lowercase();
        let score = lead.commercial_value_score;

        // 简单分词：按空格和常见标点
        for word in
            text.split(|c: char| c.is_whitespace() || "，。！？、；：\"'（）【】".contains(c))
        {
            let trimmed = word.trim();
            if trimmed.len() >= 2 && trimmed.len() <= 10 {
                let entry = keyword_freq.entry(trimmed.to_string()).or_insert((0, 0.0));
                entry.0 += 1;
                entry.1 += score;
            }
        }
    }

    // 3) 识别未被现有能力覆盖的高频关键词
    let mut missing_keywords: Vec<(String, usize, f64)> = Vec::new();
    for (keyword, (freq, total_score)) in &keyword_freq {
        let covered = capability_keywords.iter().any(|ck| ck.contains(keyword));
        if !covered && *freq >= 2 {
            missing_keywords.push((keyword.clone(), *freq, *total_score));
        }
    }

    // 按频率排序（降序）
    missing_keywords.sort_by_key(|a| std::cmp::Reverse(a.1));

    // 4) 基于配置的领域关键词分析覆盖情况
    let domain_queries = extract_domain_queries(db).await?;
    let mut domain_coverage = Vec::new();
    for domain_kw in &domain_queries {
        let domain_kw_lower = domain_kw.to_lowercase();
        let covered = capability_keywords.iter().any(|ck| ck.contains(&domain_kw_lower));
        let demand_count = leads
            .iter()
            .filter(|l| {
                let text = format!("{} {}", l.title, l.description).to_lowercase();
                text.contains(&domain_kw_lower)
            })
            .count();

        domain_coverage.push(serde_json::json!({
            "domain": domain_kw,
            "covered": covered,
            "demand_count": demand_count,
        }));
    }

    // 5) 自动创建高优先级缺口（Top 5 高频缺失）
    let auto_created = if missing_keywords.len() >= 2 {
        let mut created = Vec::new();
        for (keyword, freq, _) in missing_keywords.iter().take(5) {
            let gap_id = format!("gap-auto-{}", uuid::Uuid::new_v4().simple());
            let priority = if *freq >= 5 {
                1
            } else if *freq >= 3 {
                2
            } else {
                3
            };

            let result = opc_capability_gap::ActiveModel {
                id: Set(gap_id.clone()),
                lead_id: Set(None),
                title: Set(format!("[主动分析] 高频缺失能力: {}", keyword)),
                description: Set(format!(
                    "关键词 '{}' 在 {} 条高价值需求中出现，但现有能力库未覆盖。建议新增对应能力。",
                    keyword, freq
                )),
                missing_capability: Set(keyword.clone()),
                gap_type: Set("proactive".to_string()),
                suggested_action: Set(format!(
                    "针对 '{}' 领域新增工具/技能/工作流模板，或扫描市场平台获取该领域需求详情",
                    keyword
                )),
                priority: Set(priority),
                status: Set("open".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                closed_at: Set(None),
            }
            .insert(db)
            .await;

            if result.is_ok() {
                created.push(keyword.clone());
            }
        }
        created
    } else {
        Vec::new()
    };

    // 6) 返回分析结果
    let result = serde_json::json!({
        "total_leads_analyzed": leads.len(),
        "total_capabilities": capability_keywords.len(),
        "missing_keywords_count": missing_keywords.len(),
        "top_missing_keywords": missing_keywords.iter().take(10).map(|(k, f, s)| {
            serde_json::json!({
                "keyword": k,
                "frequency": f,
                "total_score": s,
            })
        }).collect::<Vec<_>>(),
        "domain_coverage": domain_coverage,
        "auto_created_gaps": auto_created,
    });

    serde_json::to_value(&result).map_err(|e| {
        ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable)
            .to_string()
    })
}

// ── 状态标记 ──────────────────────────────────────────────────

/// 标记需求线索状态（expired 过期 / claimed 他人承接 / cancelled 取消等）
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "标记需求线索状态")]
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
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "执行需求交付")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出交付记录")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取交付详情")]
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
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "更新交付状态")]
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
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "重试交付任务")]
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
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "取消交付任务")]
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "测试平台连接")]
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
