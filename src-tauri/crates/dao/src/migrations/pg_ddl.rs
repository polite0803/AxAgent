// SPDX-License-Identifier: AGPL-3.0-only
//! PostgreSQL DDL 适配工具 —— 供各 migration 共用。
//!
//! 历史：v001 在内部定义 `pg_ddl()` 私有函数做 SQLite→PG 的字符串替换。
//! 后续 v005/v006/v007/v008 也写了含 `INTEGER`/`AUTOINCREMENT` 的 DDL，
//! 在 PG 上同样踩坑（`created_at INTEGER` 在 PG 下变 INT4，SeaORM entity
//! 字段是 `i64` → INT8，类型不匹配）。把 `pg_ddl` 提到这里共享避免重复。
//!
//! 仅做保守、确定性的字符串替换，不触碰语义。

/// 把 SQLite 风格 DDL 转成 PostgreSQL 兼容写法。
///
/// 替换规则：
/// - `AUTOINCREMENT`（SQLite 专有）→ 删除（PG 用 `SERIAL`/`BIGSERIAL` 表达自增）；
/// - `INTEGER NOT NULL PRIMARY KEY` / `INTEGER PRIMARY KEY` → `SERIAL PRIMARY KEY`；
/// - `BIGINT NOT NULL PRIMARY KEY` → `BIGSERIAL PRIMARY KEY`；
/// - `BIGINT PRIMARY KEY` → `BIGSERIAL PRIMARY KEY`；
/// - `datetime('now')`（SQLite 函数）→ `CURRENT_TIMESTAMP::text`；
/// - 时间戳列（`created_at` / `updated_at` / `completed_at` 等）的
///   `INTEGER` → `BIGINT`（PG 下 INT4 → INT8，匹配 SeaORM entity 里 `i64`）。
///   仅在已知时间戳列名上做替换，避免误伤其它业务列。
/// - 业务列（`token_count` / `request_tokens` / `response_tokens` / `duration_ms`
///   / `last_validated_at` / `vector_count` / `size_bytes` / `max_tokens` /
///   `thinking_budget` / `timeout_seconds` / `downloads` / `file_size` / `safe_search`
///   / `rate_limit_per_minute` / `suggested_max_tokens` / `last_linted_at` /
///   / `last_compiled_at` / `user_edited_at` / `finished_at` / `global_rpm` /
///   / `per_model_rpm` / `token_limit_per_minute` / `latency_ms` / `last_sync_at` /
///   / `token_input` / `token_output` / `execution_time_ms` / `timestamp_ms` /
///   / `x` / `y` / `pending_count` / `processing_count` / `failed_count` /
///   / `last_sync_at`) 的 `INTEGER` → `BIGINT`。
pub fn pg_ddl(sql: &str) -> String {
    let s = sql
        .replace(" AUTOINCREMENT", "")
        .replace("INTEGER NOT NULL PRIMARY KEY", "SERIAL PRIMARY KEY")
        .replace("INTEGER PRIMARY KEY", "SERIAL PRIMARY KEY")
        .replace("BIGINT NOT NULL PRIMARY KEY", "BIGSERIAL PRIMARY KEY")
        .replace("BIGINT PRIMARY KEY", "BIGSERIAL PRIMARY KEY")
        .replace("datetime('now')", "CURRENT_TIMESTAMP::text");

    // 时间戳列替换（必须出现在相关业务列替换之前，避免被误匹配）
    let s = s
        .replace("created_at INTEGER", "created_at BIGINT")
        .replace("updated_at INTEGER", "updated_at BIGINT")
        .replace("completed_at INTEGER", "completed_at BIGINT")
        .replace("processed_at INTEGER", "processed_at BIGINT")
        .replace("started_at INTEGER", "started_at BIGINT")
        .replace("last_used_at INTEGER", "last_used_at BIGINT")
        .replace("last_indexed_at INTEGER", "last_indexed_at BIGINT")
        .replace("imported_at INTEGER", "imported_at BIGINT")
        .replace("last_validated_at INTEGER", "last_validated_at BIGINT")
        .replace("last_compiled_at INTEGER", "last_compiled_at BIGINT")
        .replace("last_linted_at INTEGER", "last_linted_at BIGINT")
        .replace("user_edited_at INTEGER", "user_edited_at BIGINT")
        .replace("last_sync_at INTEGER", "last_sync_at BIGINT")
        .replace("finished_at INTEGER", "finished_at BIGINT");

    // 计数字段
    let s = s
        .replace("max_tokens INTEGER", "max_tokens BIGINT")
        .replace("thinking_budget INTEGER", "thinking_budget BIGINT")
        .replace("token_count INTEGER", "token_count BIGINT")
        .replace("prompt_tokens INTEGER", "prompt_tokens BIGINT")
        .replace("completion_tokens INTEGER", "completion_tokens BIGINT")
        .replace("first_token_latency_ms INTEGER", "first_token_latency_ms BIGINT")
        .replace("cache_creation_tokens INTEGER", "cache_creation_tokens BIGINT")
        .replace("cache_read_tokens INTEGER", "cache_read_tokens BIGINT")
        .replace("request_tokens INTEGER", "request_tokens BIGINT")
        .replace("response_tokens INTEGER", "response_tokens BIGINT")
        .replace("cached_input_tokens INTEGER", "cached_input_tokens BIGINT")
        .replace("token_input INTEGER", "token_input BIGINT")
        .replace("token_output INTEGER", "token_output BIGINT")
        .replace("vector_count INTEGER", "vector_count BIGINT")
        .replace("pending_count INTEGER", "pending_count BIGINT")
        .replace("processing_count INTEGER", "processing_count BIGINT")
        .replace("failed_count INTEGER", "failed_count BIGINT");

    // 业务数字字段
    s.replace("duration_ms INTEGER", "duration_ms BIGINT")
        .replace("execution_time_ms INTEGER", "execution_time_ms BIGINT")
        .replace("timestamp_ms INTEGER", "timestamp_ms BIGINT")
        .replace("timeout_seconds INTEGER", "timeout_seconds BIGINT")
        .replace("downloads INTEGER", "downloads BIGINT")
        .replace("file_size INTEGER", "file_size BIGINT")
        .replace("safe_search INTEGER", "safe_search BIGINT")
        .replace("size_bytes INTEGER", "size_bytes BIGINT")
        .replace("rate_limit_per_minute INTEGER", "rate_limit_per_minute BIGINT")
        .replace("suggested_max_tokens INTEGER", "suggested_max_tokens BIGINT")
        .replace("global_rpm INTEGER", "global_rpm BIGINT")
        .replace("per_model_rpm INTEGER", "per_model_rpm BIGINT")
        .replace("token_limit_per_minute INTEGER", "token_limit_per_minute BIGINT")
        .replace("latency_ms INTEGER", "latency_ms BIGINT")
        .replace("x INTEGER", "x BIGINT")
        .replace("y INTEGER", "y BIGINT")
}

/// 按后端执行 DDL：PostgreSQL 下先经 [`pg_ddl`] 转换，SQLite 原样执行。
pub async fn exec_ddl(
    db: &sea_orm::DatabaseConnection,
    is_pg: bool,
    sql: &str,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::ConnectionTrait;
    let s = if is_pg { pg_ddl(sql) } else { sql.to_string() };
    db.execute_unprepared(&s).await?;
    Ok(())
}
