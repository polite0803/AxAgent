// SPDX-License-Identifier: AGPL-3.0-only
//! v126: 创建 narrative_structures 表 —— 叙事结构持久化。
//!
//! ## Background
//!
//! 前端叙事结构面板（NarrativeStructurePanel）与 workflowEditorStore 已完整
//! 实现叙事结构（弧线/交汇点/伏笔）的创建与管理，通过 5 个 IPC 命令
//! （list/get/create/update/delete_narrative_structure）持久化，但后端
//! 命令从未接线 —— 本迁移补齐存储层。
//!
//! ## Schema
//!
//! - `structure`: JSON 文本（arcs / confluences / foreshadows 三段结构），
//!   与前端 `src/types/narrative.ts` 的 `NarrativeStructure` 一一对应。
//! - `is_template`: true 表示结构模板（可复用的三幕式/英雄之旅等），
//!   false 表示挂在具体作品上的实例。
//! - `genre`: 体裁标签（小说/剧本/散文…），面板按体裁过滤。
//! - `version`: 每次更新 +1，用于乐观并发与历史追溯（当前仅递增不存快照）。
//!
//! ## Strategy
//!
//! `CREATE TABLE IF NOT EXISTS` + 索引 —— 幂等，可重复执行；SQLite 与
//! PostgreSQL 均支持该 ANSI DDL。

use sea_orm::ConnectionTrait;
use sea_orm::DbErr;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS narrative_structures (\
         id TEXT NOT NULL PRIMARY KEY, \
         name TEXT NOT NULL, \
         description TEXT, \
         genre TEXT NOT NULL, \
         structure TEXT NOT NULL, \
         is_template BOOLEAN NOT NULL DEFAULT FALSE, \
         version INTEGER NOT NULL DEFAULT 1, \
         created_at BIGINT NOT NULL, \
         updated_at BIGINT NOT NULL)",
    )
    .await?;

    // 面板加载路径：按模板/实例过滤 + 按体裁过滤
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_narrative_structures_template \
         ON narrative_structures(is_template)",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_narrative_structures_genre \
         ON narrative_structures(genre)",
    )
    .await?;

    tracing::info!("[v126] Created narrative_structures table");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use sea_orm::DbBackend;

    /// v126 单独幂等：重复跑不报错。
    #[tokio::test]
    async fn v126_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v126 must be re-runnable in isolation");
    }

    /// 防回归：v126 之后 narrative_structures 表与索引必须存在。
    #[tokio::test]
    async fn v126_creates_table_with_indexes() {
        use sea_orm::{ConnectionTrait, Statement};
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='narrative_structures'",
            ))
            .await
            .expect("测试应成功")
            .expect("narrative_structures 应存在");
        let ddl: String = row.try_get_by("sql").unwrap_or_default();
        assert!(
            ddl.contains("structure") && ddl.contains("is_template") && ddl.contains("genre"),
            "narrative_structures 应含 structure/is_template/genre 列，实际: {}",
            ddl
        );

        for idx in ["idx_narrative_structures_template", "idx_narrative_structures_genre"] {
            let row = db
                .query_one_raw(Statement::from_string(
                    DbBackend::Sqlite,
                    format!("SELECT name FROM sqlite_master WHERE type='index' AND name='{idx}'"),
                ))
                .await
                .expect("测试应成功");
            assert!(row.is_some(), "索引 {idx} 应存在");
        }
    }
}
