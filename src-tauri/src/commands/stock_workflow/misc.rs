use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_agent_macro::agent_command;
use axagent_entities::stock_analyses;
use axagent_harness::{ToolContext, ToolRegistry};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tauri::State;

/// 将 Markdown 文本导出为 Word (.docx) 文件，通过 ToolRegistry 调用 ExportWordTool
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description =  "导出Markdown为Word文档")]
#[tauri::command]
pub async fn export_md_to_docx(
    state: State<'_, AppState>,
    markdown: String,
    output_path: String,
    title: Option<String>,
) -> Result<String, String> {
    let input = serde_json::json!({
        "markdown": markdown,
        "output_path": output_path,
        "title": title.unwrap_or_else(|| "股票分析报告".to_string()),
    });
    let ctx = ToolContext::new(std::env::temp_dir().to_string_lossy().to_string());
    let registry = state.local_tool_registry.lock().await;
    let tool = registry.get("ExportWord").ok_or_else(|| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail("ExportWord 工具未注册").to_string()
    })?;
    let result = tool.call(input, &ctx).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("导出 Word 失败: {e}"))
    })?;
    Ok(result.content)
}

/// 将 Markdown 文本导出为 PowerPoint (.pptx) 文件，通过 ToolRegistry 调用 ExportPptxTool
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description =  "导出Markdown为PPT文档")]
#[tauri::command]
pub async fn export_md_to_pptx(
    state: State<'_, AppState>,
    markdown: String,
    output_path: String,
    title: Option<String>,
) -> Result<String, String> {
    let input = serde_json::json!({
        "markdown": markdown,
        "output_path": output_path,
        "title": title.unwrap_or_else(|| "股票分析报告".to_string()),
    });
    let ctx = ToolContext::new(std::env::temp_dir().to_string_lossy().to_string());
    let registry = state.local_tool_registry.lock().await;
    let tool = registry.get("ExportPptx").ok_or_else(|| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail("ExportPptx 工具未注册").to_string()
    })?;
    let result = tool.call(input, &ctx).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("导出 Pptx 失败: {e}"))
    })?;
    Ok(result.content)
}

/// 记录用户对决策的信任选择（公式 vs LLM），存储到 decision_json.userTrustDecision。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description =  "记录用户决策信任选择")]
#[tauri::command]
pub async fn record_decision_trust(
    state: State<'_, AppState>,
    analysis_id: String,
    trust_model: String,
) -> Result<serde_json::Value, String> {
    use sea_orm::sea_query::Expr;

    let db = state.harness.db();
    // 查原始记录获取 current decision_json
    let original = stock_analyses::Entity::find_by_id(&analysis_id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询分析记录失败: {e}"))
        })?
        .ok_or_else(|| format!("分析记录不存在: {analysis_id}"))?;

    let mut dj: serde_json::Value = original
        .decision_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = dj.as_object_mut() {
        obj.insert("userTrustDecision".into(), serde_json::json!(trust_model));
    }

    stock_analyses::Entity::update_many()
        .col_expr(stock_analyses::Column::DecisionJson, Expr::value(dj.to_string()))
        .filter(stock_analyses::Column::Id.eq(&analysis_id))
        .exec(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("更新分析记录失败: {e}"))
        })?;

    tracing::warn!("[record_decision_trust] analysis_id={analysis_id}, trust_model={trust_model}");
    Ok(serde_json::json!({ "success": true, "trust_model": trust_model }))
}

/// 查询决策回测分析：返回所有有 outcome 的分析记录的比较数据。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description =  "查询决策回测分析")]
#[tauri::command]
pub async fn query_decision_backtest(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<serde_json::Value>, String> {
    use crate::commands::error::ErrorResponse;
    use sea_orm::QueryFilter;
    use sea_orm::QueryOrder;

    let db = state.harness.db();
    let records = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::Outcome.is_not_null())
        .filter(stock_analyses::Column::DecisionAction.is_not_null())
        .order_by(stock_analyses::Column::AnalysisDate, sea_orm::Order::Desc)
        .all(db)
        .await
        .map_err(|e| ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询失败: {e}")))?;

    let limit = limit.unwrap_or(100).min(500) as usize;
    let mut results: Vec<serde_json::Value> = Vec::new();
    for r in records.iter().take(limit) {
        let formula_action = r.decision_action.as_deref().unwrap_or("");
        let outcome_str = r.outcome.as_deref().unwrap_or("");
        let llm_action: Option<String> = r
            .llm_decision_json
            .as_ref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| {
                v.get("action")
                    .or_else(|| v.get("stance"))
                    .and_then(|a| a.as_str().map(String::from))
            });
        results.push(serde_json::json!({
            "stockCode": r.stock_code,
            "stockName": r.stock_name,
            "analysisDate": r.analysis_date,
            "formulaAction": formula_action,
            "llmAction": llm_action,
            "outcome": outcome_str,
            "decisionConfidence": r.decision_json.as_ref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|v| v.get("confidence").and_then(|c| c.as_f64())),
            "userTrustDecision": r.decision_json.as_ref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|v| v.get("userTrustDecision").and_then(|t| t.as_str().map(String::from))),
        }));
    }
    Ok(results)
}

// ───────────────────────────────────────────────────────────────────────────
// P3-4: 跨股票信号聚合 + 板块联动分析 Tauri 命令
// ───────────────────────────────────────────────────────────────────────────

/// P3-4: 查询跨股票信号聚合器的当前配置
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description =  "获取跨股票信号聚合器配置")]
#[tauri::command]
pub async fn get_cross_stock_aggregator_config(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let Some(agg) = state.cross_stock_aggregator.get() else {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail("跨股票信号聚合器未初始化")
            .to_string());
    };
    let cfg = agg.config().await;
    serde_json::to_value(cfg).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("序列化配置失败: {e}")).to_string()
    })
}

/// P3-4: 热更新跨股票信号聚合器配置
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description =  "热更新跨股票聚合器配置")]
#[tauri::command]
pub async fn set_cross_stock_aggregator_config(
    state: State<'_, AppState>,
    config: axagent_analysis_engine::cross_stock_aggregator::AggregatorConfig,
) -> Result<(), String> {
    let Some(agg) = state.cross_stock_aggregator.get() else {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail("跨股票信号聚合器未初始化")
            .to_string());
    };
    agg.set_config(config).await;
    Ok(())
}

/// P3-4: 查询聚合器缓冲区快照（调试用）
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description =  "查询聚合器缓冲区快照")]
#[tauri::command]
pub async fn get_cross_stock_aggregator_buffer(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let Some(agg) = state.cross_stock_aggregator.get() else {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail("跨股票信号聚合器未初始化")
            .to_string());
    };
    let buf = agg.buffer_snapshot().await;
    Ok(buf.into_iter().filter_map(|s| serde_json::to_value(s).ok()).collect())
}

/// P3-4: 计算指定板块的联动报告
///
/// 调用方需提供 `concept_id`（如 "concept_ai"），后端通过 ConceptIndex 查询成员股票，
/// 批量拉取实时行情，调用 `compute_sector_coherence` 生成报告。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description =  "计算指定板块联动报告")]
#[tauri::command]
pub async fn get_sector_coherence_report(
    state: State<'_, AppState>,
    concept_id: String,
) -> Result<serde_json::Value, String> {
    use axagent_analysis_engine::concept_index::{build_sample_index, seed_ashare_ontology};
    use axagent_analysis_engine::sector_coherence::compute_sector_coherence;

    // 构建概念索引（A 股本体）
    let mut idx = build_sample_index();
    seed_ashare_ontology(&mut idx);

    let members = idx.members(&concept_id);
    if members.is_empty() {
        return Err(ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("概念 {concept_id} 不存在或无成员股票"))
            .to_string());
    }

    // 批量拉取行情（直接走 astock_client，不依赖 monitor 是否启动）
    let quotes = state.astock_client.clone();
    let mut quote_list = Vec::with_capacity(members.len());
    for code in &members {
        if let Ok(q) = quotes.get_quote(code).await {
            quote_list.push(q);
        }
    }

    let timestamp = chrono::Utc::now().timestamp();
    let report = compute_sector_coherence(&idx, &concept_id, &quote_list, timestamp);
    report.map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null)).ok_or_else(|| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail("板块联动报告生成失败（行情数据为空？）")
            .to_string()
    })
}

/// P3-4: 批量扫描多个板块的联动情况
///
/// 返回按联动强度（coherence 绝对值）降序排列的报告列表，
/// 调用方可据此快速识别"异动板块"。
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description =  "批量扫描板块联动情况")]
#[tauri::command]
pub async fn scan_sector_coherence(
    state: State<'_, AppState>,
    concept_ids: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    use axagent_analysis_engine::concept_index::{build_sample_index, seed_ashare_ontology};
    use axagent_analysis_engine::sector_coherence::scan_sectors;
    use std::collections::HashMap;

    let mut idx = build_sample_index();
    seed_ashare_ontology(&mut idx);

    let quotes_client = state.astock_client.clone();
    let mut quotes_by_concept: HashMap<String, Vec<_>> = HashMap::new();
    for cid in &concept_ids {
        let members = idx.members(cid);
        if members.is_empty() {
            continue;
        }
        let mut ql = Vec::with_capacity(members.len());
        for code in &members {
            if let Ok(q) = quotes_client.get_quote(code).await {
                ql.push(q);
            }
        }
        quotes_by_concept.insert(cid.clone(), ql);
    }

    let timestamp = chrono::Utc::now().timestamp();
    let reports = scan_sectors(&idx, &concept_ids, &quotes_by_concept, timestamp);
    Ok(reports.into_iter().filter_map(|r| serde_json::to_value(r).ok()).collect())
}
