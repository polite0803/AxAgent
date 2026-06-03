//! ToolRegistry 抽象接口
//!
//! 在 Harness 架构中，`agent` crate 只通过此 trait 查询工具注册表，
//! 不依赖 `axagent-tools` 的具体 `ToolRegistry` 实现。
//! 由 `axagent-runtime` 在启动时注入具体实现。

use crate::tool::{Tool, ToolCategory, ToolInfo};
use std::sync::Arc;

/// Provider 注册表接口 — 抽象查找 LLM Provider 适配器
///
/// 实现方 (`axagent-providers::registry::ProviderRegistry`) 在运行时注入。
/// `rt-messaging` 和 `gateway` 仅依赖此 trait，不依赖具体实现。
pub trait ProviderRegistry: Send + Sync {
    /// 按 provider 类型名查找适配器
    fn get(&self, provider_type: &str) -> Option<Arc<dyn super::provider::ProviderAdapter>>;
}

/// 工具注册表抽象接口
///
/// 提供工具查找、列举、禁用状态查询能力。
/// 实现方 (`axagent-tools::registry::ToolRegistry`) 在运行时注入。
pub trait ToolRegistry: Send + Sync {
    /// 按名称精确查找工具
    fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;

    /// 按名称查找工具（支持别名解析）
    fn find(&self, name: &str) -> Option<Arc<dyn Tool>>;

    /// 列举全部已注册工具
    fn list(&self) -> Vec<ToolInfo>;

    /// 按类别列举工具
    fn list_by_category(&self, category: ToolCategory) -> Vec<ToolInfo>;

    /// 检查工具是否被禁用
    fn is_disabled(&self, name: &str) -> bool;
}
