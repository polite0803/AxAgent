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
//! ## 幂等策略
//!
//! - PostgreSQL: `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`，原生幂等
//! - SQLite: 直接 `ADD COLUMN`，列已存在时会报错，用 `let _ = ...` 忽略错误

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    if is_pg {
        for sql in &[
            "ALTER TABLE knowledge_bases ADD COLUMN IF NOT EXISTS kind TEXT NOT NULL DEFAULT 'indexed'",
            "ALTER TABLE knowledge_bases ADD COLUMN IF NOT EXISTS vault_path TEXT",
        ] {
            db.execute_unprepared(sql).await?;
        }
    } else {
        let _ = db
            .execute_unprepared(
                "ALTER TABLE knowledge_bases ADD COLUMN kind TEXT NOT NULL DEFAULT 'indexed'",
            )
            .await;
        let _ =
            db.execute_unprepared("ALTER TABLE knowledge_bases ADD COLUMN vault_path TEXT").await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, DbBackend, Statement};

    #[tokio::test]
    async fn v105_adds_kind_and_vault_path_columns() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();

        // 直接查表是否有 kind 列：尝试 SELECT kind，如果列不存在会报错
        let result = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT kind FROM knowledge_bases LIMIT 0",
            ))
            .await;
        assert!(result.is_ok(), "kind column should exist in knowledge_bases");

        let result = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT vault_path FROM knowledge_bases LIMIT 0",
            ))
            .await;
        assert!(result.is_ok(), "vault_path column should exist in knowledge_bases");
    }

    #[tokio::test]
    async fn v105_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        up(db).await.expect("v105 must be re-runnable in isolation");
    }
}
