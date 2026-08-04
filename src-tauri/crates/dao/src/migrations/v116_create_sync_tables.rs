// SPDX-License-Identifier: AGPL-3.0-only
//! v116: Create sync tables — 多设备同步管理的持久化基础。
//!
//! ## Background
//!
//! 多设备同步功能要求新增一系列同步相关表，用于：
//! 1. 持久化设备信息（sync_devices）
//! 2. 存储变更日志（sync_change_logs）
//! 3. 管理同步策略（sync_policies）
//! 4. 记录同步历史（sync_histories）
//! 5. 管理设备权限（sync_permissions）
//! 6. 记录审计日志（sync_audit_logs）
//!
//! ## Strategy
//!
//! - 创建 6 张同步相关表
//! - 创建必要索引以优化查询性能
//! - **双数据库兼容**：DDL 直接写 PostgreSQL 语法，通过 exec_ddl 在
//!   SQLite 侧自动转换。
//! - 全部 CREATE 使用 IF NOT EXISTS 幂等保护，可重复执行。

use sea_orm::{DbBackend, DbErr};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;

    // ========================================================================
    // PHASE 1: 创建 sync_devices 表
    // ========================================================================
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS sync_devices (\
            id TEXT PRIMARY KEY, \
            name TEXT NOT NULL, \
            device_type TEXT NOT NULL, \
            os TEXT NOT NULL, \
            app_version TEXT NOT NULL, \
            unique_id TEXT NOT NULL UNIQUE, \
            public_key TEXT NOT NULL, \
            ip_address TEXT, \
            is_paired BOOLEAN NOT NULL DEFAULT false, \
            trust_level TEXT NOT NULL DEFAULT 'standard', \
            last_synced_at BIGINT, \
            last_heartbeat_at BIGINT, \
            is_enabled BOOLEAN NOT NULL DEFAULT true, \
            created_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL)",
    )
    .await?;

    // 索引
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_devices_unique_id ON sync_devices(unique_id)",
    )
    .await?;
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_devices_is_paired ON sync_devices(is_paired)",
    )
    .await?;

    // ========================================================================
    // PHASE 2: 创建 sync_change_logs 表
    // ========================================================================
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS sync_change_logs (\
            id TEXT PRIMARY KEY, \
            device_id TEXT NOT NULL REFERENCES sync_devices(id) ON DELETE CASCADE, \
            entity_type TEXT NOT NULL, \
            entity_id TEXT NOT NULL, \
            operation TEXT NOT NULL, \
            data TEXT NOT NULL, \
            version BIGINT NOT NULL, \
            parent_version_id TEXT, \
            created_at BIGINT NOT NULL, \
            is_synced BOOLEAN NOT NULL DEFAULT false, \
            synced_to TEXT NOT NULL DEFAULT '[]')",
    )
    .await?;

    // 索引
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_change_logs_device ON sync_change_logs(device_id)",
    )
    .await?;
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_change_logs_synced ON sync_change_logs(is_synced)",
    )
    .await?;
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_change_logs_version ON sync_change_logs(version)",
    )
    .await?;

    // ========================================================================
    // PHASE 3: 创建 sync_policies 表
    // ========================================================================
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS sync_policies (\
            id TEXT PRIMARY KEY, \
            name TEXT NOT NULL, \
            description TEXT, \
            sync_mode TEXT NOT NULL DEFAULT 'manual', \
            conflict_strategy TEXT NOT NULL DEFAULT 'last_write_wins', \
            sync_interval_ms BIGINT NOT NULL DEFAULT 3600000, \
            allowed_entity_types TEXT NOT NULL DEFAULT '[]', \
            excluded_entity_types TEXT NOT NULL DEFAULT '[]', \
            compression_algorithm TEXT NOT NULL DEFAULT 'none', \
            max_transfer_size BIGINT NOT NULL DEFAULT 104857600, \
            encryption_enabled BOOLEAN NOT NULL DEFAULT false, \
            is_enabled BOOLEAN NOT NULL DEFAULT true, \
            created_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL)",
    )
    .await?;

    // 索引
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_policies_enabled ON sync_policies(is_enabled)",
    )
    .await?;

    // ========================================================================
    // PHASE 4: 创建 sync_histories 表
    // ========================================================================
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS sync_histories (\
            id TEXT PRIMARY KEY, \
            device_id TEXT NOT NULL REFERENCES sync_devices(id) ON DELETE CASCADE, \
            direction TEXT NOT NULL, \
            sync_type TEXT NOT NULL, \
            result TEXT NOT NULL, \
            conflicts TEXT NOT NULL DEFAULT '[]', \
            started_at BIGINT NOT NULL, \
            completed_at BIGINT NOT NULL, \
            initiated_by TEXT NOT NULL)",
    )
    .await?;

    // 索引
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_histories_device ON sync_histories(device_id)",
    )
    .await?;
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_histories_started_at ON sync_histories(started_at)",
    )
    .await?;

    // ========================================================================
    // PHASE 5: 创建 sync_permissions 表
    // ========================================================================
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS sync_permissions (\
            id TEXT PRIMARY KEY, \
            device_id TEXT NOT NULL REFERENCES sync_devices(id) ON DELETE CASCADE UNIQUE, \
            trust_level TEXT NOT NULL DEFAULT 'standard', \
            can_push BOOLEAN NOT NULL DEFAULT true, \
            can_pull BOOLEAN NOT NULL DEFAULT true, \
            can_full_sync BOOLEAN NOT NULL DEFAULT false, \
            can_resolve_conflicts BOOLEAN NOT NULL DEFAULT false, \
            can_manage_devices BOOLEAN NOT NULL DEFAULT false, \
            can_modify_policy BOOLEAN NOT NULL DEFAULT false, \
            expires_at BIGINT, \
            created_at BIGINT NOT NULL, \
            updated_at BIGINT NOT NULL)",
    )
    .await?;

    // 索引
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_permissions_device ON sync_permissions(device_id)",
    )
    .await?;
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_permissions_expires ON sync_permissions(expires_at)",
    )
    .await?;

    // ========================================================================
    // PHASE 6: 创建 sync_audit_logs 表
    // ========================================================================
    exec_ddl(
        &db,
        is_pg,
        "CREATE TABLE IF NOT EXISTS sync_audit_logs (\
            id TEXT PRIMARY KEY, \
            action TEXT NOT NULL, \
            target_type TEXT NOT NULL, \
            target_id TEXT NOT NULL, \
            actor_device_id TEXT NOT NULL, \
            is_successful BOOLEAN NOT NULL DEFAULT true, \
            details TEXT, \
            error_message TEXT, \
            created_at BIGINT NOT NULL)",
    )
    .await?;

    // 索引
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_audit_logs_action ON sync_audit_logs(action)",
    )
    .await?;
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_audit_logs_actor ON sync_audit_logs(actor_device_id)",
    )
    .await?;
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_audit_logs_created ON sync_audit_logs(created_at)",
    )
    .await?;
    exec_ddl(
        &db,
        is_pg,
        "CREATE INDEX IF NOT EXISTS idx_sync_audit_logs_successful ON sync_audit_logs(is_successful)",
    )
    .await?;

    Ok(())
}
