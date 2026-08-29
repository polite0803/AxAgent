// SPDX-License-Identifier: AGPL-3.0-only
//! v128: 创建 capability_policies 表 —— 能力发现策略（后置过滤器规则）持久化。
//!
//! ## Background
//!
//! 规范要求"策略是过滤器，不是检索对象"：Policy 不应参与语义检索，而是作为
//! 后置过滤器对候选列表执行裁剪（如"内网环境删掉所有云 API 调用能力"）。
//! 项目此前把策略硬编码为 8 维过滤闸门（CapabilityFilter），无法动态注册/配置。
//!
//! 本表提供可注册的策略规则载体（Phase 3 策略对象化，渐进式）：
//! - 规则为 JSON（排除型）：`{"exclude_domains": [], "exclude_tags": [], "exclude_capability_ids": []}`
//! - `CapabilityFilterImpl` 在过滤候选前加载启用策略，对命中排除规则的能力做裁剪
//! - 现有 8 维硬编码闸门保留为内置默认规则，不冲突
//!
//! ## Strategy
//!
//! `CREATE TABLE IF NOT EXISTS` —— 幂等，可重复执行；SQLite 与 PostgreSQL 均支持。

use sea_orm::ConnectionTrait;
use sea_orm::DbErr;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS capability_policies (\
         id TEXT NOT NULL PRIMARY KEY, \
         name TEXT NOT NULL, \
         description TEXT, \
         rules_json TEXT NOT NULL DEFAULT '{}', \
         enabled BOOLEAN NOT NULL DEFAULT TRUE, \
         priority INTEGER NOT NULL DEFAULT 0, \
         created_at BIGINT NOT NULL, \
         updated_at BIGINT NOT NULL)",
    )
    .await?;

    tracing::info!("[v128] Created capability_policies table");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use sea_orm::DbBackend;

    /// v128 单独幂等：重复跑不报错。
    #[tokio::test]
    async fn v128_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v128 must be re-runnable in isolation");
    }

    /// 防回归：v128 之后 capability_policies 表必须存在且含全部列。
    #[tokio::test]
    async fn v128_creates_table() {
        use sea_orm::{ConnectionTrait, Statement};
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='capability_policies'",
            ))
            .await
            .expect("测试应成功")
            .expect("capability_policies 应存在");
        let ddl: String = row.try_get_by("sql").unwrap_or_default();
        assert!(
            ddl.contains("rules_json") && ddl.contains("enabled") && ddl.contains("priority"),
            "capability_policies 应含 rules_json/enabled/priority 列，实际: {}",
            ddl
        );
    }
}
