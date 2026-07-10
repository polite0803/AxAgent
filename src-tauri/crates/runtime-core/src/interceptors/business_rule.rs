// SPDX-License-Identifier: AGPL-3.0-only

//! Business rule interceptor — checks business rules before node execution.

use axagent_harness::business_rules::{BusinessRuleEvaluator, RuleEvaluationOutcome};
use axagent_harness::interceptor::{
    HarnessInterceptor, InterceptPoint, InterceptorContext, InterceptorResult,
};
use std::sync::Arc;

/// 业务规则拦截器 — 在工作流节点执行前检查业务规则
pub struct BusinessRuleInterceptor {
    engine: Arc<dyn BusinessRuleEvaluator>,
}

impl BusinessRuleInterceptor {
    pub fn new(engine: Arc<dyn BusinessRuleEvaluator>) -> Self {
        Self { engine }
    }
}

impl std::fmt::Debug for BusinessRuleInterceptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BusinessRuleInterceptor").finish()
    }
}

#[async_trait::async_trait]
impl HarnessInterceptor for BusinessRuleInterceptor {
    fn id(&self) -> &'static str {
        "business_rule"
    }

    fn intercept_points(&self) -> Vec<InterceptPoint> {
        vec![InterceptPoint::BeforeNodeExecute]
    }

    async fn intercept(
        &self,
        _point: InterceptPoint,
        ctx: &mut InterceptorContext,
    ) -> InterceptorResult {
        let node_type = ctx.node_id.as_deref().unwrap_or("unknown");
        let input = ctx.request.clone().unwrap_or(serde_json::Value::Null);

        match self.engine.evaluate(node_type, &input) {
            RuleEvaluationOutcome::Pass => InterceptorResult::Continue,
            RuleEvaluationOutcome::Violation { reason, .. } => {
                InterceptorResult::Block { reason: format!("[业务规则] {reason}") }
            },
            RuleEvaluationOutcome::RequiresApproval { reason, .. } => {
                InterceptorResult::Block { reason: format!("[需审批] {reason}") }
            },
        }
    }
}
