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
/// - `BIGINT PRIMARY KEY` → `BIGSERIAL PRIMARY KEY`；
/// - `datetime('now')`（SQLite 函数）→ `CURRENT_TIMESTAMP::text`；
/// - 时间戳列（`created_at` / `updated_at` / `completed_at` 等）的
///   `INTEGER` → `BIGINT`（PG 下 INT4 → INT8，匹配 SeaORM entity 里 `i64`）。
///   仅在已知时间戳列名上做替换，避免误伤其它业务列。
pub fn pg_ddl(sql: &str) -> String {
    sql.replace(" AUTOINCREMENT", "")
        .replace("INTEGER NOT NULL PRIMARY KEY", "SERIAL PRIMARY KEY")
        .replace("INTEGER PRIMARY KEY", "SERIAL PRIMARY KEY")
        .replace("BIGINT PRIMARY KEY", "BIGSERIAL PRIMARY KEY")
        .replace("datetime('now')", "CURRENT_TIMESTAMP::text")
        // 普通时间戳列：INTEGER (INT4) → BIGINT (INT8)，
        // 匹配 entity 里 `i64`。主键列已在上方转为 SERIAL，不会被这里命中。
        .replace("created_at INTEGER", "created_at BIGINT")
        .replace("updated_at INTEGER", "updated_at BIGINT")
        .replace("completed_at INTEGER", "completed_at BIGINT")
        .replace("processed_at INTEGER", "processed_at BIGINT")
        .replace("started_at INTEGER", "started_at BIGINT")
        .replace("last_used_at INTEGER", "last_used_at BIGINT")
        .replace("last_indexed_at INTEGER", "last_indexed_at BIGINT")
        .replace("imported_at INTEGER", "imported_at BIGINT")
        // 业务列：DDL 写 INTEGER 但 entity 类型是 i64 → 也需要 BIGINT
        .replace("max_tokens INTEGER", "max_tokens BIGINT")
        .replace("thinking_budget INTEGER", "thinking_budget BIGINT")
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
