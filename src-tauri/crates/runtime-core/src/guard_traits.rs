// SPDX-License-Identifier: AGPL-3.0-only

//! G9/G10/G11 接入 trait 定义
//!
//! 在 `runtime-core` 中定义可选的 trait 接口，让 `ConversationRuntime` 可以
//! 在不依赖 `agent` crate 的前提下接入 G9 ToolCallGuardrail、G10 ThinkScrubber。
//! 实际实现由 `agent` crate 提供，通过 setter 注入。
//!
//! 设计原则：
//! - trait 接口最小化，仅暴露 `ConversationRuntime` 需要的方法
//! - 默认实现为 noop（`NoopToolCallGuardrail` / `NoopThinkScrubber`）
//! - 不改变 `ConversationRuntime::new` 签名，通过 `set_*` 方法注入

use axagent_harness::conversation_model::TokenUsage;

/// G9 工具调用护栏决策
#[derive(Debug, Clone)]
pub enum GuardrailVerdict {
    /// 允许调用
    Allow,
    /// 允许但发出警告
    Warn(String),
    /// 阻止此次调用
    Block(String),
    /// 停止整个 Agent Loop
    Halt(String),
}

impl GuardrailVerdict {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow | Self::Warn(_))
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Block(_) | Self::Halt(_))
    }
}

/// G9 工具调用护栏 trait
///
/// 实现方：`agent::guardrails::ToolCallGuardrailController`（通过 adapter 适配）
pub trait ToolCallGuardrail: Send + Sync {
    /// 检查是否允许调用工具
    fn check_allowed(&self, tool_name: &str, args: &str) -> GuardrailVerdict;

    /// 记录工具调用结果
    fn record_call(&self, tool_name: &str, args: &str, success: bool);
}

/// Noop 实现（默认行为，不进行任何限制）
#[derive(Debug, Default, Clone)]
pub struct NoopToolCallGuardrail;

impl ToolCallGuardrail for NoopToolCallGuardrail {
    fn check_allowed(&self, _tool_name: &str, _args: &str) -> GuardrailVerdict {
        GuardrailVerdict::Allow
    }

    fn record_call(&self, _tool_name: &str, _args: &str, _success: bool) {}
}

/// G10 思考链清理 trait
///
/// 实现方：`agent::think_scrubber::ThinkScrubber`（通过 adapter 适配）
pub trait ThinkScrubber: Send + Sync {
    /// 清理思考链内容
    fn scrub(&self, content: &str) -> String;
}

/// Noop 实现（默认行为，不清理）
#[derive(Debug, Default, Clone)]
pub struct NoopThinkScrubber;

impl ThinkScrubber for NoopThinkScrubber {
    fn scrub(&self, content: &str) -> String {
        content.to_string()
    }
}

/// G11 token 用量回调 trait
///
/// 让 `ConversationRuntime` 在 LLM 调用完成后通知外部 ledger，
/// 而无需直接持有 `SessionTokenLedger`（避免在 consumer crate 中持有具体类型）。
/// 实际实现由 `agent` 或 `wiring` 层提供，包装 `SessionTokenLedger::record`。
pub trait TokenUsageSink: Send + Sync {
    /// 记录一次 LLM 调用的 token 用量
    fn record(&self, provider_id: &str, model_id: &str, usage: TokenUsage, cost_usd: f64);

    /// 记录上下文压缩事件
    fn record_compaction(
        &self,
        tokens_before: u64,
        tokens_after: u64,
        tokens_saved: u64,
        strategy: &str,
    );
}

/// Noop 实现
#[derive(Debug, Default, Clone)]
pub struct NoopTokenUsageSink;

impl TokenUsageSink for NoopTokenUsageSink {
    fn record(&self, _provider_id: &str, _model_id: &str, _usage: TokenUsage, _cost_usd: f64) {}

    fn record_compaction(
        &self,
        _tokens_before: u64,
        _tokens_after: u64,
        _tokens_saved: u64,
        _strategy: &str,
    ) {
    }
}
