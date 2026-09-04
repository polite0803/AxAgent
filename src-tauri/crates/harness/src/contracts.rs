// SPDX-License-Identifier: AGPL-3.0-only

//! 公共契约 trait — 依赖反转边界。
//!
//! 该模块定义 `axagent-harness` 级别的 trait 接口，使其可供上层 crate
//!（如 `axagent-agent`、`axagent-runtime`）自由实现和消费，而无需
//! 在每一层都直接依赖 `axagent-runtime-core`。

use crate::ToolError;

/// 工具执行器 — 依赖反转后的 harness 级别契约。
///
/// `axagent-runtime-core` 的 `ToolExecutor` trait 实现此 trait，
/// 对话运行时（`create_conversation_runtime` → `run_turn_with_tools`）
/// 通过该契约接受工具实现，而不必绑定到具体类型。
pub trait HarnessToolExecutor: Send {
    /// 同步执行单个工具调用。
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;

    /// 批量执行工具调用。默认实现串行逐个执行，子类型可覆盖为并发编排。
    fn execute_batch(
        &mut self,
        requests: &[(String, String, String)], // (tool_use_id, tool_name, input)
    ) -> Vec<(String, String, Result<String, ToolError>)> {
        requests
            .iter()
            .map(|(id, name, input)| {
                let result = self.execute(name, input);
                (id.clone(), name.clone(), result)
            })
            .collect()
    }
}

/// API 客户端 — harness 级别最小契约。
///
/// 使用泛型事件类型避免直接依赖 `axagent-runtime-core`。
/// 具体实现（如 `axagent-runtime-core::conversation::ApiClient`）可桥接
/// 到自身的事件枚举。
pub trait HarnessApiClient<E>: Send {
    /// 发起流式请求，返回模型事件序列。
    fn stream(
        &mut self,
        system_prompt: &[String],
        messages: &[serde_json::Value],
    ) -> Result<Vec<E>, String>;
}
