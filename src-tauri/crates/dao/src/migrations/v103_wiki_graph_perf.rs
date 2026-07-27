// SPDX-License-Identifier: AGPL-3.0-only
//! v103: Wiki 知识图谱性能优化 — 索引 + 图谱/社区缓存表。
//!
//! ## Background
//!
//! 现有 notes / note_links / note_backlinks 三张核心表**完全无二级索引**，
//! `get_vault_graph` 三次全表扫描；`wiki_graph_communities` 每次实时跑
//! Louvain（含 O(N²) modularity），10 万节点规模下不可用。
//!
//! ## Strategy
//!
//! 1. 给三张表加索引（vault_id / source_note_id / target_note_id 复合索引）
//!    —— 10 万节点下 `list_notes` / `get_vault_graph` / `get_note_links`
//!    查询从全表扫变为索引扫，性能提升 100-1000 倍。
//! 2. 新增 `wiki_graph_cache` 表：缓存 GraphData + LouvainResult 序列化
//!    JSON，按 `vault_id` 唯一，`updated_at` 失效。前端读取时优先命中
//!    缓存，避免每次实时计算。
//!
//! ## 索引设计说明
//!
//! - `idx_notes_vault_deleted` (vault_id, is_deleted)：list_notes /
//!   get_vault_graph 都按 vault_id + is_deleted=0 过滤，复合索引覆盖。
//! - `idx_note_links_vault_source` (vault_id, source_note_id)：
//!   sync_note_links 按 source 删除+插入，复合索引覆盖。
//! - `idx_note_links_vault_target` (vault_id, target_note_id)：
//!   get_note_backlinks 反向查询。
//! - note_backlinks 同理。
//!
//! ## 幂等性
//!
//! 全部 `CREATE INDEX IF NOT EXISTS` + `CREATE TABLE IF NOT EXISTS`，
//! 可重复执行。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: notes 表索引
    //
    // (vault_id, is_deleted)：覆盖 list_notes + get_vault_graph 的过滤条件
    // ========================================================================
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_notes_vault_deleted ON notes(vault_id, is_deleted)",
    )
    .await?;

    // ========================================================================
    // PHASE 2: note_links 表索引
    //
    // (vault_id, source_note_id)：覆盖 sync_note_links 的删除+按源查询
    // (vault_id, target_note_id)：覆盖反向链接查询
    // ========================================================================
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_note_links_vault_source ON note_links(vault_id, source_note_id)",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_note_links_vault_target ON note_links(vault_id, target_note_id)",
    )
    .await?;

    // ========================================================================
    // PHASE 3: note_backlinks 表索引（同 note_links）
    // ========================================================================
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_note_backlinks_vault_source ON note_backlinks(vault_id, source_note_id)",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_note_backlinks_vault_target ON note_backlinks(vault_id, target_note_id)",
    )
    .await?;

    // ========================================================================
    // PHASE 4: wiki_graph_cache 表
    //
    // 缓存 GraphData + LouvainResult 序列化 JSON。按 vault_id 唯一，
    // updated_at 用于失效判断（notes 表有更新时清除对应缓存）。
    //
    // DDL 用 PG 语法（BIGINT/TEXT），exec_ddl 在 SQLite 下原样执行。
    // ========================================================================
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS wiki_graph_cache (\
            vault_id TEXT PRIMARY KEY, \
            graph_data_json TEXT NOT NULL, \
            communities_json TEXT, \
            node_count INTEGER NOT NULL DEFAULT 0, \
            edge_count INTEGER NOT NULL DEFAULT 0, \
            computed_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL)",
    )
    .await?;

    Ok(())
}
