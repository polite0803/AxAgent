use std::sync::Arc;
use tokio::sync::RwLock;

use super::plugin_hooks::{
    HookDecision, LlmCallContext, LlmCallResult, SharedHook, ToolCallContext, ToolCallResult,
};

pub struct HookChain {
    hooks: Arc<RwLock<Vec<SharedHook>>>,
}

impl HookChain {
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register(&self, hook: SharedHook) {
        let mut hooks = self.hooks.write().await;
        hooks.push(hook);
    }

    pub async fn unregister(&self, name: &str) {
        let mut hooks = self.hooks.write().await;
        hooks.retain(|h| h.name() != name);
    }

    pub async fn list(&self) -> Vec<String> {
        let hooks = self.hooks.read().await;
        hooks.iter().map(|h| h.name().to_string()).collect()
    }

    pub async fn execute_pre_tool_call(&self, ctx: &ToolCallContext) -> Option<HookDecision> {
        let hooks = self.hooks.read().await;
        let mut sorted: Vec<_> = hooks.iter().collect();
        sorted.sort_by_key(|h| h.priority());

        for hook in sorted {
            if let Some(decision) = hook.pre_tool_call(ctx).await {
                match decision {
                    HookDecision::Veto { ref reason } => {
                        tracing::warn!(
                            "Hook '{}' vetoed tool {}: {}",
                            hook.name(),
                            ctx.tool_name,
                            reason
                        );
                        return Some(decision);
                    }
                    HookDecision::Modify { .. } => {
                        tracing::debug!(
                            "Hook '{}' modified tool {} context",
                            hook.name(),
                            ctx.tool_name
                        );
                    }
                    HookDecision::Allow => {}
                }
            }
        }
        None
    }

    pub async fn execute_post_tool_call(&self, ctx: &ToolCallContext, result: &ToolCallResult) {
        let hooks = self.hooks.read().await;
        let mut sorted: Vec<_> = hooks.iter().collect();
        sorted.sort_by_key(|h| h.priority());

        for hook in sorted {
            hook.post_tool_call(ctx, result).await;
        }
    }

    pub async fn execute_transform_tool_result(
        &self,
        tool_name: &str,
        mut result: serde_json::Value,
    ) -> serde_json::Value {
        let hooks = self.hooks.read().await;
        let mut sorted: Vec<_> = hooks.iter().collect();
        sorted.sort_by_key(|h| h.priority());

        for hook in sorted {
            let current = std::mem::replace(&mut result, serde_json::Value::Null);
            if let Some(transformed) = hook.transform_tool_result(tool_name, current).await {
                result = transformed;
            }
        }
        result
    }

    pub async fn execute_pre_llm_call(&self, ctx: &LlmCallContext) -> Option<HookDecision> {
        let hooks = self.hooks.read().await;
        let mut sorted: Vec<_> = hooks.iter().collect();
        sorted.sort_by_key(|h| h.priority());

        for hook in sorted {
            if let Some(HookDecision::Veto { ref reason }) = hook.pre_llm_call(ctx).await {
                tracing::warn!("Hook '{}' vetoed LLM call: {}", hook.name(), reason);
                return Some(HookDecision::Veto {
                    reason: reason.clone(),
                });
            }
        }
        None
    }

    pub async fn execute_post_llm_call(&self, ctx: &LlmCallContext, result: &LlmCallResult) {
        let hooks = self.hooks.read().await;
        let mut sorted: Vec<_> = hooks.iter().collect();
        sorted.sort_by_key(|h| h.priority());

        for hook in sorted {
            hook.post_llm_call(ctx, result).await;
        }
    }

    pub async fn execute_transform_llm_response(&self, mut content: String) -> String {
        let hooks = self.hooks.read().await;
        let mut sorted: Vec<_> = hooks.iter().collect();
        sorted.sort_by_key(|h| h.priority());

        for hook in sorted {
            content = hook.transform_llm_response(content).await;
        }
        content
    }

    pub async fn execute_transform_terminal_output(&self, mut output: String) -> String {
        let hooks = self.hooks.read().await;
        let mut sorted: Vec<_> = hooks.iter().collect();
        sorted.sort_by_key(|h| h.priority());

        for hook in sorted {
            output = hook.transform_terminal_output(output).await;
        }
        output
    }

    pub async fn notify_session_start(&self, session_id: &str) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            hook.on_session_start(session_id).await;
        }
    }

    pub async fn notify_session_end(&self, session_id: &str) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            hook.on_session_end(session_id).await;
        }
    }

    pub async fn notify_error(&self, error: &str, context: Option<serde_json::Value>) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            hook.on_error(error, context.clone()).await;
        }
    }
}

impl Default for HookChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_hooks::PluginHook;
    use async_trait::async_trait;

    struct TestHook {
        name: String,
        should_veto: bool,
    }

    #[async_trait]
    impl PluginHook for TestHook {
        fn name(&self) -> &str {
            &self.name
        }

        async fn pre_tool_call(&self, _ctx: &ToolCallContext) -> Option<HookDecision> {
            if self.should_veto {
                Some(HookDecision::Veto {
                    reason: "test veto".to_string(),
                })
            } else {
                None
            }
        }
    }

    #[tokio::test]
    async fn test_hook_chain_veto() {
        let chain = HookChain::new();
        chain
            .register(Arc::new(TestHook {
                name: "test_hook".to_string(),
                should_veto: true,
            }))
            .await;

        let ctx = ToolCallContext {
            tool_name: "test_tool".to_string(),
            tool_namespace: None,
            arguments: serde_json::json!({}),
            session_id: None,
        };

        let decision = chain.execute_pre_tool_call(&ctx).await;
        assert!(decision.is_some());
        match decision.unwrap() {
            HookDecision::Veto { reason } => assert_eq!(reason, "test veto"),
            _ => panic!("Expected veto"),
        }
    }

    #[tokio::test]
    async fn test_hook_chain_allows() {
        let chain = HookChain::new();
        chain
            .register(Arc::new(TestHook {
                name: "passive_hook".to_string(),
                should_veto: false,
            }))
            .await;

        let ctx = ToolCallContext {
            tool_name: "harmless_tool".to_string(),
            tool_namespace: None,
            arguments: serde_json::json!({}),
            session_id: None,
        };

        let decision = chain.execute_pre_tool_call(&ctx).await;
        assert!(decision.is_none());
    }
}
