// SPDX-License-Identifier: AGPL-3.0-only

//! 动态上下文注入器 trait（参考 nomifun-tauri 的 ContextContributor 设计）。
//!
//! 允许插件化注册上下文来源（RAG、记忆、用户画像、steer 指令等），
//! 避免在 conversation.rs 或 coordinator.rs 中硬编码上下文组装逻辑。

use std::collections::HashMap;

use crate::feature_flags;
use crate::session::Session;

/// 上下文贡献者 trait。
///
/// 每个 contributor 在每次 LLM 调用前被调用，返回 `Option<String>`。
/// `None` 表示跳过（不贡献上下文），`Some(content)` 表示要注入的内容。
///
/// Contributor 在 `ConversationRuntime` 中按注册顺序执行。
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
    /// 当前完整会话（包含消息历史）
    pub session: &'a Session,
    /// 当前系统提示列表
    pub system_prompt: &'a [String],
    /// 特性标记
    pub feature_flags: &'a feature_flags::FeatureFlags,
    /// 额外参数扩展点（供特殊 contributor 传递上下文）
    pub extras: &'a HashMap<String, String>,
}
