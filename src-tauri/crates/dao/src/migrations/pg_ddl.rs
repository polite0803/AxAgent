// SPDX-License-Identifier: AGPL-3.0-only
//! PostgreSQL DDL 适配工具 —— 供各 migration 共用。
//!
//! 历史：v001 在内部定义 `pg_ddl()` 私有函数做 SQLite→PG 的字符串替换。
//! 后续 v005/v006/v007/v008 也写了含 `INTEGER`/`AUTOINCREMENT` 的 DDL，
//! 在 PG 上同样踩坑（`created_at INTEGER` 在 PG 下变 INT4，SeaORM entity
//! 字段是 `i64` → INT8，类型不匹配）。把 `pg_ddl` 提到这里共享避免重复。
//!
//! 仅做保守、确定性的字符串替换，不触碰语义。
//!
//! ## REAL 精度问题
//!
//! SQLite 的 `REAL` 是 8 字节双精度（对应 Rust `f64`），但 PostgreSQL 的
//! `REAL` 是 4 字节单精度（对应 Rust `f32`，即 FLOAT4）。entity 中绝大多数
//! 浮点字段是 `f64`，在 PG 上必须用 `DOUBLE PRECISION`（FLOAT8）才能匹配。
//! 因此 `pg_ddl` 默认把所有 `REAL` 替换为 `DOUBLE PRECISION`，再把 entity
//! 中确为 `f32` 的列（`retrieval_threshold`、`avg_reward`）替换回 `REAL`。

/// 把 SQLite 风格 DDL 转成 PostgreSQL 兼容写法。
///
/// 替换规则：
/// - `AUTOINCREMENT`（SQLite 专有）→ 删除（PG 用 `SERIAL`/`BIGSERIAL` 表达自增）；
/// - `INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT` / `INTEGER PRIMARY KEY AUTOINCREMENT`
///   → `BIGSERIAL PRIMARY KEY`（i64 PK，INT8 序列）。
///   **背景**：SQLite 限制 `AUTOINCREMENT` 只能用于 `INTEGER PRIMARY KEY`，而
///   SQLite 的 `INTEGER` 本身就是 64 位（与 Rust `i64` 兼容），因此所有 i64 自增
///   主键表在 SQLite DDL 中统一写为 `INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT`。
///   到了 PG 侧，本函数把这种写法转成 `BIGSERIAL PRIMARY KEY`（INT8 序列）以
///   匹配 entity 的 `i64` 字段。
/// - `BIGINT NOT NULL PRIMARY KEY` / `BIGINT PRIMARY KEY` → `BIGSERIAL PRIMARY KEY`
///   （i64 PK，INT8 序列）；保留以兼容历史 DDL。
/// - `INTEGER NOT NULL PRIMARY KEY` / `INTEGER PRIMARY KEY` → `SERIAL PRIMARY KEY`
///   （i32 PK，INT4 序列）；仅对**不带** AUTOINCREMENT 的 INTEGER PK 生效，
///   因为带 AUTOINCREMENT 的已经被上面的规则转成 BIGSERIAL。
/// - `datetime('now')`（SQLite 函数）→ PG `to_char(...)`，输出格式与 DAO 写入的
///   `"%Y-%m-%d %H:%M:%S"` 一致（UTC 无时区无微秒），确保字符串排序与时序一致。
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
///   / `last_sync_at`）的 `INTEGER` → `BIGINT`。
/// - `REAL` → `DOUBLE PRECISION`（PG 上 `REAL` 是 FLOAT4，entity 里 `f64` 需要
///   FLOAT8）；但 `retrieval_threshold` 和 `avg_reward` 的 entity 字段是 `f32`，
///   保持 `REAL`。
pub fn pg_ddl(sql: &str) -> String {
    // 替换顺序非常重要：
    //   1. 先处理带 AUTOINCREMENT 的 INTEGER PK（i64 自增主键的 SQLite 写法）→ BIGSERIAL。
    //      必须在删除 AUTOINCREMENT 和 INTEGER→SERIAL 转换之前处理，否则会丢失
    //      AUTOINCREMENT 信号导致 i64 PK 被错误降级为 SERIAL（INT4）。
    //   2. 再删除剩余的 AUTOINCREMENT（兼容历史 BIGINT NOT NULL PRIMARY KEY AUTOINCREMENT
    //      写法，虽然 v100 已经统一为 INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT）。
    //   3. 最后处理不带 AUTOINCREMENT 的 BIGINT/INTEGER PK → BIGSERIAL/SERIAL。
    let s = sql
        // i64 自增 PK：SQLite 写 INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT
        // （SQLite 的 INTEGER 是 64 位，与 Rust i64 兼容），PG 需要 BIGSERIAL。
        // 必须在删除 AUTOINCREMENT 和 INTEGER→SERIAL 转换之前处理。
        .replace(
            "INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT",
            "BIGSERIAL PRIMARY KEY",
        )
        .replace("INTEGER PRIMARY KEY AUTOINCREMENT", "BIGSERIAL PRIMARY KEY")
        // 删除剩余的 AUTOINCREMENT（如果有遗漏）
        .replace(" AUTOINCREMENT", "")
        .replace("BIGINT NOT NULL PRIMARY KEY", "BIGSERIAL PRIMARY KEY")
        .replace("BIGINT PRIMARY KEY", "BIGSERIAL PRIMARY KEY")
        .replace("INTEGER NOT NULL PRIMARY KEY", "SERIAL PRIMARY KEY")
        .replace("INTEGER PRIMARY KEY", "SERIAL PRIMARY KEY")
        .replace(
            "datetime('now')",
            "to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')",
        );

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
        .replace("total_tokens INTEGER", "total_tokens BIGINT")
        .replace("vector_count INTEGER", "vector_count BIGINT")
        .replace("pending_count INTEGER", "pending_count BIGINT")
        .replace("processing_count INTEGER", "processing_count BIGINT")
        .replace("failed_count INTEGER", "failed_count BIGINT");

    // 业务数字字段
    let s = s
        .replace("duration_ms INTEGER", "duration_ms BIGINT")
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
        .replace("y INTEGER", "y BIGINT");

    // REAL 精度修正：PG 的 `REAL` 是 FLOAT4（f32），但 entity 中绝大多数浮点字段
    // 是 `f64`，需要 `DOUBLE PRECISION`（FLOAT8）。先把所有 `REAL` 升级为
    // `DOUBLE PRECISION`，再把 entity 确为 `f32` 的列（retrieval_threshold、
    // avg_reward）替换回 `REAL`。
    s.replace(" REAL", " DOUBLE PRECISION")
        .replace("retrieval_threshold DOUBLE PRECISION", "retrieval_threshold REAL")
        .replace("avg_reward DOUBLE PRECISION", "avg_reward REAL")
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
