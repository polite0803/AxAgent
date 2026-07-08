// SPDX-License-Identifier: AGPL-3.0-only

//! IR 渲染器 trait — 将中间表示（`Vec<ContentBlock>`）渲染为清爽纯文本。
//!
//! 定义在 harness 顶层，遵循「组件 → harness ← 实现」架构契约。
//! 实现方（agent）通过 `LlmCallConfig.ir_renderer` 注入。

use crate::types::ContentBlock;

/// 将 `Vec<ContentBlock>` 渲染为最终展示文本。
///
/// ## 渲染规则（由实现方决定）
/// - `ContentBlock::Text` → 保留段落
/// - `ContentBlock::ToolUse` → 可跳过或格式化
/// - `ContentBlock::ToolResult` → 翻译为自然语言描述
/// - 最终正则清理：去多余空行、特殊占位符、重复标点
#[async_trait::async_trait]
pub trait IrRenderer: Send + Sync {
    /// 将 IR 块列表渲染为最终展示文本
    async fn render(&self, blocks: &[ContentBlock]) -> String;
}
