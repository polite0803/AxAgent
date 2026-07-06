// SPDX-License-Identifier: AGPL-3.0-only

//! Feature Flag 提供者 trait
//!
//! 抽象 `runtime_core::feature_flags` 的全局开关查询和（测试用）激活方法。

/// Feature Flag 提供者 — 查询和设置功能开关状态。
pub trait FeatureFlagProvider: Send + Sync {
    /// 检查指定名称的 feature flag 是否已启用。
    fn is_enabled(&self, name: &str) -> bool;

    /// 启用指定名称的 feature flag（测试/开发环境使用）。
    fn enable(&self, name: &str);
}

/// `FeatureFlagProvider` 的共享引用类型。
pub type SharedFeatureFlagProvider = std::sync::Arc<dyn FeatureFlagProvider>;
