// SPDX-License-Identifier: AGPL-3.0-only

use axagent_entities::gateway_request_logs;
use axagent_harness::core_error::Result;
use axagent_harness::types::GatewayRequestLog;
use axagent_harness::util_fns::{gen_id, now_ts};
use sea_orm::*;

#[allow(clippy::too_many_arguments)]
pub async fn record_request_log(
    db: &DatabaseConnection,
    key_id: &str,
    key_name: &str,
    method: &str,
    path: &str,
    model: Option<&str>,
    provider_id: Option<&str>,
    status_code: i32,
    duration_ms: i32,
    request_tokens: i64,
    response_tokens: i64,
    error_message: Option<&str>,
) -> Result<()> {
    gateway_request_logs::ActiveModel {
        id: Set(gen_id()),
        key_id: Set(key_id.to_string()),
        key_name: Set(key_name.to_string()),
        method: Set(method.to_string()),
        path: Set(path.to_string()),
        model: Set(model.map(|s| s.to_string())),
        provider_id: Set(provider_id.map(|s| s.to_string())),
        status_code: Set(status_code),
        duration_ms: Set(duration_ms),
        request_tokens: Set(request_tokens),
        response_tokens: Set(response_tokens),
        error_message: Set(error_message.map(|s| s.to_string())),
        created_at: Set(now_ts()),
    }
    .insert(db)
    .await?;
    Ok(())
}

pub async fn list_request_logs(
    db: &DatabaseConnection,
    limit: u64,
    offset: u64,
) -> Result<Vec<GatewayRequestLog>> {
    let rows = gateway_request_logs::Entity::find()
        .order_by_desc(gateway_request_logs::Column::CreatedAt)
        .limit(limit)
        .offset(offset)
        .all(db)
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| GatewayRequestLog {
            id: r.id,
            key_id: r.key_id,
            key_name: r.key_name,
            method: r.method,
            path: r.path,
            model: r.model,
            provider_id: r.provider_id,
            status_code: r.status_code,
            duration_ms: r.duration_ms,
            request_tokens: r.request_tokens,
            response_tokens: r.response_tokens,
            error_message: r.error_message,
            created_at: r.created_at,
        })
        .collect())
}

pub async fn clear_request_logs(db: &DatabaseConnection) -> Result<u64> {
    let result = gateway_request_logs::Entity::delete_many().exec(db).await?;
    Ok(result.rows_affected)
}
