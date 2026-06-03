//! 持久化层契约
//!
//! `Persistence` trait 让 agent / tools / runtime 等组件
//! 通过抽象句柄访问数据库，无需在自身 Cargo.toml 中直接依赖 sea-orm。
//!
//! 实际定义在 `axagent-core::persistence`（与 `DbHandle` 同 crate，
//! 才能满足 Rust orphan rule），本模块只做 re-export。
//!
//! 后续如需替换底层 ORM（如 sqlx、diesel），只需新写一个 Persistence 实现，
//! 所有组件无感升级。

pub use axagent_core::persistence::{DatabaseConnection, Persistence, SharedPersistence};
