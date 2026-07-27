// SPDX-License-Identifier: AGPL-3.0-only
//! v105_kb_vault_kind: 为 knowledge_bases 表添加 kind/vault_path 字段。
//!
//! ## 背景
//!
//! 参考 DeepTutor `deeptutor/capabilities/obsidian/` 设计：
//! KB 类型分为 `Indexed`（默认，走 RAG 索引）和 `ConnectedVault`（指针型，
//! 指向用户已有的 Obsidian vault，agent 通过 9 个 `obsidian_*` 工具直接读写
//! live 文件，不索引、不向量化）。
//!
//! 本迁移加两列：
//! - `kind TEXT NOT NULL DEFAULT 'indexed'`：KB 类型字符串
//! - `vault_path TEXT`：ConnectedVault 类型时的 vault 根路径
//!
//! ## 幂等
//!
//! ALTER TABLE ADD COLUMN 在 SQLite 下不支持 IF NOT EXISTS，需先查
//! PRAGMA table_info；PostgreSQL 用 `ADD COLUMN IF NOT EXISTS`。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    if is_pg {
        // PostgreSQL: 原生支持 IF NOT EXISTS
        for sql in &[
            "ALTER TABLE knowledge_bases ADD COLUMN IF NOT EXISTS kind TEXT NOT NULL DEFAULT 'indexed'",
            "ALTER TABLE knowledge_bases ADD COLUMN IF NOT EXISTS vault_path TEXT",
        ] {
            db.execute_unprepared(sql).await?;
        }
    } else {
        // SQLite: ALTER TABLE 不支持 IF NOT EXISTS，需先查 PRAGMA
        let existing_cols = existing_columns(&db, "knowledge_bases").await?;
        if !existing_cols.iter().any(|c| c == "kind") {
            db.execute_unprepared(
                "ALTER TABLE knowledge_bases ADD COLUMN kind TEXT NOT NULL DEFAULT 'indexed'",
            )
            .await?;
        }
        if !existing_cols.iter().any(|c| c == "vault_path") {
            db.execute_unprepared("ALTER TABLE knowledge_bases ADD COLUMN vault_path TEXT").await?;
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
    async fn v105_adds_kind_and_vault_path_columns() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        // 先跑 v100 建 knowledge_bases 表
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        // 再跑 v105
        up(db.clone()).await.unwrap();

        let cols = existing_columns(&db, "knowledge_bases").await.unwrap();
        assert!(cols.iter().any(|c| c == "kind"), "kind column should exist");
        assert!(cols.iter().any(|c| c == "vault_path"), "vault_path column should exist");
    }

    #[tokio::test]
    async fn v105_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑：列已存在，应跳过 ALTER，不报错
        up(db).await.expect("v105 must be re-runnable in isolation");
    }
}
