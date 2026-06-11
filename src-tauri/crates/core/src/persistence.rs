// SPDX-License-Identifier: AGPL-3.0-only

//! 持久化层抽象
//!
//! `Persistence` trait 定义在 `axagent-harness` 中。
//! `DbHandle` 的 `impl Persistence` 已迁移至 `axagent-dao::db`。

pub use axagent_harness::{DatabaseConnection, Persistence, SharedPersistence};
