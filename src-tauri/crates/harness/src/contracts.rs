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
/// `axagent-agent::AgentRuntime` 可接受 `Arc<dyn HarnessToolExecutor>`
/// 而不必绑定到具体实现。
pub trait HarnessToolExecutor: Send {
    /// 同步执行单个工具调用。
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;
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
