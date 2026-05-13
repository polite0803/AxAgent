//! Session Manager for AxAgent Agent

use crate::event_bus::AgentPermissionPayload;
use crate::provider_adapter::AxAgentApiClient;
use crate::shared_blackboard::SharedBlackboard;
use crate::ToolRegistry;
use axagent_core::repo::agent_session;
use axagent_prompt_guard::{GuardConfig, PromptGuardPipeline};
use axagent_runtime_core::{
    compact_session, should_compact, CompactionConfig, ContentBlock, ConversationMessage,
    ConversationRuntime, HookEvent, HookProgressEvent, HookProgressReporter, MessageRole,
    PermissionMode, PermissionPolicy, PermissionPromptDecision, PermissionPrompter,
    PermissionRequest, RuntimeError, Session,
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
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
    pub fn from_turn_summary(
        usage: &axagent_runtime_core::TokenUsage,
        estimated_chars: usize,
    ) -> Self {
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

    pub fn tokens_delta(&self) -> i32 {
        self.total_tokens as i32
    }
}

pub fn estimate_tokens_from_text(text: &str) -> usize {
    text.len() / TOKEN_ESTIMATION_CHARS_PER_TOKEN
}

pub fn estimate_tokens_from_messages(
    messages: &[axagent_runtime_core::ConversationMessage],
) -> usize {
    messages
        .iter()
        .map(|m| estimate_tokens_from_content_blocks(&m.blocks))
        .sum()
}

fn estimate_tokens_from_content_blocks(blocks: &[axagent_runtime_core::ContentBlock]) -> usize {
    blocks
        .iter()
        .map(|block| match block {
            axagent_runtime_core::ContentBlock::Text { text } => estimate_tokens_from_text(text),
            axagent_runtime_core::ContentBlock::ToolUse { id, name, input } => {
                estimate_tokens_from_text(id)
                    + estimate_tokens_from_text(name)
                    + estimate_tokens_from_text(input)
            },
            axagent_runtime_core::ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                ..
            } => {
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
pub fn dynamic_max_iterations(complexity: &axagent_trajectory::Complexity) -> usize {
    match complexity {
        axagent_trajectory::Complexity::Low => 20,
        axagent_trajectory::Complexity::Medium => 50,
        axagent_trajectory::Complexity::High => 100,
    }
}

/// 处理并追加用户消息到 agent 会话（带提示词注入防护）
///
/// 使用 PromptGuardPipeline 对用户输入进行多层过滤后，
/// 直接推送已处理的消息到 Session，避免 push_user_text 的二次包装。
pub fn append_user_message(session: &mut Session, text: &str) -> Result<(), String> {
    let config = GuardConfig::default();
    let pipeline = PromptGuardPipeline::new(config);
    let processed = pipeline.process_user_input(text)?;
    // 直接推送已处理的消息，避免 push_user_text 的二次包装
    session
        .push_message(ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text { text: processed }],
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
        }
    }

    pub fn with_team(mut self, team_id: String) -> Self {
        self.team_id = Some(team_id);
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
    pub fn record_to_blackboard(&self, field: &str, value: &str) {
        if let Some(ref bb) = self.blackboard {
            if let Ok(mut board) = bb.write() {
                board.record_decision(
                    &self.axagent_session_id.clone().unwrap_or_default(),
                    &self.conversation_id,
                    field,
                    value,
                );
            }
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
    db: Arc<DatabaseConnection>,
    app_handle: std::sync::Mutex<Option<AppHandle>>,
    default_workspace_dir: std::sync::Mutex<Option<String>>,
}

/// Maximum number of sessions to keep in memory (LRU eviction).
const MAX_CACHED_SESSIONS: usize = 100;
/// Time-to-live for idle sessions (24 hours in seconds).
const SESSION_TTL_SECS: u64 = 24 * 60 * 60;

impl SessionManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            sessions: Mutex::new(std::collections::HashMap::new()),
            conversation_index: Mutex::new(std::collections::HashMap::new()),
            session_last_access: Mutex::new(std::collections::HashMap::new()),
            db: Arc::new(db),
            app_handle: std::sync::Mutex::new(None),
            default_workspace_dir: std::sync::Mutex::new(None),
        }
    }

    pub fn set_default_workspace_dir(&self, dir: Option<String>) {
        let mut default_workspace_dir = self.default_workspace_dir.lock().unwrap();
        *default_workspace_dir = dir;
    }

    pub fn set_app_handle(&self, app_handle: AppHandle) {
        let mut handle = self.app_handle.lock().unwrap();
        *handle = Some(app_handle);
    }

    pub fn has_app_handle(&self) -> bool {
        self.app_handle.lock().unwrap().is_some()
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
            let conv_index = self.conversation_index.lock().await;
            if let Some(session_id) = conv_index.get(&conversation_id) {
                let sessions = self.sessions.lock().await;
                if let Some(existing) = sessions.get(session_id) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    self.session_last_access
                        .lock()
                        .await
                        .insert(session_id.clone(), now);
                    return Ok(existing.clone());
                }
            }
        }

        self.create_session(provider_id, conversation_id).await
    }

    pub async fn create_session(
        &self,
        provider_id: String,
        conversation_id: String,
    ) -> Result<AgentSession, String> {
        let mut session = AgentSession::new(provider_id, conversation_id.clone());
        let session_id = session.session().session_id.clone();

        let default_workspace_dir = {
            let guard = self.default_workspace_dir.lock().unwrap();
            guard.clone()
        };

        let cwd_to_use = if session.session().workspace_root.is_none() {
            default_workspace_dir.as_deref()
        } else {
            session
                .session()
                .workspace_root
                .as_ref()
                .map(|p| p.to_str().unwrap_or(""))
        };

        let axagent_session = agent_session::upsert_agent_session(
            &self.db,
            &conversation_id,
            cwd_to_use,
            Some("default"),
        )
        .await
        .map_err(|e| e.to_string())?;

        session = session.with_axagent_session_id(axagent_session.id);

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session.clone());

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.session_last_access
            .lock()
            .await
            .insert(session_id.clone(), now);

        let mut conv_index = self.conversation_index.lock().await;
        conv_index.insert(conversation_id, session_id);

        Ok(session)
    }

    /// Update the session in memory after a turn completes, preserving conversation history.
    pub async fn update_session_after_turn(&self, conversation_id: &str, updated_session: Session) {
        let conv_index = self.conversation_index.lock().await;
        if let Some(session_id) = conv_index.get(conversation_id) {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.session_mut().messages = updated_session.messages;
                session.session_mut().updated_at_ms = updated_session.updated_at_ms;

                // Update AxAgent's agent_sessions table
                if let Some(axagent_session_id) = session.axagent_session_id() {
                    let db = self.db.clone();
                    let axagent_sid = axagent_session_id.to_string();
                    // Use a rough token estimate from message count
                    let tokens_delta = 0; // Will be updated by caller

                    drop(sessions);
                    drop(conv_index);

                    let _ = agent_session::update_agent_session_after_query(
                        &db,
                        &axagent_sid,
                        "idle",
                        None,
                        tokens_delta,
                        0.0,
                    )
                    .await;
                }
            }
        }
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

            let _ = agent_session::update_agent_session_status(&self.db, &session_id, "idle").await;
            let _ = agent_session::clear_sdk_context_by_conversation_id(&self.db, conversation_id)
                .await;
        }
    }

    /// Evict sessions that exceed the TTL or LRU limit.
    /// Called automatically by `get_or_create_session`.
    /// Lock order: conversation_index → sessions → session_last_access (consistent with other methods).
    async fn evict_stale_sessions(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let ttl_cutoff = now.saturating_sub(SESSION_TTL_SECS);
        let ttl_cutoff_ms = ttl_cutoff * 1000;

        // Find session_ids to evict: TTL expired (only need session_last_access)
        let mut to_evict = Vec::new();
        {
            let last_access = self.session_last_access.lock().await;
            for (session_id, &last_ms) in last_access.iter() {
                if last_ms < ttl_cutoff_ms {
                    to_evict.push(session_id.clone());
                }
            }
        }

        // LRU eviction: if still over limit after TTL cleanup, evict oldest
        {
            let last_access = self.session_last_access.lock().await;
            let sessions = self.sessions.lock().await;
            if sessions.len() > MAX_CACHED_SESSIONS {
                // Collect all sessions with their access times, sort by oldest first
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

        // Perform eviction with consistent lock order: conversation_index → sessions → session_last_access
        if !to_evict.is_empty() {
            info!("[SessionManager] Evicting {} stale sessions", to_evict.len());
            let mut conv_index = self.conversation_index.lock().await;
            let mut sessions = self.sessions.lock().await;
            let mut last_access = self.session_last_access.lock().await;
            for session_id in to_evict {
                sessions.remove(&session_id);
                last_access.remove(&session_id);
                // Also remove from conversation_index (reverse lookup)
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
    /// - Building the `AxAgentApiClient` with tools, model, params, and streaming callbacks
    /// - Persisting user/assistant messages to the DB
    /// - Emitting Tauri events
    pub async fn run_turn_with_tools(
        &self,
        session_id: &str,
        user_input: String,
        api_client: AxAgentApiClient,
        tool_registry: ToolRegistry,
        system_prompt: Vec<String>,
        conversation_id: String,
        permission_mode: PermissionMode,
        prompters: Arc<
            tokio::sync::Mutex<std::collections::HashMap<String, ChannelPermissionPrompter>>,
        >,
        cancel_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<(axagent_runtime_core::TurnSummary, axagent_runtime_core::Session), RuntimeError>
    {
        let session = self
            .get_session(session_id)
            .await
            .ok_or_else(|| RuntimeError::new(format!("Session not found: {}", session_id)))?;

        // Auto-compact if the session exceeds the token threshold.
        // Use CompactionConfig::default() consistently for both the check
        // and the compaction to avoid configuration mismatch.
        let compaction_config = CompactionConfig::default();
        let mut session = if should_compact(session.session(), compaction_config) {
            let result = compact_session(session.session(), compaction_config);

            // Build MessageRecords for integrity verification
            let original_msgs: Vec<axagent_trajectory::MessageRecord> = session
                .session()
                .messages
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let role_str = match m.role {
                        axagent_runtime_core::MessageRole::System => "system",
                        axagent_runtime_core::MessageRole::User => "user",
                        axagent_runtime_core::MessageRole::Assistant => "assistant",
                        axagent_runtime_core::MessageRole::Tool => "tool",
                    };
                    let content: String = m
                        .blocks
                        .iter()
                        .map(|b| match b {
                            axagent_runtime_core::ContentBlock::Text { text } => text.clone(),
                            axagent_runtime_core::ContentBlock::ToolUse { name, input, .. } => {
                                format!("[ToolUse: {} {}]", name, input)
                            },
                            axagent_runtime_core::ContentBlock::ToolResult {
                                tool_name,
                                output,
                                ..
                            } => format!("[ToolResult: {} {}]", tool_name, output),
                        })
                        .collect();
                    axagent_trajectory::MessageRecord {
                        id: format!("orig-{}", i),
                        role: role_str.to_string(),
                        content,
                        message_type: None,
                        timestamp: i as i64,
                        tool_calls: Some(m.blocks.iter().any(|b| {
                            matches!(b, axagent_runtime_core::ContentBlock::ToolUse { .. })
                        })),
                    }
                })
                .collect();
            let compressed_msgs: Vec<axagent_trajectory::MessageRecord> = result
                .compacted_session
                .messages
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let role_str = match m.role {
                        axagent_runtime_core::MessageRole::System => "system",
                        axagent_runtime_core::MessageRole::User => "user",
                        axagent_runtime_core::MessageRole::Assistant => "assistant",
                        axagent_runtime_core::MessageRole::Tool => "tool",
                    };
                    let content: String = m
                        .blocks
                        .iter()
                        .map(|b| match b {
                            axagent_runtime_core::ContentBlock::Text { text } => text.clone(),
                            axagent_runtime_core::ContentBlock::ToolUse { name, input, .. } => {
                                format!("[ToolUse: {} {}]", name, input)
                            },
                            axagent_runtime_core::ContentBlock::ToolResult {
                                tool_name,
                                output,
                                ..
                            } => format!("[ToolResult: {} {}]", tool_name, output),
                        })
                        .collect();
                    axagent_trajectory::MessageRecord {
                        id: format!("comp-{}", i),
                        role: role_str.to_string(),
                        content,
                        message_type: None,
                        timestamp: i as i64,
                        tool_calls: Some(m.blocks.iter().any(|b| {
                            matches!(b, axagent_runtime_core::ContentBlock::ToolUse { .. })
                        })),
                    }
                })
                .collect();

            // Use SessionCompactor to extract key entities for enhanced integrity verification
            let compactor = axagent_trajectory::SessionCompactor::new();
            let key_entities = compactor.extract_entities(&original_msgs);

            let integrity = axagent_trajectory::verify_compression_integrity(
                &original_msgs,
                &compressed_msgs,
                &key_entities,
            );
            if !integrity.is_valid {
                let failed_checks: Vec<&str> = integrity
                    .checks
                    .iter()
                    .filter(|c| !c.passed)
                    .map(|c| c.name.as_str())
                    .collect();
                info!("Compression integrity warning: failed checks: {:?}", failed_checks);
            } else {
                info!("Compression integrity verified: all {} checks passed ({} key entities tracked)", integrity.checks.len(), key_entities.len());
            }

            let mut compacted = session;
            compacted.session_mut().messages = result.compacted_session.messages;
            compacted
        } else {
            session
        };

        // Create permission policy from the provided mode
        let permission_policy = PermissionPolicy::new(permission_mode);

        // Get app handle for event emission
        let app_handle = self.app_handle.lock().unwrap().clone();

        // Create runtime with ToolRegistry and progress reporter
        let mut runtime = ConversationRuntime::new(
            session.session().clone(),
            api_client,
            tool_registry,
            permission_policy,
            system_prompt,
        )
        .with_max_iterations(dynamic_max_iterations(
            &axagent_trajectory::estimate_complexity_public(&user_input),
        ))
        .with_auto_compaction_input_tokens_threshold(AUTO_COMPACTION_TOKEN_THRESHOLD as u32);

        // Attach cancel token if provided
        if let Some(token) = cancel_token {
            runtime = runtime.with_cancel_token(token);
        }

        // Add Tauri event reporter for tool progress
        if let Some(handle) = app_handle {
            let reporter =
                Box::new(TauriHookProgressReporter::new(handle, conversation_id.clone()));
            runtime = runtime.with_hook_progress_reporter(reporter);
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
        let conv_id_for_prompter = conversation_id.clone();
        let mut prompter_opt = prompters.lock().await.remove(&conv_id_for_prompter);
        let summary = tokio::task::block_in_place(|| {
            if let Some(ref mut p) = prompter_opt {
                runtime.run_turn(user_input, Some(p))
            } else {
                runtime.run_turn(user_input, None)
            }
        })?;
        // Re-register the prompter (it may still be needed for subsequent approvals)
        if let Some(p) = prompter_opt {
            prompters.lock().await.insert(conv_id_for_prompter, p);
        }

        // Extract updated session
        let updated_session = runtime.into_session();
        session.session_mut().messages = updated_session.messages.clone();
        session.session_mut().updated_at_ms = updated_session.updated_at_ms;

        // Persist updates
        if let Some(axagent_session_id) = session.axagent_session_id() {
            let tokens_delta =
                summary.usage.input_tokens as i32 + summary.usage.output_tokens as i32;
            // Cost is now calculated in agent_query command and emitted via agent-done event.
            // The DB field is kept for historical records; we store 0.0 here as the
            // authoritative cost comes from the event payload.
            let cost_delta = 0.0;

            let _ = agent_session::update_agent_session_after_query(
                &self.db,
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

        Ok((summary, updated_session))
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

struct ChannelPermissionPrompterInner {
    /// Maps request_id → Sender that agent_approve will use to unblock.
    pending_senders: std::sync::Mutex<
        std::collections::HashMap<String, std::sync::mpsc::Sender<PermissionPromptDecision>>,
    >,
    /// Tools the user has marked "always allow" for this conversation.
    always_allowed: std::sync::Mutex<HashSet<String>>,
    /// Workspace root directory for file write boundary checks.
    workspace_root: std::sync::Mutex<String>,
}

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
                pending_senders: std::sync::Mutex::new(std::collections::HashMap::new()),
                always_allowed: std::sync::Mutex::new(always_allowed),
                workspace_root: std::sync::Mutex::new(workspace_root),
            }),
        }
    }

    /// Returns the number of pending permission requests.
    pub fn pending_count(&self) -> usize {
        self.inner
            .pending_senders
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    /// Register a sender for a pending request. Called by `agent_approve` command
    /// to deliver the user's decision.
    pub fn deliver_decision(&self, request_id: &str, decision: PermissionPromptDecision) -> bool {
        if let Ok(mut map) = self.inner.pending_senders.lock() {
            if let Some(sender) = map.remove(request_id) {
                sender.send(decision).is_ok()
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Add a tool to the "always allowed" set for this conversation.
    pub fn add_always_allowed(&self, tool_name: &str) {
        if let Ok(mut set) = self.inner.always_allowed.lock() {
            set.insert(tool_name.to_string());
        }
    }

    /// Get the current "always allowed" set.
    pub fn get_always_allowed(&self) -> HashSet<String> {
        self.inner
            .always_allowed
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Clean up any stale pending senders (e.g. on conversation switch).
    pub fn clear_pending(&self) {
        if let Ok(mut map) = self.inner.pending_senders.lock() {
            map.clear();
        }
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
        // Check "always allowed" first
        if let Ok(set) = self.inner.always_allowed.lock() {
            if set.contains(&request.tool_name) {
                info!(
                    "[ChannelPermissionPrompter] Auto-allowing '{}' (always allowed)",
                    request.tool_name
                );
                return PermissionPromptDecision::Allow;
            }
        }

        // Fine-grained enforcement checks before prompting the user.
        // These catch operations that should be hard-denied regardless of user choice
        // (e.g., writing outside workspace, dangerous bash commands in read-only mode).
        let enforcer = axagent_runtime_core::permission_enforcer::PermissionEnforcer::new(
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
                    let workspace_root = self
                        .inner
                        .workspace_root
                        .lock()
                        .map(|s| s.clone())
                        .unwrap_or_default();
                    if !workspace_root.is_empty() {
                        let result = enforcer.check_file_write(path, &workspace_root);
                        if let axagent_runtime_core::permission_enforcer::EnforcementResult::Denied {
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
        if tool_name_lower.contains("bash")
            || tool_name_lower.contains("shell")
            || tool_name_lower.contains("exec")
            || tool_name_lower.contains("run")
        {
            if let Ok(input_val) = serde_json::from_str::<serde_json::Value>(&request.input) {
                let command = input_val
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !command.is_empty() {
                    let result = enforcer.check_bash(command);
                    if let axagent_runtime_core::permission_enforcer::EnforcementResult::Denied {
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
        if let Ok(mut map) = self.inner.pending_senders.lock() {
            map.insert(request_id.clone(), tx);
        } else {
            return PermissionPromptDecision::Deny {
                reason: "Internal error: failed to register permission sender".to_string(),
            };
        }

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
                if let Ok(mut map) = self.inner.pending_senders.lock() {
                    map.remove(&request_id);
                }
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
                if let Ok(mut map) = self.inner.pending_senders.lock() {
                    map.remove(&request_id);
                }
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
}

impl TauriHookProgressReporter {
    pub fn new(app_handle: AppHandle, conversation_id: String) -> Self {
        Self {
            app_handle,
            conversation_id,
        }
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
                let _ = self.app_handle.emit(
                    "agent-tool-start",
                    serde_json::json!({
                        "conversationId": conversation_id,
                        "toolUseId": tool_use_id.as_deref().unwrap_or(""),
                        "toolName": tool_name,
                        "input": serde_json::Value::Null,
                        "assistantMessageId": "",
                    }),
                );
            },
            HookProgressEvent::Completed {
                event: HookEvent::PostToolUse,
                tool_name,
                command: _,
                tool_use_id,
            } => {
                let _ = self.app_handle.emit(
                    "agent-tool-result",
                    serde_json::json!({
                        "conversationId": conversation_id,
                        "toolUseId": tool_use_id.as_deref().unwrap_or(""),
                        "toolName": tool_name,
                        "input": serde_json::Value::Null,
                        "content": "",
                        "isError": false,
                        "assistantMessageId": "",
                    }),
                );
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
                let _ = self.app_handle.emit(
                    "agent-tool-result",
                    serde_json::json!({
                        "conversationId": conversation_id,
                        "toolUseId": tool_use_id.as_deref().unwrap_or(""),
                        "toolName": tool_name,
                        "input": serde_json::Value::Null,
                        "content": "",
                        "isError": true,
                        "assistantMessageId": "",
                    }),
                );
            },
            _ => {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let messages: Vec<axagent_runtime_core::ConversationMessage> = vec![];
        assert_eq!(estimate_tokens_from_messages(&messages), 0);
    }

    #[test]
    fn test_estimate_tokens_from_messages_with_text() {
        use axagent_runtime_core::{ContentBlock, ConversationMessage, MessageRole};
        let messages = vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "hello world".to_string(),
            }],
            usage: None,
        }];
        assert_eq!(estimate_tokens_from_messages(&messages), 2);
    }

    #[test]
    fn test_estimate_tokens_from_messages_with_tool_use() {
        use axagent_runtime_core::{ContentBlock, ConversationMessage, MessageRole};
        let messages = vec![ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::ToolUse {
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
        use axagent_runtime_core::{ContentBlock, ConversationMessage, MessageRole};
        let messages = vec![ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
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
        assert_eq!(dynamic_max_iterations(&axagent_trajectory::Complexity::Low), 20);
    }

    #[test]
    fn test_dynamic_max_iterations_medium() {
        assert_eq!(dynamic_max_iterations(&axagent_trajectory::Complexity::Medium), 50);
    }

    #[test]
    fn test_dynamic_max_iterations_high() {
        assert_eq!(dynamic_max_iterations(&axagent_trajectory::Complexity::High), 100);
    }

    #[test]
    fn test_token_usage_breakdown_with_actual_usage() {
        let usage = axagent_runtime_core::TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        assert_eq!(breakdown.input_tokens, 100);
        assert_eq!(breakdown.output_tokens, 50);
        assert_eq!(breakdown.total_tokens, 150);
        assert!(!breakdown.estimated_from_chars);
    }

    #[test]
    fn test_token_usage_breakdown_with_estimated_chars() {
        let usage = axagent_runtime_core::TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 400);
        assert_eq!(breakdown.total_tokens, 100);
        assert!(breakdown.estimated_from_chars);
    }

    #[test]
    fn test_token_usage_breakdown_tokens_delta() {
        let usage = axagent_runtime_core::TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
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
        let blocks = vec![axagent_runtime_core::ContentBlock::Text {
            text: "hello world test!".to_string(),
        }];
        let tokens = estimate_tokens_from_content_blocks(&blocks);
        assert_eq!(tokens, 4);
    }

    #[test]
    fn test_estimate_tokens_from_content_blocks_tool_use() {
        let blocks = vec![axagent_runtime_core::ContentBlock::ToolUse {
            id: "id-1234".to_string(),
            name: "read_file".to_string(),
            input: "{\"path\": \"/test\"}".to_string(),
        }];
        let tokens = estimate_tokens_from_content_blocks(&blocks);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_tokens_from_content_blocks_tool_result() {
        let blocks = vec![axagent_runtime_core::ContentBlock::ToolResult {
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
            axagent_runtime_core::ContentBlock::Text {
                text: "hello".to_string(),
            },
            axagent_runtime_core::ContentBlock::Text {
                text: "world".to_string(),
            },
        ];
        let tokens = estimate_tokens_from_content_blocks(&blocks);
        assert_eq!(tokens, 2);
    }

    #[test]
    fn test_token_usage_breakdown_zero_tokens_zero_chars() {
        let usage = axagent_runtime_core::TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        assert_eq!(breakdown.total_tokens, 0);
        assert!(!breakdown.estimated_from_chars);
    }

    #[test]
    fn test_token_usage_breakdown_actual_overrides_estimate() {
        let usage = axagent_runtime_core::TokenUsage {
            input_tokens: 50,
            output_tokens: 25,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 1000);
        assert_eq!(breakdown.total_tokens, 75);
        assert!(!breakdown.estimated_from_chars);
    }

    #[test]
    fn test_dynamic_max_iterations_matches_complexity() {
        assert_eq!(dynamic_max_iterations(&axagent_trajectory::Complexity::Low), 20);
        assert_eq!(dynamic_max_iterations(&axagent_trajectory::Complexity::Medium), 50);
        assert_eq!(dynamic_max_iterations(&axagent_trajectory::Complexity::High), 100);
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
        let usage = axagent_runtime_core::TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        let json = serde_json::to_string(&breakdown).unwrap();
        let deserialized: TokenUsageBreakdown = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.input_tokens, 100);
        assert_eq!(deserialized.output_tokens, 50);
        assert_eq!(deserialized.total_tokens, 150);
        assert!(!deserialized.estimated_from_chars);
    }

    async fn setup_test_db() -> DatabaseConnection {
        use axagent_migration::MigratorTrait;
        let db = sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
            .await
            .unwrap();
        axagent_migration::Migrator::up(&db, None).await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_session_manager_new() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        assert_eq!(mgr.session_count().await, 0);
        assert!(!mgr.has_app_handle());
    }

    #[tokio::test]
    async fn test_session_manager_set_default_workspace_dir() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        mgr.set_default_workspace_dir(Some("/tmp/workspace".to_string()));
        let default_dir = mgr.default_workspace_dir.lock().unwrap();
        assert_eq!(*default_dir, Some("/tmp/workspace".to_string()));
    }

    #[tokio::test]
    async fn test_session_manager_set_default_workspace_dir_none() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        mgr.set_default_workspace_dir(Some("/tmp/workspace".to_string()));
        mgr.set_default_workspace_dir(None);
        let default_dir = mgr.default_workspace_dir.lock().unwrap();
        assert!(default_dir.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_has_app_handle_initially_false() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        assert!(!mgr.has_app_handle());
    }

    #[tokio::test]
    async fn test_session_manager_get_session_not_found() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        let result = mgr.get_session("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_remove_session_not_found() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        let result = mgr.remove_session("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_create_session() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        let session = mgr
            .create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        assert_eq!(session.provider_id(), "provider-1");
        assert_eq!(session.conversation_id(), "conv-1");
        assert!(session.axagent_session_id().is_some());
        assert_eq!(mgr.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_session_manager_create_and_get_session() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        let session = mgr
            .create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        let session_id = session.session().session_id.clone();
        let retrieved = mgr.get_session(&session_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().provider_id(), "provider-1");
    }

    #[tokio::test]
    async fn test_session_manager_create_and_remove_session() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        let session = mgr
            .create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        let session_id = session.session().session_id.clone();
        assert_eq!(mgr.session_count().await, 1);
        let removed = mgr.remove_session(&session_id).await;
        assert!(removed.is_some());
        assert_eq!(mgr.session_count().await, 0);
        assert!(mgr.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_get_or_create_session_new() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        let session = mgr
            .get_or_create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        assert_eq!(session.provider_id(), "provider-1");
        assert_eq!(session.conversation_id(), "conv-1");
    }

    #[tokio::test]
    async fn test_session_manager_get_or_create_session_existing() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        let session1 = mgr
            .get_or_create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        let session2 = mgr
            .get_or_create_session("provider-2".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        assert_eq!(session1.session().session_id, session2.session().session_id);
        assert_eq!(mgr.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_session_manager_clear_session() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        mgr.create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        assert_eq!(mgr.session_count().await, 1);
        mgr.clear_session("conv-1").await;
        assert_eq!(mgr.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_manager_clear_session_nonexistent() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        mgr.create_session("provider-1".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        mgr.clear_session("nonexistent").await;
        assert_eq!(mgr.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_session_manager_multiple_sessions() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        mgr.create_session("p1".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        mgr.create_session("p2".to_string(), "conv-2".to_string())
            .await
            .unwrap();
        mgr.create_session("p3".to_string(), "conv-3".to_string())
            .await
            .unwrap();
        assert_eq!(mgr.session_count().await, 3);
    }

    #[tokio::test]
    async fn test_session_manager_conversation_index() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        mgr.create_session("p1".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        let conv_index = mgr.conversation_index.lock().await;
        assert!(conv_index.contains_key("conv-1"));
    }

    #[tokio::test]
    async fn test_session_manager_session_last_access_updated() {
        let db = setup_test_db().await;
        let mgr = SessionManager::new(db);
        mgr.create_session("p1".to_string(), "conv-1".to_string())
            .await
            .unwrap();
        let session_id = {
            let conv_index = mgr.conversation_index.lock().await;
            conv_index.get("conv-1").cloned().unwrap()
        };
        let last_access = mgr.session_last_access.lock().await;
        assert!(last_access.contains_key(&session_id));
    }

    #[tokio::test]
    async fn test_channel_permission_prompter_inner_pending_count() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: std::sync::Mutex::new(std::collections::HashMap::new()),
            always_allowed: std::sync::Mutex::new(HashSet::new()),
            workspace_root: std::sync::Mutex::new("/workspace".to_string()),
        };
        let inner = Arc::new(inner);
        assert_eq!(
            inner
                .pending_senders
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
            0
        );
    }

    #[test]
    fn test_channel_permission_prompter_inner_add_always_allowed() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: std::sync::Mutex::new(std::collections::HashMap::new()),
            always_allowed: std::sync::Mutex::new(HashSet::new()),
            workspace_root: std::sync::Mutex::new("/workspace".to_string()),
        };
        let inner = Arc::new(inner);
        {
            let mut set = inner.always_allowed.lock().unwrap();
            set.insert("read_file".to_string());
            set.insert("bash".to_string());
        }
        let set = inner.always_allowed.lock().unwrap();
        assert_eq!(set.len(), 2);
        assert!(set.contains("read_file"));
        assert!(set.contains("bash"));
    }

    #[test]
    fn test_channel_permission_prompter_inner_deliver_decision_no_pending() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: std::sync::Mutex::new(std::collections::HashMap::new()),
            always_allowed: std::sync::Mutex::new(HashSet::new()),
            workspace_root: std::sync::Mutex::new(String::new()),
        };
        let inner = Arc::new(inner);
        let result = {
            let mut map = inner.pending_senders.lock().unwrap();
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
            pending_senders: std::sync::Mutex::new(std::collections::HashMap::new()),
            always_allowed: std::sync::Mutex::new(HashSet::new()),
            workspace_root: std::sync::Mutex::new(String::new()),
        };
        let inner = Arc::new(inner);
        let (tx, rx) = std::sync::mpsc::channel::<PermissionPromptDecision>();
        {
            let mut map = inner.pending_senders.lock().unwrap();
            map.insert("req-1".to_string(), tx);
        }
        {
            let mut map = inner.pending_senders.lock().unwrap();
            assert_eq!(map.len(), 1);
            map.clear();
        }
        {
            let map = inner.pending_senders.lock().unwrap();
            assert!(map.is_empty());
        }
        drop(rx);
    }

    #[test]
    fn test_channel_permission_prompter_inner_deliver_decision_success() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: std::sync::Mutex::new(std::collections::HashMap::new()),
            always_allowed: std::sync::Mutex::new(HashSet::new()),
            workspace_root: std::sync::Mutex::new(String::new()),
        };
        let inner = Arc::new(inner);
        let (tx, rx) = std::sync::mpsc::channel::<PermissionPromptDecision>();
        {
            let mut map = inner.pending_senders.lock().unwrap();
            map.insert("req-1".to_string(), tx);
        }
        let result = {
            let mut map = inner.pending_senders.lock().unwrap();
            if let Some(sender) = map.remove("req-1") {
                sender.send(PermissionPromptDecision::Allow).is_ok()
            } else {
                false
            }
        };
        assert!(result);
        let decision = rx
            .recv_timeout(std::time::Duration::from_millis(100))
            .unwrap();
        assert!(matches!(decision, PermissionPromptDecision::Allow));
    }

    #[test]
    fn test_channel_permission_prompter_inner_always_allowed_check() {
        let mut allowed_set = HashSet::new();
        allowed_set.insert("read_file".to_string());
        let inner = ChannelPermissionPrompterInner {
            pending_senders: std::sync::Mutex::new(std::collections::HashMap::new()),
            always_allowed: std::sync::Mutex::new(allowed_set),
            workspace_root: std::sync::Mutex::new(String::new()),
        };
        let inner = Arc::new(inner);
        let set = inner.always_allowed.lock().unwrap();
        assert!(set.contains("read_file"));
        assert!(!set.contains("bash"));
    }

    #[test]
    fn test_channel_permission_prompter_inner_workspace_root() {
        let inner = ChannelPermissionPrompterInner {
            pending_senders: std::sync::Mutex::new(std::collections::HashMap::new()),
            always_allowed: std::sync::Mutex::new(HashSet::new()),
            workspace_root: std::sync::Mutex::new("/my/workspace".to_string()),
        };
        let inner = Arc::new(inner);
        let root = inner.workspace_root.lock().unwrap();
        assert_eq!(*root, "/my/workspace");
    }

    #[test]
    fn test_estimate_tokens_from_messages_mixed_blocks() {
        use axagent_runtime_core::{ContentBlock, ConversationMessage, MessageRole};
        let messages = vec![
            ConversationMessage {
                role: MessageRole::User,
                blocks: vec![ContentBlock::Text {
                    text: "hello".to_string(),
                }],
                usage: None,
            },
            ConversationMessage {
                role: MessageRole::Assistant,
                blocks: vec![
                    ContentBlock::Text {
                        text: "response text here".to_string(),
                    },
                    ContentBlock::ToolUse {
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
        let usage = axagent_runtime_core::TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        assert_eq!(breakdown.tokens_delta(), 0);
    }

    #[test]
    fn test_token_usage_breakdown_large_values() {
        let usage = axagent_runtime_core::TokenUsage {
            input_tokens: 100000,
            output_tokens: 50000,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };
        let breakdown = TokenUsageBreakdown::from_turn_summary(&usage, 0);
        assert_eq!(breakdown.total_tokens, 150000);
        assert_eq!(breakdown.tokens_delta(), 150000);
    }

    #[test]
    fn test_estimate_tokens_from_content_blocks_empty() {
        let blocks: Vec<axagent_runtime_core::ContentBlock> = vec![];
        assert_eq!(estimate_tokens_from_content_blocks(&blocks), 0);
    }

    #[test]
    fn test_estimate_tokens_from_messages_multiple_messages() {
        use axagent_runtime_core::{ContentBlock, ConversationMessage, MessageRole};
        let messages = vec![
            ConversationMessage {
                role: MessageRole::User,
                blocks: vec![ContentBlock::Text {
                    text: "first message".to_string(),
                }],
                usage: None,
            },
            ConversationMessage {
                role: MessageRole::Assistant,
                blocks: vec![ContentBlock::Text {
                    text: "second message".to_string(),
                }],
                usage: None,
            },
            ConversationMessage {
                role: MessageRole::User,
                blocks: vec![ContentBlock::Text {
                    text: "third message here".to_string(),
                }],
                usage: None,
            },
        ];
        let tokens = estimate_tokens_from_messages(&messages);
        assert!(tokens >= 3);
    }
}
