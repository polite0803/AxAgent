//! 全局状态访问（最小化）
//!
//! 仅保留数据库连接访问等跨模块必需的状态。
//! TODO: 后续通过 ToolContext.extra 传递，彻底消除全局状态。

use sea_orm::DatabaseConnection;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::RwLock;

// ── 数据库路径 ────────────────────────────────────────────────────────────

static GLOBAL_DB_PATH: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));

pub fn set_db_path(path: &str) {
    let mut db_path = GLOBAL_DB_PATH.write().expect("GLOBAL_DB_PATH poisoned");
    *db_path = Some(path.to_string());
}

pub fn get_db_path() -> Option<String> {
    GLOBAL_DB_PATH
        .read()
        .expect("GLOBAL_DB_PATH poisoned")
        .clone()
}

// ── SeaORM 数据库连接 ─────────────────────────────────────────────────────

static GLOBAL_SEA_DB: LazyLock<RwLock<Option<Arc<DatabaseConnection>>>> =
    LazyLock::new(|| RwLock::new(None));

pub fn set_sea_db(db: Arc<DatabaseConnection>) {
    let mut sea_db = GLOBAL_SEA_DB.write().expect("GLOBAL_SEA_DB poisoned");
    *sea_db = Some(db);
}

pub fn get_sea_db() -> Option<Arc<DatabaseConnection>> {
    GLOBAL_SEA_DB
        .read()
        .expect("GLOBAL_SEA_DB poisoned")
        .clone()
}
