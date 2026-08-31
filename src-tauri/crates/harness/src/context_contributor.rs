// SPDX-License-Identifier: AGPL-3.0-only

//! 动态上下文注入器 trait。
//!
//! 允许插件化注册上下文来源（已加载能力、RAG、记忆、用户画像、steer 指令等），
//! 定义在 harness 层，业务层和实现层均可引用。
//!
//! # 为什么 `contribute` 是异步的
//!
//! 注入器的典型实现要读会话状态（`CapabilityLoad` 写入的加载记录）—— 那是一次
//! 数据库读。同步签名会把调用方逼进 `block_in_place` 或缓存预热这类绕路方案，
//! 前者在 tokio 多线程运行时下会拖慢整个 worker，后者让「加载后立刻生效」退化成
//! 「下一轮才生效」。异步是这条链路的正确形状。

use std::collections::HashMap;

/// 上下文贡献者 trait。
///
/// 每个 contributor 在每次 LLM 调用前被调用，返回 `Option<String>`。
/// `None` 表示跳过，`Some(content)` 表示要注入的内容。
#[async_trait::async_trait]
pub trait ContextContributor: Send + Sync {
    /// 贡献一段上下文文本，返回 `None` 表示跳过。
    async fn contribute(&self, ctx: &ContextRequest<'_>) -> Option<String>;

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
    /// Agent 作用域（多 Agent 隔离的载体；`None` = 单 Agent 场景）
    pub agent_id: Option<&'a str>,
    /// 当前系统提示列表
    pub system_prompt: &'a [String],
    /// 额外扩展参数（实现层可按需注入 session / feature_flags 等数据）
    pub extras: &'a HashMap<String, String>,
}
