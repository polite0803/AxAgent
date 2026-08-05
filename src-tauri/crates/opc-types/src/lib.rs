// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 业务领域类型、trait 和 DTO 定义
//!
//! 本 crate 是 OPC 业务系统的契约层，类似 harness 在 AxAgent 中的地位：
//! 仅包含 trait 接口定义、纯数据 DTO、常量和错误类型。零业务逻辑。
//!
//! OPC 业务 crate 依赖本 crate 定义的 trait，不依赖具体实现。

pub mod analytics;
pub mod automation;
pub mod customer;
pub mod data_service;
pub mod error;
pub mod finance;
pub mod industry_adapter;
pub mod invoice;
pub mod project;
pub mod site;

// Re-export key types at crate level
pub use analytics::*;
pub use automation::*;
pub use customer::*;
pub use data_service::*;
pub use error::*;
pub use finance::*;
pub use industry_adapter::*;
pub use invoice::*;
pub use project::*;
pub use site::*;
