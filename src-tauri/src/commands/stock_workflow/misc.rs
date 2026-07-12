use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_entities::stock_analyses;
use axagent_harness::{ToolContext, ToolRegistry};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tauri::State;

/// 将 Markdown 文本导出为 Word (.docx) 文件，通过 ToolRegistry 调用 ExportWordTool
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
    let tool = registry.get("ExportWord").ok_or_else(|| "ExportWord 工具未注册".to_string())?;
    let result = tool.call(input, &ctx).await.map_err(|e| e.to_string())?;
    Ok(result.content)
}

/// 将 Markdown 文本导出为 PowerPoint (.pptx) 文件，通过 ToolRegistry 调用 ExportPptxTool
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
    let tool = registry.get("ExportPptx").ok_or_else(|| "ExportPptx 工具未注册".to_string())?;
    let result = tool.call(input, &ctx).await.map_err(|e| e.to_string())?;
    Ok(result.content)
}

/// 记录用户对决策的信任选择（公式 vs LLM），存储到 decision_json.userTrustDecision。
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
