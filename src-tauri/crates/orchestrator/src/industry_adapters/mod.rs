// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业适配器模块 — 从 harness 重导出核心 trait
//!
//! `IndustryAdapter` trait 和 `IndustryAdapterRegistry` 的权威定义
//! 已迁移至 `axagent-harness::industry_orchestration`。
//! 本模块仅保留重导出和 `BaseIndustryAdapter`。

pub mod base_adapter;
pub mod types;

pub use axagent_harness::industry_orchestration::{
    IndustryAdapter, IndustryAdapterRegistry,
};

// 导出基础适配器
pub use base_adapter::BaseIndustryAdapter;
