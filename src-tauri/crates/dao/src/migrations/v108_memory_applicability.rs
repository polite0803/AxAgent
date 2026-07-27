// SPDX-License-Identifier: AGPL-3.0-only
//! v108_memory_applicability: 为 memory_items 表添加 applicability_tags + confirmed 字段。
//!
//! ## 背景
//!
//! 自进化闭环「标准记忆管理」要求:
//! - 划分记忆的适用范围边界(applicability_tags)
//! - 人工确认关键记忆的准确性(confirmed)
//!
//! Reflector 自动沉淀的经验默认 `confirmed=0`(未确认),
//! 仅在晋升到 core 层(promote_memory_entry)时需要 `confirmed=1` 门槛。
//! `applicability_tags` 为 JSON 数组字符串(如 `["rust","frontend"]`),
//! RAG 检索时可按当前任务上下文标签过滤,降低无关记忆干扰。
//!
//! 本迁移加两列:
//! - `applicability_tags TEXT`: JSON 数组字符串,默认 '[]'
//! - `confirmed INTEGER NOT NULL DEFAULT 0`: 0=未确认, 1=已确认
//!
//! ## 幂等
//!
//! SQLite: ALTER TABLE 不支持 IF NOT EXISTS,需先查 PRAGMA;
//! PostgreSQL: 用 `ADD COLUMN IF NOT EXISTS`。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    if is_pg {
        db.execute_unprepared(
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS applicability_tags TEXT DEFAULT '[]'",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE memory_items ADD COLUMN IF NOT EXISTS confirmed INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
    } else {
        let existing_cols = existing_columns(&db, "memory_items").await?;
        if !existing_cols.iter().any(|c| c == "applicability_tags") {
            db.execute_unprepared(
                "ALTER TABLE memory_items ADD COLUMN applicability_tags TEXT DEFAULT '[]'",
            )
            .await?;
        }
        if !existing_cols.iter().any(|c| c == "confirmed") {
            db.execute_unprepared(
                "ALTER TABLE memory_items ADD COLUMN confirmed INTEGER NOT NULL DEFAULT 0",
            )
            .await?;
        }
    }

    Ok(())
}

/// 查询指定表的所有列名（SQLite 走 PRAGMA，PG 走 information_schema）
async fn existing_columns(
    db: &sea_orm::DatabaseConnection,
    table: &str,
) -> Result<Vec<String>, DbErr> {
    let backend = db.get_database_backend();
    let rows = match backend {
        DbBackend::Sqlite => {
            let stmt = Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM pragma_table_info(?)",
                [table.into()],
            );
            db.query_all_raw(stmt).await?
        },
        DbBackend::Postgres => {
            let stmt = Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT column_name AS name FROM information_schema.columns \
                 WHERE table_name = $1",
                [table.into()],
            );
            db.query_all_raw(stmt).await?
        },
        _ => return Ok(vec![]),
    };

    let mut cols = Vec::with_capacity(rows.len());
    for row in rows {
        if let Ok(name) = row.try_get_by::<String, _>("name") {
            cols.push(name);
        }
    }
    Ok(cols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn v108_adds_applicability_and_confirmed_columns() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 先跑 v100 建 memory_items 表
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        // 再跑 v108
        up(db.clone()).await.unwrap();

        let cols = existing_columns(&db, "memory_items").await.unwrap();
        assert!(
            cols.iter().any(|c| c == "applicability_tags"),
            "applicability_tags column should exist"
        );
        assert!(cols.iter().any(|c| c == "confirmed"), "confirmed column should exist");
    }

    #[tokio::test]
    async fn v108_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑：列已存在，应跳过 ALTER，不报错
        up(db).await.expect("v108 must be re-runnable in isolation");
    }
}
