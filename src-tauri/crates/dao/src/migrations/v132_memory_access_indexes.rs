// SPDX-License-Identifier: AGPL-3.0-only
//! v132_memory_access_indexes: 为 memory_items 补衰减/淘汰路径所需索引。
//!
//! ## 背景
//!
//! 记忆衰减 tick（repo/memory.rs `apply_decay_tick`）与容量淘汰按以下列过滤/分组：
//! - 过期删除：`expires_at < now`
//! - 低分淘汰：`importance < 0.05`
//! - 衰减计算：`last_accessed IS NOT NULL`
//! - 容量淘汰：按 `(namespace_id, tier)` 分组 + `importance` 排序
//!
//! 上述列此前均无索引 → 每次 tick 全表扫描。本迁移补齐 4 个索引，
//! 全部使用 `CREATE INDEX IF NOT EXISTS`，幂等可重跑。

use sea_orm::{ConnectionTrait, DbErr};

const INDEX_SQLS: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_memory_items_expires_at ON memory_items(expires_at)",
    "CREATE INDEX IF NOT EXISTS idx_memory_items_last_accessed ON memory_items(last_accessed)",
    "CREATE INDEX IF NOT EXISTS idx_memory_items_importance ON memory_items(importance)",
    "CREATE INDEX IF NOT EXISTS idx_memory_items_ns_tier ON memory_items(namespace_id, tier)",
];

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    tracing::info!("[v132] 为 memory_items 补衰减/淘汰路径索引");

    for sql in INDEX_SQLS {
        match db.execute_unprepared(sql).await {
            Ok(_) => tracing::debug!("[v132] OK: {}", sql),
            Err(e) => {
                // 索引缺失只影响性能不影响正确性，容忍失败不阻断启动
                tracing::warn!("[v132] 建索引失败（容忍，不阻断启动）: {} — {}", sql, e);
            },
        }
    }

    tracing::info!("[v132] memory_items 索引迁移完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, Statement};

    #[tokio::test]
    async fn v132_creates_memory_indexes() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.expect("连接测试库");

        // 建最小 memory_items 表（仅含索引引用的列）
        db.execute_unprepared(
            "CREATE TABLE memory_items (\
             id TEXT PRIMARY KEY, \
             namespace_id TEXT NOT NULL DEFAULT '', \
             tier TEXT NOT NULL DEFAULT 'short_term', \
             importance REAL NOT NULL DEFAULT 0.5, \
             expires_at INTEGER, \
             last_accessed INTEGER)",
        )
        .await
        .expect("建表");

        super::up(db.clone()).await.expect("v132 迁移应成功");

        // 验证 4 个索引存在
        for name in [
            "idx_memory_items_expires_at",
            "idx_memory_items_last_accessed",
            "idx_memory_items_importance",
            "idx_memory_items_ns_tier",
        ] {
            let row = db
                .query_one_raw(Statement::from_string(
                    sea_orm::DatabaseBackend::Sqlite,
                    format!("SELECT name FROM sqlite_master WHERE type='index' AND name='{name}'"),
                ))
                .await
                .expect("查询索引")
                .expect("索引应存在");
            let idx_name: String = row.try_get("", "name").expect("读取索引名");
            assert_eq!(idx_name, name);
        }

        // 幂等：重跑不报错
        super::up(db.clone()).await.expect("v132 幂等重跑应成功");
    }
}
