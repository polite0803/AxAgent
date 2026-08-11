// SPDX-License-Identifier: AGPL-3.0-only
//! v106_context_source_doc_ids: 为 context_sources 表添加 doc_ids_json 字段。
//!
//! ## 背景
//!
//! 第二阶段功能「多文档协同」：用户在会话中可指定知识库后，
//! 进一步勾选该 KB 下的具体文档（doc_id[]），RAG 检索时仅在这些
//! 文档内检索。需要持久化每个 context_source 关联的文档 ID 列表。
//!
//! 本迁移加一列：
//! - `doc_ids_json TEXT`：JSON 数组字符串，如 `["doc1","doc2"]`；
//!   空数组或 NULL 表示不限制（检索整个容器）
//!
//! ## 幂等
//!
//! SQLite: ALTER TABLE 不支持 IF NOT EXISTS，需先查 PRAGMA；
//! PostgreSQL: 用 `ADD COLUMN IF NOT EXISTS`。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    if is_pg {
        db.execute_unprepared(
            "ALTER TABLE context_sources ADD COLUMN IF NOT EXISTS doc_ids_json TEXT",
        )
        .await?;
    } else {
        let existing_cols = existing_columns(&db, "context_sources").await?;
        if !existing_cols.iter().any(|c| c == "doc_ids_json") {
            db.execute_unprepared("ALTER TABLE context_sources ADD COLUMN doc_ids_json TEXT")
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
    async fn v106_adds_doc_ids_json_column() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        // 先跑 v100 建 context_sources 表
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        // 再跑 v106
        up(db.clone()).await.expect("测试：异步操作应成功");

        let cols = existing_columns(&db, "context_sources").await.expect("测试：异步操作应成功");
        assert!(cols.iter().any(|c| c == "doc_ids_json"), "doc_ids_json column should exist");
    }

    #[tokio::test]
    async fn v106_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        // 第二次跑：列已存在，应跳过 ALTER，不报错
        up(db).await.expect("v106 must be re-runnable in isolation");
    }
}
