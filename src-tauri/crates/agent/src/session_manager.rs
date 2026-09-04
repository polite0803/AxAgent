// SPDX-License-Identifier: AGPL-3.0-only

//! Session Manager for AxAgent Agent

use crate::event_bus::AgentPermissionPayload;
use crate::shared_blackboard::SharedBlackboard;
use axagent_harness::AgentSessionRepository;
use axagent_harness::ConversationMessage;
use axagent_harness::compact_session::compact_session;
use axagent_harness::conversation_model::{
    ContentBlock as HarnessContentBlock, ConversationMessage as HarnessConversationMessage,
    TokenUsage as HarnessTokenUsage,
};
use axagent_harness::prompt_provider::NoopPromptProvider;
use axagent_harness::runtime_types::compact::CompactionConfig;
use axagent_harness::runtime_types::compact::should_compact;
use axagent_harness::runtime_types::conversation::RuntimeError;
use axagent_harness::runtime_types::conversation::{ConversationRuntimeHost, TurnSummary};
use axagent_harness::runtime_types::execution_progress::AgentExecutionProgress;
use axagent_harness::runtime_types::hooks::{HookEvent, HookProgressEvent, HookProgressReporter};
use axagent_harness::runtime_types::permissions::{
    PermissionMode, PermissionPolicy, PermissionPromptDecision, PermissionPrompter,
    PermissionRequest,
};
use axagent_harness::runtime_types::session::Session;
use axagent_harness::{TaskComplexity, TrajectoryService};

const NP: &NoopPromptProvider = &NoopPromptProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, RwLock};
use tracing::info;

const AUTO_COMPACTION_TOKEN_THRESHOLD: usize = 100_000;

const TOKEN_ESTIMATION_CHARS_PER_TOKEN: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageBreakdown {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub estimated_from_chars: bool,
}

impl TokenUsageBreakdown {
    pub fn from_turn_summary(usage: &HarnessTokenUsage, estimated_chars: usize) -> Self {
        let total = usage.total_tokens();
        let estimated_from_chars = total == 0 && estimated_chars > 0;
        Self {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: if total > 0 {
                total
            } else {
                (estimated_chars / TOKEN_ESTIMATION_CHARS_PER_TOKEN) as u32
            },
            estimated_from_chars,
        }
    }

    pub fn tokens_delta(&self) -> i64 {
        self.total_tokens as i64
    }
}

pub fn estimate_tokens_from_text(text: &str) -> usize {
    text.len() / TOKEN_ESTIMATION_CHARS_PER_TOKEN
}

pub fn estimate_tokens_from_messages(messages: &[HarnessConversationMessage]) -> usize {
    messages.iter().map(|m| estimate_tokens_from_content_blocks(&m.blocks)).sum()
}

fn estimate_tokens_from_content_blocks(blocks: &[HarnessContentBlock]) -> usize {
    blocks
        .iter()
        .map(|block| match block {
            HarnessContentBlock::Text { text } => estimate_tokens_from_text(text),
            HarnessContentBlock::ToolUse { id, name, input } => {
                estimate_tokens_from_text(id)
                    + estimate_tokens_from_text(name)
                    + estimate_tokens_from_text(input)
            },
            HarnessContentBlock::ToolResult { tool_use_id, tool_name, output, .. } => {
                estimate_tokens_from_text(tool_use_id)
                    + estimate_tokens_from_text(tool_name)
                    + estimate_tokens_from_text(output)
            },
        })
        .sum()
}

// ---------------------------------------------------------------------------
// P4-4: Dynamic max_iterations based on task complexity
// ---------------------------------------------------------------------------

/// Calculate the maximum number of agent loop iterations based on task complexity.
///
/// | Complexity | max_iterations | Rationale |
/// |------------|---------------|-----------|
/// | Low        | 20            | Simple queries need few tool-use rounds |
/// | Medium     | 50            | Standard tasks with moderate tool usage |
/// | High       | 100           | Complex multi-step tasks need more iterations |
pub fn dynamic_max_iterations(complexity: &TaskComplexity) -> usize {
    match complexity {
        TaskComplexity::Low => 20,
        TaskComplexity::Medium => 50,
        TaskComplexity::High => 100,
    }
}

/// 处理并追加用户消息到 agent 会话（带提示词注入防护）
///
/// 使用 Session 中注入的 PromptGuard 对用户输入进行多层过滤，
/// 直接推送已处理的消息到 Session，避免 push_user_text 的二次包装。
pub fn append_user_message(session: &mut Session, text: &str) -> Result<(), String> {
    let processed = match session.prompt_guard.as_ref() {
        Some(guard) => guard.process_user_input(text)?,
        None => text.to_string(),
    };
    // 直接创建 harness ConversationMessage 并推送（无需 serde 转换）
    session
        .push_message(ConversationMessage {
            role: axagent_harness::conversation_model::MessageRole::User,
            blocks: vec![axagent_harness::ContentBlock::Text { text: processed }],
            usage: None,
        })
        .map_err(|e| e.to_string())
}

/// Agent Session wrapper
#[derive(Debug, Clone)]
pub struct AgentSession {
    session: Session,
    provider_id: String,
    conversation_id: String,
    team_id: Option<String>,
    role: Option<String>,
    axagent_session_id: Option<String>,
    /// 多 Agent 协作 Blackboard（可选）
    pub blackboard: Option<Arc<RwLock<SharedBlackboard>>>,
    /// 轨迹学习服务（可选，用于会话压缩完整性校验）
    pub trajectory: Option<Arc<dyn TrajectoryService>>,
}

impl AgentSession {
    pub fn new(provider_id: String, conversation_id: String) -> Self {
        Self {
            session: Session::new(),
            provider_id,
            conversation_id,
            team_id: None,
            role: None,
            axagent_session_id: None,
            blackboard: None,
            trajectory: None,
        }
    }

    pub fn with_team(mut self, team_id: String) -> Self {
        self.team_id = Some(team_id);
        self
    }

    /// 注入轨迹学习服务（用于压缩完整性校验 + 复杂度评估）
    #[must_use]
    pub fn with_trajectory_service(mut self, svc: Arc<dyn TrajectoryService>) -> Self {
        self.trajectory = Some(svc);
        self
    }

    pub fn with_role(mut self, role: String) -> Self {
        self.role = Some(role);
        self
    }

    pub fn with_axagent_session_id(mut self, axagent_session_id: String) -> Self {
        self.axagent_session_id = Some(axagent_session_id);
        self
    }

    pub fn with_blackboard(mut self, bb: Arc<RwLock<SharedBlackboard>>) -> Self {
        self.blackboard = Some(bb);
        self
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    pub fn team_id(&self) -> Option<&str> {
        self.team_id.as_deref()
    }

    pub fn role(&self) -> Option<&str> {
        self.role.as_deref()
    }

    pub fn axagent_session_id(&self) -> Option<&str> {
        self.axagent_session_id.as_deref()
    }

    /// 记录此 Agent 的决策到 Blackboard
    pub async fn record_to_blackboard(&self, field: &str, value: &str) {
        if let Some(ref bb) = self.blackboard {
            let mut board = bb.write().await;
            board.record_decision(
                &self.axagent_session_id.clone().unwrap_or_default(),
                &self.conversation_id,
                field,
                value,
            );
        }
    }
}

/// Session Manager
pub struct SessionManager {
    sessions: Mutex<std::collections::HashMap<String, AgentSession>>,
    /// Reverse index: conversation_id → runtime session_id
    conversation_index: Mutex<std::collections::HashMap<String, String>>,
    /// Tracks last access time for each session_id (epoch millis)
    session_last_access: Mutex<std::collections::HashMap<String, u64>>,
    app_handle: tokio::sync::Mutex<Option<AppHandle>>,
    default_workspace_dir: tokio::sync::Mutex<Option<String>>,
    /// Per-conversation execution progress trackers for frontend panels.
    progress_trackers:
        tokio::sync::RwLock<std::collections::HashMap<String, Arc<AgentExecutionProgress>>>,
    /// 轨迹学习服务（可选，用于压缩完整性校验和任务复杂度估算）
    trajectory: Option<Arc<dyn TrajectoryService>>,
    agent_session_repo: Arc<dyn AgentSessionRepository>,
    /// 统一事件总线（可选，由 wiring 层注入）。
    ///
    /// 注入后,SessionManager 在 turn 开始 / 结束等关键节点额外 publish
    /// `DomainEvent` 到统一总线,供跨 crate 订阅者消费。
    /// 未注入时保持原有行为,不影响现有功能。
    /// 用 RwLock 包裹以支持在 `Arc<SessionManager>` 上运行时注入。
    event_bus: tokio::sync::RwLock<Option<Arc<dyn axagent_harness::EventBus>>>,
    /// session_events 事件持久化 sink（PLAN-codex-parity P0-3）。
    ///
    /// 注入后,SessionManager 在 turn 开始 / 结束时同时发事件到 session_events 表,
    /// 为 agent_resume_from_events 提供事件流。未注入时保持零开销。
    /// 由 wiring 层(src/init/state.rs)构造 DbSessionEventSink 后注入。
    session_event_sink: tokio::sync::RwLock<Option<Arc<dyn axagent_harness::SessionEventSink>>>,
    /// 可选反思器。注入后每个 turn 完成时自动 spawn 复盘任务(不阻塞主流程)。
    ///
    /// 未注入时保持原行为(零调用 Reflector::reflect())。
    /// 通常由 wiring 层(src/init/state.rs)在构造 AppState 时调用
    /// `set_reflector()` 注入。
    reflector: tokio::sync::RwLock<Option<Arc<crate::reflector::Reflector>>>,
    /// 自改进循环开关(由 wiring 层从 DB 读取前端 FeatureFlag 后注入)。
    ///
    /// - `final_output_reflection=true`:turn 完成后同步等待 Reflector 评估完成
    ///   (而非 fire-and-forget),使评估结果可立即用于质量门判定。
    /// - `self_improvement_enabled=true` 且质量不达标:把改进建议写入 session
    ///   的 nudge_lines,在下次 turn 中作为隐式提示注入 LLM。
    ///
    /// 未注入时保持原行为(异步 fire-and-forget 复盘)。
    self_improvement_flags: tokio::sync::RwLock<SelfImprovementFlags>,
    /// Per-session 运行时取消信号。
    ///
    /// 每个 session 在 create_session 时注册一个独立 Arc<AtomicBool>，
    /// HarnessAgentAdapter.execute → ReActEngine::run 检查这个 flag。
    /// cancel_session(id) 先 store(true) 唤醒 run 循环，再清理内存 HashMap。
    cancel_tokens: Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>,
}

/// 自改进循环相关开关(对应前端 FeatureFlag)。
///
/// 由 wiring 层(`src/init/state.rs`)通过 `read_self_improvement_flags()`
/// 从 DB 读取前端 `app_config.features` 后注入到 `SessionManager`。
#[derive(Debug, Clone, Copy, Default)]
pub struct SelfImprovementFlags {
    /// 是否启用自改进循环(对应 `features.selfImprovingLoop`)
    pub self_improvement_enabled: bool,
    /// 是否对最终输出做质量反射检查(对应 `features.finalOutputReflection`)
    pub final_output_reflection: bool,
}

/// Maximum number of sessions to keep in memory (LRU eviction).
const MAX_CACHED_SESSIONS: usize = 100;
/// Time-to-live for idle sessions (24 hours in seconds).
const SESSION_TTL_SECS: u64 = 24 * 60 * 60;

impl SessionManager {
    pub fn new(agent_session_repo: Arc<dyn AgentSessionRepository>) -> Self {
        Self {
            sessions: Mutex::new(std::collections::HashMap::new()),
            conversation_index: Mutex::new(std::collections::HashMap::new()),
            session_last_access: Mutex::new(std::collections::HashMap::new()),
            app_handle: tokio::sync::Mutex::new(None),
            default_workspace_dir: tokio::sync::Mutex::new(None),
            progress_trackers: tokio::sync::RwLock::new(std::collections::HashMap::new()),
            trajectory: None,
            agent_session_repo,
            event_bus: tokio::sync::RwLock::new(None),
            session_event_sink: tokio::sync::RwLock::new(None),
            reflector: tokio::sync::RwLock::new(None),
            self_improvement_flags: tokio::sync::RwLock::new(SelfImprovementFlags::default()),
            cancel_tokens: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// 注入统一事件总线,启用跨 crate 事件桥接。
    ///
    /// 注入后,SessionManager 在 turn 开始 / 结束等关键节点自动 publish
    /// `DomainEvent` 到统一总线。未注入时保持原有行为。
    /// 通常由 wiring 层(src/init/state.rs)在构造 AppState 时调用。
    pub async fn set_event_bus(&self, bus: Arc<dyn axagent_harness::EventBus>) {
        let mut guard = self.event_bus.write().await;
        *guard = Some(bus);
    }

    /// 注入 session_events 事件持久化 sink（P0-3）。
    ///
    /// 注入后,SessionManager 在 TurnStarted/TurnCompleted 时同时发事件到
    /// session_events 表,供 agent_resume_from_events 读取。
    pub async fn set_session_event_sink(&self, sink: Arc<dyn axagent_harness::SessionEventSink>) {
        let mut guard = self.session_event_sink.write().await;
        *guard = Some(sink);
    }

    /// 注入反思器。启用后每个 turn 完成时自动 spawn 复盘任务(不阻塞主流程)。
    ///
    /// 未注入时保持原行为(`Reflector::reflect()` 零调用)。
    /// 通常由 wiring 层(src/init/state.rs)在构造 AppState 时调用。
    pub async fn set_reflector(&self, reflector: Arc<crate::reflector::Reflector>) {
        let mut guard = self.reflector.write().await;
        *guard = Some(reflector);
    }

    /// 注入自改进循环开关(由 wiring 层从 DB 读取前端 FeatureFlag 后调用)。
    ///
    /// 注入后,`run_turn_with_tools` 的复盘环节会根据 flag 决定:
    /// - `final_output_reflection=true`:同步等待 Reflector 评估完成
    /// - `self_improvement_enabled=true` 且质量不达标:把改进建议写入 nudge_lines
    ///
    /// 未注入时保持原行为(异步 fire-and-forget 复盘)。
    pub async fn set_self_improvement_flags(&self, flags: SelfImprovementFlags) {
        let mut guard = self.self_improvement_flags.write().await;
        *guard = flags;
    }

    /// 发布一个 agent 领域事件到统一总线(若已注入),同时落 session_events 表(若已注入)。
    ///
    /// 未注入时静默返回,不影响原有逻辑。
    /// `kind` 对应 `AgentEventType::to_string()`(如 `"TurnStarted"`)。
    async fn publish_agent_event(&self, kind: &str, payload: serde_json::Value) {
        let bus_clone = {
            let guard = self.event_bus.read().await;
            guard.as_ref().map(Arc::clone)
        };
        if let Some(bus) = bus_clone {
            let event = axagent_harness::DomainEvent::new(
                axagent_harness::EventCategory::Agent,
                kind,
                payload.clone(),
                "agent",
            );
            bus.publish(event).await;
        }

        // P0-3: 同时落 session_events 事件表（若已注入 sink）。
        // 映射: "TurnStarted" → SessionEventType::TurnStarted
        //       "TurnCompleted" → SessionEventType::TurnEnded
        if let Some(sink) = self.session_event_sink.read().await.as_ref().map(Arc::clone) {
            let mapped = match kind {
                "TurnStarted" => axagent_harness::SessionEventType::TurnStarted,
                "TurnCompleted" | "TurnEnded" => axagent_harness::SessionEventType::TurnEnded,
                _ => return,
            };
            // 从 payload 里取 conversationId 作为 session_id
            let session_id = payload
                .get("conversationId")
                .and_then(|v| v.as_str())
                .or_else(|| payload.get("sessionId").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            sink.emit(&session_id, mapped, Some(payload)).await;
        }
    }

    /// Test-only constructor: accepts a pre-constructed `AgentSessionRepository`.
    /// 仅在 `testing` feature 启用时可用，调用方负责构造 repo（避免 agent crate 依赖 dao）。
    #[cfg(any(test, feature = "testing"))]
    #[doc(hidden)]
    pub fn new_for_test(session_repo: Arc<dyn AgentSessionRepository>) -> Self {
        Self::new(session_repo)
    }

    pub async fn set_default_workspace_dir(&self, dir: Option<String>) {
        let mut default_workspace_dir = self.default_workspace_dir.lock().await;
        *default_workspace_dir = dir;
    }

    pub async fn set_app_handle(&self, app_handle: AppHandle) {
        let mut handle = self.app_handle.lock().await;
        *handle = Some(app_handle);
    }

    pub async fn has_app_handle(&self) -> bool {
        self.app_handle.lock().await.is_some()
    }

    /// Returns the number of currently cached sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }

    /// Get an existing session for the given conversation, or create a new one.
    pub async fn get_or_create_session(
        &self,
        provider_id: String,
        conversation_id: String,
    ) -> Result<AgentSession, String> {
        self.evict_stale_sessions().await;

        {
            let sessions = self.sessions.lock().await;
            let conv_index = self.conversation_index.lock().await;
            if let Some(session_id) = conv_index.get(&conversation_id)
                && let Some(existing) = sessions.get(session_id)
            {
                let cloned = existing.clone();
                let sid = session_id.clone();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                drop(conv_index);
                drop(sessions);
                self.session_last_access.lock().await.insert(sid, now);
                return Ok(cloned);
            }
        }

        self.create_session(provider_id, conversation_id).await
    }

    /// 跨进程上下文重建（PLAN-codex-parity P0-3）。
    ///
    /// 进程重启或 Session 被 LRU 驱逐后内存会话为空，用 DB 消息历史回灌
    /// `Session.messages`（含已完成 turn 的 tool_use / tool_result 观察块），
    /// 使下一次 `agent_query` 携带完整对话上下文。
    ///
    /// 幂等：仅当目标 Session 是全新空会话（`messages` 为空）时才填充。
    /// 尾部截断保护：seed 时当前 turn 的用户输入行已落库（agent_query 先写
    /// 消息再建 Session），`run_turn` 会再次追加，故去掉结尾的 user 行防止
    /// 当前输入被计入两次。
    ///
    /// 返回是否实际填充。
    pub async fn seed_session_history(
        &self,
        conversation_id: &str,
        mut history: Vec<ConversationMessage>,
    ) -> bool {
        let mut sessions = self.sessions.lock().await;
        let conv_index = self.conversation_index.lock().await;
        let Some(session_id) = conv_index.get(conversation_id) else {
            return false;
        };
        let Some(agent_session) = sessions.get_mut(session_id) else {
            return false;
        };
        if !agent_session.session().messages.is_empty() {
            return false;
        }
        while matches!(
            history.last().map(|m| m.role),
            Some(axagent_harness::conversation_model::MessageRole::User)
        ) {
            history.pop();
        }
        if history.is_empty() {
            return false;
        }
        let count = history.len();
        agent_session.session_mut().messages = history;
        agent_session.session_mut().touch();
        info!(
            "[session_manager] Seeded {} historical messages into session {} (conversation {})",
            count, session_id, conversation_id
        );
        true
    }

    pub async fn create_session(
        &self,
        provider_id: String,
        conversation_id: String,
    ) -> Result<AgentSession, String> {
        let mut session = AgentSession::new(provider_id, conversation_id.clone());
        let session_id = session.session().session_id.clone();

        let default_workspace_dir = {
            let guard = self.default_workspace_dir.lock().await;
            guard.clone()
        };

        let cwd_to_use: Option<String> = if session.session().workspace_root.is_none() {
            default_workspace_dir.clone()
        } else {
            session
                .session()
                .workspace_root
                .as_ref()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
        };

        // 将解析出的 workspace_root 同步到运行时 Session 对象
        if let Some(ref cwd) = cwd_to_use
            && !cwd.is_empty()
        {
            session.session_mut().workspace_root = Some(std::path::PathBuf::from(cwd.as_str()));
        }

        let axagent_session = self
            .agent_session_repo
            .upsert_agent_session(&conversation_id, cwd_to_use.as_deref(), Some("default"))
            .await
            .map_err(|e| e.to_string())?;

        session = session.with_axagent_session_id(axagent_session.id);

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session.clone());
        drop(sessions);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        {
            let mut last_access = self.session_last_access.lock().await;
            last_access.insert(session_id.clone(), now);
        }

        let mut conv_index = self.conversation_index.lock().await;
        conv_index.insert(conversation_id, session_id.clone());

        // 注册 per-session 运行时取消 token（初始 false）
        {
            let mut tokens = self.cancel_tokens.lock().await;
            tokens.insert(session_id.clone(), Arc::new(AtomicBool::new(false)));
        }

        Ok(session)
    }

    /// 获取指定 session 的运行时取消 token（create_session 时注册）。
    ///
    /// 供 HarnessAgentAdapter.execute → ReActEngine::run 检查取消信号。
    pub async fn get_cancel_token(&self, session_id: &str) -> Option<Arc<AtomicBool>> {
        let tokens = self.cancel_tokens.lock().await;
        tokens.get(session_id).cloned()
    }

    /// Update the session in memory after a turn completes, preserving conversation history.
    ///
    /// `usage` 携带本轮的 token 统计 (input + output + cache_*),作为权威来源。
    /// 若调用方未传,则回退从 `updated_session.messages` 末尾的 `usage` 字段汇总。
    pub async fn update_session_after_turn(
        &self,
        conversation_id: &str,
        updated_session: Session,
        usage: Option<axagent_harness::conversation_model::TokenUsage>,
    ) {
        let mut sessions = self.sessions.lock().await;
        let conv_index = self.conversation_index.lock().await;
        if let Some(session_id) = conv_index.get(conversation_id)
            && let Some(session) = sessions.get_mut(session_id)
        {
            session.session_mut().messages = updated_session.messages;
            session.session_mut().updated_at_ms = updated_session.updated_at_ms;

            if let Some(axagent_session_id) = session.axagent_session_id() {
                let axagent_sid = axagent_session_id.to_string();
                // 优先使用调用方传入的 usage;否则从 messages 末尾的 usage 字段汇总
                let effective_usage =
                    usage.or_else(|| session.session().messages.iter().rev().find_map(|m| m.usage));
                let tokens_delta = effective_usage
                    .as_ref()
                    .map(|u| u.input_tokens as i64 + u.output_tokens as i64)
                    .unwrap_or(0);

                drop(conv_index);
                drop(sessions);

                let _ = self
                    .agent_session_repo
                    .update_agent_session_after_query(&axagent_sid, "idle", None, tokens_delta, 0.0)
                    .await;
            }
        }

        // 桥接到统一事件总线(若已注入):发布 TurnCompleted
        self.publish_agent_event(
            "TurnCompleted",
            serde_json::json!({
                "conversationId": conversation_id,
            }),
        )
        .await;
    }

    /// Clear the session for a given conversation (used when context is cleared).
    pub async fn clear_session(&self, conversation_id: &str) {
        let session_id = {
            let mut conv_index = self.conversation_index.lock().await;
            conv_index.remove(conversation_id)
        };
        if let Some(session_id) = session_id {
            let mut sessions = self.sessions.lock().await;
            sessions.remove(&session_id);
            self.session_last_access.lock().await.remove(&session_id);

            let _ = self.agent_session_repo.update_agent_session_status(&session_id, "idle").await;
            let _ =
                self.agent_session_repo.clear_sdk_context_by_conversation_id(conversation_id).await;
        }
    }

    async fn evict_stale_sessions(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let ttl_cutoff = now.saturating_sub(SESSION_TTL_SECS);
        let ttl_cutoff_ms = ttl_cutoff * 1000;

        let mut to_evict = Vec::new();
        {
            // 锁顺序: session_last_access (只读,单独)
            let last_access = self.session_last_access.lock().await;
            for (session_id, &last_ms) in last_access.iter() {
                if last_ms < ttl_cutoff_ms {
                    to_evict.push(session_id.clone());
                }
            }
        }

        {
            // 锁顺序: sessions -> session_last_access
            let sessions = self.sessions.lock().await;
            if sessions.len() > MAX_CACHED_SESSIONS {
                let last_access = self.session_last_access.lock().await;
                let mut all_entries: Vec<(String, u64)> =
                    last_access.iter().map(|(id, &t)| (id.clone(), t)).collect();
                all_entries.sort_by_key(|(_, t)| *t);
                let excess = sessions.len() - MAX_CACHED_SESSIONS;
                for (session_id, _) in all_entries.into_iter().take(excess) {
                    if !to_evict.contains(&session_id) {
                        to_evict.push(session_id);
                    }
                }
            }
        }

        if !to_evict.is_empty() {
            info!("[SessionManager] Evicting {} stale sessions", to_evict.len());
            // 锁顺序: sessions -> conversation_index -> session_last_access
            let mut sessions = self.sessions.lock().await;
            let mut last_access = self.session_last_access.lock().await;
            let mut conv_index = self.conversation_index.lock().await;
            for session_id in to_evict {
                sessions.remove(&session_id);
                last_access.remove(&session_id);
                conv_index.retain(|_, v| v != &session_id);
            }
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Option<AgentSession> {
        let sessions = self.sessions.lock().await;
        sessions.get(session_id).cloned()
    }

    pub async fn remove_session(&self, session_id: &str) -> Option<AgentSession> {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id)
    }

    /// Run a turn using a pre-built `AxAgentApiClient` and `ToolRegistry`.
    ///
    /// This is the single unified entry point for agent turns. It handles:
    /// - Pre-turn auto-compaction when the session exceeds the token threshold
    /// - Runtime creation with the provided API client, tools, and system prompt
    /// - Post-turn auto-compaction via `with_auto_compaction_input_tokens_threshold`
    /// - Session state persistence and DB updates
    ///
    /// The caller is responsible for:
    /// - Creating the base runtime via platform runtime builder
    /// - Persisting user/assistant messages to the DB
    /// - Emitting Tauri events
    #[allow(clippy::too_many_arguments)]
    pub async fn run_turn_with_tools(
        &self,
        session_id: &str,
        user_input: String,
        mut runtime: Box<dyn ConversationRuntimeHost>,
        conversation_id: String,
        cancel_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
        prompters: Arc<
            tokio::sync::Mutex<std::collections::HashMap<String, ChannelPermissionPrompter>>,
        >,
    ) -> Result<(TurnSummary, Session), RuntimeError> {
        let session = self
            .get_session(session_id)
            .await
            .ok_or_else(|| RuntimeError::new(format!("Session not found: {}", session_id)))?;

        // 桥接到统一事件总线(若已注入):发布 TurnStarted
        self.publish_agent_event(
            "TurnStarted",
            serde_json::json!({
                "conversationId": conversation_id,
                "sessionId": session_id,
            }),
        )
        .await;

        // Auto-compact if the session exceeds the token threshold.
        // Use CompactionConfig::default() consistently for both the check
        // and the compaction to avoid configuration mismatch.
        let compaction_config = CompactionConfig::default();
        let mut session = if should_compact(session.session(), compaction_config, NP) {
            let result = compact_session(session.session(), compaction_config, NP);

            // Build MessageRecords for integrity verification
            let original_msgs: Vec<serde_json::Value> = session
                .session()
                .messages
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let role_str = match m.role {
                        axagent_harness::conversation_model::MessageRole::System => "system",
                        axagent_harness::conversation_model::MessageRole::User => "user",
                        axagent_harness::conversation_model::MessageRole::Assistant => "assistant",
                        axagent_harness::conversation_model::MessageRole::Tool => "tool",
                    };
                    let content: String = m
                        .blocks
                        .iter()
                        .map(|b| match b {
                            axagent_harness::ContentBlock::Text { text } => text.clone(),
                            axagent_harness::ContentBlock::ToolUse { name, input, .. } => {
                                format!("[ToolUse: {} {}]", name, input)
                            },
                            axagent_harness::ContentBlock::ToolResult {
                                tool_name, output, ..
                            } => format!("[ToolResult: {} {}]", tool_name, output),
                        })
                        .collect();
                    serde_json::json!({
                        "id": format!("orig-{}", i),
                        "role": role_str.to_string(),
                        "content": content,
                        "timestamp": i as i64,
                    })
                })
                .collect();
            let compressed_msgs: Vec<serde_json::Value> = result
                .compacted_session
                .messages
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let role_str = match m.role {
                        axagent_harness::conversation_model::MessageRole::System => "system",
                        axagent_harness::conversation_model::MessageRole::User => "user",
                        axagent_harness::conversation_model::MessageRole::Assistant => "assistant",
                        axagent_harness::conversation_model::MessageRole::Tool => "tool",
                    };
                    let content: String = m
                        .blocks
                        .iter()
                        .map(|b| match b {
                            axagent_harness::ContentBlock::Text { text } => text.clone(),
                            axagent_harness::ContentBlock::ToolUse { name, input, .. } => {
                                format!("[ToolUse: {} {}]", name, input)
                            },
                            axagent_harness::ContentBlock::ToolResult {
                                tool_name, output, ..
                            } => format!("[ToolResult: {} {}]", tool_name, output),
                        })
                        .collect();
                    serde_json::json!({
                        "id": format!("comp-{}", i),
                        "role": role_str.to_string(),
                        "content": content,
                        "timestamp": i as i64,
                    })
                })
                .collect();

            // 通过 harness TrajectoryService 进行完整性校验
            let key_entities = match &self.trajectory {
                Some(svc) => svc.extract_entities(&original_msgs),
                None => Vec::new(),
            };
            let integrity = match &self.trajectory {
                Some(svc) => svc.verify_compression_integrity(
                    &original_msgs,
                    &compressed_msgs,
                    &key_entities,
                ),
                None => axagent_harness::IntegrityResult { is_valid: true, checks: Vec::new() },
            };
            if !integrity.is_valid {
                let failed_checks: Vec<&str> = integrity
                    .checks
                    .iter()
                    .filter(|c| !c.passed)
                    .map(|c| c.name.as_str())
                    .collect();
                info!("Compression integrity warning: failed checks: {:?}", failed_checks);
            } else {
                info!(
                    "Compression integrity verified: all {} checks passed ({} key entities tracked)",
                    integrity.checks.len(),
                    key_entities.len()
                );
            }

            let mut compacted = session;
            compacted.session_mut().messages = result.compacted_session.messages;
            compacted
        } else {
            session
        };

        // Compute dynamic config from trajectory, then configure runtime
        let max_iters = dynamic_max_iterations(
            &self
                .trajectory
                .as_ref()
                .map(|s| s.estimate_complexity(&user_input))
                .unwrap_or(TaskComplexity::Medium),
        );
        runtime.set_max_iterations(max_iters);
        runtime.set_auto_compaction_threshold(AUTO_COMPACTION_TOKEN_THRESHOLD as u32);

        // Attach cancel token if provided
        if let Some(token) = cancel_token {
            runtime.set_cancel_token(Some(token));
        }

        // Create execution progress tracker for frontend panels.
        let progress = Arc::new(AgentExecutionProgress::new(max_iters));
        runtime.set_progress(progress.clone());
        {
            let mut trackers = self.progress_trackers.write().await;
            trackers.insert(conversation_id.clone(), progress.clone());
        }

        // Add Tauri event reporter for tool progress.
        if let Some(handle) = self.app_handle.lock().await.clone() {
            let reporter = Box::new(TauriHookProgressReporter::with_progress(
                handle,
                conversation_id.clone(),
                progress,
            ));
            runtime.set_hook_progress_reporter(reporter);
        }

        // Run turn with prompter if available for this conversation
        // We need to extract the prompter from the shared map, then use it in run_turn.
        // Since run_turn is synchronous and PermissionPrompter is not Send,
        // we must do this within the same thread.
        //
        // run_turn may block for extended periods (e.g. waiting for user permission
        // approval via ChannelPermissionPrompter). Use block_in_place to tell
        // the tokio runtime that this section will block, allowing it to schedule
        // other tasks on this thread's core while we wait.
        //
        // NOTE: ChannelPermissionPrompter uses Arc internally (Clone just bumps
        // the refcount). We clone instead of remove so agent_approve can still
        // look up the prompter and deliver decisions during the run.
        let conv_id_for_prompter = conversation_id.clone();
        let mut prompter_opt = prompters.lock().await.get(&conv_id_for_prompter).cloned();
        // 保留 user_input 副本供复盘使用(原 user_input 会被 move 进 spawn_blocking)
        let user_input_for_reflect = user_input.clone();
        // run_turn 是同步的,通过 spawn_blocking + oneshot 在专用阻塞线程执行,
        // 避免 block_in_place 在多线程 runtime 中产生调度隐患。
        // 同时把 `updated_session` 一并从闭包中传出（runtime 已被 move 进去，
        // 闭包外无法再调用 into_session）。
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let result = if let Some(ref mut p) = prompter_opt {
                runtime.run_turn(&user_input, Some(p))
            } else {
                runtime.run_turn(&user_input, None)
            };
            let payload = result.map(|summary| (summary, runtime.into_session()));
            let _ = tx.send(payload);
        });
        let (summary, updated_session) = rx
            .await
            .map_err(|e| RuntimeError::new(format!("run_turn task dropped: {e}")))?
            .map_err(|e| RuntimeError::new(format!("run_turn failed: {e}")))?;

        session.session_mut().messages = updated_session.messages.clone();
        session.session_mut().updated_at_ms = updated_session.updated_at_ms;

        // Persist updates
        if let Some(axagent_session_id) = session.axagent_session_id() {
            let tokens_delta =
                summary.usage.input_tokens as i64 + summary.usage.output_tokens as i64;
            // Cost is now persisted separately in agent_query (agent/mod.rs) after
            // estimate_cost_usd() runs with pricing info. We keep 0.0 here to avoid
            // duplicating the pricing logic in session_manager; the real cost_delta
            // is written via a direct update_agent_session_after_query call.
            let cost_delta = 0.0;

            let _ = self
                .agent_session_repo
                .update_agent_session_after_query(
                    axagent_session_id,
                    "idle",
                    None,
                    tokens_delta,
                    cost_delta,
                )
                .await;
        }

        // Store updated session back
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.to_string(), session);

        // Clean up progress tracker — the frontend will get one final
        // snapshot via agent-done before this removal.
        {
            let mut trackers = self.progress_trackers.write().await;
            trackers.remove(&conversation_id);
        }

        // ── 自动复盘 ──
        //
        // 若注入了 Reflector,任务完成后做复盘:
        // - 从 summary.assistant_messages.blocks 提取 ToolUse 工具名序列
        // - 从 summary.tool_results.blocks 检查 ToolResult.is_error 判断成功
        // - 30s 超时保护,失败仅日志
        //
        // 复盘模式由 `self_improvement_flags` 控制(wiring 层从前端 FeatureFlag 注入):
        // 1. `final_output_reflection=true`(最终输出质量门):**同步**等待 Reflector
        //    完成,使评估结果可立即用于质量门判定。若同时启用 `self_improvement_enabled`
        //    且质量不达标,把改进建议写入 session 的 nudge_lines,下次 turn 注入 LLM。
        // 2. 默认(两个 flag 均为 false):**异步** fire-and-forget,不阻塞主流程
        //    (保持原行为,解决 `Reflector::reflect()` 零调用问题)。
        //
        // 复盘产物落 reflections.jsonl + InsightGenerator.store_insight,
        // 由 Reflector 内部逻辑处理,这里仅触发。
        let reflector_opt = self.reflector.read().await.clone();
        let flags = *self.self_improvement_flags.read().await;
        if let Some(reflector) = reflector_opt {
            use axagent_harness::conversation_model::ContentBlock;

            // 从 assistant_messages 的 blocks 中提取所有 ToolUse 工具名
            let tools_used: Vec<String> = summary
                .assistant_messages
                .iter()
                .flat_map(|m| m.blocks.iter())
                .filter_map(|b| match b {
                    ContentBlock::ToolUse { name, .. } => Some(name.clone()),
                    _ => None,
                })
                .collect();

            // 从 tool_results 的 blocks 中检查是否有错误
            let has_error = summary
                .tool_results
                .iter()
                .flat_map(|m| m.blocks.iter())
                .any(|b| matches!(b, ContentBlock::ToolResult { is_error: true, .. }));

            let iterations = summary.iterations;
            // 估算执行时长:input+output token 折算(粗略,用于反思效率评分)
            let duration_ms = (summary.usage.input_tokens as u64
                + summary.usage.output_tokens as u64)
                .saturating_mul(50);
            let task_id = format!("{}-{}", conversation_id, chrono::Utc::now().timestamp_millis());

            let mut record = axagent_harness::reflection_types::TaskExecutionRecord::new(
                task_id.clone(),
                user_input_for_reflect.clone(),
                chrono::Utc::now() - chrono::Duration::milliseconds(duration_ms as i64),
                chrono::Utc::now(),
            )
            .with_tools(tools_used)
            .with_iterations(iterations)
            .with_success(!has_error);

            record.duration_ms = duration_ms;
            if has_error {
                record = record.with_error("tool_execution_error".to_string());
            }

            if flags.final_output_reflection {
                // ── 同步质量门:等待 Reflector 完成 ──
                // 启用 finalOutputReflection 时,同步等待评估结果,
                // 使质量分数和改进建议可立即用于下游决策。
                let reflector_clone = reflector.clone();
                let reflect_result = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    reflector_clone.reflect(&record),
                )
                .await;
                match reflect_result {
                    Ok(reflection) => {
                        tracing::info!(
                            "[session_manager] sync-reflect completed for task {}: quality_score={}",
                            task_id,
                            reflection.quality_score
                        );

                        // 若同时启用自改进循环且质量不达标,把改进建议写入 nudge_lines
                        // (下次 turn 通过 runtime.set_nudge_lines 注入 LLM)
                        if flags.self_improvement_enabled
                            && !reflection.improvement_suggestions.is_empty()
                        {
                            // 质量阈值:quality_score < 7 视为不达标
                            const QUALITY_THRESHOLD: u8 = 7;
                            if reflection.quality_score < QUALITY_THRESHOLD {
                                let nudge: Vec<String> = reflection
                                    .improvement_suggestions
                                    .iter()
                                    .take(3)
                                    .map(|s| format!("改进建议: {s}"))
                                    .collect();
                                tracing::info!(
                                    "[session_manager] quality gate triggered for task {}: score={} < {}, injecting {} nudge lines",
                                    task_id,
                                    reflection.quality_score,
                                    QUALITY_THRESHOLD,
                                    nudge.len()
                                );
                                // 写入 session 的 nudge_lines,供下次 turn 使用
                                // (通过 sessions map 中的 session 引用)
                                let mut sessions = self.sessions.lock().await;
                                if let Some(session) = sessions.get_mut(session_id) {
                                    // AgentSession 没有直接的 nudge_lines 字段,
                                    // 通过 blackboard 传递(下次 turn 前由调用方读取)
                                    let _ = session
                                        .record_to_blackboard("pending_nudge", &nudge.join("\n"))
                                        .await;
                                }
                            }
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            "[session_manager] sync-reflect timeout or failed for task {}: {}",
                            task_id,
                            e
                        );
                    },
                }
            } else {
                // ── 异步 fire-and-forget(默认行为) ──
                // 异步 spawn,与主流程解耦(反思失败不影响 turn 结果)
                let reflector_clone = reflector.clone();
                tokio::spawn(async move {
                    let reflect_result = tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        reflector_clone.reflect(&record),
                    )
                    .await;
                    match reflect_result {
                        Ok(reflection) => {
                            tracing::info!(
                                "[session_manager] auto-reflect completed for task {}: quality_score={}",
                                task_id,
                                reflection.quality_score
                            );
                        },
                        Err(e) => {
                            tracing::warn!(
                                "[session_manager] auto-reflect timeout or failed for task {}: {}",
                                task_id,
                                e
                            );
                        },
                    }
                });
            }
        }

        Ok((summary, updated_session))
    }

    /// Get the execution progress tracker for a given conversation.
    /// Used by `agent_runtime_stats` IPC to return real-time progress to the frontend.
    pub async fn get_progress(&self, conversation_id: &str) -> Option<Arc<AgentExecutionProgress>> {
        let trackers = self.progress_trackers.read().await;
        trackers.get(conversation_id).cloned()
    }
}

// ---------------------------------------------------------------------------
// ChannelPermissionPrompter — bridges runtime permission prompts to the Tauri
// frontend via events + oneshot channels, then blocks until the user responds.
// ---------------------------------------------------------------------------

/// A [`PermissionPrompter`] that emits a Tauri `agent-permission-request` event
/// and blocks on a `std::sync::mpsc` channel until the frontend sends back a
/// decision via the `agent_approve` command.
///
/// Note: Clone is derived but the `pending_senders` map is NOT shared between
/// clones. The clone is only used for registering in AppState; the original
/// (passed to run_turn) is the one that actually blocks. The `deliver_decision`
/// method on the clone will not work — `agent_approve` must find the original
/// prompter. To solve this, we use a shared inner state via Arc.
pub struct ChannelPermissionPrompter {
    app_handle: AppHandle,
    conversation_id: String,
    inner: Arc<ChannelPermissionPrompterInner>,
}

// SAFETY: 此处 parking_lot::Mutex 不跨 await 使用，所有方法均为同步操作。
#[allow(clippy::disallowed_types)]
struct ChannelPermissionPrompterInner {
    /// Maps request_id → Sender that agent_approve will use to unblock.
    pending_senders: parking_lot::Mutex<
        std::collections::HashMap<String, std::sync::mpsc::Sender<PermissionPromptDecision>>,
    >,
    /// Tools the user has marked "always allow" for this conversation.
    always_allowed: parking_lot::Mutex<HashSet<String>>,
    /// Workspace root directory for file write boundary checks.
    workspace_root: parking_lot::Mutex<String>,
}

// SAFETY: 此处 parking_lot::Mutex 不跨 await 使用，所有方法均为同步操作。
#[allow(clippy::disallowed_types)]
impl ChannelPermissionPrompter {
    pub fn new(
        app_handle: AppHandle,
        conversation_id: String,
        always_allowed: HashSet<String>,
        workspace_root: String,
    ) -> Self {
        Self {
            app_handle,
            conversation_id,
            inner: Arc::new(ChannelPermissionPrompterInner {
                pending_senders: parking_lot::Mutex::new(std::collections::HashMap::new()),
                always_allowed: parking_lot::Mutex::new(always_allowed),
                workspace_root: parking_lot::Mutex::new(workspace_root),
            }),
        }
    }

    /// Returns the number of pending permission requests.
    pub fn pending_count(&self) -> usize {
        self.inner.pending_senders.lock().len()
    }

    /// Register a sender for a pending request. Called by `agent_approve` command
    /// to deliver the user's decision.
    pub fn deliver_decision(&self, request_id: &str, decision: PermissionPromptDecision) -> bool {
        let mut map = self.inner.pending_senders.lock();
        if let Some(sender) = map.remove(request_id) {
            sender.send(decision).is_ok()
        } else {
            false
        }
    }

    /// Add a tool to the "always allowed" set for this conversation.
    pub fn add_always_allowed(&self, tool_name: &str) {
        let mut set = self.inner.always_allowed.lock();
        set.insert(tool_name.to_string());
    }

    /// Get the current "always allowed" set.
    pub fn get_always_allowed(&self) -> HashSet<String> {
        self.inner.always_allowed.lock().clone()
    }

    /// Clean up any stale pending senders (e.g. on conversation switch).
    pub fn clear_pending(&self) {
        let mut map = self.inner.pending_senders.lock();
        map.clear();
    }
}

impl Clone for ChannelPermissionPrompter {
    fn clone(&self) -> Self {
        Self {
            app_handle: self.app_handle.clone(),
            conversation_id: self.conversation_id.clone(),
            inner: Arc::clone(&self.inner),
        }
    }
}

impl PermissionPrompter for ChannelPermissionPrompter {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
        // Check "always allowed" first — match by tool_name
        let set = self.inner.always_allowed.lock();
        if set.contains(&request.tool_name) {
            info!(
                "[ChannelPermissionPrompter] Auto-allowing '{}' (always allowed)",
                request.tool_name
            );
            return PermissionPromptDecision::Allow;
        }

        // Fine-grained enforcement checks before prompting the user.
        // These catch operations that should be hard-denied regardless of user choice
        // (e.g., writing outside workspace, dangerous bash commands in read-only mode).
        let enforcer = axagent_harness::runtime_types::permission_enforcer::PermissionEnforcer::new(
            PermissionPolicy::new(request.current_mode),
        );
        let tool_name_lower = request.tool_name.to_lowercase();

        // Check file write boundary for write/edit/create tools
        if tool_name_lower.contains("write")
            || tool_name_lower.contains("edit")
            || tool_name_lower.contains("create")
            || tool_name_lower.contains("patch")
        {
            // Try to extract a file path from the input JSON
            if let Ok(input_val) = serde_json::from_str::<serde_json::Value>(&request.input) {
                let path = input_val
                    .get("path")
                    .or_else(|| input_val.get("file_path"))
                    .or_else(|| input_val.get("filePath"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !path.is_empty() {
                    // Use the cwd as workspace root if available
                    let workspace_root = self.inner.workspace_root.lock().clone();
                    if !workspace_root.is_empty() {
                        let result = enforcer.check_file_write(path, &workspace_root);
                        if let axagent_harness::runtime_types::permission_enforcer::EnforcementResult::Denied {
                            reason,
                            ..
                        } = result
                        {
                            info!(
                                "[ChannelPermissionPrompter] File write denied by enforcer: {}",
                                reason
                            );
                            return PermissionPromptDecision::Deny { reason };
                        }
                    }
                }
            }
        }

        // Check bash command safety
        if (tool_name_lower.contains("bash")
            || tool_name_lower.contains("shell")
            || tool_name_lower.contains("exec")
            || tool_name_lower.contains("run"))
            && let Ok(input_val) = serde_json::from_str::<serde_json::Value>(&request.input)
        {
            let command = input_val.get("command").and_then(|v| v.as_str()).unwrap_or("");
            if !command.is_empty() {
                let result = enforcer.check_bash(command);
                if let axagent_harness::runtime_types::permission_enforcer::EnforcementResult::Denied {
                    reason,
                    ..
                } = result
                {
                    info!(
                        "[ChannelPermissionPrompter] Bash command denied by enforcer: {}",
                        reason
                    );
                    return PermissionPromptDecision::Deny { reason };
                }
            }
        }

        // Generate a unique request ID
        let request_id = format!("perm_{}", uuid::Uuid::new_v4());

        info!(
            "[ChannelPermissionPrompter] Prompting user for tool '{}' (request_id={})",
            request.tool_name, request_id
        );

        // Emit permission request event to frontend
        let risk_level = match request.required_mode {
            PermissionMode::ReadOnly => "read_only",
            PermissionMode::WorkspaceWrite => "write",
            PermissionMode::DangerFullAccess => "execute",
            _ => "write",
        };

        let input_value: serde_json::Value =
            serde_json::from_str(&request.input).unwrap_or(serde_json::Value::Null);

        let _ = self.app_handle.emit(
            "agent-permission-request",
            AgentPermissionPayload {
                conversation_id: self.conversation_id.clone(),
                assistant_message_id: String::new(),
                tool_name: request.tool_name.clone(),
                input: input_value,
                risk_level: risk_level.to_string(),
                request_id: request_id.clone(),
                tool_use_id: None,
            },
        );

        // Create a synchronous channel and register the sender
        let (tx, rx) = std::sync::mpsc::channel::<PermissionPromptDecision>();
        let mut map = self.inner.pending_senders.lock();
        map.insert(request_id.clone(), tx);

        // Block until the frontend responds via agent_approve command
        // Use a 5-minute timeout to prevent indefinite blocking if the user
        // doesn't respond (e.g. walks away, or page is refreshed).
        const PERMISSION_TIMEOUT_SECS: u64 = 300;
        match rx.recv_timeout(std::time::Duration::from_secs(PERMISSION_TIMEOUT_SECS)) {
            Ok(decision) => {
                info!(
                    "[ChannelPermissionPrompter] Received decision for '{}': {:?}",
                    request.tool_name, decision
                );
                // If the decision is Allow and the tool was approved with "allow_always",
                // add it to the always_allowed set.
                // Note: The decision variant itself doesn't carry "always" info,
                // but the frontend sends the decision string via agent_approve.
                // The "always" handling is done in the agent_approve command before
                // calling deliver_decision.
                decision
            },
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                info!(
                    "[ChannelPermissionPrompter] Permission request for '{}' timed out after {}s, auto-denying",
                    request.tool_name, PERMISSION_TIMEOUT_SECS
                );
                // Clean up the pending entry
                let mut map = self.inner.pending_senders.lock();
                map.remove(&request_id);

                // Notify frontend that the permission was auto-denied due to timeout
                let _ = self.app_handle.emit(
                    "agent-permission-timeout",
                    serde_json::json!({
                        "conversationId": self.conversation_id,
                        "requestId": request_id,
                        "toolName": request.tool_name,
                    }),
                );
                PermissionPromptDecision::Deny {
                    reason: format!(
                        "Permission request timed out after {}s (no user response)",
                        PERMISSION_TIMEOUT_SECS
                    ),
                }
            },
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // Sender was dropped (e.g. agent cancelled) — deny by default
                // Clean up the pending entry
                let mut map = self.inner.pending_senders.lock();
                map.remove(&request_id);

                PermissionPromptDecision::Deny {
                    reason: "Permission request cancelled (agent disconnected)".to_string(),
                }
            },
        }
    }
}

/// Tauri event emitter that implements HookProgressReporter for forwarding tool events to frontend
pub struct TauriHookProgressReporter {
    app_handle: AppHandle,
    conversation_id: String,
    progress: Option<Arc<AgentExecutionProgress>>,
}

impl TauriHookProgressReporter {
    pub fn new(app_handle: AppHandle, conversation_id: String) -> Self {
        Self { app_handle, conversation_id, progress: None }
    }

    pub fn with_progress(
        app_handle: AppHandle,
        conversation_id: String,
        progress: Arc<AgentExecutionProgress>,
    ) -> Self {
        Self { app_handle, conversation_id, progress: Some(progress) }
    }
}

impl HookProgressReporter for TauriHookProgressReporter {
    fn on_event(&mut self, event: &HookProgressEvent) {
        let conversation_id = self.conversation_id.clone();
        match event {
            HookProgressEvent::Started {
                event: HookEvent::PreToolUse,
                tool_name,
                command: _,
                tool_use_id,
            } => {
                let mut payload = serde_json::json!({
                    "conversationId": conversation_id,
                    "toolUseId": tool_use_id.as_deref().unwrap_or(""),
                    "toolName": tool_name,
                    "input": serde_json::Value::Null,
                    "assistantMessageId": "",
                });
                // Include server-side timestamp from the shared progress tracker
                if let Some(ref p) = self.progress {
                    let snap = p.snapshot();
                    if let Some(started_at) = snap.current_tool_started_at {
                        payload["startedAt"] = serde_json::json!(started_at);
                    }
                    payload["iteration"] = serde_json::json!(snap.current_iteration);
                    payload["maxIterations"] = serde_json::json!(snap.max_iterations);
                }
                let _ = self.app_handle.emit("agent-tool-start", payload);
            },
            HookProgressEvent::Completed {
                event: HookEvent::PostToolUse,
                tool_name,
                command: _,
                tool_use_id,
            } => {
                let mut payload = serde_json::json!({
                    "conversationId": conversation_id,
                    "toolUseId": tool_use_id.as_deref().unwrap_or(""),
                    "toolName": tool_name,
                    "input": serde_json::Value::Null,
                    "content": "",
                    "isError": false,
                    "assistantMessageId": "",
                });
                // Include duration from the shared progress tracker
                if let Some(ref p) = self.progress {
                    let snap = p.snapshot();
                    if let Some(started_at) = snap.current_tool_started_at {
                        payload["startedAt"] = serde_json::json!(started_at);
                    }
                }
                let _ = self.app_handle.emit("agent-tool-result", payload);
            },
            HookProgressEvent::Cancelled {
                event: HookEvent::PostToolUse,
                tool_name,
                command: _,
                tool_use_id,
            }
            | HookProgressEvent::Completed {
                event: HookEvent::PostToolUseFailure,
                tool_name,
                command: _,
                tool_use_id,
            } => {
                let mut payload = serde_json::json!({
                    "conversationId": conversation_id,
                    "toolUseId": tool_use_id.as_deref().unwrap_or(""),
                    "toolName": tool_name,
                    "input": serde_json::Value::Null,
                    "content": "",
                    "isError": true,
                    "assistantMessageId": "",
                });
                // Include duration from the shared progress tracker
                if let Some(ref p) = self.progress {
                    let snap = p.snapshot();
                    if let Some(started_at) = snap.current_tool_started_at {
                        payload["startedAt"] = serde_json::json!(started_at);
                    }
                }
                let _ = self.app_handle.emit("agent-tool-result", payload);
            },
            _ => {},
        }
    }

    fn on_progress(&mut self, message: &str, iteration: usize, total: usize) {
        // Build rich progress payload for the frontend watchdog and panels.
        // The `agent-status` event resets the frontend 10-min timer AND provides
        // live execution status to AgentProgressBar, ExecutionTimeline, etc.
        let mut payload = serde_json::json!({
            "conversationId": self.conversation_id,
            "phase": "running",
            "message": message,
            "iteration": iteration,
            "totalIterations": total,
        });

        if let Some(ref progress) = self.progress {
            let snap = progress.snapshot();
            payload["currentTool"] =
                serde_json::Value::String(snap.current_tool.unwrap_or_default());
            payload["executedToolCount"] = serde_json::json!(snap.executed_tool_count);
            payload["failedToolCount"] = serde_json::json!(snap.failed_tool_count);
            if let Some(err) = &snap.last_error {
                payload["lastError"] = serde_json::Value::String(err.clone());
            }
            payload["statusMessage"] = serde_json::Value::String(snap.status_message);
        }

        let _ = self.app_handle.emit("agent-status", payload);
    }
}

// ── AgentSessionBroker 实现（供 MCP / CLI 查询 agent 会话状态） ──────────

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 同步 Debug impl 禁止 .await，parking_lot::Mutex::try_lock 可用
        let session_count = self.sessions.try_lock().map(|g| g.len()).unwrap_or(0);
        f.debug_struct("SessionManager").field("sessions", &session_count).finish()
    }
}

#[async_trait::async_trait]
impl axagent_harness::AgentSessionBroker for SessionManager {
    /// 查询会话状态：两级查找。
    ///
    /// 1. 优先查内存 HashMap（活跃运行中的会话）
    /// 2. 内存未命中时，用 conversation_index 反查 conversation_id →
    ///    AgentSessionRepository::get_by_conversation_id 查 DB（进程重启后仍可查询）
    async fn get_session_status(
        &self,
        session_id: &str,
    ) -> Result<axagent_harness::AgentSessionStatusView, String> {
        // ── 第一级：内存 HashMap ──────────────────────────────────────
        if let Some(agent_session) = self.get_session(session_id).await {
            // 简化语义：内存中有会话 = Running；空 messages = Initializing
            let status = if agent_session.session.messages.is_empty() {
                axagent_harness::types::session_state::SessionStatus::Initializing
            } else {
                axagent_harness::types::session_state::SessionStatus::Running
            };
            let is_active = status.is_active();

            let turn_count = (agent_session.session.messages.len() / 2) as u32;
            let last_access = self.session_last_access.lock().await;

            return Ok(axagent_harness::AgentSessionStatusView {
                session_id: session_id.to_string(),
                status,
                provider_id: agent_session.provider_id.clone(),
                conversation_id: Some(agent_session.conversation_id.clone()),
                turn_count: Some(turn_count),
                is_active,
                last_access_ms: last_access.get(session_id).copied(),
                last_error: None,
            });
        }

        // ── 第二级：DB 回退（进程重启后会话仍可查）────────────────────
        // 尝试用 conversation_index 反查 conversation_id
        let conversation_id_opt = {
            let conv_index = self.conversation_index.lock().await;
            conv_index
                .iter()
                .find(|(_, sid)| sid.as_str() == session_id)
                .map(|(cid, _)| cid.clone())
        };

        if let Some(conv_id) = conversation_id_opt
            && let Ok(Some(db_session)) =
                self.agent_session_repo.get_by_conversation_id(&conv_id).await
        {
            // 从 runtime_status 字符串解析状态；解析失败回退 Idle
            let status = db_session
                .runtime_status
                .parse::<axagent_harness::types::session_state::SessionStatus>()
                .unwrap_or(axagent_harness::types::session_state::SessionStatus::Idle);
            let is_active = status.is_active();

            return Ok(axagent_harness::AgentSessionStatusView {
                session_id: session_id.to_string(),
                status,
                provider_id: "unknown".to_string(),
                conversation_id: Some(db_session.conversation_id),
                turn_count: None,
                is_active,
                last_access_ms: Some(db_session.updated_at as u64),
                last_error: None,
            });
        }

        Err(format!("session_not_found: {session_id}"))
    }

    /// 取消会话：幂等处理。
    ///
    /// - 不存在 → Err(session_not_found)
    /// - 非活跃 terminal 状态 → Ok（no-op）
    /// - 活跃会话 → 从 sessions HashMap 和 conversation_index 清理
    async fn cancel_session(&self, session_id: &str) -> Result<(), String> {
        let view = self.get_session_status(session_id).await?;

        // 幂等：非活跃态直接返回
        if !view.is_active {
            return Ok(());
        }

        // 先唤醒 per-session 取消 token：让正在 run 的 ReActEngine 在下一次
        // 迭代开头立即退出，返回 ReActResult::failure("Cancelled by user")。
        // 此处先 store(true) 再清理 HashMap，保证 run 循环能看到 token 为 true。
        {
            let mut tokens = self.cancel_tokens.lock().await;
            if let Some(token) = tokens.get(session_id) {
                token.store(true, Ordering::SeqCst);
                tracing::info!("[SessionManager] cancel_token set for session {session_id}");
            }
            // 清理 token（即使无注册也尝试 remove，幂等）
            tokens.remove(session_id);
        }

        // 从内存 HashMap 移除
        {
            let mut sessions = self.sessions.lock().await;
            sessions.remove(session_id);
        }

        // 从 conversation_index 清理反向索引
        {
            let mut conv_index = self.conversation_index.lock().await;
            conv_index.retain(|_, sid| sid != session_id);
        }

        // 从 session_last_access 清理
        {
            let mut last_access = self.session_last_access.lock().await;
            last_access.remove(session_id);
        }

        Ok(())
    }

    async fn list_session_ids(&self) -> Result<Vec<String>, String> {
        use std::collections::HashSet;

        let mut ids = HashSet::new();

        // 第一级：内存活跃会话
        {
            let sessions = self.sessions.lock().await;
            ids.extend(sessions.keys().cloned());
        }

        // 第二级：DB 回退（含已完成的历史会话）
        // conversation_index 反查 conversation_id → 有没有对应的 runtime session_id
        let conv_to_session = {
            let conv_index = self.conversation_index.lock().await;
            conv_index.clone()
        };

        match self.agent_session_repo.list_all().await {
            Ok(db_sessions) => {
                for db in db_sessions {
                    // 优先用 conversation_index 反查出 runtime session_id
                    if let Some(sid) = conv_to_session.get(&db.conversation_id) {
                        ids.insert(sid.clone());
                    } else {
                        // 内存已淘汰但 DB 还在：用 conversation_id 作为 fallback id
                        ids.insert(db.conversation_id);
                    }
                }
            },
            Err(e) => {
                tracing::warn!(
                    "[SessionManager] list_all DB fallback failed: {e}, returning only memory sessions"
                );
            },
        }

        Ok(ids.into_iter().collect())
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_types)]
// SAFETY: 测试模块使用 parking_lot::Mutex 保护测试桩数据，仅在同步测试场景中使用，无跨 await 风险。
mod tests {
    use super::*;
    use axagent_harness::conversation_model::MessageRole as HarnessMessageRole;

    #[test]
    fn test_estimate_tokens_from_text() {
        assert_eq!(estimate_tokens_from_text(""), 0);
        assert_eq!(estimate_tokens_from_text("abcd"), 1);
        assert_eq!(estimate_tokens_from_text("abcdefgh"), 2);
        assert_eq!(estimate_tokens_from_text("abc"), 0);
        assert_eq!(estimate_tokens_from_text("hello world test!"), 4);
    }

    #[test]
    fn test_estimate_tokens_from_text_long() {
        let long_text = "a".repeat(400);
        assert_eq!(estimate_tokens_from_text(&long_text), 100);
    }

    #[test]
    fn test_estimate_tokens_from_messages_empty() {
        let messages: Vec<HarnessConversationMessage> = vec![];
        assert_eq!(estimate_tokens_from_messages(&messages), 0);
    }

    #[test]
    fn test_estimate_tokens_from_messages_with_text() {
        let messages = vec![HarnessConversationMessage {
            role: HarnessMessageRole::User,
            blocks: vec![HarnessContentBlock::Text { text: "hello world".to_string() }],
            usage: None,
        }];
        assert_eq!(estimate_tokens_from_messages(&messages), 2);
    }

    #[test]
    fn test_estimate_tokens_from_messages_with_tool_use() {
        let messages = vec![HarnessConversationMessage {
            role: HarnessMessageRole::Assistant,
            blocks: vec![HarnessContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "read_file".to_string(),
                input: "{}".to_string(),
            }],
            usage: None,
        }];
        let tokens = estimate_tokens_from_messages(&messages);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_tokens_from_messages_with_tool_result() {
        let messages = vec![HarnessConversationMessage {
            role: HarnessMessageRole::Tool,
            blocks: vec![HarnessContentBlock::ToolResult {
                tool_use_id: "tu-1".to_string(),
                tool_name: "read_file".to_string(),
                output: "file contents here".to_string(),
                is_error: false,
            }],
            usage: None,
        }];
        let tokens = estimate_tokens_from_messages(&messages);
        assert!(tokens > 0);
    }

    #[test]
    fn test_dynamic_max_iterations_low() {
        assert_eq!(dynamic_max_iterations(&TaskComplexity::Low), 20);
    }

    #[test]
    fn test_dynamic_max_iterations_medium() {
        assert_eq!(dynamic_max_iterations(&TaskComplexity::Medium), 50);
    }

    #[test]
    fn test_dynamic_max_iterations_high() {
        assert_eq!(dynamic_max_iterations(&TaskComplexity::High), 100);
    }

    #[test]
    fn test_token_usage_breakdown_with_actual_usage() {
        let usage = HarnessTokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: Some(0),
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        assert_eq!(breakdown.input_tokens, 100);
        assert_eq!(breakdown.output_tokens, 50);
        assert_eq!(breakdown.total_tokens, 150);
        assert!(!breakdown.estimated_from_chars);
    }

    #[test]
    fn test_token_usage_breakdown_with_estimated_chars() {
        let usage = HarnessTokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: Some(0),
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 400);
        assert_eq!(breakdown.total_tokens, 100);
        assert!(breakdown.estimated_from_chars);
    }

    #[test]
    fn test_token_usage_breakdown_tokens_delta() {
        let usage = HarnessTokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: Some(0),
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        assert_eq!(breakdown.tokens_delta(), 150);
    }

    #[test]
    fn test_agent_session_new() {
        let session = AgentSession::new("provider-1".to_string(), "conv-1".to_string());
        assert_eq!(session.provider_id(), "provider-1");
        assert_eq!(session.conversation_id(), "conv-1");
        assert!(session.team_id().is_none());
        assert!(session.role().is_none());
        assert!(session.axagent_session_id().is_none());
    }

    #[test]
    fn test_agent_session_with_team() {
        let session =
            AgentSession::new("p1".to_string(), "c1".to_string()).with_team("team-1".to_string());
        assert_eq!(session.team_id(), Some("team-1"));
    }

    #[test]
    fn test_agent_session_with_role() {
        let session = AgentSession::new("p1".to_string(), "c1".to_string())
            .with_role("developer".to_string());
        assert_eq!(session.role(), Some("developer"));
    }

    #[test]
    fn test_agent_session_with_axagent_session_id() {
        let session = AgentSession::new("p1".to_string(), "c1".to_string())
            .with_axagent_session_id("ax-123".to_string());
        assert_eq!(session.axagent_session_id(), Some("ax-123"));
    }

    #[test]
    fn test_agent_session_builder_chain() {
        let session = AgentSession::new("p1".to_string(), "c1".to_string())
            .with_team("team-1".to_string())
            .with_role("reviewer".to_string())
            .with_axagent_session_id("ax-456".to_string());
        assert_eq!(session.provider_id(), "p1");
        assert_eq!(session.conversation_id(), "c1");
        assert_eq!(session.team_id(), Some("team-1"));
        assert_eq!(session.role(), Some("reviewer"));
        assert_eq!(session.axagent_session_id(), Some("ax-456"));
    }

    #[test]
    fn test_agent_session_session_accessor() {
        let session = AgentSession::new("p1".to_string(), "c1".to_string());
        let session_id = session.session().session_id.clone();
        assert!(!session_id.is_empty());
    }

    #[test]
    fn test_agent_session_clone() {
        let session =
            AgentSession::new("p1".to_string(), "c1".to_string()).with_team("team-1".to_string());
        let cloned = session.clone();
        assert_eq!(cloned.provider_id(), session.provider_id());
        assert_eq!(cloned.conversation_id(), session.conversation_id());
        assert_eq!(cloned.team_id(), session.team_id());
    }

    #[test]
    fn test_agent_session_session_mut() {
        let mut session = AgentSession::new("p1".to_string(), "c1".to_string());
        let original_updated_at = session.session().updated_at_ms;
        session.session_mut().updated_at_ms = 999;
        assert_eq!(session.session().updated_at_ms, 999);
        assert_ne!(session.session().updated_at_ms, original_updated_at);
    }

    #[test]
    fn test_estimate_tokens_from_content_blocks_text() {
        let blocks = vec![HarnessContentBlock::Text { text: "hello world test!".to_string() }];
        let tokens = estimate_tokens_from_content_blocks(&blocks);
        assert_eq!(tokens, 4);
    }

    #[test]
    fn test_estimate_tokens_from_content_blocks_tool_use() {
        let blocks = vec![HarnessContentBlock::ToolUse {
            id: "id-1234".to_string(),
            name: "read_file".to_string(),
            input: "{\"path\": \"/test\"}".to_string(),
        }];
        let tokens = estimate_tokens_from_content_blocks(&blocks);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_tokens_from_content_blocks_tool_result() {
        let blocks = vec![HarnessContentBlock::ToolResult {
            tool_use_id: "tu-1234".to_string(),
            tool_name: "bash".to_string(),
            output: "command output here".to_string(),
            is_error: false,
        }];
        let tokens = estimate_tokens_from_content_blocks(&blocks);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_tokens_from_content_blocks_multiple() {
        let blocks = vec![
            HarnessContentBlock::Text { text: "hello".to_string() },
            HarnessContentBlock::Text { text: "world".to_string() },
        ];
        let tokens = estimate_tokens_from_content_blocks(&blocks);
        assert_eq!(tokens, 2);
    }

    #[test]
    fn test_token_usage_breakdown_zero_tokens_zero_chars() {
        let usage = HarnessTokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: Some(0),
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        assert_eq!(breakdown.total_tokens, 0);
        assert!(!breakdown.estimated_from_chars);
    }

    #[test]
    fn test_token_usage_breakdown_actual_overrides_estimate() {
        let usage = HarnessTokenUsage {
            input_tokens: 50,
            output_tokens: 25,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: Some(0),
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 1000);
        assert_eq!(breakdown.total_tokens, 75);
        assert!(!breakdown.estimated_from_chars);
    }

    #[test]
    fn test_dynamic_max_iterations_matches_complexity() {
        assert_eq!(dynamic_max_iterations(&TaskComplexity::Low), 20);
        assert_eq!(dynamic_max_iterations(&TaskComplexity::Medium), 50);
        assert_eq!(dynamic_max_iterations(&TaskComplexity::High), 100);
    }

    #[test]
    fn test_agent_session_debug() {
        let session = AgentSession::new("p1".to_string(), "c1".to_string());
        let debug_str = format!("{:?}", session);
        assert!(debug_str.contains("p1"));
        assert!(debug_str.contains("c1"));
    }

    #[test]
    fn test_token_usage_breakdown_serialization() {
        let usage = HarnessTokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: Some(0),
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        let json = serde_json::to_string(&breakdown).expect("测试：JSON序列化应成功");
        let deserialized: TokenUsageBreakdown =
            serde_json::from_str(&json).expect("测试：JSON反序列化应成功");
        assert_eq!(deserialized.input_tokens, 100);
        assert_eq!(deserialized.output_tokens, 50);
        assert_eq!(deserialized.total_tokens, 150);
        assert!(!deserialized.estimated_from_chars);
    }

    #[tokio::test]
    async fn test_session_manager_new() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        assert_eq!(mgr.session_count().await, 0);
        assert!(!mgr.has_app_handle().await);
    }

    #[tokio::test]
    async fn test_session_manager_set_default_workspace_dir() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        mgr.set_default_workspace_dir(Some("/tmp/workspace".to_string())).await;
        let default_dir = mgr.default_workspace_dir.lock().await;
        assert_eq!(*default_dir, Some("/tmp/workspace".to_string()));
    }

    #[tokio::test]
    async fn test_session_manager_set_default_workspace_dir_none() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        mgr.set_default_workspace_dir(Some("/tmp/workspace".to_string())).await;
        mgr.set_default_workspace_dir(None).await;
        let default_dir = mgr.default_workspace_dir.lock().await;
        assert!(default_dir.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_has_app_handle_initially_false() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        assert!(!mgr.has_app_handle().await);
    }

    #[tokio::test]
    async fn test_session_manager_get_session_not_found() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        let result = mgr.get_session("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_remove_session_not_found() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        let result = mgr.remove_session("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_create_session() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        let session = mgr
            .create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        assert_eq!(session.provider_id(), "provider-1");
        assert_eq!(session.conversation_id(), "conv-1");
        assert!(session.axagent_session_id().is_some());
        assert_eq!(mgr.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_seed_session_history_fills_empty_session() {
        use axagent_harness::conversation_model::ContentBlock;

        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        mgr.create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");

        let history = vec![
            axagent_harness::ConversationMessage {
                role: axagent_harness::conversation_model::MessageRole::User,
                blocks: vec![ContentBlock::Text { text: "你好".to_string() }],
                usage: None,
            },
            axagent_harness::ConversationMessage {
                role: axagent_harness::conversation_model::MessageRole::Assistant,
                blocks: vec![ContentBlock::Text { text: "答复".to_string() }],
                usage: None,
            },
            // 尾部 user 行应被截断（模拟当前 turn 输入已落库的场景）
            axagent_harness::ConversationMessage {
                role: axagent_harness::conversation_model::MessageRole::User,
                blocks: vec![ContentBlock::Text { text: "当前输入".to_string() }],
                usage: None,
            },
        ];

        assert!(mgr.seed_session_history("conv-1", history).await);

        let session = mgr
            .get_or_create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        // 尾部 user 行被截掉，只保留前两条
        assert_eq!(session.session().messages.len(), 2);
        assert_eq!(
            session.session().messages[0].blocks,
            vec![ContentBlock::Text { text: "你好".to_string() }]
        );
    }

    #[tokio::test]
    async fn test_seed_session_history_skips_non_empty_session() {
        use axagent_harness::conversation_model::ContentBlock;

        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        mgr.create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        // 先 seed 一次（尾部 user 行会被截掉，需带一条 assistant 回复才能入库）
        assert!(
            mgr.seed_session_history(
                "conv-1",
                vec![
                    axagent_harness::ConversationMessage {
                        role: axagent_harness::conversation_model::MessageRole::User,
                        blocks: vec![ContentBlock::Text { text: "第一轮".to_string() }],
                        usage: None,
                    },
                    axagent_harness::ConversationMessage {
                        role: axagent_harness::conversation_model::MessageRole::Assistant,
                        blocks: vec![ContentBlock::Text { text: "第一轮回复".to_string() }],
                        usage: None,
                    },
                ],
            )
            .await
        );
        // 第二次 seed 应幂等跳过（会话已有上下文）
        assert!(
            !mgr.seed_session_history(
                "conv-1",
                vec![axagent_harness::ConversationMessage {
                    role: axagent_harness::conversation_model::MessageRole::User,
                    blocks: vec![ContentBlock::Text { text: "第二轮".to_string() }],
                    usage: None,
                }],
            )
            .await
        );
        let session = mgr
            .get_or_create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        assert_eq!(session.session().messages.len(), 2);
    }

    #[tokio::test]
    async fn test_seed_session_history_unknown_conversation() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        assert!(
            !mgr.seed_session_history(
                "conv-missing",
                vec![axagent_harness::ConversationMessage {
                    role: axagent_harness::conversation_model::MessageRole::User,
                    blocks: vec![axagent_harness::conversation_model::ContentBlock::Text {
                        text: "孤儿".to_string()
                    }],
                    usage: None,
                }],
            )
            .await
        );
    }

    #[tokio::test]
    async fn test_session_manager_create_and_get_session() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        let session = mgr
            .create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        let session_id = session.session().session_id.clone();
        let retrieved = mgr.get_session(&session_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.expect("测试应成功").provider_id(), "provider-1");
    }

    #[tokio::test]
    async fn test_session_manager_create_and_remove_session() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        let session = mgr
            .create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        let session_id = session.session().session_id.clone();
        assert_eq!(mgr.session_count().await, 1);
        let removed = mgr.remove_session(&session_id).await;
        assert!(removed.is_some());
        assert_eq!(mgr.session_count().await, 0);
        assert!(mgr.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_get_or_create_session_new() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        let session = mgr
            .get_or_create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试应成功");
        assert_eq!(session.provider_id(), "provider-1");
        assert_eq!(session.conversation_id(), "conv-1");
    }

    #[tokio::test]
    async fn test_session_manager_get_or_create_session_existing() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        let session1 = mgr
            .get_or_create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试应成功");
        let session2 = mgr
            .get_or_create_session("provider-2".to_string(), "conv-1".to_string())
            .await
            .expect("测试应成功");
        assert_eq!(session1.session().session_id, session2.session().session_id);
        assert_eq!(mgr.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_session_manager_clear_session() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        mgr.create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        assert_eq!(mgr.session_count().await, 1);
        mgr.clear_session("conv-1").await;
        assert_eq!(mgr.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_manager_clear_session_nonexistent() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        mgr.create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        mgr.clear_session("nonexistent").await;
        assert_eq!(mgr.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_session_manager_multiple_sessions() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        mgr.create_session("p1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        mgr.create_session("p2".to_string(), "conv-2".to_string())
            .await
            .expect("测试：异步操作应成功");
        mgr.create_session("p3".to_string(), "conv-3".to_string())
            .await
            .expect("测试：异步操作应成功");
        assert_eq!(mgr.session_count().await, 3);
    }

    #[tokio::test]
    async fn test_session_manager_conversation_index() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        mgr.create_session("p1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        let conv_index = mgr.conversation_index.lock().await;
        assert!(conv_index.contains_key("conv-1"));
    }

    #[tokio::test]
    async fn test_session_manager_session_last_access_updated() {
        let mgr =
            SessionManager::new_for_test(axagent_harness::test_support::empty_agent_session_repo());
        mgr.create_session("p1".to_string(), "conv-1".to_string())
            .await
            .expect("测试：异步操作应成功");
        let session_id = {
            let conv_index = mgr.conversation_index.lock().await;
            conv_index.get("conv-1").cloned().expect("测试应成功")
        };
        let last_access = mgr.session_last_access.lock().await;
        assert!(last_access.contains_key(&session_id));
    }

    #[tokio::test]
    async fn test_channel_permission_prompter_inner_pending_count() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: parking_lot::Mutex::new(std::collections::HashMap::new()),
            always_allowed: parking_lot::Mutex::new(HashSet::new()),
            workspace_root: parking_lot::Mutex::new("/workspace".to_string()),
        };
        let inner = Arc::new(inner);
        assert_eq!(inner.pending_senders.lock().len(), 0);
    }

    #[test]
    fn test_channel_permission_prompter_inner_add_always_allowed() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: parking_lot::Mutex::new(std::collections::HashMap::new()),
            always_allowed: parking_lot::Mutex::new(HashSet::new()),
            workspace_root: parking_lot::Mutex::new("/workspace".to_string()),
        };
        let inner = Arc::new(inner);
        {
            let mut set = inner.always_allowed.lock();
            set.insert("read_file".to_string());
            set.insert("bash".to_string());
        }
        let set = inner.always_allowed.lock();
        assert_eq!(set.len(), 2);
        assert!(set.contains("read_file"));
        assert!(set.contains("bash"));
    }

    #[test]
    fn test_channel_permission_prompter_inner_deliver_decision_no_pending() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: parking_lot::Mutex::new(std::collections::HashMap::new()),
            always_allowed: parking_lot::Mutex::new(HashSet::new()),
            workspace_root: parking_lot::Mutex::new(String::new()),
        };
        let inner = Arc::new(inner);
        let result = {
            let mut map = inner.pending_senders.lock();
            if let Some(sender) = map.remove("nonexistent") {
                sender.send(PermissionPromptDecision::Allow).is_ok()
            } else {
                false
            }
        };
        assert!(!result);
    }

    #[test]
    fn test_channel_permission_prompter_inner_clear_pending() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: parking_lot::Mutex::new(std::collections::HashMap::new()),
            always_allowed: parking_lot::Mutex::new(HashSet::new()),
            workspace_root: parking_lot::Mutex::new(String::new()),
        };
        let inner = Arc::new(inner);
        let (tx, rx) = std::sync::mpsc::channel::<PermissionPromptDecision>();
        {
            let mut map = inner.pending_senders.lock();
            map.insert("req-1".to_string(), tx);
        }
        {
            let mut map = inner.pending_senders.lock();
            assert_eq!(map.len(), 1);
            map.clear();
        }
        {
            let map = inner.pending_senders.lock();
            assert!(map.is_empty());
        }
        drop(rx);
    }

    #[test]
    fn test_channel_permission_prompter_inner_deliver_decision_success() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: parking_lot::Mutex::new(std::collections::HashMap::new()),
            always_allowed: parking_lot::Mutex::new(HashSet::new()),
            workspace_root: parking_lot::Mutex::new(String::new()),
        };
        let inner = Arc::new(inner);
        let (tx, rx) = std::sync::mpsc::channel::<PermissionPromptDecision>();
        {
            let mut map = inner.pending_senders.lock();
            map.insert("req-1".to_string(), tx);
        }
        let result = {
            let mut map = inner.pending_senders.lock();
            if let Some(sender) = map.remove("req-1") {
                sender.send(PermissionPromptDecision::Allow).is_ok()
            } else {
                false
            }
        };
        assert!(result);
        let decision = rx.recv_timeout(std::time::Duration::from_millis(100)).expect("测试应成功");
        assert!(matches!(decision, PermissionPromptDecision::Allow));
    }

    #[test]
    fn test_channel_permission_prompter_inner_always_allowed_check() {
        let mut allowed_set = HashSet::new();
        allowed_set.insert("read_file".to_string());
        let inner = ChannelPermissionPrompterInner {
            pending_senders: parking_lot::Mutex::new(std::collections::HashMap::new()),
            always_allowed: parking_lot::Mutex::new(allowed_set),
            workspace_root: parking_lot::Mutex::new(String::new()),
        };
        let inner = Arc::new(inner);
        let set = inner.always_allowed.lock();
        assert!(set.contains("read_file"));
        assert!(!set.contains("bash"));
    }

    #[test]
    fn test_channel_permission_prompter_inner_workspace_root() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: parking_lot::Mutex::new(std::collections::HashMap::new()),
            always_allowed: parking_lot::Mutex::new(HashSet::new()),
            workspace_root: parking_lot::Mutex::new("/my/workspace".to_string()),
        };
        let inner = Arc::new(inner);
        let root = inner.workspace_root.lock();
        assert_eq!(*root, "/my/workspace");
    }

    #[test]
    fn test_estimate_tokens_from_messages_mixed_blocks() {
        let messages = vec![
            HarnessConversationMessage {
                role: HarnessMessageRole::User,
                blocks: vec![HarnessContentBlock::Text { text: "hello".to_string() }],
                usage: None,
            },
            HarnessConversationMessage {
                role: HarnessMessageRole::Assistant,
                blocks: vec![
                    HarnessContentBlock::Text { text: "response text here".to_string() },
                    HarnessContentBlock::ToolUse {
                        id: "id-1".to_string(),
                        name: "bash".to_string(),
                        input: "{\"command\":\"ls\"}".to_string(),
                    },
                ],
                usage: None,
            },
        ];
        let tokens = estimate_tokens_from_messages(&messages);
        assert!(tokens > 0);
    }

    #[test]
    fn test_token_usage_breakdown_negative_delta() {
        let usage = HarnessTokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: Some(0),
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        assert_eq!(breakdown.tokens_delta(), 0);
    }

    #[test]
    fn test_token_usage_breakdown_large_values() {
        let usage = HarnessTokenUsage {
            input_tokens: 100000,
            output_tokens: 50000,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_miss_input_tokens: Some(0),
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        assert_eq!(breakdown.total_tokens, 150000);
        assert_eq!(breakdown.tokens_delta(), 150000);
    }

    #[test]
    fn test_estimate_tokens_from_content_blocks_empty() {
        let blocks: Vec<HarnessContentBlock> = vec![];
        assert_eq!(estimate_tokens_from_content_blocks(&blocks), 0);
    }

    #[test]
    fn test_estimate_tokens_from_messages_multiple_messages() {
        let messages = vec![
            HarnessConversationMessage {
                role: HarnessMessageRole::User,
                blocks: vec![HarnessContentBlock::Text { text: "first message".to_string() }],
                usage: None,
            },
            HarnessConversationMessage {
                role: HarnessMessageRole::Assistant,
                blocks: vec![HarnessContentBlock::Text { text: "second message".to_string() }],
                usage: None,
            },
            HarnessConversationMessage {
                role: HarnessMessageRole::User,
                blocks: vec![HarnessContentBlock::Text { text: "third message here".to_string() }],
                usage: None,
            },
        ];
        let tokens = estimate_tokens_from_messages(&messages);
        assert!(tokens >= 3);
    }
}
