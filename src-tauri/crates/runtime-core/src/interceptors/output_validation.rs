// SPDX-License-Identifier: AGPL-3.0-only

//! Output validation interceptor — validates LLM response format.

use axagent_harness::interceptor::{
    HarnessInterceptor, InterceptPoint, InterceptorContext, InterceptorResult,
};

/// 输出校验拦截器 — 在 LLM 调用后校验响应格式
#[derive(Debug)]
pub struct OutputValidationInterceptor {
    schema: serde_json::Value,
}

impl OutputValidationInterceptor {
    pub fn new(schema: serde_json::Value) -> Self {
        Self { schema }
    }
}

#[async_trait::async_trait]
impl HarnessInterceptor for OutputValidationInterceptor {
    fn id(&self) -> &'static str {
        "output_validation"
    }

    fn intercept_points(&self) -> Vec<InterceptPoint> {
        vec![InterceptPoint::AfterLlmCall]
    }

    async fn intercept(
        &self,
        _point: InterceptPoint,
        ctx: &mut InterceptorContext,
    ) -> InterceptorResult {
        let response = match ctx.response.as_ref() {
            Some(v) => v,
            None => return InterceptorResult::Continue,
        };

        if let Some(required_fields) = self.schema.get("required").and_then(|v| v.as_array()) {
            for field in required_fields {
                let field_name = match field.as_str() {
                    Some(name) => name,
                    None => continue,
                };
                if response.get(field_name).is_none() {
                    return InterceptorResult::Block {
                        reason: format!("输出校验失败: 缺少必需字段 '{field_name}'"),
                    };
                }
            }
        }

        InterceptorResult::Continue
    }
}
