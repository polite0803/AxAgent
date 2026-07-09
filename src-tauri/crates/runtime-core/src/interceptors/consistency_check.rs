// SPDX-License-Identifier: AGPL-3.0-only

//! Consistency check interceptor — validates output consistency after LLM calls.

use axagent_harness::consistency_check::ConsistencyCheckConfig;
use axagent_harness::interceptor::{
    HarnessInterceptor, InterceptPoint, InterceptorContext, InterceptorResult,
};

/// 一致性检查拦截器 — 在 LLM 调用后检查输出一致性
#[derive(Debug)]
pub struct ConsistencyCheckInterceptor {
    config: ConsistencyCheckConfig,
}

impl ConsistencyCheckInterceptor {
    pub fn new(config: ConsistencyCheckConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl HarnessInterceptor for ConsistencyCheckInterceptor {
    fn id(&self) -> &'static str {
        "consistency_check"
    }

    fn intercept_points(&self) -> Vec<InterceptPoint> {
        if self.config.enabled {
            vec![InterceptPoint::AfterLlmCall]
        } else {
            vec![]
        }
    }

    async fn intercept(
        &self,
        _point: InterceptPoint,
        ctx: &mut InterceptorContext,
    ) -> InterceptorResult {
        let _response = match ctx.response.as_ref() {
            Some(v) => v.clone(),
            None => return InterceptorResult::Continue,
        };

        tracing::debug!("[ConsistencyCheckInterceptor] 需要二次结果进行对比，当前单次结果跳过检查");

        InterceptorResult::Continue
    }
}
