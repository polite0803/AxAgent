// SPDX-License-Identifier: AGPL-3.0-only
//! v134: 为 workflow_templates 表添加 hooks_config 列（模板级生命周期钩子声明）。
//!
//! ## Background
//!
//! WorkEngine 此前只有"执行 DAG"一种语义，没有生命周期扩展点。下游 fork 的
//! 业务增强（数据预检/教训注入/结果持久化）只能在通用层硬编码业务分支或放弃。
//! 本列承载模板声明的生命周期钩子名列表（JSON）：
//! `{"pre_exec": ["hook-a"], "post_exec": ["hook-b"]}`。
//!
//! 通用层（harness / rt-workflow）只认协议与运行时注册表，不感知业务名；
//! 钩子实现由业务侧经 `WorkEngine::register_lifecycle_hook` 注入。
//!
//! ## Strategy
//!
//! 与 v121 一致的兼容写法：先查缺（PG 用 information_schema，SQLite 用
//! pragma_table_info），再执行普通 `ADD COLUMN`（不用 `ADD COLUMN IF NOT
//! EXISTS`——较老 SQLite 不支持该语法）。列类型 `TEXT`，可空，无默认值
//! （NULL 合法 = 旧模板无钩子，行为不变）。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    let exists = if is_pg {
        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT 1 AS exists_flag FROM information_schema.columns \
                 WHERE table_schema = current_schema() \
                   AND table_name = 'workflow_templates' AND column_name = 'hooks_config'",
            ))
            .await?;
        row.is_some()
    } else {
        let rows = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT name FROM pragma_table_info(?)",
                ["workflow_templates".into()],
            ))
            .await?;
        rows.iter().any(|r| {
            r.try_get_by::<String, _>("name").map(|n| n == "hooks_config").unwrap_or(false)
        })
    };

    if exists {
        tracing::info!("[v134] workflow_templates.hooks_config 已存在，跳过");
        return Ok(());
    }

    db.execute_unprepared("ALTER TABLE workflow_templates ADD COLUMN hooks_config TEXT").await?;

    tracing::info!("[v134] Added hooks_config column to workflow_templates");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    /// v134 单独幂等：重复跑不报错。
    #[tokio::test]
    async fn v134_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        // 先建表（v100）
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v134 must be re-runnable in isolation");
    }

    /// 防回归：v134 之后 workflow_templates 必须存在 hooks_config 列。
    #[tokio::test]
    async fn v134_adds_hooks_config_column() {
        use sea_orm::{ConnectionTrait, Statement};
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        super::super::v100_consolidated::up(db.clone()).await.expect("测试：异步操作应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='workflow_templates'",
            ))
            .await
            .expect("测试应成功")
            .expect("workflow_templates 应存在");
        let ddl: String = row.try_get_by("sql").unwrap_or_default();
        assert!(
            ddl.contains("hooks_config"),
            "workflow_templates 应包含 hooks_config 列，实际: {}",
            ddl
        );
    }
}
