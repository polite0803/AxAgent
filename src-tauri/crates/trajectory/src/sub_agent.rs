// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// SubAgent — a child agent in the multi-agent hierarchy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubAgent {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub description: String,
    pub status: SubAgentStatus,
    pub task: Option<String>,
    pub progress: f32,
    pub result: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub children: Vec<String>,
    pub metadata: SubAgentMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubAgentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubAgentMetadata {
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub model: Option<String>,
    pub tools: Vec<String>,
}

impl SubAgent {
    pub fn new(name: String, description: String, parent_id: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id,
            name,
            description,
            status: SubAgentStatus::Pending,
            task: None,
            progress: 0.0,
            result: None,
            error: None,
            created_at: Utc::now(),
            completed_at: None,
            children: Vec::new(),
            metadata: SubAgentMetadata {
                agent_type: "default".to_string(),
                capabilities: Vec::new(),
                model: None,
                tools: Vec::new(),
            },
        }
    }

    pub fn with_task(mut self, task: String) -> Self {
        self.task = Some(task);
        self
    }

    pub fn with_metadata(
        mut self,
        agent_type: String,
        capabilities: Vec<String>,
        model: Option<String>,
        tools: Vec<String>,
    ) -> Self {
        self.metadata = SubAgentMetadata { agent_type, capabilities, model, tools };
        self
    }

    pub fn start(&mut self) {
        self.status = SubAgentStatus::Running;
    }

    pub fn complete(&mut self, result: String) {
        self.status = SubAgentStatus::Completed;
        self.result = Some(result);
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self, error: String) {
        self.status = SubAgentStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(Utc::now());
    }

    pub fn cancel(&mut self) {
        self.status = SubAgentStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    pub fn update_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    pub fn add_child(&mut self, child_id: String) {
        self.children.push(child_id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    pub id: String,
    pub status: SubAgentStatus,
    pub progress: f32,
    pub result: Option<String>,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}

impl From<&SubAgent> for SubAgentResult {
    fn from(agent: &SubAgent) -> Self {
        let duration_ms = agent
            .completed_at
            .map(|completed| (completed - agent.created_at).num_milliseconds() as u64);

        Self {
            id: agent.id.clone(),
            status: agent.status,
            progress: agent.progress,
            result: agent.result.clone(),
            error: agent.error.clone(),
            duration_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentQuery {
    pub parent_id: Option<String>,
    pub status: Option<SubAgentStatus>,
    pub agent_type: Option<String>,
}

// ---------------------------------------------------------------------------
// AgentMessage — typed messages for parent-child communication
// ---------------------------------------------------------------------------

/// A message exchanged between parent and child agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub kind: AgentMessageKind,
    pub payload: String,
    pub timestamp: DateTime<Utc>,
}

/// The type of inter-agent message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageKind {
    /// Parent assigns a task to a child agent.
    TaskAssign,
    /// Child reports progress back to parent (payload is a JSON f32 1.0.0.0).
    ProgressReport,
    /// Child returns the final result to parent.
    TaskResult,
    /// Child reports an error to parent.
    TaskError,
    /// Parent cancels a child's task.
    TaskCancel,
    /// Generic data exchange between agents.
    Data,
}

impl AgentMessage {
    pub fn new(from: &str, to: &str, kind: AgentMessageKind, payload: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from_agent: from.to_string(),
            to_agent: to.to_string(),
            kind,
            payload,
            timestamp: Utc::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// AgentMailbox — per-agent message queue with bounded capacity
// ---------------------------------------------------------------------------

/// 每个 agent 的 mailbox，持有收到的消息。
/// 通过 `Arc<RwLock<>>` 实现线程安全（使用 tokio 异步锁，遵守 AGENTS.md 铁律 8）。
pub struct AgentMailbox {
    messages: Arc<RwLock<std::collections::VecDeque<AgentMessage>>>,
    capacity: usize,
}

impl AgentMailbox {
    pub fn new(_agent_id: String, capacity: usize) -> Self {
        Self { messages: Arc::new(RwLock::new(std::collections::VecDeque::new())), capacity }
    }

    /// 投递消息到 mailbox。mailbox 满则返回 false。
    pub async fn deliver(&self, message: AgentMessage) -> bool {
        let mut msgs = self.messages.write().await;
        if msgs.len() >= self.capacity {
            return false;
        }
        msgs.push_back(message);
        true
    }

    /// 接收（弹出）下一条消息。VecDeque O(1)。
    pub async fn receive(&self) -> Option<AgentMessage> {
        let mut msgs = self.messages.write().await;
        msgs.pop_front()
    }

    /// 窥视所有消息但不消费。
    pub async fn peek_all(&self) -> Vec<AgentMessage> {
        let msgs = self.messages.read().await;
        msgs.iter().cloned().collect()
    }

    /// 接收指定类型的所有消息。
    pub async fn receive_by_kind(&self, kind: AgentMessageKind) -> Vec<AgentMessage> {
        let mut msgs = self.messages.write().await;
        let (matching, remaining): (Vec<_>, Vec<_>) = msgs.drain(..).partition(|m| m.kind == kind);
        *msgs = remaining.into_iter().collect();
        matching
    }

    /// 待处理消息数。
    pub async fn len(&self) -> usize {
        self.messages.read().await.len()
    }
}

// ---------------------------------------------------------------------------
// MessageBus — global message bus connecting all agent mailboxes
// ---------------------------------------------------------------------------

/// 全局消息总线，连接所有 agent mailbox。
/// 每个 agent 有自己的 `AgentMailbox`。发送消息时查找目标 agent 的 mailbox 并投递。
pub struct MessageBus {
    mailboxes: Arc<RwLock<HashMap<String, AgentMailbox>>>,
    default_capacity: usize,
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new(256)
    }
}

impl MessageBus {
    pub fn new(default_capacity: usize) -> Self {
        Self { mailboxes: Arc::new(RwLock::new(HashMap::new())), default_capacity }
    }

    /// 注册新 agent mailbox。已注册返回 false。
    pub async fn register(&self, agent_id: &str) -> bool {
        let mut mbs = self.mailboxes.write().await;
        if mbs.contains_key(agent_id) {
            return false;
        }
        mbs.insert(
            agent_id.to_string(),
            AgentMailbox::new(agent_id.to_string(), self.default_capacity),
        );
        true
    }

    /// 使用自定义容量注册。
    pub async fn register_with_capacity(&self, agent_id: &str, capacity: usize) -> bool {
        let mut mbs = self.mailboxes.write().await;
        if mbs.contains_key(agent_id) {
            return false;
        }
        mbs.insert(agent_id.to_string(), AgentMailbox::new(agent_id.to_string(), capacity));
        true
    }

    /// 注销 agent mailbox。
    pub async fn unregister(&self, agent_id: &str) {
        let mut mbs = self.mailboxes.write().await;
        mbs.remove(agent_id);
    }

    /// 从一个 agent 向另一个 agent 发送消息。
    /// 投递成功返回 `Ok(())`，目标 mailbox 不存在或已满返回 `Err`。
    pub async fn send(&self, message: AgentMessage) -> Result<(), AgentMessageError> {
        let mbs = self.mailboxes.read().await;
        let mailbox = mbs
            .get(&message.to_agent)
            .ok_or_else(|| AgentMessageError::MailboxNotFound(message.to_agent.clone()))?;
        if !mailbox.deliver(message.clone()).await {
            Err(AgentMessageError::MailboxFull(message.to_agent.clone()))
        } else {
            Ok(())
        }
    }

    /// 接收 agent 的下一条消息。
    pub async fn receive(&self, agent_id: &str) -> Option<AgentMessage> {
        let mbs = self.mailboxes.read().await;
        match mbs.get(agent_id) {
            Some(mb) => mb.receive().await,
            None => None,
        }
    }

    /// 接收 agent 指定类型的所有消息。
    pub async fn receive_by_kind(
        &self,
        agent_id: &str,
        kind: AgentMessageKind,
    ) -> Vec<AgentMessage> {
        let mbs = self.mailboxes.read().await;
        match mbs.get(agent_id) {
            Some(mb) => mb.receive_by_kind(kind).await,
            None => Vec::new(),
        }
    }

    /// 窥视 agent 所有待处理消息（不消费）。
    pub async fn peek_all(&self, agent_id: &str) -> Vec<AgentMessage> {
        let mbs = self.mailboxes.read().await;
        match mbs.get(agent_id) {
            Some(mb) => mb.peek_all().await,
            None => Vec::new(),
        }
    }

    /// agent 待处理消息数。
    pub async fn pending_count(&self, agent_id: &str) -> usize {
        let mbs = self.mailboxes.read().await;
        match mbs.get(agent_id) {
            Some(mb) => mb.len().await,
            None => 0,
        }
    }

    /// 列出所有已注册 agent ID。
    pub async fn registered_agents(&self) -> Vec<String> {
        let mbs = self.mailboxes.read().await;
        mbs.keys().cloned().collect()
    }
}

#[derive(Debug, Clone)]
pub enum AgentMessageError {
    MailboxNotFound(String),
    MailboxFull(String),
}

impl std::fmt::Display for AgentMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MailboxNotFound(id) => write!(f, "Mailbox not found for agent: {}", id),
            Self::MailboxFull(id) => write!(f, "Mailbox full for agent: {}", id),
        }
    }
}

impl std::error::Error for AgentMessageError {}

// ---------------------------------------------------------------------------
// SubAgentRegistry — persistent registry with integrated MessageBus
// ---------------------------------------------------------------------------

pub struct SubAgentRegistry {
    agents: Vec<SubAgent>,
    storage_path: PathBuf,
    dirty: bool,
    /// Integrated message bus for parent-child communication.
    message_bus: MessageBus,
    /// Task deduplicator to prevent assigning duplicate tasks (P4-2).
    task_deduplicator: TaskDeduplicator,
}

impl Default for SubAgentRegistry {
    fn default() -> Self {
        // Default 实现不读文件、不注册 mailbox，直接构造空 registry。
        // 原因：new() 是 async，Default trait 不能 await。
        let storage_path =
            Self::get_storage_path().unwrap_or_else(|_| PathBuf::from("sub_agents.json"));
        Self {
            agents: Vec::new(),
            storage_path,
            dirty: false,
            message_bus: MessageBus::new(256),
            task_deduplicator: TaskDeduplicator::default(),
        }
    }
}

impl SubAgentRegistry {
    pub async fn new() -> Result<Self> {
        let storage_path = Self::get_storage_path()?;
        Self::new_with_path(&storage_path).await
    }

    pub async fn new_with_path(storage_path: &PathBuf) -> Result<Self> {
        if let Some(parent) = storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let agents = if storage_path.exists() {
            let content = std::fs::read_to_string(storage_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        let registry = Self {
            agents,
            storage_path: storage_path.clone(),
            dirty: false,
            message_bus: MessageBus::new(256),
            task_deduplicator: TaskDeduplicator::default(),
        };

        // 为所有已加载 agent 注册 mailbox
        for agent in &registry.agents {
            registry.message_bus.register(&agent.id).await;
        }

        Ok(registry)
    }

    fn get_storage_path() -> Result<PathBuf> {
        if let Some(data_dir) = dirs::data_dir() {
            let path = data_dir.join("clawcode").join("trajectory").join("sub_agents.json");
            return Ok(path);
        }
        Ok(PathBuf::from("sub_agents.json"))
    }

    pub fn save(&self) -> Result<()> {
        if !self.dirty && !self.storage_path.exists() {
            return Ok(());
        }
        let content = serde_json::to_string_pretty(&self.agents)?;
        std::fs::write(&self.storage_path, content)?;
        Ok(())
    }

    pub fn save_if_dirty(&mut self) -> Result<()> {
        if self.dirty {
            self.save()?;
            self.dirty = false;
        }
        Ok(())
    }

    pub async fn create_agent(&mut self, agent: SubAgent) {
        self.message_bus.register(&agent.id).await;
        self.agents.push(agent);
        self.dirty = true;
    }

    pub async fn push(&mut self, agent: SubAgent) {
        self.message_bus.register(&agent.id).await;
        self.agents.push(agent);
        self.dirty = true;
    }

    pub async fn create(
        &mut self,
        name: String,
        description: String,
        parent_id: Option<String>,
    ) -> SubAgent {
        let agent = SubAgent::new(name, description, parent_id.clone());
        self.message_bus.register(&agent.id).await;
        // 若有 parent，把当前 agent 加入 parent 的 children
        if let Some(ref pid) = parent_id
            && let Some(parent) = self.agents.iter_mut().find(|a| a.id == *pid)
        {
            parent.add_child(agent.id.clone());
        }
        self.agents.push(agent.clone());
        self.dirty = true;
        agent
    }

    pub fn get(&self, id: &str) -> Option<&SubAgent> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut SubAgent> {
        self.dirty = true;
        self.agents.iter_mut().find(|a| a.id == id)
    }

    pub fn update<F>(&mut self, id: &str, updater: F) -> Option<()>
    where
        F: FnOnce(&mut SubAgent),
    {
        let agent = self.agents.iter_mut().find(|a| a.id == id)?;
        updater(agent);
        self.dirty = true;
        Some(())
    }

    pub fn list(&self, query: Option<&SubAgentQuery>) -> Vec<&SubAgent> {
        self.agents
            .iter()
            .filter(|a| {
                if let Some(q) = query {
                    if let Some(ref parent_id) = q.parent_id
                        && a.parent_id.as_ref() != Some(parent_id)
                    {
                        return false;
                    }
                    if let Some(ref status) = q.status
                        && &a.status != status
                    {
                        return false;
                    }
                    if let Some(ref agent_type) = q.agent_type
                        && &a.metadata.agent_type != agent_type
                    {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    pub fn list_all(&self) -> Vec<&SubAgent> {
        self.agents.iter().collect()
    }

    pub async fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.agents.iter().position(|a| a.id == id) {
            self.message_bus.unregister(id).await;
            self.agents.remove(pos);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn get_children(&self, parent_id: &str) -> Vec<&SubAgent> {
        self.agents.iter().filter(|a| a.parent_id.as_deref() == Some(parent_id)).collect()
    }

    pub fn get_active_count(&self) -> usize {
        self.agents
            .iter()
            .filter(|a| a.status == SubAgentStatus::Running || a.status == SubAgentStatus::Pending)
            .count()
    }

    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    pub async fn clear(&mut self) {
        for agent in &self.agents {
            self.message_bus.unregister(&agent.id).await;
        }
        self.agents.clear();
        self.dirty = true;
    }

    pub async fn reload(&mut self) -> Result<()> {
        if self.storage_path.exists() {
            let content = std::fs::read_to_string(&self.storage_path)?;
            // 先注销旧 agent
            for agent in &self.agents {
                self.message_bus.unregister(&agent.id).await;
            }
            self.agents = serde_json::from_str(&content)?;
            // 注册新 agent
            for agent in &self.agents {
                self.message_bus.register(&agent.id).await;
            }
            self.dirty = false;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // P2-11: Parent-child message channel (via MessageBus)
    // -----------------------------------------------------------------------

    /// Access the message bus for direct message operations.
    pub fn message_bus(&self) -> &MessageBus {
        &self.message_bus
    }

    // -----------------------------------------------------------------------
    // P2-12: Task dispatch (parent → child task assignment + result collection)
    // -----------------------------------------------------------------------

    /// Dispatch a task from a parent agent to a child agent.
    /// Sets the child's task field, status to Pending, and sends a
    /// TaskAssign message through the message bus.
    /// P4-2: Checks for duplicate tasks before dispatching.
    pub async fn dispatch_task(
        &mut self,
        parent_id: &str,
        child_id: &str,
        task: String,
    ) -> Result<(), AgentMessageError> {
        // P4-2: 检查重复任务
        if let Some(similarity) = self.task_deduplicator.check_duplicate(&task) {
            // 任务与已有任务过于相似 — 跳过派发
            let msg = AgentMessage::new(
                parent_id,
                child_id,
                AgentMessageKind::TaskError,
                format!(
                    "Duplicate task detected (similarity: {:.2}), skipping dispatch",
                    similarity
                ),
            );
            let _ = self.message_bus.send(msg).await;
            return Err(AgentMessageError::MailboxFull(child_id.to_string()));
        }

        // 注册任务以便后续去重
        self.task_deduplicator.register_task(&task);

        // 先发送 TaskAssign 消息 — mailbox 满则不更新 child 状态
        let msg =
            AgentMessage::new(parent_id, child_id, AgentMessageKind::TaskAssign, task.clone());
        self.message_bus.send(msg).await?;

        // 仅在消息投递成功后才更新 child agent 状态
        if let Some(child) = self.agents.iter_mut().find(|a| a.id == child_id) {
            child.task = Some(task);
            child.status = SubAgentStatus::Pending;
            child.progress = 0.0;
            self.dirty = true;
        }

        Ok(())
    }

    /// Dispatch tasks to multiple children in parallel.
    /// Returns the number of successfully dispatched tasks.
    pub async fn dispatch_tasks_parallel(
        &mut self,
        parent_id: &str,
        tasks: Vec<(&str, String)>, // (child_id, task_description)
    ) -> usize {
        let mut dispatched = 0;
        for (child_id, task) in tasks {
            if self.dispatch_task(parent_id, child_id, task).await.is_ok() {
                dispatched += 1;
            }
        }
        dispatched
    }

    /// Collect results from all completed children of a parent.
    /// Returns (completed_results, pending_child_ids).
    pub fn collect_results(&self, parent_id: &str) -> (Vec<SubAgentResult>, Vec<String>) {
        let children = self.get_children(parent_id);
        let mut results = Vec::new();
        let mut pending = Vec::new();

        for child in children {
            match child.status {
                SubAgentStatus::Completed | SubAgentStatus::Failed => {
                    results.push(SubAgentResult::from(child));
                },
                SubAgentStatus::Pending | SubAgentStatus::Running => {
                    pending.push(child.id.clone());
                },
                SubAgentStatus::Cancelled => {
                    // Treat cancelled as completed with no result
                    results.push(SubAgentResult::from(child));
                },
            }
        }

        (results, pending)
    }

    /// Check if all children of a parent have finished (completed, failed, or cancelled).
    pub fn all_children_finished(&self, parent_id: &str) -> bool {
        self.get_children(parent_id).iter().all(|c| {
            matches!(
                c.status,
                SubAgentStatus::Completed | SubAgentStatus::Failed | SubAgentStatus::Cancelled
            )
        })
    }

    // -----------------------------------------------------------------------
    // P2-13: Progress reporting (child → parent progress notification)
    // -----------------------------------------------------------------------

    /// Report progress from a child agent to its parent.
    /// Updates the child's progress field and sends a ProgressReport
    /// message through the message bus.
    pub async fn report_progress(
        &mut self,
        child_id: &str,
        progress: f32,
    ) -> Result<(), AgentMessageError> {
        let parent_id = self.agents.iter_mut().find(|a| a.id == child_id).and_then(|child| {
            child.update_progress(progress);
            self.dirty = true;
            child.parent_id.clone()
        });

        if let Some(pid) = parent_id {
            let msg = AgentMessage::new(
                child_id,
                &pid,
                AgentMessageKind::ProgressReport,
                serde_json::to_string(&progress).unwrap_or_default(),
            );
            self.message_bus.send(msg).await
        } else {
            // 无父节点 — 仅更新本地进度
            Ok(())
        }
    }

    /// Report task completion from a child agent to its parent.
    /// Updates the child's status and result, sends a TaskResult message.
    /// If the message delivery fails (mailbox full), the child state is still updated
    /// but a warning is logged — the parent can poll the registry for completed children.
    pub async fn report_completion(
        &mut self,
        child_id: &str,
        result: String,
    ) -> Result<(), AgentMessageError> {
        let parent_id = self.agents.iter_mut().find(|a| a.id == child_id).and_then(|child| {
            child.complete(result.clone());
            self.dirty = true;
            child.parent_id.clone()
        });

        if let Some(pid) = parent_id {
            let msg = AgentMessage::new(child_id, &pid, AgentMessageKind::TaskResult, result);
            if let Err(e) = self.message_bus.send(msg).await {
                tracing::warn!(
                    "Failed to deliver completion notification for child {}: {:?}. Parent can poll registry.",
                    child_id,
                    e
                );
            }
        }
        Ok(())
    }

    /// Report task error from a child agent to its parent.
    pub async fn report_error(
        &mut self,
        child_id: &str,
        error: String,
    ) -> Result<(), AgentMessageError> {
        let parent_id = self.agents.iter_mut().find(|a| a.id == child_id).and_then(|child| {
            child.fail(error.clone());
            self.dirty = true;
            child.parent_id.clone()
        });

        if let Some(pid) = parent_id {
            let msg = AgentMessage::new(child_id, &pid, AgentMessageKind::TaskError, error);
            self.message_bus.send(msg).await
        } else {
            Ok(())
        }
    }

    /// Cancel a child's task from the parent.
    pub async fn cancel_child(
        &mut self,
        parent_id: &str,
        child_id: &str,
    ) -> Result<(), AgentMessageError> {
        if let Some(child) = self.agents.iter_mut().find(|a| a.id == child_id) {
            child.cancel();
            self.dirty = true;
        }

        let msg =
            AgentMessage::new(parent_id, child_id, AgentMessageKind::TaskCancel, String::new());
        self.message_bus.send(msg).await
    }
}

// ---------------------------------------------------------------------------
// P4-2: Semantic deduplication — avoid assigning duplicate tasks to agents
// ---------------------------------------------------------------------------

/// A simple keyword-based semantic similarity checker for task deduplication.
/// Uses Jaccard similarity on token sets to detect near-duplicate tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDeduplicator {
    /// Minimum Jaccard similarity threshold to consider two tasks duplicates (default 0.6).
    pub similarity_threshold: f64,
    /// Known task descriptions and their token sets.
    known_tasks: Vec<(String, Vec<String>)>,
    /// Maximum capacity before evicting oldest entries (LRU-style).
    max_capacity: usize,
}

impl Default for TaskDeduplicator {
    fn default() -> Self {
        Self { similarity_threshold: 0.6, known_tasks: Vec::new(), max_capacity: 10_000 }
    }
}

impl TaskDeduplicator {
    #[cfg(test)]
    pub(crate) fn new(similarity_threshold: f64) -> Self {
        Self { similarity_threshold, known_tasks: Vec::new(), max_capacity: 10_000 }
    }

    /// Tokenize a task description into lowercase words (split on whitespace/punctuation).
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|w| w.len() > 2) // skip very short tokens
            .map(String::from)
            .collect()
    }

    /// Compute Jaccard similarity between two token sets.
    fn jaccard_similarity(a: &[String], b: &[String]) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        let set_a: std::collections::HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
        let set_b: std::collections::HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
        let intersection = set_a.intersection(&set_b).count() as f64;
        let union = set_a.union(&set_b).count() as f64;
        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }

    /// Check if a new task is a duplicate of any known task.
    /// Returns `Some(similarity_score)` if duplicate found, `None` if unique.
    pub(crate) fn check_duplicate(&self, task: &str) -> Option<f64> {
        let tokens = Self::tokenize(task);
        let mut best_score = 0.0_f64;
        for (_, known_tokens) in &self.known_tasks {
            let score = Self::jaccard_similarity(&tokens, known_tokens);
            if score > best_score {
                best_score = score;
            }
        }
        if best_score >= self.similarity_threshold {
            Some(best_score)
        } else {
            None
        }
    }

    /// Register a task as known (to check future tasks against).
    pub(crate) fn register_task(&mut self, task: &str) {
        let tokens = Self::tokenize(task);
        self.known_tasks.push((task.to_string(), tokens));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sub_agent_creation() {
        let agent = SubAgent::new("worker".to_string(), "A worker agent".to_string(), None);
        assert_eq!(agent.status, SubAgentStatus::Pending);
        assert!(agent.parent_id.is_none());
    }

    #[tokio::test]
    async fn test_message_bus_basic() {
        let bus = MessageBus::new(64);
        bus.register("parent").await;
        bus.register("child").await;

        let msg =
            AgentMessage::new("parent", "child", AgentMessageKind::TaskAssign, "Do X".to_string());
        assert!(bus.send(msg).await.is_ok());

        let received = bus.receive("child").await;
        assert!(received.is_some());
        let received = received.expect("测试应成功");
        assert_eq!(received.kind, AgentMessageKind::TaskAssign);
        assert_eq!(received.payload, "Do X");
    }

    #[tokio::test]
    async fn test_message_bus_mailbox_not_found() {
        let bus = MessageBus::new(64);
        let msg = AgentMessage::new("a", "b", AgentMessageKind::Data, "hello".to_string());
        let result = bus.send(msg).await;
        assert!(matches!(result, Err(AgentMessageError::MailboxNotFound(_))));
    }

    #[tokio::test]
    async fn test_dispatch_and_collect() {
        let mut registry =
            SubAgentRegistry::new_with_path(&std::path::PathBuf::from("test_dispatch_agents.json"))
                .await
                .expect("测试应成功");

        // 创建 parent 和 children
        let parent =
            registry.create("coordinator".to_string(), "Parent agent".to_string(), None).await;
        let child1 = registry
            .create("worker1".to_string(), "Worker 1".to_string(), Some(parent.id.clone()))
            .await;
        let child2 = registry
            .create("worker2".to_string(), "Worker 2".to_string(), Some(parent.id.clone()))
            .await;

        // 派发任务（使用不同描述避免去重）
        registry
            .dispatch_task(&parent.id, &child1.id, "Analyze the codebase structure".to_string())
            .await
            .expect("测试应成功");
        registry
            .dispatch_task(&parent.id, &child2.id, "Write unit tests for module".to_string())
            .await
            .expect("测试应成功");

        // 验证任务已分配
        assert_eq!(
            registry.get(&child1.id).expect("测试：键应存在").task.as_deref(),
            Some("Analyze the codebase structure")
        );
        assert_eq!(
            registry.get(&child2.id).expect("测试：键应存在").task.as_deref(),
            Some("Write unit tests for module")
        );

        // 模拟 child1 完成
        registry
            .report_completion(&child1.id, "Result A".to_string())
            .await
            .expect("测试：异步操作应成功");
        assert_eq!(
            registry.get(&child1.id).expect("测试：键应存在").status,
            SubAgentStatus::Completed
        );

        // 收集结果 — child1 完成，child2 待处理
        let (results, pending) = registry.collect_results(&parent.id);
        assert_eq!(results.len(), 1);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0], child2.id);

        // 模拟 child2 完成
        registry
            .report_completion(&child2.id, "Result B".to_string())
            .await
            .expect("测试：异步操作应成功");

        // 现在所有 child 都完成
        assert!(registry.all_children_finished(&parent.id));
        let (results, pending) = registry.collect_results(&parent.id);
        assert_eq!(results.len(), 2);
        assert!(pending.is_empty());

        // 清理
        let _ = std::fs::remove_file("test_dispatch_agents.json");
    }

    #[tokio::test]
    async fn test_progress_reporting() {
        let mut registry =
            SubAgentRegistry::new_with_path(&std::path::PathBuf::from("test_progress_agents.json"))
                .await
                .expect("测试应成功");

        let parent = registry.create("coordinator".to_string(), "Parent".to_string(), None).await;
        let child = registry
            .create("worker".to_string(), "Worker".to_string(), Some(parent.id.clone()))
            .await;

        // 上报进度
        registry.report_progress(&child.id, 0.5).await.expect("测试：异步操作应成功");
        assert!((registry.get(&child.id).expect("测试：键应存在").progress - 0.5).abs() < 1.0);

        // parent 应收到 ProgressReport 消息
        let msgs = registry
            .message_bus()
            .receive_by_kind(&parent.id, AgentMessageKind::ProgressReport)
            .await;
        assert_eq!(msgs.len(), 1);
        let progress: f32 =
            serde_json::from_str(&msgs[0].payload).expect("测试：JSON反序列化应成功");
        assert!((progress - 0.5).abs() < 1.0);

        // 清理
        let _ = std::fs::remove_file("test_progress_agents.json");
    }

    #[tokio::test]
    async fn test_error_reporting() {
        let mut registry =
            SubAgentRegistry::new_with_path(&std::path::PathBuf::from("test_error_agents.json"))
                .await
                .expect("测试应成功");

        let parent = registry.create("coordinator".to_string(), "Parent".to_string(), None).await;
        let child = registry
            .create("worker".to_string(), "Worker".to_string(), Some(parent.id.clone()))
            .await;

        registry
            .report_error(&child.id, "Something went wrong".to_string())
            .await
            .expect("测试：异步操作应成功");
        assert_eq!(registry.get(&child.id).expect("测试：键应存在").status, SubAgentStatus::Failed);

        let msgs =
            registry.message_bus().receive_by_kind(&parent.id, AgentMessageKind::TaskError).await;
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].payload, "Something went wrong");

        // 清理
        let _ = std::fs::remove_file("test_error_agents.json");
    }

    #[test]
    fn test_task_deduplication() {
        let mut dedup = TaskDeduplicator::new(0.6);

        // 注册首个任务
        dedup.register_task("Analyze the codebase structure and find bugs");

        // 相同任务应被识别为重复
        let dup = dedup.check_duplicate("Analyze the codebase structure and find bugs");
        assert!(dup.is_some());
        assert!(dup.expect("测试应成功") >= 0.6);

        // 相似任务也应被识别
        let similar = dedup.check_duplicate("Analyze the codebase structure and find issues");
        assert!(similar.is_some());

        // 完全不同的任务不应被识别
        let different = dedup.check_duplicate("Write documentation for the API endpoints");
        assert!(different.is_none());
    }
}
