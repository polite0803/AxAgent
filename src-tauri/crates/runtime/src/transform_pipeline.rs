use std::sync::Arc;

use super::hook_chain::HookChain;

pub struct TransformPipeline {
    hook_chain: Arc<HookChain>,
}

impl TransformPipeline {
    pub fn new(hook_chain: Arc<HookChain>) -> Self {
        Self { hook_chain }
    }

    pub async fn transform_tool_output(
        &self,
        tool_name: &str,
        raw_result: serde_json::Value,
    ) -> serde_json::Value {
        self.hook_chain
            .execute_transform_tool_result(tool_name, raw_result)
            .await
    }

    pub async fn transform_terminal_output(&self, raw_output: String) -> String {
        self.hook_chain
            .execute_transform_terminal_output(raw_output)
            .await
    }

    pub async fn transform_llm_response(&self, raw_content: String) -> String {
        self.hook_chain
            .execute_transform_llm_response(raw_content)
            .await
    }
}
