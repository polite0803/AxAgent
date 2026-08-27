// SPDX-License-Identifier: AGPL-3.0-only
//! v123: 创建 workflow_tools 表 —— 工作流运行时工具定义持久化。
//!
//! ## Background
//!
//! 工作流执行时可能动态发现/生成所需工具（Rhai 脚本 / 工作流 DAG / LLM
//! 声明式工具），需要独立于 `workflow_template.tool_defs`（模板内置、随版本
//! 快照）持久化，支持：
//! - 运行时发现工具写回（pending → active 审批流）
//! - 跨工作流复用（工具定义与模板解耦）
//! - 使用统计与真实执行反馈（usage_count / success_rate）
//!
//! ## Schema
//!
//! - `workflow_id` + `tool_name` 唯一约束：同一工作流内工具名唯一；
//!   跨工作流可用相同 `tool_name` 各自注册（运行时按 source 区分）。
//! - `tool_type`: `rhai_script | workflow_dag | llm_function`
//! - `status`: `pending | active | disabled` —— 运行时只注册 active。
//! - `code`: Rhai 源码 / DAG JSON / LLM 函数定义体。
//! - `input_schema`: JSON Schema（TEXT，SQLite 与 PG 通用）。
//!
//! ## Strategy
//!
//! `CREATE TABLE IF NOT EXISTS` + 唯一索引 —— 幂等，可重复执行；SQLite 与
//! PostgreSQL 均支持该 ANSI DDL。

use sea_orm::ConnectionTrait;
use sea_orm::DbErr;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS workflow_tools (\
         id TEXT NOT NULL PRIMARY KEY, \
         workflow_id TEXT NOT NULL, \
         tool_name TEXT NOT NULL, \
         tool_type TEXT NOT NULL DEFAULT 'rhai_script', \
         description TEXT, \
         code TEXT, \
         input_schema TEXT, \
         source TEXT NOT NULL DEFAULT 'runtime_discovery', \
         status TEXT NOT NULL DEFAULT 'pending', \
         usage_count INTEGER NOT NULL DEFAULT 0, \
         success_rate REAL NOT NULL DEFAULT 0, \
         created_at INTEGER NOT NULL, \
         updated_at INTEGER NOT NULL, \
         UNIQUE (workflow_id, tool_name))",
    )
    .await?;

    // 查询索引：按 workflow_id 批量加载 + 按 status 过滤
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_workflow_tools_workflow \
         ON workflow_tools(workflow_id, status)",
    )
    .await?;

    tracing::info!("[v123] Created workflow_tools table");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use sea_orm::DbBackend;

    /// v123 单独幂等：重复跑不报错。
    #[tokio::test]
    async fn v123_is_self_idempotent() {
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");
        up(db).await.expect("v123 must be re-runnable in isolation");
    }

    /// 防回归：v123 之后 workflow_tools 表、唯一约束与索引必须存在。
    #[tokio::test]
    async fn v123_creates_table_with_unique_constraint() {
        use sea_orm::{ConnectionTrait, Statement};
        let db = Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        up(db.clone()).await.expect("测试：异步操作应成功");

        let row = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='workflow_tools'",
            ))
            .await
            .expect("测试应成功")
            .expect("workflow_tools 应存在");
        let ddl: String = row.try_get_by("sql").unwrap_or_default();
        assert!(
            ddl.contains("UNIQUE") && ddl.contains("workflow_id") && ddl.contains("tool_name"),
            "workflow_tools 应含 (workflow_id, tool_name) 唯一约束，实际: {}",
            ddl
        );

        let idx = db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_workflow_tools_workflow'",
            ))
            .await
            .expect("测试应成功");
        assert!(idx.is_some(), "idx_workflow_tools_workflow 索引应存在");
    }
}
