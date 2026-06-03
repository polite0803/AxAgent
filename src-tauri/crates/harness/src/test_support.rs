//! 测试替身支持
//!
//! 提供 `axagent-harness` 自身定义的 mock / empty 实现，使测试代码
//! 无需在 dev-dependencies 里引入具体的 provider / tool 实现 crate。
//!
//! 目的：测试也走 trait 抽象，与生产代码风格一致。
//!
//! 注意：本模块始终编译，但内部仅导出"返回 None / 空实现"的轻量辅助函数，
//! 不会引入运行时依赖，也不会进入生产热路径（仅测试代码使用）。

use std::sync::Arc;

use crate::provider::ProviderAdapter;

/// 一个返回 `None` 的空 ProviderRegistry，测试 gateway / runtime 时使用。
pub struct EmptyProviderRegistry;

impl crate::registry::ProviderRegistry for EmptyProviderRegistry {
    fn get(&self, _provider_type: &str) -> Option<Arc<dyn ProviderAdapter>> {
        None
    }
}

/// 工厂：构造一个 `Arc<dyn ProviderRegistry>` 测试替身
pub fn empty_provider_registry() -> Arc<dyn crate::registry::ProviderRegistry> {
    Arc::new(EmptyProviderRegistry)
}
