// SPDX-License-Identifier: AGPL-3.0-only
//! 能力路由器实现 — 工厂函数和便捷构造器
//!
//! 本模块提供将 Retriever / Filter / Ranker 组合成
//! CapabilityRouter 的便捷方法，以及预配置的路由策略。

use std::sync::Arc;

use axagent_harness::{
    CapabilityDiscoveryRequest, CapabilityDiscoveryResult, CapabilityRouter,
    DefaultCapabilityRouter,
};

use crate::capability_filter_impl::CapabilityFilterImpl;
use crate::capability_ranker_impl::CapabilityRankerImpl;
use crate::capability_retriever_impl::CapabilityRetrieverImpl;

/// 从预构造的组件构建完整的能力路由器
///
/// # 参数
/// - `retriever` — 已构造的检索器
/// - `filter` — 已构造的过滤器
/// - `ranker` — 已构造的排序器
///
/// # 返回
/// 组合完成的 `DefaultCapabilityRouter`
pub fn build_router(
    retriever: Arc<CapabilityRetrieverImpl>,
    filter: Arc<CapabilityFilterImpl>,
    ranker: Arc<CapabilityRankerImpl>,
) -> DefaultCapabilityRouter {
    DefaultCapabilityRouter::new(retriever, filter, ranker)
}

/// 使用默认配置构建完整能力路由器的便捷方法
///
/// 从 AppState 的各个组件组装完整路由管线。
pub fn build_default_router(retriever: Arc<CapabilityRetrieverImpl>) -> DefaultCapabilityRouter {
    let filter = Arc::new(CapabilityFilterImpl::new());
    let ranker = Arc::new(CapabilityRankerImpl::default());

    DefaultCapabilityRouter::new(retriever, filter, ranker)
}

/// 执行能力发现管线的便捷方法
///
/// 封装了完整的 discover 调用，处理错误转换。
pub async fn discover(
    router: &dyn CapabilityRouter,
    request: &CapabilityDiscoveryRequest,
) -> Result<CapabilityDiscoveryResult, String> {
    router.discover(request).await
}
