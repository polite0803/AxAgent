// SPDX-License-Identifier: AGPL-3.0-only

//! v202: workflow_templates.mission_hash 列
//!
//! ## 版本号策略
//!
//! AxInvest 本地迁移从 **v200** 起单调递增，预留 v101–v199 给上游 AxAgent 未来
//! 新增迁移使用。本文件原名 `v102_mission_hash.rs`，现重命名为 v202 以落实该策略。
//!
//! ## 背景
//!
//! 目标架构要求「运行时一定基于已经生成的工作流（利于运行时稳定）」。
//! 设计阶段通过 `compile_mission_to_template` 命令把 mission（用户高层意图）
//! 编译成 workflow_template，运行时直接使用已编译的 template，不再调用 LLM。
//!
//! 为了支持「相同 mission 命中已有 template」的去重缓存，workflow_templates
//! 表新增 `mission_hash` 列，存储 mission 文本的 SHA-256 哈希。
//!
//! ## 幂等性
//!
//! - PostgreSQL：使用 `ADD COLUMN IF NOT EXISTS`，幂等。
//! - SQLite：`ADD COLUMN` 重复执行会报错，用 `let _ = ...` 吞掉「重复列」错误实现幂等。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    // 添加 mission_hash 列（用于 compile_mission_to_template 去重缓存）
    if backend == DbBackend::Postgres {
        db.execute_unprepared(
            "ALTER TABLE workflow_templates ADD COLUMN IF NOT EXISTS mission_hash TEXT",
        )
        .await?;
    } else {
        // SQLite 不支持 IF NOT EXISTS，吞掉重复列错误实现幂等
        let _ = db
            .execute_unprepared("ALTER TABLE workflow_templates ADD COLUMN mission_hash TEXT")
            .await;
    }

    // 为 mission_hash 创建索引（仅在非 NULL 时索引，加速查重）
    // 注意：SQLite 不支持部分索引的 IF NOT EXISTS，吞错误实现幂等
    let _ = db
        .execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_workflow_templates_mission_hash \
             ON workflow_templates(mission_hash) WHERE mission_hash IS NOT NULL",
        )
        .await;

    Ok(())
}
