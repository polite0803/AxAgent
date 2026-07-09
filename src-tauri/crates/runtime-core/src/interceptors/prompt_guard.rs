// SPDX-License-Identifier: AGPL-3.0-only

//! PromptGuard interceptor — filters user input before LLM calls.

use axagent_harness::interceptor::{HarnessInterceptor, InterceptPoint, InterceptorContext, InterceptorResult};
use axagent_harness::prompt_guard::PromptGuard;
use std::sync::Arc;

/// PromptGuard 拦截器 — 在 LLM 调用前过滤用户输入
#[derive(Debug)]
pub struct PromptGuardInterceptor {
    guard: Arc<dyn PromptGuard>,
}

impl PromptGuardInterceptor {
    pub fn new(guard: Arc<dyn PromptGuard>) -> Self {
        Self { guard }
    }
}

#[async_trait::async_trait]
impl HarnessInterceptor for PromptGuardInterceptor {
    fn id(&self) -> &'static str {
        "prompt_guard"
    }

    fn intercept_points(&self) -> Vec<InterceptPoint> {
        vec![InterceptPoint::BeforeLlmCall]
    }

    async fn intercept(
        &self,
        _point: InterceptPoint,
        ctx: &mut InterceptorContext,
    ) -> InterceptorResult {
        let request = match ctx.request.as_ref() {
            Some(v) => v.clone(),
            None => return InterceptorResult::Continue,
        };

        let messages = match request.get("messages").and_then(|v| v.as_array()) {
            Some(msgs) if !msgs.is_empty() => msgs,
            _ => return InterceptorResult::Continue,
        };

        for msg in messages {
            let content = match msg.get("content") {
                Some(serde_json::Value::String(text)) => text.clone(),
                _ => continue,
            };

            match self.guard.process_user_input(&content) {
                Ok(safe) => {
                    if safe != content {
                        tracing::debug!("[PromptGuardInterceptor] 已过滤消息内容");
                    }
                },
                Err(blocked) => {
                    let reason = format!("PromptGuard 阻断: {blocked}");
                    tracing::warn!("[PromptGuardInterceptor] {reason}");
                    ctx.error = Some(reason.clone());
                    return InterceptorResult::Block { reason };
                },
            }
        }

        InterceptorResult::Continue
    }
}
