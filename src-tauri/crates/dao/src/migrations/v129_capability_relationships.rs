// SPDX-License-Identifier: AGPL-3.0-only
//! v129: 创建 capability_relationships 表 —— 能力关系图谱持久化（统一能力模型第四层）。
//!
//! ## Background
//!
//! 统一能力建模（docs/unified-capability-model.md）第四层定义 `CapabilityRelationship`
//! （source_id / target_id / relationship_type / weight / context / metadata），
//! 用于关联发现与编排。护照 `upstream`/`downstream` 字段此前只有声明式一跳扩展，
//! 无持久化、无关系类型/权重元信息。
//!
//! 本表是关系图谱的物化镜像 + 元信息载体：
//! - 启动时从护照声明重建（sync_from_passports，见 dao/repo/capability_relationship.rs）
//! - relationship_type 为 snake_case 字符串（depends_on / uses / alternative_to /
//!   conflicts_with / parent_of / precedes / follows / requires_knowledge）
//! - 复合主键 (source_id, target_id, relationship_type)：图边自然主键，
//!   规避 SQLite 下 BIGINT PRIMARY KEY 不自增的问题；upsert 幂等
//!
//! ## Strategy
//!
//! `CREATE TABLE IF NOT EXISTS` —— 幂等，可重复执行；SQLite 与 PostgreSQL 均支持。

use sea_orm::ConnectionTrait;
use sea_orm::DbErr;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS capability_relationships (\
         source_id TEXT NOT NULL, \
         target_id TEXT NOT NULL, \
         relationship_type TEXT NOT NULL, \
         weight REAL NOT NULL DEFAULT 1.0, \
         context TEXT, \
         metadata TEXT, \
         created_at BIGINT NOT NULL, \
         PRIMARY KEY (source_id, target_id, relationship_type))",
    )
    .await?;

    tracing::info!("[v129] Created capability_relationships table");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use sea_orm::DbBackend;

    /// v129 单独幂等：重复跑不报错。
    #[tokio::test]
    async fn v129_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v129 must be re-runnable in isolation");
    }

    /// 防回归：v129 之后 capability_relationships 表必须存在且含全部列。
    #[tokio::test]
    async fn v129_creates_table() {
        use sea_orm::{ConnectionTrait, Statement};
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='capability_relationships'",
            ))
            .await
            .expect("测试应成功")
            .expect("capability_relationships 应存在");
        let ddl: String = row.try_get_by("sql").unwrap_or_default();
        assert!(
            ddl.contains("source_id")
                && ddl.contains("target_id")
                && ddl.contains("relationship_type")
                && ddl.contains("weight"),
            "capability_relationships 应含 source_id/target_id/relationship_type/weight 列，实际: {}",
            ddl
        );
    }
}
