// SPDX-License-Identifier: AGPL-3.0-only

//! 提示缓存管理服务 trait
//!
//! 抽象 `PromptCache` + `CacheGuard` 的实际用法，
//! 仅暴露 agent::coordinator.rs 中实际调用的方法。

use async_trait::async_trait;

/// 缓存服务 — 管理提示缓存的生命周期，包括有效性检查、失效控制与强制刷新。
#[async_trait]
pub trait CacheService: Send + Sync {
    /// 检查当前缓存是否有效。
    async fn is_cache_valid(&self) -> bool;

    /// 检查是否有待处理的变更（变更将在下次会话时生效）。
    async fn has_pending_changes(&self) -> bool;

    /// 立即失效缓存（例如 `--now` 标志触发）。
    async fn invalidate(&self, reason: &str);

    /// 为新会话失效缓存。
    async fn invalidate_for_new_session(&self);

    /// 设置"强制即时"模式：`true` 时所有缓存敏感操作立即执行并失效缓存。
    async fn set_force_immediate(&self, force: bool);
}

/// `CacheService` 的共享引用类型。
pub type SharedCacheService = std::sync::Arc<dyn CacheService>;
