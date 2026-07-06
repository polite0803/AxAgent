// SPDX-License-Identifier: AGPL-3.0-only
//! v008_credentials_and_rl_policies: 补建 `credentials` 与 `rl_policies` 表
//!
//! 历史：两张表的 Sea-ORM entity 早已在 `crates/entities/src/credentials.rs`
//! 与 `crates/entities/src/rl_policies.rs` 定义，且 `crates/dao/src/repo/`
//! 下对应 repository 已被业务层大量调用，但 v001-v007 所有 migration 都未
//! 创建这两张表，导致首次访问时 Sea-ORM 直接抛 "no such table"。
//!
//! 修复策略：
//! - 显式 `CREATE TABLE IF NOT EXISTS`，与 v001 风格一致，幂等可重入。
//! - 字段、类型、默认值与 entity 一一对应（字符串主键 / 加密 JSON / 指标数值）。
//! - 补关键查询索引：按 `name`、`credential_type`、`model_id`。
//!
//! 风险：
//! - 若旧 DB 已存在非 Sea-ORM 风格的同名表，IF NOT EXISTS 不会触碰，仓库
//!   层读取时若 schema 不一致需要数据迁移。本 migration 不做数据迁移，仅
//!   保证空表场景下功能可用。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // 1) 凭证表：AES-256-GCM 加密凭据存储
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS credentials (\
         id TEXT NOT NULL PRIMARY KEY, \
         name TEXT NOT NULL, \
         credential_type TEXT NOT NULL, \
         data_encrypted TEXT NOT NULL, \
         created_at INTEGER NOT NULL, \
         updated_at INTEGER NOT NULL)",
    )
    .await?;

    // 凭据按名称查询、按类型过滤
    db.execute_unprepared("CREATE INDEX IF NOT EXISTS idx_credentials_name ON credentials (name)")
        .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_credentials_type ON credentials (credential_type)",
    )
    .await?;

    // 2) RL 策略表：智能体强化学习策略持久化
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS rl_policies (\
         id TEXT NOT NULL PRIMARY KEY, \
         name TEXT NOT NULL, \
         policy_type TEXT NOT NULL, \
         model_id TEXT NOT NULL, \
         reward_signals_json TEXT NOT NULL, \
         experiences_json TEXT NOT NULL, \
         total_experiences INTEGER NOT NULL, \
         episodes_completed INTEGER NOT NULL, \
         avg_reward REAL NOT NULL, \
         last_update TEXT NOT NULL, \
         created_at TEXT NOT NULL)",
    )
    .await?;

    // RL 策略按 model_id 与 policy_type 频繁过滤
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_rl_policies_model ON rl_policies (model_id)",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_rl_policies_type ON rl_policies (policy_type)",
    )
    .await?;

    Ok(())
}
