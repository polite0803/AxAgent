// SPDX-License-Identifier: AGPL-3.0-only

//! 会话类型 — 从 `axagent-runtime-core::session` 搬迁的纯数据 + 核心方法。
//!
//! 仅包含不依赖 I/O / 自定义 JSON 的类型和方法。
//! 序列化 & 文件持久化保留在 runtime-core（通过 `SessionExt` 扩展 trait）。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::conversation_model::ConversationMessage;

use crate::PromptGuard;

const SESSION_VERSION: u32 = 1;

static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
static LAST_TIMESTAMP_MS: AtomicU64 = AtomicU64::new(0);

/// 压缩元数据。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SessionCompaction {
    pub count: u32,
    pub removed_message_count: usize,
    pub summary: String,
}

/// 会话分叉溯源。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SessionFork {
    pub parent_session_id: String,
    pub branch_name: Option<String>,
}

/// 单条用户提示记录（带时间戳）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SessionPromptEntry {
    pub timestamp_ms: u64,
    pub text: String,
}

/// 持久化路径内部结构。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SessionPersistence {
    pub path: PathBuf,
}

/// 持久化的会话状态。
#[derive(Debug, Clone, serde::Serialize)]
pub struct Session {
    pub version: u32,
    pub session_id: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub messages: Vec<ConversationMessage>,
    pub compaction: Option<SessionCompaction>,
    pub fork: Option<SessionFork>,
    pub workspace_root: Option<PathBuf>,
    pub prompt_history: Vec<SessionPromptEntry>,
    pub last_health_check_ms: Option<u64>,
    pub model: Option<String>,
    #[serde(skip)]
    pub persistence: Option<SessionPersistence>,
    /// Prompt 注入防护（可选，由 harness 注入）。
    #[serde(skip)]
    pub prompt_guard: Option<Arc<dyn PromptGuard>>,
}

impl PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.session_id == other.session_id
            && self.created_at_ms == other.created_at_ms
            && self.updated_at_ms == other.updated_at_ms
            && self.messages == other.messages
            && self.compaction == other.compaction
            && self.fork == other.fork
            && self.workspace_root == other.workspace_root
            && self.prompt_history == other.prompt_history
            && self.last_health_check_ms == other.last_health_check_ms
    }
}

impl Eq for Session {}

/// 会话错误。
#[derive(Debug)]
pub enum SessionError {
    Io(std::io::Error),
    Format(String),
    /// 用户输入被提示词注入防护拦截。
    ContentBlocked(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Format(error) => write!(f, "{error}"),
            Self::ContentBlocked(reason) => write!(f, "Content blocked by prompt guard: {reason}"),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<std::io::Error> for SessionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl Session {
    #[must_use]
    pub fn new() -> Self {
        let now = current_time_millis();
        Self {
            version: SESSION_VERSION,
            session_id: generate_session_id(),
            created_at_ms: now,
            updated_at_ms: now,
            messages: Vec::new(),
            compaction: None,
            fork: None,
            workspace_root: None,
            prompt_history: Vec::new(),
            last_health_check_ms: None,
            model: None,
            persistence: None,
            prompt_guard: None,
        }
    }

    #[must_use]
    pub fn with_persistence_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.persistence = Some(SessionPersistence { path: path.into() });
        self
    }

    #[must_use]
    pub fn with_workspace_root(mut self, workspace_root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(workspace_root.into());
        self
    }

    #[must_use]
    pub fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    #[must_use]
    pub fn with_prompt_guard(mut self, guard: Arc<dyn PromptGuard>) -> Self {
        self.prompt_guard = Some(guard);
        self
    }

    #[must_use]
    pub fn persistence_path(&self) -> Option<&Path> {
        self.persistence.as_ref().map(|p| p.path.as_path())
    }

    /// 设置持久化路径（由 runtime-core 扩展使用）。
    #[doc(hidden)]
    pub fn set_persistence(&mut self, persistence: Option<SessionPersistence>) {
        self.persistence = persistence;
    }

    /// 读取持久化路径（由 runtime-core 扩展使用）。
    #[doc(hidden)]
    #[must_use]
    pub fn get_persistence(&self) -> Option<&SessionPersistence> {
        self.persistence.as_ref()
    }

    /// 记录压缩事件。
    pub fn record_compaction(&mut self, summary: impl Into<String>, removed_message_count: usize) {
        let count = self.compaction.as_ref().map_or(1, |value| value.count + 1);
        self.compaction =
            Some(SessionCompaction { count, removed_message_count, summary: summary.into() });
    }

    /// 更新 updated_at。
    pub fn touch(&mut self) {
        self.updated_at_ms = current_time_millis();
    }

    /// 追加消息（不含持久化，纯内存操作）。
    pub fn push_message(&mut self, message: ConversationMessage) -> Result<(), SessionError> {
        self.touch();
        self.messages.push(message);
        Ok(())
    }

    /// 分叉会话。
    #[must_use]
    pub fn fork(&self, branch_name: Option<String>) -> Self {
        let now = current_time_millis();
        Self {
            version: self.version,
            session_id: generate_session_id(),
            created_at_ms: now,
            updated_at_ms: now,
            messages: self.messages.clone(),
            compaction: self.compaction.clone(),
            fork: Some(SessionFork {
                parent_session_id: self.session_id.clone(),
                branch_name: normalize_optional_string(branch_name),
            }),
            workspace_root: self.workspace_root.clone(),
            prompt_history: self.prompt_history.clone(),
            last_health_check_ms: self.last_health_check_ms,
            model: self.model.clone(),
            persistence: None,
            prompt_guard: self.prompt_guard.clone(),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

// ── 内部辅助函数 ──

fn current_time_millis() -> u64 {
    let wall_clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default();

    let mut candidate = wall_clock;
    loop {
        let previous = LAST_TIMESTAMP_MS.load(Ordering::Acquire);
        if candidate <= previous {
            candidate = previous.saturating_add(1);
        }
        match LAST_TIMESTAMP_MS.compare_exchange(
            previous,
            candidate,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => return candidate,
            Err(actual) => candidate = actual.saturating_add(1),
        }
    }
}

fn generate_session_id() -> String {
    let millis = current_time_millis();
    let counter = SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("session-{millis}-{counter}")
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
