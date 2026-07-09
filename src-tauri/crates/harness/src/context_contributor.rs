// SPDX-License-Identifier: AGPL-3.0-only

//! 动态上下文注入器 trait。
//!
//! 允许插件化注册上下文来源（RAG、记忆、用户画像、steer 指令等），
//! 定义在 harness 层，业务层和实现层均可引用。

use std::collections::HashMap;

/// 上下文贡献者 trait。
///
/// 每个 contributor 在每次 LLM 调用前被调用，返回 `Option<String>`。
/// `None` 表示跳过，`Some(content)` 表示要注入的内容。
pub trait ContextContributor: Send + Sync {
    /// 贡献一段上下文文本，返回 `None` 表示跳过。
    fn contribute(&self, ctx: &ContextRequest) -> Option<String>;

    /// Contributor 的名称（用于日志和调试）。
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// 传递给 `ContextContributor::contribute` 的上下文元数据。
#[derive(Debug, Clone)]
pub struct ContextRequest<'a> {
    /// 当前会话 ID
    pub session_id: &'a str,
    /// 前端对话 ID
    pub conversation_id: Option<&'a str>,
    /// 当前系统提示列表
    pub system_prompt: &'a [String],
    /// 额外扩展参数（实现层可按需注入 session / feature_flags 等数据）
    pub extras: &'a HashMap<String, String>,
}
