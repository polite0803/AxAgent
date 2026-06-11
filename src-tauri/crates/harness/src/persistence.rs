// SPDX-License-Identifier: AGPL-3.0-only

//! 持久化层契约
//!
//! `Persistence` trait 供 agent / tools / runtime 等组件
//! 通过抽象句柄访问数据库，无需直接依赖 sea-orm。
//!
//! 注：`impl Persistence for DbHandle` 留在 `axagent-core` 中
//!（orphan rule 要求 trait 和实现类型在同一 crate）。

pub use crate::persistence_mod::{DatabaseConnection, Persistence, SharedPersistence};
