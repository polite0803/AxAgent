// SPDX-License-Identifier: AGPL-3.0-only
//! DDL 适配工具 —— 供各 migration 共用。
//!
//! 设计理念：DDL 直接写 PostgreSQL 语法（强类型，能区分 INT4/INT8/FLOAT4/FLOAT8），
//! SQLite 侧利用动态类型亲和性自动兼容，仅需转换 PG 独有的自增序列和日期函数。
//!
//! ## SQLite 类型亲和性
//!
//! SQLite 不区分 INT4/INT8/FLOAT4/FLOAT8，按类型名映射亲和性：
//! - `BIGINT`（含 "INT"）→ INTEGER 亲和性（64 位）
//! - `DOUBLE PRECISION`（含 "DOUB"）→ REAL 亲和性（8 字节双精度）
//! - `BOOLEAN` → NUMERIC 亲和性（存 0/1）
//! - `TEXT` → TEXT 亲和性
//!
//! 因此 `BIGINT`/`DOUBLE PRECISION`/`BOOLEAN`/`TEXT` 在 SQLite 下无需转换。
//! 只有 `BIGSERIAL`/`SERIAL`（PG 自增序列）和 `to_char`（PG 日期函数）需要转换。

/// 把 PostgreSQL 风格 DDL 转成 SQLite 兼容写法。
///
/// 仅 3 条确定性替换：
/// - `BIGSERIAL PRIMARY KEY` → `INTEGER PRIMARY KEY AUTOINCREMENT`（i64 自增主键）
/// - `SERIAL PRIMARY KEY` → `INTEGER PRIMARY KEY`（i32 自增主键）
/// - `to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')`
///   → `datetime('now')`（UTC 时间默认值）
///
/// 其他类型（BIGINT/DOUBLE PRECISION/BOOLEAN/TEXT）SQLite 直接接受。
pub fn sqlite_ddl(sql: &str) -> String {
    sql.replace("BIGSERIAL PRIMARY KEY", "INTEGER PRIMARY KEY AUTOINCREMENT")
        .replace("SERIAL PRIMARY KEY", "INTEGER PRIMARY KEY")
        .replace(
            "to_char(CURRENT_TIMESTAMP AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS')",
            "datetime('now')",
        )
}

/// 把 SQLite 风格 DDL 转成 PostgreSQL 兼容写法（已废弃，保留向后兼容）。
///
/// v100 consolidated migration 已改为直接写 PG 语法，新代码不应使用此函数。
pub fn pg_ddl(sql: &str) -> String {
    sql.to_string()
}

/// 按后端执行 DDL：SQLite 下经 [`sqlite_ddl`] 转换，PostgreSQL 原样执行。
pub async fn exec_ddl(
    db: &sea_orm::DatabaseConnection,
    is_pg: bool,
    sql: &str,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::ConnectionTrait;
    let s = if is_pg {
        sql.to_string()
    } else {
        sqlite_ddl(sql)
    };
    db.execute_unprepared(&s).await?;
    Ok(())
}
