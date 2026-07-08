// SPDX-License-Identifier: AGPL-3.0-only

//! 响应规范化器 trait — 将 LLM 响应的原生格式映射为统一的中间表示（IR）。
//!
//! 定义在 harness 顶层，遵循「组件 → harness ← 实现」架构契约。
//! 实现方（runtime-core / agent）通过 `LlmCallConfig.response_normalizer` 注入。

use crate::types::{ChatResponse, ContentBlock};

/// 将 `ChatResponse` 的内容规范化为 `Vec<ContentBlock>` 中间表示。
///
/// ## 职责
/// - 提取 `response.content` 中的 Markdown 代码块（` ```tool_json ... ``` ` 等）
/// - 合并 `response.tool_calls`（结构化字段）转为 `ContentBlock::ToolUse`
/// - 纯文本段落转为 `ContentBlock::Text`
#[async_trait::async_trait]
pub trait ResponseNormalizer: Send + Sync {
    /// 将 ChatResponse 中的原生内容规范化为 IR 块列表
    async fn normalize(&self, response: &ChatResponse) -> Vec<ContentBlock>;
}
