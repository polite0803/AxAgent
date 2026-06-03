//! 持久化层抽象
//!
//! 定义在 `axagent-core`（与 `DbHandle` 同 crate）以满足 Rust orphan rule。
//! `axagent-harness` re-export 此 trait + `DatabaseConnection`，
//! 业务组件 `use axagent_harness::Persistence` 即可，无需直接依赖 sea-orm。

use std::sync::Arc;

// sea-orm 的连接类型，由 core 再导出，harness 再再次再导出。
pub use sea_orm::DatabaseConnection;

/// 持久化层抽象接口
///
/// 由 `axagent_core::db::DbHandle` 实现（见下方）。
/// 在 `axagent-runtime` 启动时注入。
pub trait Persistence: Send + Sync {
    /// 拿到底层连接句柄。
    /// 返回 sea-orm 的 `DatabaseConnection` 引用，消费者应当仅用它做查询，
    /// 不应自行构造或管理连接。
    fn connection(&self) -> &DatabaseConnection;

    /// 数据库文件路径（仅用于诊断 / 日志）。
    fn db_path(&self) -> &str;
}

/// 便于在异步上下文中共享的便利别名
pub type SharedPersistence = Arc<dyn Persistence>;

// ── 为 DbHandle 实现 Persistence ──────────────────────────────────

impl Persistence for super::db::DbHandle {
    fn connection(&self) -> &DatabaseConnection {
        &self.conn
    }

    fn db_path(&self) -> &str {
        &self.path
    }
}
