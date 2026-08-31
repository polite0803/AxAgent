// SPDX-License-Identifier: AGPL-3.0-only

//! `SessionStateStore` 的 SQLite 实现 —— 会话状态持久化的 wiring 载体。
//!
//! 全部语义（key 构造、TTL 单位、过期判定）委托给 `axagent_harness::session_state`
//! 与 `crate::repo::session_state`，本文件只做 trait 适配与错误归一。

use axagent_harness::session_state::{SessionStateEntry, SessionStateStore};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 基于 SeaORM 的会话状态存储。
///
/// 持有 `DatabaseConnection` 的 Arc —— `SessionStateStore` 需 `Send + Sync`，
/// 而 `DatabaseConnection` 本身是 `Clone + Send + Sync`（内部连接池），
/// 直接存值即可，无需再包一层锁。
#[derive(Clone)]
pub struct DaoSessionStateStore {
    db: Arc<DatabaseConnection>,
}

impl DaoSessionStateStore {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl SessionStateStore for DaoSessionStateStore {
    async fn set(&self, key: &str, value: &str, ttl_ms: Option<i64>) -> Result<(), String> {
        crate::repo::session_state::set(&self.db, key, value, ttl_ms)
            .await
            .map_err(|e| format!("会话状态写入失败: {e}"))
    }

    async fn get(&self, key: &str) -> Result<Option<String>, String> {
        crate::repo::session_state::get(&self.db, key)
            .await
            .map_err(|e| format!("会话状态读取失败: {e}"))
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        crate::repo::session_state::delete(&self.db, key)
            .await
            .map_err(|e| format!("会话状态删除失败: {e}"))
    }

    async fn list_by_prefix(&self, prefix: &str) -> Result<Vec<SessionStateEntry>, String> {
        crate::repo::session_state::list_by_prefix(&self.db, prefix)
            .await
            .map_err(|e| format!("会话状态列举失败: {e}"))
    }

    async fn purge_expired(&self) -> Result<usize, String> {
        crate::repo::session_state::purge_expired(&self.db)
            .await
            .map_err(|e| format!("会话状态清理失败: {e}"))
    }
}
