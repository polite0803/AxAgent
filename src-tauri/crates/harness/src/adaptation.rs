// SPDX-License-Identifier: AGPL-3.0-only
//! 用户配置自适应相关的共享枚举契约
//!
//! 这里定义 `Verbosity` / `TechnicalLevel` / `ContentFormat` 三个枚举，
//! 它们同时被 `profile::UserProfile`（用户画像更新方法）和
//! trajectory 的实时学习模块（`adaptation::RealTimeLearning`）使用。
//!
//! 权威定义放在 harness，下游 crate（如 trajectory）通过 `pub use` 引用，
//! 避免重复定义（符合 AGENTS.md 铁律 4）。
use serde::{Deserialize, Serialize};

/// 详略程度调整信号
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    #[default]
    Unchanged,
    Shorter,
    Longer,
}

/// 技术深度调整信号
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TechnicalLevel {
    #[default]
    Unchanged,
    Simpler,
    MoreDetailed,
}

/// 内容呈现格式调整信号
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContentFormat {
    #[default]
    Unchanged,
    List,
    Paragraph,
    Code,
}
