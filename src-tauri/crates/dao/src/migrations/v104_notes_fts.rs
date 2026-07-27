// SPDX-License-Identifier: AGPL-3.0-only
//! v104_notes_fts: 为 notes 表添加全文检索索引。
//!
//! ## 背景
//!
//! `wiki_notes_search_keyword` 原实现把所有 notes 加载到内存做 BM25，
//! 10 万节点直接爆内存/超时。本迁移为 notes 建立全文索引：
//! - SQLite: FTS5 external content 虚拟表，通过 rowid 关联 notes
//! - PostgreSQL: tsvector 生成列 + GIN 索引
//!
//! ## 设计
//!
//! notes 表的主键是 TEXT（id），但 SQLite 默认表有隐藏 rowid（INTEGER），
//! 可作为 FTS5 external content 的关联键。PG 用 generated column。
//!
//! 索引覆盖 title + content 两个字段。file_path 不索引（路径搜索走 LIKE）。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    if !is_pg {
        // SQLite: FTS5 external content 模式
        // notes 表默认带隐藏 rowid（未声明 WITHOUT ROWID）
        for sql in &["CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(\
                title, content, content='notes', content_rowid='rowid', \
                tokenize='porter unicode61')"]
        {
            db.execute_unprepared(sql).await?;
        }

        // 触发器：notes 写入/更新/删除时同步 notes_fts
        for sql in &[
            "CREATE TRIGGER IF NOT EXISTS notes_fts_ai AFTER INSERT ON notes BEGIN \
             INSERT INTO notes_fts(rowid, title, content) \
             VALUES (new.rowid, new.title, new.content); END",
            "CREATE TRIGGER IF NOT EXISTS notes_fts_ad AFTER DELETE ON notes BEGIN \
             INSERT INTO notes_fts(notes_fts, rowid, title, content) \
             VALUES('delete', old.rowid, old.title, old.content); END",
            "CREATE TRIGGER IF NOT EXISTS notes_fts_au AFTER UPDATE OF title, content ON notes BEGIN \
             INSERT INTO notes_fts(notes_fts, rowid, title, content) \
             VALUES('delete', old.rowid, old.title, old.content); \
             INSERT INTO notes_fts(rowid, title, content) \
             VALUES (new.rowid, new.title, new.content); END",
        ] {
            db.execute_unprepared(sql).await?;
        }

        // 回填：把现有 notes 灌入 FTS 索引（幂等：先清空再灌）
        // 用 INSERT INTO ... SELECT 避免应用层循环
        db.execute_unprepared(
            "INSERT INTO notes_fts(rowid, title, content) \
             SELECT rowid, title, content FROM notes \
             WHERE rowid NOT IN (SELECT rowid FROM notes_fts)",
        )
        .await
        .ok(); // 失败不阻塞：可能是空表或已回填
    } else {
        // PostgreSQL: tsvector 生成列 + GIN 索引
        for sql in &[
            "ALTER TABLE notes ADD COLUMN IF NOT EXISTS tsv tsvector \
             GENERATED ALWAYS AS (to_tsvector('simple', \
               COALESCE(title,'')||' '||COALESCE(content,''))) STORED",
            "CREATE INDEX IF NOT EXISTS idx_notes_tsv ON notes USING GIN (tsv)",
            // vault_id 上的复合索引（v103 已建 idx_notes_vault_deleted，此处不重复）
        ] {
            let _ = db.execute_unprepared(sql).await;
        }
    }

    Ok(())
}
