// SPDX-License-Identifier: AGPL-3.0-only

//! v103: 工作流反思历史持久化表
//!
//! ## 背景
//!
//! 阶段 5 接入工作流反思 / 进化 / 优化能力后，`WorkflowReflectorImpl` 默认仅在内存
//! `RwLock<HashMap<workflow_id, Vec<Reflection>>>` 中保留历史，进程重启即丢失。
//! 优化 3 在 `TrajectoryStorage` 中新增 `save_workflow_reflection` /
//! `get_workflow_reflections` 方法，`WorkflowReflectorImpl` 通过 `with_storage()`
//! 注入 storage 后即可在每次反思后落库。
//!
//! ## 表结构
//!
//! `trajectory_workflow_reflections` 与 `Reflection` + `WorkflowReflectionMetadata`
//! 字段一一对应，JSON 字段使用 TEXT 存储（兼容 SQLite / PG）。
//!
//! ## 幂等性
//!
//! `CREATE TABLE IF NOT EXISTS` + `CREATE INDEX IF NOT EXISTS`，PG / SQLite 均幂等。

use sea_orm::ConnectionTrait;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    // 主表：trajectory_workflow_reflections
    //
    // 字段类型选择：
    // - quality_score: INTEGER（u8 转 i32，PG/SQLite 一致）
    // - timestamp / created_at: TEXT（RFC3339 字符串，避免 PG/SQLite 时间类型差异）
    // - error_patterns_json / reusable_patterns_json / metadata_json: TEXT（JSON 字符串）
    //
    // 注：所有字段均为 TEXT/INTEGER，PG 与 SQLite 语法一致，无需分支。
    // 不使用 PG 的 JSONB 是因为反思数据以读为主，无需 GIN 索引；保持与
    // trajectories.patterns 等已有 JSON 字段一致的 TEXT 存储方式。
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS trajectory_workflow_reflections (\
            id TEXT NOT NULL PRIMARY KEY, \
            workflow_id TEXT NOT NULL, \
            execution_id TEXT NOT NULL, \
            template_id TEXT, \
            quality_score INTEGER NOT NULL, \
            summary TEXT NOT NULL DEFAULT '', \
            error_patterns_json TEXT NOT NULL DEFAULT '[]', \
            reusable_patterns_json TEXT NOT NULL DEFAULT '[]', \
            metadata_json TEXT NOT NULL DEFAULT '{}', \
            timestamp TEXT NOT NULL, \
            created_at TEXT NOT NULL)",
    )
    .await?;

    // 索引 1：按 workflow_id 查询历史反思（聚合进化用）
    // 索引 2：按 timestamp 倒序查询（最近反思列表 / 分页用）
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_workflow_reflections_workflow \
         ON trajectory_workflow_reflections(workflow_id)",
    )
    .await?;

    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_workflow_reflections_timestamp \
         ON trajectory_workflow_reflections(timestamp)",
    )
    .await?;

    Ok(())
}
