// SPDX-License-Identifier: AGPL-3.0-only
//! v102: Create fleets abstraction — 多办公室 AI 团队的持久化基础。
//!
//! ## Background
//!
//! AgentFleet 集成方案要求新增「舰队」一等公民抽象，用于：
//! 1. 持久化多个 AI 团队（每个团队对应一个像素办公室）
//! 2. 跟踪成员状态与 token 用量
//! 3. 支撑 Dispatcher 智能路由与生命周期管理
//!
//! ## Strategy
//!
//! - 创建 `fleets` 表：舰队基础信息 + 元数据 JSON
//! - 创建 `fleet_members` 表：成员与舰队归属 + 实时状态
//! - 创建必要索引（fleet_id / status / agent_slug）
//! - **双数据库兼容**：DDL 直接写 PostgreSQL 语法，通过 [`exec_ddl`] 在
//!   SQLite 侧自动转换。所用类型（`BIGINT`/`TEXT`）SQLite 动态类型亲和性
//!   原样接受，无需分支。
//! - 全部 `CREATE` 使用 `IF NOT EXISTS` 幂等保护，可重复执行。

use sea_orm::{ConnectionTrait, DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: 创建 fleets 表
    //
    // 注：DDL 用 PG 语法写（BIGINT/TEXT），exec_ddl 在 SQLite 下原样
    // 执行（动态类型亲和性自动兼容）。无 BIGSERIAL/SERIAL/to_char，不
    // 触发 sqlite_ddl 的任何替换。
    // ========================================================================
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS fleets (\
            id TEXT PRIMARY KEY, \
            name TEXT NOT NULL, \
            scene_template_slug TEXT, \
            status TEXT NOT NULL DEFAULT 'active', \
            created_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL, \
            metadata_json TEXT NOT NULL DEFAULT '{}')",
    )
    .await?;

    // ========================================================================
    // PHASE 2: 创建 fleet_members 表
    //
    // 外键 ON DELETE CASCADE：PG 原生支持；SQLite 需启动时开启
    // PRAGMA foreign_keys=ON（项目 init 已开启），DDL 本身 SQLite 也支持。
    // ========================================================================
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS fleet_members (\
            id TEXT PRIMARY KEY, \
            fleet_id TEXT NOT NULL REFERENCES fleets(id) ON DELETE CASCADE, \
            agent_id TEXT NOT NULL, \
            agent_slug TEXT NOT NULL, \
            display_name TEXT NOT NULL, \
            role TEXT NOT NULL DEFAULT '', \
            room_id TEXT NOT NULL DEFAULT 'workspace', \
            status TEXT NOT NULL DEFAULT 'idle', \
            joined_at BIGINT NOT NULL, \
            today_tokens BIGINT NOT NULL DEFAULT 0, \
            total_tokens BIGINT NOT NULL DEFAULT 0)",
    )
    .await?;

    // ========================================================================
    // PHASE 3: 创建索引
    //
    // CREATE INDEX IF NOT EXISTS 两端都原生支持，无需 exec_ddl 转换。
    // ========================================================================
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_fleet_members_fleet_id ON fleet_members(fleet_id)",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_fleet_members_agent_slug ON fleet_members(agent_slug)",
    )
    .await?;
    db.execute_unprepared("CREATE INDEX IF NOT EXISTS idx_fleet_members_status ON fleet_members(status)")
        .await?;
    db.execute_unprepared("CREATE INDEX IF NOT EXISTS idx_fleets_status ON fleets(status)").await?;

    Ok(())
}
