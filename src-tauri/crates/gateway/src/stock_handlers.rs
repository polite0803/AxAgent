//! Gateway HTTP API for Stock Analysis
//!
//! 对外暴露股票数据查询与分析接口，供外部脚本调用。

use axagent_entities::{stock_analyses, watchlist_items};
use axagent_harness::market_data::MarketDataProvider;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, QuerySelect, Set};
use serde::{Deserialize, Serialize};

use crate::server::GatewayAppState;

// ── Query / Payload types ──

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub keyword: String,
}

#[derive(Debug, Deserialize)]
pub struct QuoteQuery {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct KlineQuery {
    pub code: String,
    #[serde(default = "default_period")]
    pub period: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_period() -> String {
    "daily".to_string()
}

fn default_limit() -> u32 {
    120
}

#[derive(Debug, Deserialize)]
pub struct AnalysisListQuery {
    #[serde(default = "default_20")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_20() -> u32 {
    20
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisRequest {
    pub stock_code: String,
    pub date: Option<String>,
    pub provider_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WatchlistAddRequest {
    pub stock_code: String,
    pub stock_name: String,
}

// ── Helpers ──

fn ok_json<T: Serialize>(data: T) -> Response {
    Json(serde_json::json!({ "data": data })).into_response()
}

fn error_json(status: StatusCode, msg: &str) -> Response {
    (status, Json(serde_json::json!({ "error": msg }))).into_response()
}

fn aclient(state: &GatewayAppState) -> &dyn MarketDataProvider {
    &*state.astock_client
}

// ── Handlers ──

/// GET /api/stock/search?keyword=茅台
pub async fn search_stock(
    State(state): State<GatewayAppState>,
    Query(q): Query<SearchQuery>,
) -> Response {
    match aclient(&state).search_stock(&q.keyword).await {
        Ok(results) => ok_json(results),
        Err(e) => error_json(StatusCode::BAD_GATEWAY, &e.to_string()),
    }
}

/// GET /api/stock/quote?code=600519
pub async fn get_quote(
    State(state): State<GatewayAppState>,
    Query(q): Query<QuoteQuery>,
) -> Response {
    match aclient(&state).get_quote(&q.code).await {
        Ok(quote) => ok_json(quote),
        Err(e) => error_json(StatusCode::BAD_GATEWAY, &e.to_string()),
    }
}

/// GET /api/stock/kline?code=600519&period=daily&limit=120
pub async fn get_kline(
    State(state): State<GatewayAppState>,
    Query(q): Query<KlineQuery>,
) -> Response {
    match aclient(&state)
        .get_klines(&q.code, &q.period, q.limit, None)
        .await
    {
        Ok(klines) => ok_json(klines),
        Err(e) => error_json(StatusCode::BAD_GATEWAY, &e.to_string()),
    }
}

/// POST /api/stock/analysis — 提交分析任务（同步返回分析ID，实际分析后台执行）
pub async fn start_analysis(
    State(state): State<GatewayAppState>,
    Json(req): Json<AnalysisRequest>,
) -> Response {
    let stock_code = req.stock_code.clone();
    let date = req
        .date
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    let provider_id = req.provider_id.clone().unwrap_or_default();

    // 获取股票名称
    let stock_name = match aclient(&state).get_quote(&stock_code).await {
        Ok(q) => q.name,
        Err(e) => {
            return error_json(StatusCode::BAD_GATEWAY, &format!("获取行情失败: {}", e));
        },
    };

    let analysis_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let conversation_id = uuid::Uuid::new_v4().to_string();

    let model = stock_analyses::ActiveModel {
        id: Set(analysis_id.clone()),
        stock_code: Set(stock_code.clone()),
        stock_name: Set(stock_name.clone()),
        analysis_date: Set(date.clone()),
        provider_id: Set(provider_id),
        conversation_id: Set(conversation_id),
        status: Set("submitted".to_string()),
        decision_action: Set(None),
        decision_position_pct: Set(None),
        decision_reasoning: Set(None),
        decision_json: Set(None),
        blackboard_snapshot: Set(None),
        config_id: Set(None),
        analysis_kind: Set("live".into()),
        as_of_date: Set(None),
        model_version: Set(None),
        data_snapshot_id: Set(None),
        outcome: Set(None),
        decision_time_horizon: Set(None),
        decision_expected_holding_days: Set(None),
        llm_decision_json: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    match model.insert(&state.db).await {
        Ok(record) => {
            tracing::info!(
                "[gateway:stock] 分析任务已提交: id={} code={}",
                analysis_id,
                stock_code
            );
            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({
                    "data": {
                        "analysisId": record.id,
                        "stockCode": stock_code,
                        "stockName": stock_name,
                        "status": "submitted",
                    }
                })),
            )
                .into_response()
        },
        Err(e) => error_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// GET /api/stock/analysis/:id — 获取单个分析详情
pub async fn get_analysis(
    State(state): State<GatewayAppState>,
    Path(analysis_id): Path<String>,
) -> Response {
    match stock_analyses::Entity::find_by_id(&analysis_id)
        .one(&state.db)
        .await
    {
        Ok(Some(record)) => ok_json(record),
        Ok(None) => error_json(StatusCode::NOT_FOUND, &format!("分析记录不存在: {}", analysis_id)),
        Err(e) => error_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// GET /api/stock/analyses?limit=20&offset=0 — 分析记录列表
pub async fn list_analyses(
    State(state): State<GatewayAppState>,
    Query(q): Query<AnalysisListQuery>,
) -> Response {
    match stock_analyses::Entity::find()
        .order_by_desc(stock_analyses::Column::CreatedAt)
        .limit(Some(q.limit as u64))
        .offset(Some(q.offset as u64))
        .all(&state.db)
        .await
    {
        Ok(records) => ok_json(records),
        Err(e) => error_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// GET /api/stock/watchlist — 自选股列表
pub async fn get_watchlist(State(state): State<GatewayAppState>) -> Response {
    match watchlist_items::Entity::find()
        .order_by_desc(watchlist_items::Column::CreatedAt)
        .all(&state.db)
        .await
    {
        Ok(items) => ok_json(items),
        Err(e) => error_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// POST /api/stock/watchlist — 添加自选股
pub async fn add_watchlist(
    State(state): State<GatewayAppState>,
    Json(req): Json<WatchlistAddRequest>,
) -> Response {
    let now = chrono::Utc::now().timestamp_millis();
    let model = watchlist_items::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        stock_code: Set(req.stock_code),
        stock_name: Set(req.stock_name),
        notes: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    match model.insert(&state.db).await {
        Ok(record) => {
            (StatusCode::CREATED, Json(serde_json::json!({ "data": record }))).into_response()
        },
        Err(e) => error_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// DELETE /api/stock/watchlist/:id — 移除自选股
pub async fn delete_watchlist(
    State(state): State<GatewayAppState>,
    Path(id): Path<String>,
) -> Response {
    match watchlist_items::Entity::delete_by_id(&id)
        .exec(&state.db)
        .await
    {
        Ok(_) => ok_json(serde_json::json!({ "deleted": true })),
        Err(e) => error_json(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}
