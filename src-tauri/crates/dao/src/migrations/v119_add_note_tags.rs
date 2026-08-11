// SPDX-License-Identifier: AGPL-3.0-only
//! v119: 为 notes 表添加 tags 字段，用于存储笔记的标签列表。
//!
//! ## Background
//!
//! 修复 Wiki 图谱节点标签缺失问题：原代码在 get_vault_graph 中返回空 tags，
//! 导致前端 tooltip 无法显示节点标签。tags 需要从 content 中提取（以 `#` 开头的行），
//! 但加载 content 会增加内存开销。解决方案是在 notes 表中持久化存储 tags，
//! 在保存/更新笔记时自动提取并更新。
//!
//! ## Strategy
//!
//! - 为 notes 表新增 tags 列（TEXT 类型，存储 JSON 数组）
//! - 使用 exec_ddl 做幂等 + SQLite/PG 兼容
//! - 后续在保存笔记时自动提取 tags 并更新此字段
//! - 迁移时对历史笔记分批回填（tags IS NULL 的记录从 content 提取），
//!   避免大 vault 一次性内存峰值；空 tags 也写入 `[]` 使回填可重入
//! - list_notes_for_graph 查询时加载 tags 列

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DbBackend, DbErr, EntityTrait, QueryFilter, QuerySelect, Set,
};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // PHASE 1: 为 notes 表添加 tags 列
    exec_ddl(&db, is_pg, "ALTER TABLE notes ADD COLUMN tags TEXT").await.or_else(|e| {
        tracing::warn!("[v119] notes.tags 列可能已存在，忽略错误: {}", e);
        Ok::<(), DbErr>(())
    })?;

    // PHASE 2: 回填历史笔记 tags（分批；up 失败重跑时仅处理仍为 NULL 的记录）
    backfill_tags(&db).await?;

    tracing::info!("[v119] Added tags column to notes table (backfill done)");
    Ok(())
}

/// 为 tags 为空的历史笔记从 content 提取标签。
/// 分批处理（每批 500 条）避免大 vault 一次性内存峰值；
/// 单条失败仅告警不中断，保证迁移可重入（已处理的记录 tags 非 NULL，下次跳过）。
async fn backfill_tags(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    use crate::axagent_entities::notes;

    let batch_size = 500u64;
    loop {
        let rows = notes::Entity::find()
            .filter(notes::Column::Tags.is_null())
            .limit(batch_size)
            .all(db)
            .await?;

        if rows.is_empty() {
            break;
        }

        for model in &rows {
            let tags = crate::repo::note::extract_tags_from_content(&model.content);
            let tags_json =
                serde_json::to_value(tags).unwrap_or_else(|_| sea_orm::JsonValue::Array(vec![]));

            let am = notes::ActiveModel {
                id: Set(model.id.clone()),
                tags: Set(Some(tags_json)),
                ..Default::default()
            };
            if let Err(e) = am.update(db).await {
                tracing::warn!("[v119] 笔记 {} tags 回填失败: {}", model.id, e);
            }
        }
    }

    Ok(())
}
