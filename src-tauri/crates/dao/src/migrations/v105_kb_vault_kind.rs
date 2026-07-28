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
//! 本迁移曾用 ALTER TABLE 加两列，现在由 v100 PHASE 3.9 全表合规检查统一处理，
//! 本迁移保留为无操作（兼容历史数据库标记此版本已应用）。
//!
//! ## 幂等
//!
//! 已由 v100 PHASE 3.9 保证列存在，本迁移无操作不依赖后端。

use sea_orm::DbErr;

pub async fn up(_db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    tracing::info!("[v105] 列定义已由 v100 PHASE 3.9 统一保障，无需 ALTER TABLE");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

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
