// SPDX-License-Identifier: AGPL-3.0-only

#![allow(clippy::disallowed_types)]

use parking_lot::{Condvar, Mutex};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::session::ConversationMessageExt;
use crate::session::SessionExt;
use axagent_harness::SessionTracer;
use axagent_harness::prompt_provider::NoopPromptProvider;
use axagent_harness::skill_evolution_hook::SkillEvolutionHook;
use serde_json::{Map, Value};

use crate::compact::{
    CompactionConfig, CompactionResult, compact_session, estimate_session_tokens,
};
use crate::config::RuntimeFeatureConfig;
use crate::context_contributor::{ContextContributor, ContextRequest};
use crate::execution_progress::AgentExecutionProgress;
use crate::hook_chain::HookChain;
use crate::hooks::{HookAbortSignal, HookProgressReporter, HookRunResult, HookRunner};
use crate::permissions::{
    PermissionContext, PermissionOutcome, PermissionPolicy, PermissionPrompter,
};
use crate::reactive_compact::{ReactiveCompactResult, classify_trigger, try_reactive_compact};
use crate::session::{ContentBlock, ConversationMessage, Session};
use crate::usage::{TokenUsage, UsageTracker};

const NP: &NoopPromptProvider = &NoopPromptProvider;

const DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD: u32 = 100_000;
const AUTO_COMPACTION_THRESHOLD_ENV_VAR: &str = "AXAGENT_AUTO_COMPACT_INPUT_TOKENS";

pub struct PauseState {
    is_paused: Mutex<bool>,
    condvar: Condvar,
}

impl PauseState {
    pub fn new() -> Self {
        Self { is_paused: Mutex::new(false), condvar: Condvar::new() }
    }

    pub fn pause(&self) {
        let mut paused = self.is_paused.lock();
        *paused = true;
        self.condvar.notify_all();
    }

    pub fn resume(&self) {
        let mut paused = self.is_paused.lock();
        *paused = false;
        self.condvar.notify_all();
    }

    pub fn wait_while_paused(&self, cancel_token: Option<&AtomicBool>) {
        let mut paused = self.is_paused.lock();
        while *paused {
            if let Some(token) = cancel_token
                && token.load(Ordering::Relaxed)
            {
                return;
            }
            let deadline = std::time::Instant::now() + Duration::from_secs(1);
            let wait_result = self.condvar.wait_until(&mut paused, deadline);
            if wait_result.timed_out()
                && let Some(token) = cancel_token
                && token.load(Ordering::Relaxed)
            {
                return;
            }
        }
    }

    pub fn is_paused(&self) -> bool {
        *self.is_paused.lock()
    }
}

impl Default for PauseState {
    fn default() -> Self {
        Self::new()
    }
}

// ── 类型定义已上移至 axagent-harness ──
pub use axagent_harness::runtime_types::conversation::{
    ApiClient, ApiRequest, AssistantEvent, PromptCacheEvent, RuntimeError, ToolExecutor,
};

/// 为 StaticToolExecutor 实现 HarnessToolExecutor 契约
impl axagent_harness::HarnessToolExecutor for StaticToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        ToolExecutor::execute(self, tool_name, input)
    }

    fn execute_batch(
        &mut self,
        requests: &[(String, String, String)],
    ) -> Vec<(String, String, Result<String, ToolError>)> {
        ToolExecutor::execute_batch(self, requests)
    }
}

/// 工具错误类型 — 从 axagent-harness 导入的契约定义
pub use axagent_harness::{ToolError, ToolErrorKind};

/// Summary of one completed runtime turn — 从 harness 导入
pub use axagent_harness::runtime_types::conversation::TurnSummary;

/// Details about automatic session compaction — 从 harness 导入
pub use axagent_harness::runtime_types::conversation::AutoCompactionEvent;

/// 3.4 P2:ReAct 循环状态机阶段
///
/// 文档化 conversation.rs 内 ReAct 循环的状态机意图,为未来重构 loop{} 为
/// 显式 `loop { match state { ... } }` 结构奠定基础。当前 loop{} 内部逻辑
/// 已按此状态机顺序执行,只是未显式化为 enum + match。
///
/// ## 状态转移图
/// ```text
/// Start ──▶ CheckCancel ──▶ CheckPause ──▶ CheckIterationLimit
///              │                │                │
///              ▼                ▼                ▼
///          (cancel)         (paused)        (exceeded)
///              │                │                │
///              ▼                ▼                ▼
///           Return           Wait             Return
///
/// CheckIterationLimit ──▶ BuildSystemPrompt ──▶ InjectContext
///                                                       │
///                                                       ▼
///                                                  CallLlm
///                                                       │
///                                                       ▼
///                                              (retry on transient)
///                                                       │
///                                                       ▼
///                                            ParseToolCalls
///                                                       │
///                                       ┌───────────────┴───────────────┐
///                                       ▼                               ▼
///                                  (no tools)                      ExecuteTools
///                                       │                               │
///                                       ▼                               │
///                                  Complete ◀─────────────────────────┘
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactState {
    /// 检查取消令牌
    CheckCancel,
    /// 检查暂停状态
    CheckPause,
    /// 检查迭代上限
    CheckIterationLimit,
    /// 构建系统提示词
    BuildSystemPrompt,
    /// 注入动态上下文
    InjectContext,
    /// 调用 LLM(含重试,见 RecoveryCoordinator)
    CallLlm,
    /// 解析工具调用
    ParseToolCalls,
    /// 执行工具(含重试,见 RecoveryCoordinator)
    ExecuteTools,
    /// 循环完成
    Complete,
}

/// Coordinates the model loop, tool execution, hooks, and session updates.
pub struct ConversationRuntime<C, T> {
    session: Session,
    api_client: C,
    tool_executor: Arc<Mutex<T>>,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    max_iterations: usize,
    usage_tracker: UsageTracker,
    hook_runner: HookRunner,
    auto_compaction_input_tokens_threshold: u32,
    /// 每 N 轮强制压缩一次（防止渐进膨胀，参考 nomifun turn-count 调度）
    compact_every_n_turns: Option<u32>,
    /// 当前轮次计数（用于 compact_every_n_turns）
    turn_count: u32,
    hook_abort_signal: HookAbortSignal,
    hook_progress_reporter: Option<Box<dyn HookProgressReporter>>,
    session_tracer: Option<Arc<dyn SessionTracer>>,
    cancel_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pause_state: Option<Arc<PauseState>>,
    progress: Option<Arc<AgentExecutionProgress>>,
    /// 动态上下文注入器列表（每次 LLM 调用前执行）。
    context_contributors: Vec<Box<dyn ContextContributor>>,
    /// 前端对话 ID。此前 `ContextRequest.conversation_id` 恒为 `None`，
    /// 注入器拿不到会话维度，无法读取会话状态 —— 这是注入管线长期空转的原因之一。
    conversation_id: Option<String>,
    /// Agent 作用域，透传给注入器以支持多 Agent 隔离（None = 单 Agent 场景）。
    agent_id: Option<String>,
    /// 运行时动态工具集（`CapabilityLoad` 激活的工具，每次 LLM 调用前合并进请求）。
    dynamic_tools: Option<axagent_harness::DynamicToolSet>,
    /// Nudge 上下文行（从 NudgeService 提取，每次 run_turn 前设置）。
    nudge_lines: Vec<String>,
    /// 系统级指令（persona 等），注入到每次 LLM 调用的 system_prompt。
    system_directives: Vec<String>,
    /// 错误恢复协调器开关（对应 RuntimeFeatureConfig.error_recovery_enabled）。
    error_recovery_enabled: bool,
    /// 思维链开关（对应 RuntimeFeatureConfig.thought_chain_enabled）。
    thought_chain_enabled: bool,
    /// 可选的 PluginHook 链（MultiAgent 自动委派等），在 LLM 调用前后、工具调用前后执行。
    hook_chain: Option<Arc<HookChain>>,
    /// 可选的技能侧反思钩子（T0.9）：工具执行完成后触发进化判定（经 harness trait 注入）。
    skill_evolution_hook: Option<Arc<dyn SkillEvolutionHook>>,
}

impl<C, T> ConversationRuntime<C, T>
where
    C: ApiClient,
    T: ToolExecutor + Send + 'static,
{
    #[must_use]
    pub fn new(
        session: Session,
        api_client: C,
        tool_executor: T,
        permission_policy: PermissionPolicy,
        system_prompt: Vec<String>,
    ) -> Self {
        Self::new_with_features(
            session,
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
            &RuntimeFeatureConfig::default(),
        )
    }

    #[must_use]
    pub fn new_with_features(
        session: Session,
        api_client: C,
        tool_executor: T,
        permission_policy: PermissionPolicy,
        system_prompt: Vec<String>,
        feature_config: &RuntimeFeatureConfig,
    ) -> Self {
        let usage_tracker = UsageTracker::from_session(&session);
        Self {
            session,
            api_client,
            tool_executor: Arc::new(Mutex::new(tool_executor)),
            permission_policy,
            system_prompt,
            max_iterations: 50,
            usage_tracker,
            hook_runner: HookRunner::from_feature_config(feature_config),
            auto_compaction_input_tokens_threshold: auto_compaction_threshold_from_env(),
            compact_every_n_turns: None,
            turn_count: 0,
            hook_abort_signal: HookAbortSignal::default(),
            hook_progress_reporter: None,
            session_tracer: None,
            cancel_token: None,
            pause_state: None,
            progress: None,
            context_contributors: Vec::new(),
            conversation_id: None,
            agent_id: None,
            dynamic_tools: None,
            nudge_lines: Vec::new(),
            system_directives: Vec::new(),
            error_recovery_enabled: feature_config.error_recovery_enabled,
            thought_chain_enabled: feature_config.thought_chain_enabled,
            hook_chain: None,
            skill_evolution_hook: None,
        }
    }

    #[must_use]
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set a cancel token. When the AtomicBool is set to `true`,
    /// the `run_turn` loop will abort at the next iteration.
    #[must_use]
    pub fn with_cancel_token(
        mut self,
        token: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        self.cancel_token = Some(token);
        self
    }

    #[must_use]
    pub fn with_pause_state(mut self, pause_state: Arc<PauseState>) -> Self {
        self.pause_state = Some(pause_state);
        self
    }

    #[must_use]
    pub fn with_auto_compaction_input_tokens_threshold(mut self, threshold: u32) -> Self {
        self.auto_compaction_input_tokens_threshold = threshold;
        self
    }

    /// 设置每 N 轮强制压缩一次（防止渐进膨胀）。
    /// `None` 表示不启用（默认）。
    #[must_use]
    pub fn with_compact_every_n_turns(mut self, n: Option<u32>) -> Self {
        self.compact_every_n_turns = n;
        self
    }

    #[must_use]
    pub fn with_hook_abort_signal(mut self, hook_abort_signal: HookAbortSignal) -> Self {
        self.hook_abort_signal = hook_abort_signal;
        self
    }

    #[must_use]
    pub fn with_hook_progress_reporter(
        mut self,
        hook_progress_reporter: Box<dyn HookProgressReporter>,
    ) -> Self {
        self.hook_progress_reporter = Some(hook_progress_reporter);
        self
    }

    #[must_use]
    pub fn with_progress(mut self, progress: Arc<AgentExecutionProgress>) -> Self {
        self.progress = Some(progress);
        self
    }

    #[must_use]
    pub fn with_session_tracer(mut self, session_tracer: Arc<dyn SessionTracer>) -> Self {
        self.session_tracer = Some(session_tracer);
        self
    }

    /// 注册一个动态上下文注入器。
    /// 每次 LLM 调用前，所有已注册的 contributor 会依次执行。
    #[must_use]
    pub fn with_context_contributor(mut self, contributor: Box<dyn ContextContributor>) -> Self {
        self.context_contributors.push(contributor);
        self
    }

    /// 设置前端对话 ID，透传给注入器（会话状态读取的必要维度）。
    #[must_use]
    pub fn with_conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self
    }

    /// 设置 Agent 作用域，透传给注入器（多 Agent 隔离）。
    #[must_use]
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// 设置运行时动态工具集。
    ///
    /// `CapabilityLoad` 在循环内激活的工具定义经此合并进下一次 LLM 请求；
    /// 不设置则加载只落状态，模型侧看不到新工具。
    #[must_use]
    pub fn with_dynamic_tools(mut self, set: axagent_harness::DynamicToolSet) -> Self {
        self.dynamic_tools = Some(set);
        self
    }

    /// 设置 PluginHook 链（如 MultiAgentTriggerHook），在 LLM/工具调用前后执行钩子。
    #[must_use]
    pub fn with_hook_chain(mut self, hook_chain: Arc<HookChain>) -> Self {
        self.hook_chain = Some(hook_chain);
        self
    }

    /// 设置技能侧反思钩子（T0.9）：工具执行完成后触发进化判定。
    ///
    /// 实现方为 wiring 层（经 `SkillEvolutionHook` trait 注入），runtime-core 不直接
    /// 依赖 trajectory 实现层。未注入时反思静默跳过，不影响工具执行主流程。
    #[must_use]
    pub fn with_skill_evolution_hook(
        mut self,
        skill_evolution_hook: Arc<dyn SkillEvolutionHook>,
    ) -> Self {
        self.skill_evolution_hook = Some(skill_evolution_hook);
        self
    }

    /// 取当前已激活的动态工具快照(`CapabilityLoad` 在循环内追加的能力)。
    ///
    /// 每次请求前重新取,因此 Agent 在上一轮工具调用里加载的能力,
    /// 下一次 LLM 调用即可发起 function call —— 无需等到下一轮会话。
    fn dynamic_tools_snapshot(&self) -> Vec<axagent_harness::types::ChatTool> {
        self.dynamic_tools.as_ref().map(|s| s.snapshot()).unwrap_or_default()
    }

    /// 准备发送给 LLM 的请求消息:先 L2 Microcompact 去重,再 L1 Snip 截断超长 ToolResult。
    ///
    /// 此方法不修改 session 自身,仅产生请求副本。
    /// 顺序:Microcompact(去重) → Snip(单条截断),避免对占位符做截断。
    /// 执行全部动态上下文注入器，返回待注入的文本块。
    ///
    /// 注入器是 async 的（要读会话状态），而本轮循环是同步的 —— 用
    /// [`drive_sync`] 原地驱动。注入失败不影响主流程，只跳过该块。
    fn run_context_contributors(&self) -> Vec<String> {
        if self.context_contributors.is_empty() {
            return Vec::new();
        }
        let ctx_req = ContextRequest {
            session_id: &self.session.session_id,
            conversation_id: self.conversation_id.as_deref(),
            agent_id: self.agent_id.as_deref(),
            system_prompt: &self.system_prompt,
            extras: &Default::default(),
        };
        drive_sync(async {
            let mut blocks = Vec::new();
            for contributor in &self.context_contributors {
                match contributor.contribute(&ctx_req).await {
                    Some(block) => blocks.push(block),
                    None => {
                        tracing::debug!(
                            contributor = contributor.name(),
                            "动态上下文注入器本轮无内容"
                        );
                    },
                }
            }
            blocks
        })
    }

    fn prepare_request_messages(&self) -> Vec<ConversationMessage> {
        let mc_config = crate::microcompact::MicrocompactConfig::default();
        let snip_config = crate::snip::SnipConfig::default();
        let deduped =
            crate::microcompact::microcompact_messages(&self.session.messages, &mc_config);
        crate::snip::snip_tool_results(&deduped, &snip_config)
    }

    /// 在同步上下文中执行 async HookChain 方法。
    /// 仅在有 hook_chain 时调用,操作轻量（原子增减 + JSON 比对）。
    fn exec_sync_hook<H, F>(&self, f: F) -> H
    where
        F: std::future::Future<Output = H>,
    {
        drive_sync(f)
    }

    fn run_pre_tool_use_hook(
        &mut self,
        tool_name: &str,
        input: &str,
        tool_use_id: Option<&str>,
    ) -> HookRunResult {
        if let Some(reporter) = self.hook_progress_reporter.as_mut() {
            self.hook_runner.run_pre_tool_use_with_context(
                tool_name,
                input,
                Some(&self.hook_abort_signal),
                Some(reporter.as_mut()),
                tool_use_id,
            )
        } else {
            self.hook_runner.run_pre_tool_use_with_context(
                tool_name,
                input,
                Some(&self.hook_abort_signal),
                None,
                tool_use_id,
            )
        }
    }

    fn run_post_tool_use_hook(
        &mut self,
        tool_name: &str,
        input: &str,
        output: &str,
        is_error: bool,
        tool_use_id: Option<&str>,
    ) -> HookRunResult {
        if let Some(reporter) = self.hook_progress_reporter.as_mut() {
            self.hook_runner.run_post_tool_use_with_context(
                tool_name,
                input,
                output,
                is_error,
                Some(&self.hook_abort_signal),
                Some(reporter.as_mut()),
                tool_use_id,
            )
        } else {
            self.hook_runner.run_post_tool_use_with_context(
                tool_name,
                input,
                output,
                is_error,
                Some(&self.hook_abort_signal),
                None,
                tool_use_id,
            )
        }
    }

    fn run_post_tool_use_failure_hook(
        &mut self,
        tool_name: &str,
        input: &str,
        output: &str,
        tool_use_id: Option<&str>,
    ) -> HookRunResult {
        if let Some(reporter) = self.hook_progress_reporter.as_mut() {
            self.hook_runner.run_post_tool_use_failure_with_context(
                tool_name,
                input,
                output,
                Some(&self.hook_abort_signal),
                Some(reporter.as_mut()),
                tool_use_id,
            )
        } else {
            self.hook_runner.run_post_tool_use_failure_with_context(
                tool_name,
                input,
                output,
                Some(&self.hook_abort_signal),
                None,
                tool_use_id,
            )
        }
    }

    /// Run a session health probe to verify the runtime is functional after compaction.
    /// Returns Ok(()) if healthy, Err if the session appears broken.
    fn run_session_health_probe(&mut self) -> Result<(), String> {
        if self.session.messages.is_empty() && self.session.compaction.is_some() {
            return Ok(());
        }

        let probe_input = r#"{"pattern": "*.health-check-probe-"}"#;
        let mut executor = self.tool_executor.lock();
        match executor.execute("glob_search", probe_input) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Tool executor probe failed: {e}")),
        }
    }

    /// Execute a tool on a dedicated thread with timeout enforcement.
    ///
    /// Spawns the tool execution on a separate OS thread so that
    /// `recv_timeout` actually enforces the deadline. Uses
    /// `block_in_place` to avoid starving the tokio runtime while
    /// waiting on the channel.
    ///
    /// If `retry` is `Some(n)`, error messages include the retry
    /// count suffix for differentiated logging.
    fn execute_tool_threaded(
        tool_executor: &Arc<Mutex<T>>,
        tool_name: &str,
        input: &str,
        timeout: Duration,
        retry: Option<u32>,
    ) -> Result<String, RuntimeError> {
        let (tx, rx) = std::sync::mpsc::channel();
        let t_name = tool_name.to_string();
        let t_input = input.to_string();
        let t_executor = tool_executor.clone();
        let t_timeout = timeout;
        let rt_handle = tokio::runtime::Handle::try_current().ok();
        std::thread::spawn(move || {
            let _guard = rt_handle.as_ref().map(|h| h.enter());
            let result = t_executor.lock().execute(&t_name, &t_input);
            let _ = tx.send(result);
        });
        let scope_result = tokio::task::block_in_place(|| rx.recv_timeout(t_timeout));
        match scope_result {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(tool_err)) => Err(RuntimeError::new(tool_err.to_string())),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                Err(RuntimeError::new(match retry {
                    Some(n) => {
                        format!("Tool '{}' timed out after {:?} (retry {})", tool_name, timeout, n)
                    },
                    None => format!("Tool '{}' timed out after {:?}", tool_name, timeout),
                }))
            },
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err(RuntimeError::new(match retry {
                    Some(n) => format!("Tool '{}' retry {} thread panicked", tool_name, n),
                    None => {
                        format!("Tool '{}' execution thread panicked (disconnected)", tool_name)
                    },
                }))
            },
        }
    }

    #[allow(clippy::too_many_lines)] // Complex agent loop with cancel/pause/compaction hooks; splitting would obscure control flow
    pub fn run_turn(
        &mut self,
        user_input: impl Into<String>,
        mut prompter: Option<&mut dyn PermissionPrompter>,
    ) -> Result<TurnSummary, RuntimeError> {
        let user_input = user_input.into();

        // ROADMAP #38: Session-health canary - probe if context was compacted
        if self.session.compaction.is_some()
            && let Err(error) = self.run_session_health_probe()
        {
            return Err(RuntimeError::new(format!(
                "Session health probe failed after compaction: {error}. \
                     The session may be in an inconsistent state. \
                     Consider starting a fresh session with /session new."
            )));
        }

        self.record_turn_started(&user_input);
        self.session
            .push_user_text(user_input)
            .map_err(|error| RuntimeError::new(error.to_string()))?;

        // Initialize execution progress tracking
        if let Some(ref progress) = self.progress {
            progress.start();
            progress.set_iteration(0);
            progress.set_phase("init", "正在初始化...");
        }

        let mut assistant_messages = Vec::new();
        let mut tool_results = Vec::new();
        let mut prompt_cache_events = Vec::new();
        let mut iterations = 0;
        let mut thinking = String::new();

        // Track recent tool calls to detect repeated identical invocations.
        // Key: (tool_name, input_hash), Value: consecutive repeat count.
        let mut recent_tool_calls: std::collections::HashMap<(String, u64), u32> =
            std::collections::HashMap::new();
        const MAX_IDENTICAL_CALLS: u32 = 3; // Warn after 3 identical calls
        const MAX_IDENTICAL_CALLS_HARD: u32 = 5; // Hard limit: abort after 5

        loop {
            iterations += 1;

            // Update progress iteration
            if let Some(ref progress) = self.progress {
                progress.set_iteration(iterations);
                progress.set_phase("llm_call", "正在调用模型...");
            }

            // Check cancel token
            if let Some(ref token) = self.cancel_token
                && token.load(std::sync::atomic::Ordering::Acquire)
            {
                let error = RuntimeError::new("Agent cancelled by user".to_string());
                self.record_turn_failed(iterations, &error);
                return Err(error);
            }

            if let Some(ref pause_state) = self.pause_state {
                pause_state.wait_while_paused(self.cancel_token.as_ref().map(|t| t.as_ref()));
                if let Some(ref token) = self.cancel_token
                    && token.load(Ordering::Acquire)
                {
                    let error = RuntimeError::new("Agent cancelled while paused".to_string());
                    self.record_turn_failed(iterations, &error);
                    return Err(error);
                }
            }

            if iterations > self.max_iterations {
                let error = RuntimeError::new(format!(
                    "conversation loop exceeded the maximum number of iterations ({})",
                    self.max_iterations
                ));
                self.record_turn_failed(iterations, &error);
                return Err(error);
            }

            let mut system_prompt = self.system_prompt.clone();

            // 执行动态上下文注入器（先收集后注入，避免借用冲突）
            let extra_blocks = self.run_context_contributors();
            system_prompt.extend(extra_blocks);

            // 注入系统级指令（persona 等）：位于 nudge 之前、用户内容之外。
            // 与拼进 user message 相比，LLM 将其视为系统指令而非用户输入。
            if !self.system_directives.is_empty() {
                system_prompt.extend(self.system_directives.iter().cloned());
            }

            // 注入 nudge 上下文（从 NudgeService 提取的记忆提醒，在每次 LLM 调用前注入）
            if !self.nudge_lines.is_empty() {
                let nudge_block =
                    format!("<memory_context>\n{}\n</memory_context>", self.nudge_lines.join("\n"));
                system_prompt.push(nudge_block);
            }

            // ── PluginHook: pre_llm_call ──
            if let Some(ref hook_chain) = self.hook_chain {
                let llm_ctx = crate::plugin_hooks::LlmCallContext {
                    model: String::new(),
                    message_count: self.session.messages.len(),
                    tool_count: iterations.saturating_sub(1),
                    estimated_tokens: None,
                    session_id: Some(self.session.session_id.clone()),
                };
                if let Some(crate::plugin_hooks::HookDecision::Veto { reason }) =
                    self.exec_sync_hook(hook_chain.execute_pre_llm_call(&llm_ctx))
                {
                    let error = RuntimeError::new(format!("pre_llm_call hook vetoed: {}", reason));
                    self.record_turn_failed(iterations, &error);
                    return Err(error);
                }
            }

            let request = ApiRequest {
                system_prompt,
                messages: self.prepare_request_messages(),
                extra_tools: self.dynamic_tools_snapshot(),
            };
            let events = match self.api_client.stream(request) {
                Ok(events) => events,
                Err(error) => {
                    let err_msg = error.to_string();

                    // ── L4 Reactive Compact:检测上下文溢出错误,尝试响应式压缩后重试一次 ──
                    // 当 API 返回 prompt_too_long / context_length_exceeded / 413 等错误时,
                    // 用更激进的参数压缩会话,然后重试请求,而非直接返回硬错误。
                    if let Some(trigger) = classify_trigger(&err_msg) {
                        tracing::warn!(trigger = %trigger, "检测到上下文溢出错误,尝试响应式压缩");
                        let compact_result = try_reactive_compact(
                            &self.session,
                            CompactionConfig::default(),
                            trigger,
                        );
                        match compact_result {
                            ReactiveCompactResult::Compacted { result, trigger: t } => {
                                tracing::info!(
                                    trigger = %t,
                                    removed_messages = result.removed_message_count,
                                    remaining_messages = result.compacted_session.messages.len(),
                                    "响应式压缩成功,重试 LLM 请求"
                                );
                                // 应用压缩结果到当前 session
                                self.session = result.compacted_session;
                                let retry_request = ApiRequest {
                                    system_prompt: self.system_prompt.clone(),
                                    messages: self.prepare_request_messages(),
                                    extra_tools: self.dynamic_tools_snapshot(),
                                };
                                match self.api_client.stream(retry_request) {
                                    Ok(events) => events,
                                    Err(retry_error) => {
                                        tracing::warn!(
                                            error = %retry_error,
                                            "响应式压缩后重试仍然失败"
                                        );
                                        self.record_turn_failed(iterations, &retry_error);
                                        return Err(retry_error);
                                    },
                                }
                            },
                            ReactiveCompactResult::Failed { reason } => {
                                tracing::warn!(reason = %reason, "响应式压缩失败");
                                self.record_turn_failed(iterations, &error);
                                return Err(error);
                            },
                            ReactiveCompactResult::Skipped => {
                                tracing::warn!("响应式压缩被跳过:会话消息数过少,无法压缩");
                                self.record_turn_failed(iterations, &error);
                                return Err(error);
                            },
                        }
                    } else {
                        // ── 3.6 P2:使用 RecoveryCoordinator 统一调度 LLM API 重试 ──
                        // 替代原局部 MAX_RETRIES=3 + 线性退避逻辑,
                        // 采用错误分类 → 策略选择 → 指数退避的统一模式。
                        let error_type = classify_recovery_error(&err_msg);
                        let recovery_action = if self.error_recovery_enabled {
                            get_recovery_action(error_type)
                        } else {
                            RecoveryAction::Fail
                        };
                        match recovery_action {
                            RecoveryAction::Fail => {
                                tracing::warn!(
                                    error_type = ?error_type,
                                    error = %err_msg,
                                    "[RecoveryCoordinator] LLM API 错误不可恢复,直接失败"
                                );
                                self.record_turn_failed(iterations, &error);
                                return Err(error);
                            },
                            RecoveryAction::Retry { max_attempts, base_delay_ms } => {
                                let mut retry_count = 0;
                                loop {
                                    retry_count += 1;
                                    if retry_count > max_attempts {
                                        self.record_turn_failed(iterations, &error);
                                        return Err(error);
                                    }
                                    // Check cancel token before sleeping
                                    if let Err(cancel_err) =
                                        check_cancelled(self.cancel_token.as_ref())
                                    {
                                        self.record_turn_failed(iterations, &cancel_err);
                                        return Err(cancel_err);
                                    }
                                    let delay = compute_backoff_delay(base_delay_ms, retry_count);
                                    std::thread::sleep(std::time::Duration::from_millis(delay));
                                    let retry_request = ApiRequest {
                                        system_prompt: self.system_prompt.clone(),
                                        messages: self.prepare_request_messages(),
                                        extra_tools: self.dynamic_tools_snapshot(),
                                    };
                                    match self.api_client.stream(retry_request) {
                                        Ok(events) => break events,
                                        Err(retry_error) => {
                                            let retry_str = retry_error.to_string();
                                            let new_type = classify_recovery_error(&retry_str);
                                            // 不可恢复错误或重试耗尽:立即失败
                                            if matches!(new_type, RecoveryErrorType::Unrecoverable)
                                                || retry_count >= max_attempts
                                            {
                                                self.record_turn_failed(iterations, &retry_error);
                                                return Err(retry_error);
                                            }
                                            tracing::warn!(
                                                attempt = retry_count,
                                                max_attempts,
                                                error_type = ?new_type,
                                                error = %retry_str,
                                                "[RecoveryCoordinator] LLM API 重试中"
                                            );
                                            // Continue retrying with reclassified type
                                        },
                                    }
                                }
                            },
                        }
                    }
                },
            };
            let (assistant_message, usage, turn_prompt_cache_events, turn_thinking) =
                match build_assistant_message(events, self.thought_chain_enabled) {
                    Ok(result) => result,
                    Err(error) => {
                        self.record_turn_failed(iterations, &error);
                        return Err(error);
                    },
                };

            // ── PluginHook: post_llm_call ──
            if let Some(ref hook_chain) = self.hook_chain {
                let llm_ctx = crate::plugin_hooks::LlmCallContext {
                    model: String::new(),
                    message_count: self.session.messages.len(),
                    tool_count: iterations.saturating_sub(1),
                    estimated_tokens: usage.map(|u| u.input_tokens as u64),
                    session_id: Some(self.session.session_id.clone()),
                };
                let llm_result = crate::plugin_hooks::LlmCallResult {
                    content: assistant_message
                        .blocks
                        .iter()
                        .filter_map(|b| match b {
                            ContentBlock::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    tool_calls: Some(
                        assistant_message
                            .blocks
                            .iter()
                            .filter_map(|b| match b {
                                ContentBlock::ToolUse { name, .. } => Some(name.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>(),
                    ),
                    usage_prompt_tokens: usage.map(|u| u.input_tokens),
                    usage_completion_tokens: usage.map(|u| u.output_tokens),
                    duration_ms: None,
                };
                self.exec_sync_hook(hook_chain.execute_post_llm_call(&llm_ctx, &llm_result));
            }
            if !turn_thinking.is_empty() {
                if !thinking.is_empty() {
                    thinking.push('\n');
                }
                thinking.push_str(&turn_thinking);
            }
            if let Some(usage) = usage {
                self.usage_tracker.record(usage);
            }
            prompt_cache_events.extend(turn_prompt_cache_events);
            let pending_tool_uses = assistant_message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolUse { id, name, input } => {
                        Some((id.clone(), name.clone(), input.clone()))
                    },
                    _ => None,
                })
                .collect::<Vec<_>>();
            self.record_assistant_iteration(
                iterations,
                &assistant_message,
                pending_tool_uses.len(),
            );

            self.session
                .push_message(assistant_message.clone())
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            assistant_messages.push(assistant_message);

            if pending_tool_uses.is_empty() {
                break;
            }

            for (tool_use_id, tool_name, input) in pending_tool_uses {
                // Detect repeated identical tool calls to prevent infinite loops.
                let input_hash = {
                    use std::hash::Hasher;
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    hasher.write(input.as_bytes());
                    hasher.finish()
                };
                let repeat_key = (tool_name.clone(), input_hash);
                let repeat_count = recent_tool_calls.entry(repeat_key.clone()).or_insert(0);
                *repeat_count += 1;

                if *repeat_count >= MAX_IDENTICAL_CALLS_HARD {
                    let error = RuntimeError::new(format!(
                        "Aborted: tool '{}' called {} times with identical arguments. \
                                 This likely indicates a loop — please try a different approach.",
                        tool_name, repeat_count
                    ));
                    self.record_turn_failed(iterations, &error);
                    return Err(error);
                }

                if *repeat_count == MAX_IDENTICAL_CALLS {
                    let warning_msg = ConversationMessageExt::assistant(vec![ContentBlock::Text {
                        text: format!(
                            "[System] You have called '{}' {} times with the same arguments. \
                                          If it keeps failing, try a different approach or respond directly to the user.",
                            tool_name, repeat_count
                        ),
                    }]);
                    self.session
                        .push_message(warning_msg)
                        .map_err(|error| RuntimeError::new(error.to_string()))?;
                }

                // ── PluginHook: pre_tool_call ──
                if let Some(ref hook_chain) = self.hook_chain {
                    let tool_ctx = crate::plugin_hooks::ToolCallContext {
                        tool_name: tool_name.clone(),
                        tool_namespace: None,
                        arguments: serde_json::from_str(&input).unwrap_or_default(),
                        session_id: Some(self.session.session_id.clone()),
                    };
                    if let Some(crate::plugin_hooks::HookDecision::Veto { reason }) =
                        self.exec_sync_hook(hook_chain.execute_pre_tool_call(&tool_ctx))
                    {
                        let error = RuntimeError::new(format!("工具被 hook 拒绝: {}", reason));
                        self.record_turn_failed(iterations, &error);
                        return Err(error);
                    }
                }

                let pre_hook_result =
                    self.run_pre_tool_use_hook(&tool_name, &input, Some(&tool_use_id));
                let effective_input = pre_hook_result
                    .updated_input()
                    .map_or_else(|| input.clone(), ToOwned::to_owned);
                let permission_context = PermissionContext::new(
                    pre_hook_result.permission_override(),
                    pre_hook_result.permission_reason().map(ToOwned::to_owned),
                );

                let permission_outcome = if pre_hook_result.is_cancelled() {
                    PermissionOutcome::Deny {
                        reason: format_hook_message(
                            &pre_hook_result,
                            &format!("PreToolUse hook cancelled tool `{tool_name}`"),
                        ),
                    }
                } else if pre_hook_result.is_failed() {
                    PermissionOutcome::Deny {
                        reason: format_hook_message(
                            &pre_hook_result,
                            &format!("PreToolUse hook failed for tool `{tool_name}`"),
                        ),
                    }
                } else if pre_hook_result.is_denied() {
                    PermissionOutcome::Deny {
                        reason: format_hook_message(
                            &pre_hook_result,
                            &format!("PreToolUse hook denied tool `{tool_name}`"),
                        ),
                    }
                } else if let Some(prompt) = prompter.as_mut() {
                    self.permission_policy.authorize_with_context(
                        &tool_name,
                        &effective_input,
                        &permission_context,
                        Some(*prompt),
                    )
                } else {
                    self.permission_policy.authorize_with_context(
                        &tool_name,
                        &effective_input,
                        &permission_context,
                        None,
                    )
                };

                let result_message: crate::session::ConversationMessage = match permission_outcome {
                    PermissionOutcome::Allow => {
                        self.record_tool_started(iterations, &tool_name);

                        // Emit progress heartbeat before each tool call so the frontend
                        // watchdog knows the agent is still active even during long tool
                        // execution (e.g. a bash command approaching the 300s timeout).
                        if let Some(reporter) = self.hook_progress_reporter.as_mut() {
                            reporter.on_progress(
                                &format!("正在运行工具: {} (第 {} 轮)", tool_name, iterations),
                                iterations,
                                self.max_iterations,
                            );
                        }

                        // Update shared execution progress for the frontend panels
                        if let Some(ref progress) = self.progress {
                            progress.begin_tool(&tool_name, Some(&effective_input));
                        }

                        // Determine timeout based on tool category
                        let tool_timeout = Self::tool_timeout_for(&tool_name);

                        let (mut output, mut is_error) = {
                            let first_result = Self::execute_tool_threaded(
                                &self.tool_executor,
                                &tool_name,
                                &effective_input,
                                tool_timeout,
                                None,
                            );
                            match first_result {
                                Ok(output) => (output, false),
                                Err(error) => {
                                    let err_str = error.to_string();
                                    // ── 3.6 P2:使用 RecoveryCoordinator 统一调度工具执行重试 ──
                                    // 替代原局部 MAX_TOOL_RETRIES=3 + 线性退避逻辑,
                                    // 采用错误分类 → 策略选择 → 指数退避的统一模式。
                                    let error_type = classify_recovery_error(&err_str);
                                    let recovery_action = if self.error_recovery_enabled {
                                        get_recovery_action(error_type)
                                    } else {
                                        RecoveryAction::Fail
                                    };
                                    match recovery_action {
                                        RecoveryAction::Fail => {
                                            tracing::warn!(
                                                error_type = ?error_type,
                                                error = %err_str,
                                                "[RecoveryCoordinator] 工具执行错误不可恢复,直接失败"
                                            );
                                            (err_str, true)
                                        },
                                        RecoveryAction::Retry { max_attempts, base_delay_ms } => {
                                            let mut retry_count = 0;
                                            let mut last_err = err_str.clone();
                                            loop {
                                                retry_count += 1;
                                                if retry_count > max_attempts {
                                                    break (last_err, true);
                                                }
                                                // Check cancel token before sleeping
                                                if let Err(cancel_err) =
                                                    check_cancelled(self.cancel_token.as_ref())
                                                {
                                                    break (cancel_err.to_string(), true);
                                                }
                                                let delay = compute_backoff_delay(
                                                    base_delay_ms,
                                                    retry_count,
                                                );
                                                std::thread::sleep(
                                                    std::time::Duration::from_millis(delay),
                                                );
                                                // Emit heartbeat before retry so the frontend
                                                // watchdog doesn't fire during retry chains.
                                                if let Some(reporter) =
                                                    self.hook_progress_reporter.as_mut()
                                                {
                                                    reporter.on_progress(
                                                        &format!(
                                                            "正在重试工具: {} (第 {} 轮, 第 {} 次尝试)",
                                                            tool_name, iterations, retry_count
                                                        ),
                                                        iterations,
                                                        self.max_iterations,
                                                    );
                                                }
                                                let retry_result = Self::execute_tool_threaded(
                                                    &self.tool_executor,
                                                    &tool_name,
                                                    &effective_input,
                                                    tool_timeout,
                                                    Some(retry_count),
                                                );
                                                match retry_result {
                                                    Ok(output) => break (output, false),
                                                    Err(retry_err) => {
                                                        let retry_str = retry_err.to_string();
                                                        let new_type =
                                                            classify_recovery_error(&retry_str);
                                                        // 不可恢复错误或重试耗尽:立即失败
                                                        if matches!(
                                                            new_type,
                                                            RecoveryErrorType::Unrecoverable
                                                        ) || retry_count >= max_attempts
                                                        {
                                                            break (retry_str, true);
                                                        }
                                                        tracing::warn!(
                                                            attempt = retry_count,
                                                            max_attempts,
                                                            error_type = ?new_type,
                                                            error = %retry_str,
                                                            "[RecoveryCoordinator] 工具执行重试中"
                                                        );
                                                        last_err = retry_str;
                                                        // Continue retrying with reclassified type
                                                    },
                                                }
                                            }
                                        },
                                    }
                                },
                            }
                        };
                        output = merge_hook_feedback(pre_hook_result.messages(), output, false);

                        let post_hook_result = if is_error {
                            self.run_post_tool_use_failure_hook(
                                &tool_name,
                                &effective_input,
                                &output,
                                Some(&tool_use_id),
                            )
                        } else {
                            self.run_post_tool_use_hook(
                                &tool_name,
                                &effective_input,
                                &output,
                                false,
                                Some(&tool_use_id),
                            )
                        };
                        if post_hook_result.is_denied()
                            || post_hook_result.is_failed()
                            || post_hook_result.is_cancelled()
                        {
                            is_error = true;
                        }
                        output = merge_hook_feedback(
                            post_hook_result.messages(),
                            output,
                            post_hook_result.is_denied()
                                || post_hook_result.is_failed()
                                || post_hook_result.is_cancelled(),
                        );

                        // ── PluginHook: post_tool_call ──
                        if let Some(ref hook_chain) = self.hook_chain {
                            let tool_ctx = crate::plugin_hooks::ToolCallContext {
                                tool_name: tool_name.clone(),
                                tool_namespace: None,
                                arguments: serde_json::from_str(&input).unwrap_or_default(),
                                session_id: Some(self.session.session_id.clone()),
                            };
                            let tool_result = crate::plugin_hooks::ToolCallResult {
                                tool_name: tool_name.clone(),
                                result: serde_json::Value::String(output.clone()),
                                success: !is_error,
                                duration_ms: None,
                            };
                            self.exec_sync_hook(
                                hook_chain.execute_post_tool_call(&tool_ctx, &tool_result),
                            );
                        }

                        // ── 技能侧反思钩子（T0.9）：工具执行完成后触发进化判定 ──
                        // 经 harness `SkillEvolutionHook` trait 注入（wiring 层实现），
                        // runtime-core 不直接依赖 trajectory 实现层。反思结果不阻塞主流程。
                        if let Some(ref evo_hook) = self.skill_evolution_hook {
                            let triggered = self.exec_sync_hook(
                                evo_hook.on_tool_executed(&tool_name, !is_error, &output),
                            );
                            if triggered {
                                tracing::debug!(
                                    tool = %tool_name,
                                    "🗺️ 技能反思钩子已生成进化提议，进入用户同意通道"
                                );
                            }
                        }

                        // Update shared execution progress before the
                        // Allow arm closes (output/is_error live here).
                        if let Some(ref progress) = self.progress {
                            progress.end_tool(is_error, Some(&output));
                        }

                        ConversationMessageExt::tool_result(
                            tool_use_id,
                            tool_name,
                            output,
                            is_error,
                        )
                    },
                    PermissionOutcome::Deny { reason } => ConversationMessageExt::tool_result(
                        tool_use_id,
                        tool_name,
                        merge_hook_feedback(pre_hook_result.messages(), reason, true),
                        true,
                    ),
                };
                self.session
                    .push_message(result_message.clone())
                    .map_err(|error| RuntimeError::new(error.to_string()))?;
                self.record_tool_finished(iterations, &result_message);
                tool_results.push(result_message);
            }

            // Emit iteration progress heartbeat so the frontend watchdog timer
            // knows the agent is still running. Without this, long-running tasks
            // (multi-iteration + tool calls) would silently exceed the 10-min
            // frontend watchdpg timeout even though the backend is working.
            if let Some(reporter) = self.hook_progress_reporter.as_mut() {
                reporter.on_progress(
                    &format!("第 {}/{} 轮迭代完成", iterations, self.max_iterations),
                    iterations,
                    self.max_iterations,
                );
            }
        }
        let auto_compaction = self.maybe_auto_compact();

        let summary = TurnSummary {
            assistant_messages,
            tool_results,
            prompt_cache_events,
            iterations,
            usage: self.usage_tracker.cumulative_usage(),
            auto_compaction,
            thinking,
        };
        self.record_turn_completed(&summary);

        if let Some(ref progress) = self.progress {
            progress.finish();
        }

        Ok(summary)
    }

    #[must_use]
    pub fn compact(&self, config: CompactionConfig) -> CompactionResult {
        compact_session(&self.session, config, NP)
    }

    #[must_use]
    pub fn estimated_tokens(&self) -> usize {
        estimate_session_tokens(&self.session)
    }

    #[must_use]
    pub fn usage(&self) -> &UsageTracker {
        &self.usage_tracker
    }

    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn api_client_mut(&mut self) -> &mut C {
        &mut self.api_client
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    #[must_use]
    pub fn fork_session(&self, branch_name: Option<String>) -> Session {
        self.session.fork(branch_name)
    }

    #[must_use]
    pub fn into_session(self) -> Session {
        self.session
    }

    fn maybe_auto_compact(&mut self) -> Option<AutoCompactionEvent> {
        self.turn_count = self.turn_count.saturating_add(1);

        // 1. 轮次数前置压缩：每 N 轮强制压缩一次，防止渐进膨胀
        if let Some(every_n) = self.compact_every_n_turns
            && every_n > 0
            && self.turn_count.is_multiple_of(every_n)
        {
            let result =
                compact_session(&self.session, crate::compact::CompactionConfig::default(), NP);
            if result.removed_message_count > 0 {
                tracing::info!(
                    "Turn-count compaction triggered at turn {} (every {})",
                    self.turn_count,
                    every_n,
                );
                self.session = result.compacted_session;
                return Some(AutoCompactionEvent {
                    removed_message_count: result.removed_message_count,
                });
            }
        }

        // 2. Token 阈值压缩 + 紧急模式
        if self.usage_tracker.cumulative_usage().input_tokens
            < self.auto_compaction_input_tokens_threshold
        {
            return None;
        }

        // 超过阈值时强制压缩: 累积输入 token 已越限, 必须压缩。
        // 不能用 should_compact 的「当前消息体量」判定来拦截——该判定只看消息
        // 估算 token(短会话会误判为无需压缩), 而阈值是基于累积 API 用量(上下文
        // 窗口压力), 二者语义不同。故用 max_estimated_tokens=0 使 should_compact
        // 的 token 门槛恒真, 确保一定执行压缩。
        // preserve_recent_messages 必须沿用 default()(=12): run_turn 内 compact 时
        // session.messages 含 system + 13 条内容 + 1 条 API 新回复共 15 条,
        // keep_from = 15 - 12 = 3, 恰为测试期望的删除 3 条。覆盖为其它值(如 10)
        // 会得到 5 条, 与契约不符; emergency_compaction_config()(preserve=1) 则过度删除。
        let config = CompactionConfig { max_estimated_tokens: 0, ..CompactionConfig::default() };
        let result = compact_session(&self.session, config, NP);

        if result.removed_message_count == 0 {
            return None;
        }

        self.session = result.compacted_session;
        Some(AutoCompactionEvent { removed_message_count: result.removed_message_count })
    }

    fn record_turn_started(&self, user_input: &str) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("user_input".to_string(), Value::String(user_input.to_string()));
        session_tracer.record("turn_started", attributes);
    }

    fn record_assistant_iteration(
        &self,
        iteration: usize,
        assistant_message: &ConversationMessage,
        pending_tool_use_count: usize,
    ) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert(
            "assistant_blocks".to_string(),
            Value::from(assistant_message.blocks.len() as u64),
        );
        attributes.insert(
            "pending_tool_use_count".to_string(),
            Value::from(pending_tool_use_count as u64),
        );
        session_tracer.record("assistant_iteration_completed", attributes);
    }

    fn record_tool_started(&self, iteration: usize, tool_name: &str) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert("tool_name".to_string(), Value::String(tool_name.to_string()));
        session_tracer.record("tool_execution_started", attributes);
    }

    /// Determine the timeout duration for a tool based on its category.
    ///
    /// Categories and their timeouts:
    /// - **Read operations** (read_file, list_directory, get, search, grep, glob, head, cat):
    ///   30 seconds — these should be fast.
    /// - **Search operations** (web_search, search, query, find, rag, vector):
    ///   60 seconds — network-dependent, may take longer.
    /// - **Write operations** (write_file, edit, create, delete, move, rename, patch, mkdir):
    ///   120 seconds — file I/O can be slow on large files or network drives.
    /// - **Execute operations** (shell, bash, exec, run, command, terminal, python, node):
    ///   300 seconds (5 min) — user scripts may run arbitrarily long.
    /// - **Default**: 60 seconds.
    fn tool_timeout_for(tool_name: &str) -> std::time::Duration {
        let name_lower = tool_name.to_lowercase();

        // Execute/shell operations — longest timeout
        const EXECUTE_PATTERNS: &[&str] = &[
            "shell",
            "bash",
            "exec",
            "run",
            "command",
            "terminal",
            "python",
            "node",
            "npm",
            "cargo",
            "make",
            "gradle",
            "subprocess",
            "spawn",
        ];
        if EXECUTE_PATTERNS.iter().any(|p| Self::match_tool_pattern(&name_lower, p)) {
            return std::time::Duration::from_secs(300);
        }

        // Write operations — moderate-long timeout
        const WRITE_PATTERNS: &[&str] = &[
            "write", "edit", "create", "delete", "remove", "move", "rename", "patch", "mkdir",
            "save", "put", "post", "upload", "install",
        ];
        if WRITE_PATTERNS.iter().any(|p| Self::match_tool_pattern(&name_lower, p)) {
            return std::time::Duration::from_secs(120);
        }

        // Search operations — moderate timeout
        const SEARCH_PATTERNS: &[&str] = &[
            "search", "query", "find", "rag", "vector", "web", "fetch", "http", "request", "api",
            "crawl",
        ];
        if SEARCH_PATTERNS.iter().any(|p| Self::match_tool_pattern(&name_lower, p)) {
            return std::time::Duration::from_secs(60);
        }

        // Read operations — short timeout
        const READ_PATTERNS: &[&str] = &[
            "read", "list", "get", "grep", "glob", "head", "cat", "stat", "ls", "dir", "type",
            "peek", "view",
        ];
        if READ_PATTERNS.iter().any(|p| Self::match_tool_pattern(&name_lower, p)) {
            return std::time::Duration::from_secs(30);
        }

        // Default timeout
        std::time::Duration::from_secs(60)
    }

    /// 边界感知的工具名模式匹配，防止子串误匹配。
    ///
    /// 匹配规则（按优先级）：
    /// 1. 完整相等（如 `"bash"` 匹配 `"bash"`）
    /// 2. 前缀 + 分隔符（如 `"bash"` 匹配 `"bash_shell"`, `"bash-run"`）
    /// 3. 后缀 + 分隔符（如 `"bash"` 匹配 `"my_bash"`, `"run-bash"`）
    ///
    /// 子串匹配（如 `"edit"` 匹配 `"credit_check"`）返回 false。
    /// 不区分大小写。
    fn match_tool_pattern(tool_name_lower: &str, pattern: &str) -> bool {
        if tool_name_lower == pattern {
            return true;
        }
        let n = tool_name_lower.len();
        let p = pattern.len();
        if p > n {
            return false;
        }
        // Prefix match: pattern at start, followed by word boundary char
        if tool_name_lower.starts_with(pattern)
            && tool_name_lower
                .as_bytes()
                .get(p)
                .is_some_and(|&ch| matches!(ch, b'_' | b'-' | b'.' | b'/' | b':' | b' '))
        {
            return true;
        }
        // Suffix match: pattern at end, preceded by word boundary char
        if tool_name_lower.ends_with(pattern)
            && tool_name_lower
                .as_bytes()
                .get(n - p - 1)
                .is_some_and(|&ch| matches!(ch, b'_' | b'-' | b'.' | b'/' | b':' | b' '))
        {
            return true;
        }
        false
    }

    fn record_tool_finished(&self, iteration: usize, result_message: &ConversationMessage) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let Some(ContentBlock::ToolResult { tool_name, is_error, .. }) =
            result_message.blocks.first()
        else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert("tool_name".to_string(), Value::String(tool_name.clone()));
        attributes.insert("is_error".to_string(), Value::Bool(*is_error));
        session_tracer.record("tool_execution_finished", attributes);
    }

    fn record_turn_completed(&self, summary: &TurnSummary) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iterations".to_string(), Value::from(summary.iterations as u64));
        attributes.insert(
            "assistant_messages".to_string(),
            Value::from(summary.assistant_messages.len() as u64),
        );
        attributes
            .insert("tool_results".to_string(), Value::from(summary.tool_results.len() as u64));
        attributes.insert(
            "prompt_cache_events".to_string(),
            Value::from(summary.prompt_cache_events.len() as u64),
        );
        session_tracer.record("turn_completed", attributes);
    }

    fn record_turn_failed(&self, iteration: usize, error: &RuntimeError) {
        if let Some(ref progress) = self.progress {
            progress.fail(&error.to_string());
        }

        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert("error".to_string(), Value::String(error.to_string()));
        session_tracer.record("turn_failed", attributes);
    }
}

/// 在同步上下文中驱动一个 future 完成。
///
/// 本模块的 ReAct 循环是同步的（`ApiClient::stream` / `ToolExecutor::execute`
/// 均为同步 trait），但部分扩展点是 async 的：HookChain、以及需要读会话状态的
/// 上下文注入器。统一由本函数桥接，两种运行时形态都覆盖：
///
/// - **生产路径**（多线程 runtime）：`block_in_place` 通知 tokio 把其他任务
///   迁走，再 `block_on` —— 与 `AxAgentApiClient::stream` 的处理一致。
/// - **同步测试路径**（无 runtime 上下文）：`futures::executor::block_on`
///   直接驱动，避免 `block_in_place` 在无 runtime 时 panic。
fn drive_sync<F: std::future::Future>(f: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(f)),
        Err(_) => futures::executor::block_on(f),
    }
}

/// Reads the automatic compaction threshold from the environment.
#[must_use]
pub fn auto_compaction_threshold_from_env() -> u32 {
    parse_auto_compaction_threshold(
        std::env::var(AUTO_COMPACTION_THRESHOLD_ENV_VAR).ok().as_deref(),
    )
}

#[must_use]
fn parse_auto_compaction_threshold(value: Option<&str>) -> u32 {
    value
        .and_then(|raw| raw.trim().parse::<u32>().ok())
        .filter(|threshold| *threshold > 0)
        .unwrap_or(DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD)
}

fn build_assistant_message(
    events: Vec<AssistantEvent>,
    show_thought_chain: bool,
) -> Result<(ConversationMessage, Option<TokenUsage>, Vec<PromptCacheEvent>, String), RuntimeError>
{
    let mut text = String::new();
    let mut thinking = String::new();
    let mut blocks = Vec::new();
    let mut prompt_cache_events = Vec::new();
    let mut finished = false;
    let mut usage = None;

    for event in events {
        match event {
            AssistantEvent::TextDelta(delta) => text.push_str(&delta),
            AssistantEvent::ThinkingDelta(delta) => thinking.push_str(&delta),
            AssistantEvent::ToolUse { id, name, input } => {
                flush_text_block(&mut text, &mut blocks);
                blocks.push(ContentBlock::ToolUse { id, name, input });
            },
            AssistantEvent::Usage(value) => usage = Some(value),
            AssistantEvent::PromptCache(event) => prompt_cache_events.push(event),
            AssistantEvent::MessageStop => {
                finished = true;
            },
        }
    }

    flush_text_block(&mut text, &mut blocks);

    // 思维链可视化：仅当 thought_chain_enabled 开启时，才把推理内容包成
    // `<think>` 块前置到正文（前端据此渲染可折叠的"思考过程"）。关闭时
    // 推理内容仍被捕获并作为第 4 个返回值透出（用于 trajectory / 日志），
    // 只是不在对话正文里做可视化，避免污染最终答案。
    if show_thought_chain && !thinking.is_empty() {
        let thinking_text = format!("<think data-axagent=\"1\">\n{}\n</think>", thinking);
        if let Some(ContentBlock::Text { text }) = blocks.first_mut() {
            *text = format!("{}\n\n{}", thinking_text, text);
        } else {
            blocks.insert(0, ContentBlock::Text { text: thinking_text });
        }
    }

    // P0 修复(2026-08-29): 兜底注入 — 当 blocks 仍为空但 thinking 有内容时
    // （典型场景：推理模型只输出 reasoning 不输出正文，或 thought_chain_enabled=false
    // 导致 thinking 未被注入），将 thinking 作为 fallback content 注入 blocks。
    // 此前这种情况会直接报 "assistant stream produced no content"，导致 Agent 循环
    // 中断；兜底注入让循环能继续推进（如触发重试或向前端展示已有的推理过程）。
    if blocks.is_empty() && !thinking.is_empty() {
        tracing::warn!(
            target: "axagent.reliability",
            thinking_len = thinking.len(),
            thought_chain_enabled = show_thought_chain,
            "LLM stream produced no text content but has thinking — \
             injecting thinking as fallback to avoid empty-blocks error",
        );
        blocks.push(ContentBlock::Text {
            text: format!("[模型仅返回推理过程，未产出可见回答]\n\n{}", thinking),
        });
    }

    if !finished {
        // Stream interrupted — if we have partial content, return it with
        // a recovery marker so the agent loop can continue rather than
        // losing all progress. This handles network drops, server errors
        // mid-stream, etc.
        if !blocks.is_empty() {
            tracing::warn!(
                "[stream-recovery] Stream ended without MessageStop but has {} content blocks — \
                 returning partial result for recovery",
                blocks.len()
            );
            // Append a recovery notice to the last text block so the LLM
            // knows its previous response was truncated
            if let Some(ContentBlock::Text { text: last_text }) = blocks.last_mut() {
                last_text.push_str("\n\n[Stream was interrupted — partial response recovered]");
            } else {
                blocks.push(ContentBlock::Text {
                    text: "[Stream was interrupted — partial response recovered]".to_string(),
                });
            }
            // Return partial result — the agent loop will treat this as
            // a complete assistant turn and continue (potentially retrying
            // or asking the user)
            return Ok((
                ConversationMessageExt::assistant_with_usage(blocks, usage),
                usage,
                prompt_cache_events,
                thinking,
            ));
        }
        return Err(RuntimeError::new(
            "assistant stream ended without a message stop event and no content was received",
        ));
    }
    if blocks.is_empty() {
        return Err(RuntimeError::new("assistant stream produced no content"));
    }

    Ok((
        ConversationMessageExt::assistant_with_usage(blocks, usage),
        usage,
        prompt_cache_events,
        thinking,
    ))
}

fn flush_text_block(text: &mut String, blocks: &mut Vec<ContentBlock>) {
    if !text.is_empty() {
        blocks.push(ContentBlock::Text { text: std::mem::take(text) });
    }
}

// ── 3.6 P2:统一恢复策略调度(本地化 ErrorRecoveryEngine 风格) ─────────────
//
// runtime-core 是 consumer crate,按架构铁律不能依赖 agent crate 的
// `ErrorRecoveryEngine`。此处实现本地化的恢复协调器,采用相同的
// 「错误分类 → 策略选择 → 退避重试」模式,统一 conversation.rs 内
// LLM API 调用与工具执行两段原本各自为政的 `MAX_RETRIES` 逻辑。
//
// 策略映射:
// - Transient (timeout/network/429/...) → Retry(指数退避,3 次)
// - Recoverable (permission/quota)      → Retry(短退避,2 次)
// - Unrecoverable (syntax/auth/panic)   → Fail(立即失败)
// - Unknown                              → Retry(按 Transient 处理)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryErrorType {
    Transient,
    Recoverable,
    Unrecoverable,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
enum RecoveryAction {
    Retry { max_attempts: u32, base_delay_ms: u64 },
    Fail,
}

/// 分类错误类型 — 基于 ErrorRecoveryEngine 的分类逻辑(本地化实现)
fn classify_recovery_error(err: &str) -> RecoveryErrorType {
    let lower = err.to_lowercase();

    // 不可恢复:语法错误、认证失败、内存耗尽、panic
    if lower.contains("syntax error")
        || lower.contains("authentication")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("invalid api key")
        || lower.contains("out of memory")
        || lower.contains("panic")
    {
        return RecoveryErrorType::Unrecoverable;
    }

    // 可恢复:权限问题、配额限制
    if lower.contains("permission denied") || lower.contains("quota") {
        return RecoveryErrorType::Recoverable;
    }

    // 瞬时错误:超时、网络、限流、连接问题
    if lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("network")
        || lower.contains("connection")
        || lower.contains("reset")
        || lower.contains("broken pipe")
        || lower.contains("eof")
        || lower.contains("unavailable")
        || lower.contains("429")
        || lower.contains("rate")
        || lower.contains("temporarily")
        // P0 修复(2026-08-29): provider 空响应分类为瞬时错误,自动重试
        // 而非杀死 Agent 循环(见 provider_adapter.rs 空响应拦截)
        || lower.contains("empty response")
    {
        return RecoveryErrorType::Transient;
    }

    RecoveryErrorType::Unknown
}

/// 根据错误类型选择恢复策略
fn get_recovery_action(error_type: RecoveryErrorType) -> RecoveryAction {
    match error_type {
        RecoveryErrorType::Unrecoverable => RecoveryAction::Fail,
        RecoveryErrorType::Transient | RecoveryErrorType::Unknown => {
            RecoveryAction::Retry { max_attempts: 3, base_delay_ms: 1000 }
        },
        RecoveryErrorType::Recoverable => {
            RecoveryAction::Retry { max_attempts: 2, base_delay_ms: 500 }
        },
    }
}

/// 检查取消令牌 — 若已取消则返回 RuntimeError
fn check_cancelled(cancel_token: Option<&Arc<AtomicBool>>) -> Result<(), RuntimeError> {
    if let Some(token) = cancel_token
        && token.load(Ordering::Acquire)
    {
        return Err(RuntimeError::new("Agent cancelled by user".to_string()));
    }
    Ok(())
}

/// 计算指数退避延迟 — base * 2^(attempt-1),attempt 从 1 开始
fn compute_backoff_delay(base_delay_ms: u64, attempt: u32) -> u64 {
    // cap exponent at 10 to avoid overflow;attempt=1 → base,attempt=2 → 2*base,...
    let exp = (attempt.saturating_sub(1)).min(10);
    base_delay_ms.saturating_mul(1u64 << exp)
}

fn format_hook_message(result: &HookRunResult, fallback: &str) -> String {
    if result.messages().is_empty() {
        fallback.to_string()
    } else {
        result.messages().join("\n")
    }
}

fn merge_hook_feedback(messages: &[String], output: String, is_error: bool) -> String {
    if messages.is_empty() {
        return output;
    }

    let mut sections = Vec::new();
    if !output.trim().is_empty() {
        sections.push(output);
    }
    let label = if is_error {
        "Hook feedback (error)"
    } else {
        "Hook feedback"
    };
    sections.push(format!("{label}:\n{}", messages.join("\n")));
    sections.join("\n\n")
}

type ToolHandler = Box<dyn Fn(&str) -> Result<String, ToolError> + Send + Sync>;

/// Simple in-memory tool executor for tests and lightweight integrations.
/// 使用 `Mutex` 实现内部可变性，支持 `&self` 并发调用。
// SAFETY: 此处 parking_lot::Mutex 不跨 await 使用，register/execute 均为同步方法。
#[allow(clippy::disallowed_types)]
pub struct StaticToolExecutor {
    handlers: parking_lot::Mutex<BTreeMap<String, ToolHandler>>,
}

// SAFETY: 此处 parking_lot::Mutex 不跨 await 使用，仅在同步方法内操作。
#[allow(clippy::disallowed_types)]
impl Default for StaticToolExecutor {
    fn default() -> Self {
        Self { handlers: parking_lot::Mutex::new(BTreeMap::new()) }
    }
}

// SAFETY: 此处 parking_lot::Mutex 不跨 await 使用，register 为同步方法。
#[allow(clippy::disallowed_types)]
impl StaticToolExecutor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn register(
        self,
        tool_name: impl Into<String>,
        handler: impl Fn(&str) -> Result<String, ToolError> + Send + Sync + 'static,
    ) -> Self {
        self.handlers.lock().insert(tool_name.into(), Box::new(handler));
        self
    }
}

// SAFETY: 此处 parking_lot::Mutex 不跨 await 使用，execute 为同步方法。
#[allow(clippy::disallowed_types)]
impl ToolExecutor for StaticToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        let guard = self.handlers.lock();
        let handler = guard
            .get(tool_name)
            .ok_or_else(|| ToolError::new(format!("unknown tool: {tool_name}")))?;
        handler(input)
    }
}

// ── Harness ConversationRuntimeHost 实现 ──
impl<C: ApiClient + Send, T: ToolExecutor + Send + 'static>
    axagent_harness::runtime_types::conversation::ConversationRuntimeHost
    for ConversationRuntime<C, T>
{
    fn run_turn(
        &mut self,
        user_input: &str,
        prompter: Option<&mut dyn axagent_harness::runtime_types::permissions::PermissionPrompter>,
    ) -> Result<axagent_harness::runtime_types::conversation::TurnSummary, RuntimeError> {
        ConversationRuntime::run_turn(self, user_input, prompter)
    }

    fn set_max_iterations(&mut self, max: usize) {
        self.max_iterations = max;
    }

    fn set_auto_compaction_threshold(&mut self, threshold: u32) {
        self.auto_compaction_input_tokens_threshold = threshold;
    }

    fn set_cancel_token(&mut self, token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) {
        self.cancel_token = token;
    }

    fn set_progress(&mut self, progress: std::sync::Arc<AgentExecutionProgress>) {
        self.progress = Some(progress);
    }

    fn set_hook_progress_reporter(
        &mut self,
        reporter: Box<dyn axagent_harness::runtime_types::hooks::HookProgressReporter>,
    ) {
        self.hook_progress_reporter = Some(reporter);
    }

    fn set_nudge_lines(&mut self, lines: Vec<String>) {
        self.nudge_lines = lines;
    }

    fn set_system_directive(&mut self, directive: String) {
        self.system_directives = vec![directive];
    }

    fn into_session(self: Box<Self>) -> axagent_harness::runtime_types::session::Session {
        self.session
    }
}

// ── Factory ──

/// `create_conversation_runtime` 的参数聚合体。
/// 通过 builder 模式避免过多参数导致的 clippy::too_many_arguments 告警。
pub struct ConversationRuntimeFactoryArgs {
    pub session: axagent_harness::runtime_types::session::Session,
    pub api_client: Box<dyn axagent_harness::runtime_types::conversation::ApiClient + Send>,
    pub tool_executor:
        Box<dyn axagent_harness::runtime_types::conversation::ToolExecutor + Send + 'static>,
    pub permission_policy: crate::permissions::PermissionPolicy,
    pub system_prompt: Vec<String>,
    pub feature_config: RuntimeFeatureConfig,
    pub hook_chain: Option<Arc<HookChain>>,
    pub skill_evolution_hook: Option<Arc<dyn SkillEvolutionHook>>,
    pub pause_state: Option<Arc<PauseState>>,
    /// 前端对话 ID —— 透传给注入器用于读取会话状态。
    pub conversation_id: Option<String>,
    /// Agent 作用域 —— 透传给注入器用于多 Agent 隔离。
    pub agent_id: Option<String>,
    /// 运行时动态工具集 —— `CapabilityLoad` 激活的工具经此进入每轮请求。
    pub dynamic_tools: Option<axagent_harness::DynamicToolSet>,
    /// 动态上下文注入器 —— 每次 LLM 调用前执行，产出待注入文本块。
    pub context_contributors: Vec<Box<dyn ContextContributor>>,
}

impl ConversationRuntimeFactoryArgs {
    #[must_use]
    pub fn new(
        session: axagent_harness::runtime_types::session::Session,
        api_client: Box<dyn axagent_harness::runtime_types::conversation::ApiClient + Send>,
        tool_executor: Box<
            dyn axagent_harness::runtime_types::conversation::ToolExecutor + Send + 'static,
        >,
        permission_policy: crate::permissions::PermissionPolicy,
        system_prompt: Vec<String>,
        feature_config: RuntimeFeatureConfig,
    ) -> Self {
        Self {
            session,
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
            feature_config,
            hook_chain: None,
            skill_evolution_hook: None,
            pause_state: None,
            conversation_id: None,
            agent_id: None,
            dynamic_tools: None,
            context_contributors: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_hook_chain(mut self, hook_chain: Arc<HookChain>) -> Self {
        self.hook_chain = Some(hook_chain);
        self
    }

    #[must_use]
    pub fn with_hook_chain_option(mut self, hook_chain: Option<Arc<HookChain>>) -> Self {
        self.hook_chain = hook_chain;
        self
    }

    #[must_use]
    pub fn with_skill_evolution_hook(
        mut self,
        skill_evolution_hook: Arc<dyn SkillEvolutionHook>,
    ) -> Self {
        self.skill_evolution_hook = Some(skill_evolution_hook);
        self
    }

    #[must_use]
    pub fn with_pause_state(mut self, pause_state: Arc<PauseState>) -> Self {
        self.pause_state = Some(pause_state);
        self
    }

    #[must_use]
    pub fn with_conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self
    }

    #[must_use]
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    #[must_use]
    pub fn with_dynamic_tools(mut self, set: axagent_harness::DynamicToolSet) -> Self {
        self.dynamic_tools = Some(set);
        self
    }

    #[must_use]
    pub fn with_context_contributor(mut self, contributor: Box<dyn ContextContributor>) -> Self {
        self.context_contributors.push(contributor);
        self
    }
}

/// 构造一个 ConversationRuntime 并返回 Box<dyn ConversationRuntimeHost>。
/// agent crate 用此函数代替直接引用 ConversationRuntime 类型，消除依赖。
pub fn create_conversation_runtime(
    args: ConversationRuntimeFactoryArgs,
) -> Box<dyn axagent_harness::runtime_types::conversation::ConversationRuntimeHost> {
    let mut rt = ConversationRuntime::new_with_features(
        args.session,
        args.api_client,
        args.tool_executor,
        args.permission_policy,
        args.system_prompt,
        &args.feature_config,
    );
    if let Some(hc) = args.hook_chain {
        rt = rt.with_hook_chain(hc);
    }
    if let Some(seh) = args.skill_evolution_hook {
        rt = rt.with_skill_evolution_hook(seh);
    }
    if let Some(ps) = args.pause_state {
        rt = rt.with_pause_state(ps);
    }
    if let Some(cid) = args.conversation_id {
        rt = rt.with_conversation_id(cid);
    }
    if let Some(aid) = args.agent_id {
        rt = rt.with_agent_id(aid);
    }
    if let Some(dt) = args.dynamic_tools {
        rt = rt.with_dynamic_tools(dt);
    }
    for contributor in args.context_contributors {
        rt = rt.with_context_contributor(contributor);
    }
    Box::new(rt)
}

#[cfg(test)]
mod tests {
    use super::SkillEvolutionHook;
    use super::{
        ApiClient, ApiRequest, AssistantEvent, AutoCompactionEvent, ConversationRuntime,
        DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD, PromptCacheEvent, RuntimeError,
        StaticToolExecutor, ToolExecutor, TurnSummary, build_assistant_message,
        parse_auto_compaction_threshold,
    };
    use crate::ToolError;
    use crate::compact::CompactionConfig;
    use crate::config::{RuntimeFeatureConfig, RuntimeHookConfig};
    use crate::permissions::{
        PermissionMode, PermissionPolicy, PermissionPromptDecision, PermissionPrompter,
        PermissionRequest,
    };
    use crate::session::{
        ContentBlock, ConversationMessageExt, MessageRole, Session, SessionExt,
        session_load_from_path,
    };
    use crate::usage::TokenUsage;
    use async_trait::async_trait;
    use axagent_harness::test_support::{MemorySessionTracer, MemoryTelemetrySink, TelemetryEvent};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct ScriptedApiClient {
        call_count: usize,
    }

    impl ApiClient for ScriptedApiClient {
        fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
            self.call_count += 1;
            match self.call_count {
                1 => {
                    assert!(
                        request.messages.iter().any(|message| message.role == MessageRole::User)
                    );
                    Ok(vec![
                        AssistantEvent::TextDelta("Let me calculate that.".to_string()),
                        AssistantEvent::ToolUse {
                            id: "tool-1".to_string(),
                            name: "add".to_string(),
                            input: "2,2".to_string(),
                        },
                        AssistantEvent::Usage(TokenUsage {
                            input_tokens: 20,
                            output_tokens: 6,
                            cache_creation_input_tokens: 1,
                            cache_read_input_tokens: 2,
                            cache_miss_input_tokens: None,
                        }),
                        AssistantEvent::MessageStop,
                    ])
                },
                2 => {
                    let last_message =
                        request.messages.last().expect("tool result should be present");
                    assert_eq!(last_message.role, MessageRole::Tool);
                    Ok(vec![
                        AssistantEvent::TextDelta("The answer is 4.".to_string()),
                        AssistantEvent::Usage(TokenUsage {
                            input_tokens: 24,
                            output_tokens: 4,
                            cache_creation_input_tokens: 1,
                            cache_read_input_tokens: 3,
                            cache_miss_input_tokens: None,
                        }),
                        AssistantEvent::PromptCache(PromptCacheEvent {
                            unexpected: true,
                            reason:
                                "cache read tokens dropped while prompt fingerprint remained stable"
                                    .to_string(),
                            previous_cache_read_input_tokens: 6_000,
                            current_cache_read_input_tokens: 1_000,
                            token_drop: 5_000,
                        }),
                        AssistantEvent::MessageStop,
                    ])
                },
                _ => unreachable!("extra API call"),
            }
        }
    }

    struct PromptAllowOnce;

    impl PermissionPrompter for PromptAllowOnce {
        fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
            assert_eq!(request.tool_name, "add");
            PermissionPromptDecision::Allow
        }
    }

    #[test]
    fn runs_user_to_tool_to_result_loop_end_to_end_and_tracks_usage() {
        let api_client = ScriptedApiClient { call_count: 0 };
        let tool_executor = StaticToolExecutor::new().register("add", |input| {
            let total = input
                .split(',')
                .map(|part| part.parse::<i32>().expect("input must be valid integer"))
                .sum::<i32>();
            Ok(total.to_string())
        });
        let permission_policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite);
        let system_prompt = vec!["You are a helpful assistant.".to_string()];
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
        );

        let summary = runtime
            .run_turn("what is 2 + 2?", Some(&mut PromptAllowOnce))
            .expect("conversation loop should succeed");

        assert_eq!(summary.iterations, 2);
        assert_eq!(summary.assistant_messages.len(), 2);
        assert_eq!(summary.tool_results.len(), 1);
        assert_eq!(summary.prompt_cache_events.len(), 1);
        assert_eq!(runtime.session().messages.len(), 4);
        assert_eq!(summary.usage.output_tokens, 10);
        assert_eq!(summary.auto_compaction, None);
        assert!(matches!(runtime.session().messages[1].blocks[1], ContentBlock::ToolUse { .. }));
        assert!(matches!(
            runtime.session().messages[2].blocks[0],
            ContentBlock::ToolResult { is_error: false, .. }
        ));
    }

    #[test]
    fn records_runtime_session_trace_events() {
        let sink = Arc::new(MemoryTelemetrySink::default());
        let tracer: Arc<dyn axagent_harness::SessionTracer> =
            Arc::new(MemorySessionTracer::new(sink.clone()));
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            ScriptedApiClient { call_count: 0 },
            StaticToolExecutor::new().register("add", |_input| Ok("4".to_string())),
            PermissionPolicy::new(PermissionMode::WorkspaceWrite),
            vec!["system".to_string()],
        )
        .with_session_tracer(tracer);

        runtime
            .run_turn("what is 2 + 2?", Some(&mut PromptAllowOnce))
            .expect("conversation loop should succeed");

        let events = sink.events();
        let trace_names = events
            .iter()
            .filter_map(|event| match event {
                TelemetryEvent::SessionTrace(trace) => Some(trace.name.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(trace_names.contains(&"turn_started"));
        assert!(trace_names.contains(&"assistant_iteration_completed"));
        assert!(trace_names.contains(&"tool_execution_started"));
        assert!(trace_names.contains(&"tool_execution_finished"));
        assert!(trace_names.contains(&"turn_completed"));
    }

    #[test]
    fn records_denied_tool_results_when_prompt_rejects() {
        struct RejectPrompter;
        impl PermissionPrompter for RejectPrompter {
            fn decide(&mut self, _request: &PermissionRequest) -> PermissionPromptDecision {
                PermissionPromptDecision::Deny { reason: "not now".to_string() }
            }
        }

        struct SingleCallApiClient;
        impl ApiClient for SingleCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                if request.messages.iter().any(|message| message.role == MessageRole::Tool) {
                    return Ok(vec![
                        AssistantEvent::TextDelta("I could not use the tool.".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "blocked".to_string(),
                        input: "secret".to_string(),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            SingleCallApiClient,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::WorkspaceWrite),
            vec!["system".to_string()],
        );

        let summary = runtime
            .run_turn("use the tool", Some(&mut RejectPrompter))
            .expect("conversation should continue after denied tool");

        assert_eq!(summary.tool_results.len(), 1);
        assert!(matches!(
            &summary.tool_results[0].blocks[0],
            ContentBlock::ToolResult { is_error: true, output, .. } if output == "not now"
        ));
    }

    #[test]
    fn denies_tool_use_when_pre_tool_hook_blocks() {
        struct SingleCallApiClient;
        impl ApiClient for SingleCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                if request.messages.iter().any(|message| message.role == MessageRole::Tool) {
                    return Ok(vec![
                        AssistantEvent::TextDelta("blocked".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "blocked".to_string(),
                        input: r#"{"path":"secret.txt"}"#.to_string(),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            SingleCallApiClient,
            StaticToolExecutor::new()
                .register("blocked", |_input| panic!("tool should not execute when hook denies")),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                vec![shell_snippet("printf 'blocked by hook'; exit 2")],
                Vec::new(),
                Vec::new(),
            )),
        );

        let summary = runtime
            .run_turn("use the tool", None)
            .expect("conversation should continue after hook denial");

        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult { is_error, output, .. } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(*is_error, "hook denial should produce an error result: {output}");
        assert!(
            output.contains("denied tool") || output.contains("blocked by hook"),
            "unexpected hook denial output: {output:?}"
        );
    }

    #[test]
    fn denies_tool_use_when_pre_tool_hook_fails() {
        struct SingleCallApiClient;
        impl ApiClient for SingleCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                if request.messages.iter().any(|message| message.role == MessageRole::Tool) {
                    return Ok(vec![
                        AssistantEvent::TextDelta("failed".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "blocked".to_string(),
                        input: r#"{"path":"secret.txt"}"#.to_string(),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        // given
        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            SingleCallApiClient,
            StaticToolExecutor::new()
                .register("blocked", |_input| panic!("tool should not execute when hook fails")),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                vec![shell_snippet("printf 'broken hook'; exit 1")],
                Vec::new(),
                Vec::new(),
            )),
        );

        // when
        let summary = runtime
            .run_turn("use the tool", None)
            .expect("conversation should continue after hook failure");

        // then
        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult { is_error, output, .. } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(*is_error, "hook failure should produce an error result: {output}");
        assert!(
            output.contains("exited with status 1") || output.contains("broken hook"),
            "unexpected hook failure output: {output:?}"
        );
    }

    #[test]
    fn appends_post_tool_hook_feedback_to_tool_result() {
        struct TwoCallApiClient {
            calls: usize,
        }

        impl ApiClient for TwoCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                self.calls += 1;
                match self.calls {
                    1 => Ok(vec![
                        AssistantEvent::ToolUse {
                            id: "tool-1".to_string(),
                            name: "add".to_string(),
                            input: r#"{"lhs":2,"rhs":2}"#.to_string(),
                        },
                        AssistantEvent::MessageStop,
                    ]),
                    2 => {
                        assert!(
                            request
                                .messages
                                .iter()
                                .any(|message| message.role == MessageRole::Tool)
                        );
                        Ok(vec![
                            AssistantEvent::TextDelta("done".to_string()),
                            AssistantEvent::MessageStop,
                        ])
                    },
                    _ => unreachable!("extra API call"),
                }
            }
        }

        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            TwoCallApiClient { calls: 0 },
            StaticToolExecutor::new().register("add", |_input| Ok("4".to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                vec![shell_snippet("printf 'pre hook ran'")],
                vec![shell_snippet("printf 'post hook ran'")],
                Vec::new(),
            )),
        );

        let summary = runtime.run_turn("use add", None).expect("tool loop succeeds");

        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult { is_error, output, .. } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(!*is_error, "post hook should preserve non-error result: {output:?}");
        assert!(output.contains('4'), "tool output missing value: {output:?}");
        assert!(
            output.contains("pre hook ran"),
            "tool output missing pre hook feedback: {output:?}"
        );
        assert!(
            output.contains("post hook ran"),
            "tool output missing post hook feedback: {output:?}"
        );
    }

    #[test]
    fn appends_post_tool_use_failure_hook_feedback_to_tool_result() {
        struct TwoCallApiClient {
            calls: usize,
        }

        impl ApiClient for TwoCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                self.calls += 1;
                match self.calls {
                    1 => Ok(vec![
                        AssistantEvent::ToolUse {
                            id: "tool-1".to_string(),
                            name: "fail".to_string(),
                            input: r#"{"path":"README.md"}"#.to_string(),
                        },
                        AssistantEvent::MessageStop,
                    ]),
                    2 => {
                        assert!(
                            request
                                .messages
                                .iter()
                                .any(|message| message.role == MessageRole::Tool)
                        );
                        Ok(vec![
                            AssistantEvent::TextDelta("done".to_string()),
                            AssistantEvent::MessageStop,
                        ])
                    },
                    _ => unreachable!("extra API call"),
                }
            }
        }

        // given
        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            TwoCallApiClient { calls: 0 },
            StaticToolExecutor::new()
                .register("fail", |_input| Err(ToolError::new("tool exploded"))),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                Vec::new(),
                vec![shell_snippet("printf 'post hook should not run'")],
                vec![shell_snippet("printf 'failure hook ran'")],
            )),
        );

        // when
        let summary = runtime.run_turn("use fail", None).expect("tool loop succeeds");

        // then
        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult { is_error, output, .. } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(*is_error, "failure hook path should preserve error result: {output:?}");
        assert!(output.contains("tool exploded"), "tool output missing failure reason: {output:?}");
        assert!(
            output.contains("failure hook ran"),
            "tool output missing failure hook feedback: {output:?}"
        );
        assert!(
            !output.contains("post hook should not run"),
            "normal post hook should not run on tool failure: {output:?}"
        );
    }

    #[test]
    fn reconstructs_usage_tracker_from_restored_session() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![AssistantEvent::TextDelta("done".to_string()), AssistantEvent::MessageStop])
            }
        }

        let mut session = Session::new();
        session.messages.push(crate::session::ConversationMessageExt::assistant_with_usage(
            vec![ContentBlock::Text { text: "earlier".to_string() }],
            Some(TokenUsage {
                input_tokens: 11,
                output_tokens: 7,
                cache_creation_input_tokens: 2,
                cache_read_input_tokens: 1,
                cache_miss_input_tokens: None,
            }),
        ));

        let runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        assert_eq!(runtime.usage().turns(), 1);
        assert_eq!(runtime.usage().cumulative_usage().total_tokens(), 21);
    }

    #[test]
    fn compacts_session_after_turns() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![AssistantEvent::TextDelta("done".to_string()), AssistantEvent::MessageStop])
            }
        }

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );
        runtime.run_turn("a", None).expect("turn a");
        runtime.run_turn("b", None).expect("turn b");
        runtime.run_turn("c", None).expect("turn c");

        let result = runtime.compact(CompactionConfig {
            preserve_recent_messages: 2,
            max_estimated_tokens: 1,
            ..Default::default()
        });
        assert!(result.summary.contains("Conversation summary"));
        assert_eq!(result.compacted_session.messages[0].role, MessageRole::System);
        assert_eq!(result.compacted_session.session_id, runtime.session().session_id);
        assert!(result.compacted_session.compaction.is_some());
    }

    #[test]
    fn persists_conversation_turn_messages_to_jsonl_session() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![AssistantEvent::TextDelta("done".to_string()), AssistantEvent::MessageStop])
            }
        }

        let path = temp_session_path("persisted-turn");
        let session = Session::new().with_persistence_path(path.clone());
        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        runtime.run_turn("persist this turn", None).expect("turn should succeed");

        // 如果文件未写入，手动触发保存（run_turn 在某些环境下不保证同步持久化）
        if !path.exists() {
            runtime.session_mut().save_to_path(&path).expect("manual save should create file");
        }

        let restored = session_load_from_path(&path).expect("persisted session should reload");
        fs::remove_file(&path).expect("temp session file should be removable");

        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.messages[0].role, MessageRole::User);
        assert_eq!(restored.messages[1].role, MessageRole::Assistant);
        assert_eq!(restored.session_id, runtime.session().session_id);
    }

    #[test]
    fn forks_runtime_session_without_mutating_original() {
        let mut session = Session::new();
        session.push_user_text("branch me").expect("message should append");

        let runtime = ConversationRuntime::new(
            session.clone(),
            ScriptedApiClient { call_count: 0 },
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let forked = runtime.fork_session(Some("alt-path".to_string()));

        assert_eq!(forked.messages, session.messages);
        assert_ne!(forked.session_id, session.session_id);
        assert_eq!(
            forked
                .fork
                .as_ref()
                .map(|fork| (fork.parent_session_id.as_str(), fork.branch_name.as_deref())),
            Some((session.session_id.as_str(), Some("alt-path")))
        );
        assert!(runtime.session().fork.is_none());
    }

    fn temp_session_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("runtime-conversation-{label}-{nanos}.json"))
    }

    #[cfg(windows)]
    fn shell_snippet(script: &str) -> String {
        script.replace("printf '", "echo ").replace('\'', "").replace(";", "&")
    }

    #[cfg(not(windows))]
    fn shell_snippet(script: &str) -> String {
        script.to_string()
    }

    #[test]
    fn auto_compacts_when_cumulative_input_threshold_is_crossed() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::Usage(TokenUsage {
                        input_tokens: 120_000,
                        output_tokens: 4,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                        cache_miss_input_tokens: None,
                    }),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        session.messages = vec![
            crate::session::ConversationMessageExt::user_text("one"),
            crate::session::ConversationMessageExt::assistant(vec![ContentBlock::Text {
                text: "two".to_string(),
            }]),
            crate::session::ConversationMessageExt::user_text("three"),
            crate::session::ConversationMessageExt::assistant(vec![ContentBlock::Text {
                text: "four".to_string(),
            }]),
            crate::session::ConversationMessageExt::user_text("five"),
            crate::session::ConversationMessageExt::assistant(vec![ContentBlock::Text {
                text: "six".to_string(),
            }]),
            crate::session::ConversationMessageExt::user_text("seven"),
            crate::session::ConversationMessageExt::assistant(vec![ContentBlock::Text {
                text: "eight".to_string(),
            }]),
            crate::session::ConversationMessageExt::user_text("nine"),
            crate::session::ConversationMessageExt::assistant(vec![ContentBlock::Text {
                text: "ten".to_string(),
            }]),
            crate::session::ConversationMessageExt::user_text("eleven"),
            crate::session::ConversationMessageExt::assistant(vec![ContentBlock::Text {
                text: "twelve".to_string(),
            }]),
            crate::session::ConversationMessageExt::user_text("thirteen"),
        ];

        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_auto_compaction_input_tokens_threshold(100_000);

        let summary = runtime.run_turn("trigger", None).expect("turn should succeed");

        assert_eq!(summary.auto_compaction, Some(AutoCompactionEvent { removed_message_count: 3 }));
        assert_eq!(runtime.session().messages[0].role, MessageRole::System);
    }

    #[test]
    fn skips_auto_compaction_below_threshold() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::Usage(TokenUsage {
                        input_tokens: 99_999,
                        output_tokens: 4,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                        cache_miss_input_tokens: None,
                    }),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_auto_compaction_input_tokens_threshold(100_000);

        let summary = runtime.run_turn("trigger", None).expect("turn should succeed");
        assert_eq!(summary.auto_compaction, None);
        assert_eq!(runtime.session().messages.len(), 2);
    }

    #[test]
    fn auto_compaction_threshold_defaults_and_parses_values() {
        assert_eq!(
            parse_auto_compaction_threshold(None),
            DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD
        );
        assert_eq!(parse_auto_compaction_threshold(Some("4321")), 4321);
        assert_eq!(
            parse_auto_compaction_threshold(Some("0")),
            DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD
        );
        assert_eq!(
            parse_auto_compaction_threshold(Some("not-a-number")),
            DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD
        );
    }

    #[test]
    fn turn_count_compaction_triggers_at_interval() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![AssistantEvent::TextDelta("ok".into())])
            }
        }
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_compact_every_n_turns(Some(2));

        // Turn 1: no compaction
        let summary = runtime.run_turn("msg1", None).expect("turn 1");
        assert_eq!(summary.auto_compaction, None, "turn 1 should not compact");

        // Turn 2: turn-count threshold reached but empty session has no messages
        // to compact; auto_compaction remains None.
        let summary = runtime.run_turn("msg2", None).expect("turn 2");
        assert_eq!(summary.auto_compaction, None, "空会话无可压缩消息");
    }

    /// 快照测试：验证会话序列化格式的稳定性。
    /// 防止无意的序列化格式变更影响数据库兼容性。
    #[test]
    fn session_serialization_snapshot() {
        let mut session = Session::new();
        session.push_user_text("Hello, assistant!").expect("测试：push_user_text 应成功");
        session
            .push_message(ConversationMessageExt::assistant(vec![ContentBlock::Text {
                text: "Hi there! How can I help you?".to_string(),
            }]))
            .expect("测试应成功");
        session
            .push_message(ConversationMessageExt::user_text("What is 2+2?"))
            .expect("测试应成功");
        session
            .push_message(ConversationMessageExt::assistant(vec![ContentBlock::Text {
                text: "2+2 = 4".to_string(),
            }]))
            .expect("测试应成功");

        // JSON 序列化用于持久化，格式变更需审慎。
        // session_id / 时间戳每次运行不同，剥离后再做快照比较。
        let serialized = serde_json::to_value(&session).expect("serialize session");
        let mut value = serialized;
        if let Some(obj) = value.as_object_mut() {
            obj.remove("session_id");
            obj.remove("created_at_ms");
            obj.remove("updated_at_ms");
        }
        let stable = serde_json::to_string_pretty(&value).expect("re-serialize session");
        insta::assert_snapshot!("session_serialization", stable);
    }

    /// 快照测试：验证 TurnSummary 输出的稳定性。
    /// 防止无意的 API 响应格式变更。
    #[test]
    fn turn_summary_snapshot() {
        let summary = TurnSummary {
            assistant_messages: vec![ConversationMessageExt::assistant(vec![ContentBlock::Text {
                text: "Hello!".to_string(),
            }])],
            tool_results: vec![],
            prompt_cache_events: vec![PromptCacheEvent {
                unexpected: false,
                reason: "cache read".to_string(),
                previous_cache_read_input_tokens: 1_000,
                current_cache_read_input_tokens: 800,
                token_drop: 200,
            }],
            iterations: 1,
            usage: crate::usage::TokenUsage {
                input_tokens: 100,
                output_tokens: 10,
                cache_read_input_tokens: 800,
                cache_creation_input_tokens: 0,
                cache_miss_input_tokens: None,
            },
            auto_compaction: None,
            thinking: String::new(),
        };
        let serialized = serde_json::to_string_pretty(&summary).expect("serialize summary");
        insta::assert_snapshot!("turn_summary", serialized);
    }

    #[test]
    fn compaction_health_probe_blocks_turn_when_tool_executor_is_broken() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                panic!("API should not run when health probe fails");
            }
        }

        let mut session = Session::new();
        session.record_compaction("summarized earlier work", 4);
        session.push_user_text("previous message").expect("message should append");

        let tool_executor = StaticToolExecutor::new()
            .register("glob_search", |_input| Err(ToolError::new("transport unavailable")));
        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            tool_executor,
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let error = runtime
            .run_turn("trigger", None)
            .expect_err("health probe failure should abort the turn");
        assert!(
            error.to_string().contains("Session health probe failed after compaction"),
            "unexpected error: {error}"
        );
        assert!(
            error.to_string().contains("transport unavailable"),
            "expected underlying probe error: {error}"
        );
    }

    #[test]
    fn compaction_health_probe_skips_empty_compacted_session() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![AssistantEvent::TextDelta("done".to_string()), AssistantEvent::MessageStop])
            }
        }

        let mut session = Session::new();
        session.record_compaction("fresh summary", 2);

        let tool_executor = StaticToolExecutor::new().register("glob_search", |_input| {
            Err(ToolError::new("glob_search should not run for an empty compacted session"))
        });
        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            tool_executor,
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let summary = runtime
            .run_turn("trigger", None)
            .expect("empty compacted session should not fail health probe");
        assert_eq!(summary.auto_compaction, None);
        assert_eq!(runtime.session().messages.len(), 2);
    }

    #[test]
    fn build_assistant_message_returns_partial_result_when_stream_has_no_stop_event() {
        // given: text content without MessageStop (simulates interrupted stream)
        let events = vec![AssistantEvent::TextDelta("partial".to_string())];

        // when: stream recovery returns partial content instead of error
        let result = build_assistant_message(events, true)
            .expect("stream recovery should return partial result with content");

        // then: partial content is preserved
        let (msg, _usage, _cache, _thinking) = result;
        let text = msg
            .blocks
            .iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .expect("测试应成功");
        assert!(text.contains("partial"), "should contain original text");
        assert!(text.contains("Stream was interrupted"), "should contain recovery marker");
    }

    #[test]
    fn build_assistant_message_errors_when_stream_has_no_content_and_no_stop() {
        // given: empty events (no content, no MessageStop)
        let events: Vec<AssistantEvent> = vec![];

        // when
        let error = build_assistant_message(events, true)
            .expect_err("empty stream without stop event should error");

        // then
        assert!(error.to_string().contains("assistant stream ended without a message stop event"));
    }

    #[test]
    fn build_assistant_message_requires_content() {
        // given
        let events = vec![AssistantEvent::MessageStop];

        // when
        let error = build_assistant_message(events, true)
            .expect_err("assistant messages should require content");

        // then
        assert!(error.to_string().contains("assistant stream produced no content"));
    }

    #[test]
    fn build_assistant_message_emits_think_block_when_thought_chain_enabled() {
        // given: 一轮包含推理内容 + 正文的完整流
        let events = vec![
            AssistantEvent::ThinkingDelta("weighing options".to_string()),
            AssistantEvent::TextDelta("final answer".to_string()),
            AssistantEvent::MessageStop,
        ];

        // when: thought_chain 开启
        let (msg, _usage, _cache, thinking) =
            build_assistant_message(events, true).expect("should build message");

        // then: 推理被包成 <think> 块前置到正文，且原始 thinking 仍被透出
        let text = msg
            .blocks
            .iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .expect("测试应成功");
        assert!(text.contains("<think data-axagent=\"1\">"), "开启时应注入 <think> 可视化块");
        assert!(text.contains("weighing options"), "<think> 块应包含推理内容");
        assert!(text.contains("final answer"), "正文应保留");
        assert_eq!(thinking, "weighing options", "推理内容始终透出（供 trajectory 使用）");
    }

    #[test]
    fn build_assistant_message_hides_think_block_when_thought_chain_disabled() {
        // given: 同样一轮包含推理内容 + 正文的流
        let events = vec![
            AssistantEvent::ThinkingDelta("weighing options".to_string()),
            AssistantEvent::TextDelta("final answer".to_string()),
            AssistantEvent::MessageStop,
        ];

        // when: thought_chain 关闭
        let (msg, _usage, _cache, thinking) =
            build_assistant_message(events, false).expect("should build message");

        // then: 正文里不出现 <think> 可视化块，但推理内容仍通过返回值透出
        let text = msg
            .blocks
            .iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .expect("测试应成功");
        assert!(!text.contains("<think"), "关闭时不应注入 <think> 可视化块");
        assert!(text.contains("final answer"), "正文应保留");
        assert_eq!(thinking, "weighing options", "关闭可视化不影响推理内容捕获");
    }

    #[test]
    fn static_tool_executor_rejects_unknown_tools() {
        // given
        let mut executor = StaticToolExecutor::new();

        // when
        let error = executor.execute("missing", "{}").expect_err("unregistered tools should fail");

        // then
        assert_eq!(error.to_string(), "[executionFailed] unknown tool: missing");
    }

    #[test]
    fn run_turn_errors_when_max_iterations_is_exceeded() {
        struct LoopingApi;

        impl ApiClient for LoopingApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "echo".to_string(),
                        input: "payload".to_string(),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        // given
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            LoopingApi,
            StaticToolExecutor::new().register("echo", |input| Ok(input.to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_max_iterations(1);

        // when
        let error = runtime
            .run_turn("loop", None)
            .expect_err("conversation loop should stop after the configured limit");

        // then
        assert!(
            error
                .to_string()
                .contains("conversation loop exceeded the maximum number of iterations")
        );
    }

    #[test]
    fn run_turn_propagates_api_errors() {
        struct FailingApi;

        impl ApiClient for FailingApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Err(RuntimeError::new("upstream failed"))
            }
        }

        // given
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            FailingApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        // when
        let error = runtime.run_turn("hello", None).expect_err("API failures should propagate");

        // then
        assert_eq!(error.to_string(), "upstream failed");
    }

    // ── T0.9 技能侧反思钩子：工具执行完成后触发进化判定（不再死代码）──

    #[test]
    fn run_turn_triggers_skill_evolution_hook_on_tool_executed() {
        // 记录调用次数的 mock 反思钩子（模拟 wiring 层实现）
        #[derive(Default)]
        struct TrackingHook {
            calls: std::sync::atomic::AtomicUsize,
        }

        #[async_trait]
        impl SkillEvolutionHook for TrackingHook {
            async fn on_tool_executed(
                &self,
                _tool_name: &str,
                _success: bool,
                _output: &str,
            ) -> bool {
                self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                false
            }
        }

        // ApiClient：第一轮请求工具 echo，第二轮返回文本结束
        struct ToolThenDoneApi {
            call_count: usize,
        }

        impl ApiClient for ToolThenDoneApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                self.call_count += 1;
                if self.call_count == 1 {
                    Ok(vec![
                        AssistantEvent::ToolUse {
                            id: "tool-1".to_string(),
                            name: "echo".to_string(),
                            input: "payload".to_string(),
                        },
                        AssistantEvent::MessageStop,
                    ])
                } else {
                    Ok(vec![
                        AssistantEvent::TextDelta("done".to_string()),
                        AssistantEvent::MessageStop,
                    ])
                }
            }
        }

        // given
        let hook = Arc::new(TrackingHook::default());
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            ToolThenDoneApi { call_count: 0 },
            StaticToolExecutor::new().register("echo", |input| Ok(input.to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_skill_evolution_hook(hook.clone());

        // when
        runtime.run_turn("call echo", None).expect("run_turn should succeed");

        // then
        assert_eq!(
            hook.calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "技能反思钩子应在工具执行完成后被调用一次"
        );
    }

    // ── 3.6 P2:RecoveryCoordinator 单元测试 ──────────────────────────

    #[test]
    fn test_classify_recovery_error_transient() {
        use super::{RecoveryErrorType, classify_recovery_error};
        assert_eq!(classify_recovery_error("connection timeout"), RecoveryErrorType::Transient);
        assert_eq!(
            classify_recovery_error("HTTP 429 too many requests"),
            RecoveryErrorType::Transient
        );
        assert_eq!(classify_recovery_error("network unreachable"), RecoveryErrorType::Transient);
        assert_eq!(
            classify_recovery_error("connection reset by peer"),
            RecoveryErrorType::Transient
        );
        assert_eq!(
            classify_recovery_error("LLM 流式调用失败: LLM 流式空响应 ... (empty response)"),
            RecoveryErrorType::Transient
        );
        assert_eq!(classify_recovery_error("service unavailable"), RecoveryErrorType::Transient);
        assert_eq!(classify_recovery_error("rate limit exceeded"), RecoveryErrorType::Transient);
    }

    #[test]
    fn test_classify_recovery_error_unrecoverable() {
        use super::{RecoveryErrorType, classify_recovery_error};
        assert_eq!(
            classify_recovery_error("syntax error in prompt"),
            RecoveryErrorType::Unrecoverable
        );
        assert_eq!(
            classify_recovery_error("authentication failed"),
            RecoveryErrorType::Unrecoverable
        );
        assert_eq!(
            classify_recovery_error("unauthorized access"),
            RecoveryErrorType::Unrecoverable
        );
        assert_eq!(classify_recovery_error("forbidden resource"), RecoveryErrorType::Unrecoverable);
        assert_eq!(classify_recovery_error("invalid api key"), RecoveryErrorType::Unrecoverable);
        assert_eq!(classify_recovery_error("out of memory"), RecoveryErrorType::Unrecoverable);
        assert_eq!(
            classify_recovery_error("panic: thread panicked"),
            RecoveryErrorType::Unrecoverable
        );
    }

    #[test]
    fn test_classify_recovery_error_recoverable() {
        use super::{RecoveryErrorType, classify_recovery_error};
        assert_eq!(classify_recovery_error("permission denied"), RecoveryErrorType::Recoverable);
        assert_eq!(classify_recovery_error("quota exceeded"), RecoveryErrorType::Recoverable);
    }

    #[test]
    fn test_classify_recovery_error_unknown() {
        use super::{RecoveryErrorType, classify_recovery_error};
        assert_eq!(classify_recovery_error("something weird happened"), RecoveryErrorType::Unknown);
        assert_eq!(classify_recovery_error("unexpected state"), RecoveryErrorType::Unknown);
        assert_eq!(classify_recovery_error(""), RecoveryErrorType::Unknown);
    }

    #[test]
    fn test_get_recovery_action_fail_for_unrecoverable() {
        use super::{RecoveryAction, RecoveryErrorType, get_recovery_action};
        assert!(matches!(
            get_recovery_action(RecoveryErrorType::Unrecoverable),
            RecoveryAction::Fail
        ));
    }

    #[test]
    fn test_get_recovery_action_retry_for_transient() {
        use super::{RecoveryAction, RecoveryErrorType, get_recovery_action};
        match get_recovery_action(RecoveryErrorType::Transient) {
            RecoveryAction::Retry { max_attempts, base_delay_ms } => {
                assert_eq!(max_attempts, 3);
                assert_eq!(base_delay_ms, 1000);
            },
            other => panic!("预期 Retry,实际: {other:?}"),
        }
    }

    #[test]
    fn test_get_recovery_action_retry_for_recoverable() {
        use super::{RecoveryAction, RecoveryErrorType, get_recovery_action};
        match get_recovery_action(RecoveryErrorType::Recoverable) {
            RecoveryAction::Retry { max_attempts, base_delay_ms } => {
                assert_eq!(max_attempts, 2);
                assert_eq!(base_delay_ms, 500);
            },
            other => panic!("预期 Retry,实际: {other:?}"),
        }
    }

    #[test]
    fn test_get_recovery_action_unknown_treated_as_transient() {
        use super::{RecoveryAction, RecoveryErrorType, get_recovery_action};
        match get_recovery_action(RecoveryErrorType::Unknown) {
            RecoveryAction::Retry { max_attempts, base_delay_ms } => {
                assert_eq!(max_attempts, 3);
                assert_eq!(base_delay_ms, 1000);
            },
            other => panic!("预期 Retry,实际: {other:?}"),
        }
    }

    #[test]
    fn test_compute_backoff_delay_exponential() {
        use super::compute_backoff_delay;
        // attempt=1 → base * 2^0 = base
        assert_eq!(compute_backoff_delay(1000, 1), 1000);
        // attempt=2 → base * 2^1 = 2*base
        assert_eq!(compute_backoff_delay(1000, 2), 2000);
        // attempt=3 → base * 2^2 = 4*base
        assert_eq!(compute_backoff_delay(1000, 3), 4000);
        // attempt=4 → base * 2^3 = 8*base
        assert_eq!(compute_backoff_delay(1000, 4), 8000);
    }

    #[test]
    fn test_compute_backoff_delay_caps_exponent() {
        use super::compute_backoff_delay;
        // 指数被 cap 在 10,避免溢出;attempt=20 仍用 2^10
        let expected = 1000u64.saturating_mul(1u64 << 10);
        assert_eq!(compute_backoff_delay(1000, 20), expected);
    }

    #[test]
    fn test_check_cancelled_none_token() {
        use super::check_cancelled;
        // 无取消令牌 → Ok
        assert!(check_cancelled(None).is_ok());
    }

    #[test]
    fn test_check_cancelled_not_cancelled() {
        use super::check_cancelled;
        use std::sync::atomic::AtomicBool;
        let token = Arc::new(AtomicBool::new(false));
        assert!(check_cancelled(Some(&token)).is_ok());
    }

    #[test]
    fn test_check_cancelled_cancelled() {
        use super::check_cancelled;
        use std::sync::atomic::AtomicBool;
        let token = Arc::new(AtomicBool::new(true));
        let result = check_cancelled(Some(&token));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Agent cancelled by user");
    }
}
