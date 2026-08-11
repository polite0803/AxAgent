// SPDX-License-Identifier: AGPL-3.0-only

//! Interceptor chain execution logic — migrated from harness to separate
//! runtime orchestration from pure contract definitions.

use axagent_harness::interceptor::{
    InterceptPoint, InterceptorChain, InterceptorContext, InterceptorResult,
};

/// 在指定拦截点执行 `InterceptorChain` 中所有匹配的拦截器。
///
/// - `Continue`：继续执行下一个拦截器
/// - `SkipRemaining`：跳过后续拦截器，但视为通过
/// - `Block`：立即阻断，返回错误
/// - `Degrade`：立即降级，返回 fallback 值
pub async fn execute_interceptor_chain(
    chain: &InterceptorChain,
    point: InterceptPoint,
    ctx: &mut InterceptorContext,
) -> InterceptorResult {
    for interceptor in chain.interceptors() {
        if interceptor.intercept_points().contains(&point) {
            let result = interceptor.intercept(point, ctx).await;
            match &result {
                InterceptorResult::Continue => continue,
                InterceptorResult::SkipRemaining => break,
                InterceptorResult::Block { .. } | InterceptorResult::Degrade { .. } => {
                    return result;
                },
            }
        }
    }
    InterceptorResult::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::interceptor::InterceptorContext;

    #[test]
    fn test_empty_chain() {
        let chain = InterceptorChain::new();
        let mut ctx = InterceptorContext::before_llm(None);
        let rt = tokio::runtime::Runtime::new().expect("测试：创建Tokio Runtime应成功");
        let result =
            rt.block_on(execute_interceptor_chain(&chain, InterceptPoint::BeforeLlmCall, &mut ctx));
        assert!(matches!(result, InterceptorResult::Continue));
    }
}
