// SPDX-License-Identifier: AGPL-3.0-only

//! 持久化层契约
//!
//! `Persistence` trait 供 agent / tools / runtime 等组件
//! 通过抽象句柄访问数据库，无需直接依赖 sea-orm。
//!
//! 注：trait 由下层连接句柄类型实现（trait 与实现类型分属不同 crate，
//! 满足 Rust orphan rule），运行时注入。

pub use crate::persistence_mod::{DatabaseConnection, Persistence, SharedPersistence};
