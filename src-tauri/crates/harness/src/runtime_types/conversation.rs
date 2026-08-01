// SPDX-License-Identifier: AGPL-3.0-only

//! 会话相关类型 — 原 `axagent-runtime-core::conversation` 的数据类型。

use crate::conversation_model::{ConversationMessage, TokenUsage};
use crate::runtime_types::execution_progress::AgentExecutionProgress;
use crate::runtime_types::hooks::HookProgressReporter;
use crate::runtime_types::permissions::PermissionPrompter;
use crate::runtime_types::session::Session;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// 完整请求负载。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiRequest {
    pub system_prompt: Vec<String>,
    pub messages: Vec<ConversationMessage>,
}

/// 流式助手事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssistantEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolUse { id: String, name: String, input: String },
    Usage(TokenUsage),
    PromptCache(PromptCacheEvent),
    MessageStop,
}

/// Prompt 缓存遥测。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PromptCacheEvent {
    pub unexpected: bool,
    pub reason: String,
    pub previous_cache_read_input_tokens: u32,
    pub current_cache_read_input_tokens: u32,
    pub token_drop: u32,
}

/// 运行时错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub(crate) message: String,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

/// 最小化流式 API 契约。
pub trait ApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError>;
}

impl<T: ApiClient + Send + ?Sized> ApiClient for Box<T> {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        (**self).stream(request)
    }
}

/// 工具执行器 trait — 执行模型请求的工具调用。
pub trait ToolExecutor: Send {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, crate::ToolError>;

    /// 批量执行工具调用。默认实现串行逐个执行。
    fn execute_batch(
        &mut self,
        requests: &[(String, String, String)], // (tool_use_id, tool_name, input)
    ) -> Vec<(String, String, Result<String, crate::ToolError>)> {
        requests
            .iter()
            .map(|(id, name, input)| {
                let result = self.execute(name, input);
                (id.clone(), name.clone(), result)
            })
            .collect()
    }
}

impl<T: ToolExecutor + ?Sized> ToolExecutor for Box<T> {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, crate::ToolError> {
        (**self).execute(tool_name, input)
    }
}

/// 自动压缩事件（回合结束后的上下文压缩结果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct AutoCompactionEvent {
    pub removed_message_count: usize,
}

/// 回合摘要（运行时内部+外部调用者均使用）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TurnSummary {
    pub assistant_messages: Vec<ConversationMessage>,
    pub tool_results: Vec<ConversationMessage>,
    pub prompt_cache_events: Vec<PromptCacheEvent>,
    pub iterations: usize,
    pub usage: TokenUsage,
    pub auto_compaction: Option<AutoCompactionEvent>,
    pub thinking: String,
}

/// ConversationRuntime 的 trait 接口。
/// 让 consumer crate 无需直接依赖 axagent-runtime-core 的具体类型。
pub trait ConversationRuntimeHost: Send {
    /// 执行一轮对话（模型推理 + 工具执行），返回完整的回合摘要。
    fn run_turn(
        &mut self,
        user_input: &str,
        prompter: Option<&mut dyn PermissionPrompter>,
    ) -> Result<TurnSummary, RuntimeError>;

    /// 设置最大迭代次数。
    fn set_max_iterations(&mut self, max: usize);

    /// 设置自动压缩阈值（输入 token 数）。
    fn set_auto_compaction_threshold(&mut self, _threshold: u32) {}

    /// 设置取消 token。
    fn set_cancel_token(&mut self, _token: Option<Arc<AtomicBool>>) {}

    /// 设置进度追踪器。
    fn set_progress(&mut self, _progress: Arc<AgentExecutionProgress>) {}

    /// 设置 hook 进度报告器。
    fn set_hook_progress_reporter(&mut self, _reporter: Box<dyn HookProgressReporter>) {}

    /// 注入 nudge 上下文行（每次 run_turn 前调用）。
    /// nudge 文本会被注入到 system_prompt 中，使 LLM 感知记忆提醒。
    /// 默认实现为空操作（保持向后兼容）。
    fn set_nudge_lines(&mut self, _lines: Vec<String>) {}

    /// 设置系统级指令（persona 等），注入到每次 LLM 调用的 system_prompt。
    /// 与拼接进 user message 相比，语义更准确：LLM 将其视为系统指令而非用户内容。
    /// 默认实现为空操作（保持向后兼容）。
    fn set_system_directive(&mut self, _directive: String) {}

    /// 消费 runtime，提取 Session。
    fn into_session(self: Box<Self>) -> Session;
}

impl<T: ?Sized + ConversationRuntimeHost> ConversationRuntimeHost for Box<T> {
    fn run_turn(
        &mut self,
        user_input: &str,
        prompter: Option<&mut dyn PermissionPrompter>,
    ) -> Result<TurnSummary, RuntimeError> {
        (**self).run_turn(user_input, prompter)
    }

    fn set_max_iterations(&mut self, max: usize) {
        (**self).set_max_iterations(max)
    }

    fn set_auto_compaction_threshold(&mut self, threshold: u32) {
        (**self).set_auto_compaction_threshold(threshold)
    }

    fn set_cancel_token(&mut self, token: Option<Arc<AtomicBool>>) {
        (**self).set_cancel_token(token)
    }

    fn set_progress(&mut self, progress: Arc<AgentExecutionProgress>) {
        (**self).set_progress(progress)
    }

    fn set_hook_progress_reporter(&mut self, reporter: Box<dyn HookProgressReporter>) {
        (**self).set_hook_progress_reporter(reporter)
    }

    fn set_nudge_lines(&mut self, lines: Vec<String>) {
        (**self).set_nudge_lines(lines)
    }

    fn set_system_directive(&mut self, directive: String) {
        (**self).set_system_directive(directive)
    }

    fn into_session(self: Box<Self>) -> Session {
        (*self).into_session()
    }
}
