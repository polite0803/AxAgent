// SPDX-License-Identifier: AGPL-3.0-only
//! v118: 为 wikis 表添加 knowledge_base_id 字段，建立 Wiki 与 KB 的显式关联。
//!
//! ## Background
//!
//! 修复架构缺陷：原代码在融合 Wiki 图谱与知识图谱时硬编码假设
//! `wiki_id == kb_id`，这在绝大多数场景下不成立。
//!
//! ## Strategy
//!
//! - 为 wikis 表新增 knowledge_base_id 列
//! - 建立 Wiki 与 KnowledgeBase 的 1:1 关联（可选，Option<String>）
//! - 使用 exec_ddl 做幂等 + SQLite/PG 兼容
//! - 迁移完成后，get_wiki_graph_cached 等接口可以通过关联字段
//!   获取正确的 kb_id，而非依赖 ID 相等的隐式约定

use sea_orm::{DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // PHASE 1: 为 wikis 表添加 knowledge_base_id 列
    exec_ddl(&db, is_pg, "ALTER TABLE wikis ADD COLUMN knowledge_base_id TEXT").await.or_else(
        |e| {
            tracing::warn!("[v118] knowledge_base_id 列可能已存在，忽略错误: {}", e);
            Ok::<(), DbErr>(())
        },
    )?;

    tracing::info!("[v118] Added knowledge_base_id column to wikis table");
    Ok(())
}
