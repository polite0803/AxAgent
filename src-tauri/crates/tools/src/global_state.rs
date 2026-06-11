// SPDX-License-Identifier: AGPL-3.0-only

//! 全局状态访问（最小化）
//!
//! 仅保留数据库连接访问等跨模块必需的状态。
//!
//! Future: Migrate to ToolContext.extra to eliminate global state.
//! This requires updating all tool implementations to accept db via context,
//! which is a significant refactor tracked separately.

use sea_orm::DatabaseConnection;
use std::sync::Arc;
use std::sync::LazyLock;
// SAFETY: These RwLock instances wrap global database state that is only
// accessed from synchronous set/get helpers. No lock is ever held across an
// .await boundary, and all access goes through the module-level functions
// which are themselves synchronous.
use std::sync::RwLock;

// ── 数据库路径 ────────────────────────────────────────────────────────────

static GLOBAL_DB_PATH: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));

pub fn set_db_path(path: &str) {
    let mut db_path = GLOBAL_DB_PATH.write().unwrap_or_else(|e| e.into_inner());
    *db_path = Some(path.to_string());
}

pub fn get_db_path() -> Option<String> {
    GLOBAL_DB_PATH
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

// ── SeaORM 数据库连接 ─────────────────────────────────────────────────────

static GLOBAL_SEA_DB: LazyLock<RwLock<Option<Arc<DatabaseConnection>>>> =
    LazyLock::new(|| RwLock::new(None));

pub fn set_sea_db(db: Arc<DatabaseConnection>) {
    let mut sea_db = GLOBAL_SEA_DB.write().unwrap_or_else(|e| e.into_inner());
    *sea_db = Some(db);
}

pub fn get_sea_db() -> Option<Arc<DatabaseConnection>> {
    GLOBAL_SEA_DB
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}
