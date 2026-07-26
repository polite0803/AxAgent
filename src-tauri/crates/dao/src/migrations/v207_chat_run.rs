// SPDX-License-Identifier: AGPL-3.0-only
//! v207_chat_run: G8 `/api/chat/runs` 后台 Run Lifecycle 持久化归档
//!
//! ## 背景
//!
//! G8 在 `gateway/handlers/runs.rs` 实现了后台异步 chat completion run 的
//! lifecycle 管理（create / list / get / events / cancel / delete）。
//! 默认采用进程内内存存储（`RunStore` with `tokio::sync::Mutex<HashMap>`），
//! 适合单实例网关。
//!
//! 本迁移创建持久化表，用于：
//! 1. **多实例网关共享 run 状态**：多个 gateway 实例可通过数据库共享 run 记录
//! 2. **历史 run 归档**：进程重启后仍可查询历史 run 记录和事件
//! 3. **审计与统计**：按时间 / 用户 / 模型维度统计 run 执行情况
//!
//! ## 本迁移创建的表
//!
//! - `chat_runs`：Run 主表（id / created_by / model / messages / status / 时间戳 / 响应 / 用量）
//! - `chat_run_events`：Run 事件流表（run_id / seq / event_type / data / ts），
//!   用于持久化 SSE 事件历史，支持回放
//!
//! ## 字段语义
//!
//! ### chat_runs
//! - `id`：Run ID（与 RunStore::generate_run_id 一致，格式 `run_{nanos:x}`）
//! - `created_by`：创建者 gateway key ID
//! - `model`：模型名称
//! - `messages`：请求消息 JSON
//! - `stream`：是否流式
//! - `status`：queued / running / completed / failed / cancelled
//! - `created_at` / `started_at` / `finished_at`：时间戳（ms）
//! - `error`：失败原因（Failed 时）
//! - `response`：最终响应 JSON
//! - `usage`：token 用量 JSON
//!
//! ### chat_run_events
//! - `id`：自增主键
//! - `run_id`：关联 chat_runs.id
//! - `seq`：事件序号（每个 run 内单调递增）
//! - `event_type`：事件类型（与 G7 dojo_event 一致：phase/delta/think_start/...）
//! - `data`：事件数据 JSON
//! - `ts`：时间戳（ms）
//!
//! ## 使用方式
//!
//! 默认 RunStore 仍使用内存存储。若需启用持久化，可在 gateway 配置中开启
//! `persist_runs: true`，RunStore 会同时写入内存和数据库（双写）。
//! 历史查询通过 `list_persisted_runs` / `get_persisted_run_events` 从数据库读取。
//!
//! ## DDL 风格
//!
//! 与 v204-v206 保持一致：直接写 PG 语法，SQLite 侧由
//! [`sqlite_ddl`](super::pg_ddl::sqlite_ddl) 自动转换。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: 创建 chat_runs 主表
    // ========================================================================

    for sql in &["CREATE TABLE IF NOT EXISTS chat_runs (\
            id TEXT NOT NULL PRIMARY KEY, \
            created_by TEXT NOT NULL, \
            model TEXT NOT NULL, \
            messages TEXT NOT NULL DEFAULT '[]', \
            stream INTEGER NOT NULL DEFAULT 0, \
            status TEXT NOT NULL DEFAULT 'queued', \
            created_at BIGINT NOT NULL, \
            started_at BIGINT, \
            finished_at BIGINT, \
            error TEXT, \
            response TEXT, \
            usage TEXT)"]
    {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // ========================================================================
    // PHASE 2: 创建 chat_run_events 事件流表
    // ========================================================================

    for sql in &["CREATE TABLE IF NOT EXISTS chat_run_events (\
            id BIGSERIAL PRIMARY KEY, \
            run_id TEXT NOT NULL, \
            seq INTEGER NOT NULL, \
            event_type TEXT NOT NULL, \
            data TEXT NOT NULL DEFAULT '{}', \
            ts BIGINT NOT NULL)"]
    {
        // DDL 写 PG 语法（BIGSERIAL），SQLite 侧由 sqlite_ddl 自动转换为
        // INTEGER PRIMARY KEY AUTOINCREMENT。与 v100_consolidated 风格一致。
        exec_ddl(&db, is_pg, sql).await?;
    }

    // ========================================================================
    // PHASE 3: 索引
    //   - chat_runs: 按 created_by 查（用户维度列表）/ 按 created_at 查（时间倒序）
    //                按 status 查（活跃 run 监控）
    //   - chat_run_events: 按 run_id + seq 查（事件回放顺序）
    // ========================================================================

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_chat_runs_created_by \
         ON chat_runs(created_by)",
        "CREATE INDEX IF NOT EXISTS idx_chat_runs_created_at \
         ON chat_runs(created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_chat_runs_status \
         ON chat_runs(status)",
        "CREATE INDEX IF NOT EXISTS idx_chat_run_events_run_id \
         ON chat_run_events(run_id, seq)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn v207_is_self_idempotent() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        up(db).await.expect("v207 必须可单独重复执行");
    }

    #[tokio::test]
    async fn v207_creates_chat_runs_tables() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();

        for table in &["chat_runs", "chat_run_events"] {
            let row = db
                .query_one_raw(sea_orm::Statement::from_string(
                    sea_orm::DbBackend::Sqlite,
                    format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"),
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "{table} 表应存在");
        }

        for idx in &[
            "idx_chat_runs_created_by",
            "idx_chat_runs_created_at",
            "idx_chat_runs_status",
            "idx_chat_run_events_run_id",
        ] {
            let row = db
                .query_one_raw(sea_orm::Statement::from_sql_and_values(
                    sea_orm::DbBackend::Sqlite,
                    "SELECT name FROM sqlite_master WHERE type='index' AND name=?",
                    [(*idx).into()],
                ))
                .await
                .unwrap();
            assert!(row.is_some(), "索引 {idx} 应存在");
        }
    }
}
