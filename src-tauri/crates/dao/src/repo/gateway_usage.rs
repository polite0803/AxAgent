// SPDX-License-Identifier: AGPL-3.0-only

//! gateway_usage 表的数据访问层。
//!
//! 本模块是 `gateway_usage` 表查询的唯一权威来源：record_usage 写入、
//! get_metrics 聚合统计、get_usage_by_* 维度拆分、get_connected_programs
//! 关联 gateway_keys 取今日活跃程序。`repo::gateway` 仅保留 gateway_keys
//! 的 CRUD，不再持有 usage 相关函数，避免重复定义。

use sea_orm::*;

use axagent_entities::gateway_usage;
use axagent_harness::core_error::Result;
use axagent_harness::types::*;
use axagent_harness::util_fns::{now_ts, today_start_local_ts};

/// 记录一次网关请求的 token 用量与估算成本。
///
/// `cost_usd` 由调用方基于 `axagent_harness::usage_pricing::pricing_for_model`
/// 换算后传入；未知定价时传 `0.0`，dao 层原样落库。
#[allow(clippy::too_many_arguments)]
pub async fn record_usage(
    db: &DatabaseConnection,
    key_id: &str,
    provider_id: &str,
    model_id: Option<&str>,
    request_tokens: u64,
    response_tokens: u64,
    cached_input_tokens: u64,
    cost_usd: f64,
) -> Result<()> {
    gateway_usage::ActiveModel {
        key_id: Set(key_id.to_string()),
        provider_id: Set(provider_id.to_string()),
        model_id: Set(model_id.map(|s| s.to_string())),
        request_tokens: Set(request_tokens as i64),
        response_tokens: Set(response_tokens as i64),
        cached_input_tokens: Set(cached_input_tokens as i64),
        cost: Set(cost_usd),
        created_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

/// 聚合全量与今日的请求数、token 数、估算美元成本。
///
/// 单条 SQL 同时取请求数、各类 token 数与成本，避免多次往返。
/// 今日边界用 `today_start_local_ts`（本地时区 0 点），与
/// `get_connected_programs` 保持一致，避免中国用户凌晨 0–8 点的"今日"统计偏移。
pub async fn get_metrics(db: &DatabaseConnection) -> Result<GatewayMetrics> {
    let today_start = today_start_local_ts();

    #[derive(Debug, FromQueryResult)]
    struct MetricsRow {
        total_requests: i64,
        total_request_tokens: i64,
        total_response_tokens: i64,
        total_cost_usd: Option<f64>,
    }

    let default_row = || MetricsRow {
        total_requests: 0,
        total_request_tokens: 0,
        total_response_tokens: 0,
        total_cost_usd: Some(0.0),
    };

    let all = MetricsRow::find_by_statement(Statement::from_string(
        DatabaseBackend::Sqlite,
        "SELECT COUNT(*) as total_requests, \
         COALESCE(SUM(request_tokens), 0) as total_request_tokens, \
         COALESCE(SUM(response_tokens), 0) as total_response_tokens, \
         COALESCE(SUM(cost), 0.0) as total_cost_usd \
         FROM gateway_usage",
    ))
    .one(db)
    .await?
    .unwrap_or_else(default_row);

    let today = MetricsRow::find_by_statement(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "SELECT COUNT(*) as total_requests, \
         COALESCE(SUM(request_tokens), 0) as total_request_tokens, \
         COALESCE(SUM(response_tokens), 0) as total_response_tokens, \
         COALESCE(SUM(cost), 0.0) as total_cost_usd \
         FROM gateway_usage WHERE created_at >= ?",
        [today_start.into()],
    ))
    .one(db)
    .await?
    .unwrap_or_else(default_row);

    Ok(GatewayMetrics {
        total_requests: all.total_requests as u64,
        total_tokens: (all.total_request_tokens + all.total_response_tokens) as u64,
        total_request_tokens: all.total_request_tokens as u64,
        total_response_tokens: all.total_response_tokens as u64,
        active_connections: 0, // 运行时状态，不在 DB 中追踪
        today_requests: today.total_requests as u64,
        today_tokens: (today.total_request_tokens + today.total_response_tokens) as u64,
        today_request_tokens: today.total_request_tokens as u64,
        today_response_tokens: today.total_response_tokens as u64,
        total_cost_usd: all.total_cost_usd.unwrap_or(0.0),
        today_cost_usd: today.total_cost_usd.unwrap_or(0.0),
    })
}

pub async fn get_usage_by_key(db: &DatabaseConnection) -> Result<Vec<UsageByKey>> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT gu.key_id, gk.name as key_name, \
             COUNT(*) as request_count, \
             COALESCE(SUM(gu.request_tokens + gu.response_tokens), 0) as token_count, \
             COALESCE(SUM(gu.request_tokens), 0) as request_tokens, \
             COALESCE(SUM(gu.response_tokens), 0) as response_tokens \
             FROM gateway_usage gu \
             JOIN gateway_keys gk ON gk.id = gu.key_id \
             GROUP BY gu.key_id \
             ORDER BY token_count DESC"
                .to_string(),
        ))
        .await?;

    rows.into_iter()
        .map(|r| {
            Ok(UsageByKey {
                key_id: r.try_get("", "key_id")?,
                key_name: r.try_get("", "key_name")?,
                request_count: r.try_get::<i64>("", "request_count")? as u64,
                token_count: r.try_get::<i64>("", "token_count")? as u64,
                request_tokens: r.try_get::<i64>("", "request_tokens")? as u64,
                response_tokens: r.try_get::<i64>("", "response_tokens")? as u64,
            })
        })
        .collect()
}

pub async fn get_usage_by_provider(db: &DatabaseConnection) -> Result<Vec<UsageByProvider>> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT gu.provider_id, COALESCE(p.name, gu.provider_id) as provider_name, \
             COUNT(*) as request_count, \
             COALESCE(SUM(gu.request_tokens + gu.response_tokens), 0) as token_count, \
             COALESCE(SUM(gu.request_tokens), 0) as request_tokens, \
             COALESCE(SUM(gu.response_tokens), 0) as response_tokens \
             FROM gateway_usage gu \
             LEFT JOIN providers p ON p.id = gu.provider_id \
             GROUP BY gu.provider_id \
             ORDER BY token_count DESC"
                .to_string(),
        ))
        .await?;

    rows.into_iter()
        .map(|r| {
            Ok(UsageByProvider {
                provider_id: r.try_get("", "provider_id")?,
                provider_name: r.try_get("", "provider_name")?,
                request_count: r.try_get::<i64>("", "request_count")? as u64,
                token_count: r.try_get::<i64>("", "token_count")? as u64,
                request_tokens: r.try_get::<i64>("", "request_tokens")? as u64,
                response_tokens: r.try_get::<i64>("", "response_tokens")? as u64,
            })
        })
        .collect()
}

pub async fn get_usage_by_day(db: &DatabaseConnection, days: u32) -> Result<Vec<UsageByDay>> {
    let since = now_ts() - (days as i64 * 86400);

    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT date(created_at, 'unixepoch') as date, \
             COUNT(*) as request_count, \
             COALESCE(SUM(request_tokens + response_tokens), 0) as token_count, \
             COALESCE(SUM(request_tokens), 0) as request_tokens, \
             COALESCE(SUM(response_tokens), 0) as response_tokens \
             FROM gateway_usage \
             WHERE created_at >= ? \
             GROUP BY date \
             ORDER BY date ASC",
            vec![since.into()],
        ))
        .await?;

    rows.into_iter()
        .map(|r| {
            Ok(UsageByDay {
                date: r.try_get("", "date")?,
                request_count: r.try_get::<i64>("", "request_count")? as u64,
                token_count: r.try_get::<i64>("", "token_count")? as u64,
                request_tokens: r.try_get::<i64>("", "request_tokens")? as u64,
                response_tokens: r.try_get::<i64>("", "response_tokens")? as u64,
            })
        })
        .collect()
}

pub async fn get_connected_programs(db: &DatabaseConnection) -> Result<Vec<ConnectedProgram>> {
    // 用本地时区今日 0 点，避免 UTC 日切换偏移
    let today_start = today_start_local_ts();
    let active_threshold = now_ts() - 300;

    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            concat!(
                "SELECT gk.id as key_id, gk.name as key_name, gk.key_prefix, ",
                "COALESCE(t.cnt, 0) as today_requests, ",
                "COALESCE(t.tokens, 0) as today_tokens, ",
                "COALESCE(t.request_tokens, 0) as today_request_tokens, ",
                "COALESCE(t.response_tokens, 0) as today_response_tokens, ",
                "gk.last_used_at as last_active_at ",
                "FROM gateway_keys gk ",
                "LEFT JOIN ( ",
                "SELECT key_id, COUNT(*) as cnt, ",
                "SUM(request_tokens + response_tokens) as tokens, ",
                "SUM(request_tokens) as request_tokens, ",
                "SUM(response_tokens) as response_tokens ",
                "FROM gateway_usage WHERE created_at >= ? ",
                "GROUP BY key_id ",
                ") t ON t.key_id = gk.id ",
                "WHERE gk.enabled = 1 ",
                "ORDER BY gk.created_at DESC",
            ),
            vec![today_start.into()],
        ))
        .await?;

    rows.into_iter()
        .map(|r| {
            let last_active_at: Option<i64> = r.try_get("", "last_active_at").ok();
            Ok(ConnectedProgram {
                key_id: r.try_get("", "key_id")?,
                key_name: r.try_get("", "key_name")?,
                key_prefix: r.try_get("", "key_prefix")?,
                today_requests: r.try_get::<i64>("", "today_requests")? as u64,
                today_tokens: r.try_get::<i64>("", "today_tokens")? as u64,
                today_request_tokens: r.try_get::<i64>("", "today_request_tokens")? as u64,
                today_response_tokens: r.try_get::<i64>("", "today_response_tokens")? as u64,
                last_active_at,
                is_active: last_active_at.map(|t| t >= active_threshold).unwrap_or(false),
            })
        })
        .collect()
}
