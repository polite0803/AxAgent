use anyhow::Result;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// SubAgent — a child agent in the multi-agent hierarchy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        self.metadata = SubAgentMetadata {
            agent_type,
            capabilities,
            model,
            tools,
        };
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

/// A per-agent mailbox that holds incoming messages.
/// Thread-safe via `Arc<RwLock<>>`.
pub struct AgentMailbox {
    agent_id: String,
    messages: Arc<RwLock<std::collections::VecDeque<AgentMessage>>>,
    capacity: usize,
}

impl AgentMailbox {
    pub fn new(agent_id: String, capacity: usize) -> Self {
        Self {
            agent_id,
            messages: Arc::new(RwLock::new(std::collections::VecDeque::new())),
            capacity,
        }
    }

    /// Deliver a message to this mailbox. Returns false if the mailbox is full.
    pub fn deliver(&self, message: AgentMessage) -> bool {
        let mut msgs = self.messages.write().unwrap();
        if msgs.len() >= self.capacity {
            return false;
        }
        msgs.push_back(message);
        true
    }

    /// Receive (pop) the next message from this mailbox. O(1) with VecDeque.
    pub fn receive(&self) -> Option<AgentMessage> {
        let mut msgs = self.messages.write().unwrap();
        msgs.pop_front()
    }

    /// Peek at all messages without consuming them.
    pub fn peek_all(&self) -> Vec<AgentMessage> {
        let msgs = self.messages.read().unwrap();
        msgs.iter().cloned().collect()
    }

    /// Receive all messages of a specific kind.
    pub fn receive_by_kind(&self, kind: AgentMessageKind) -> Vec<AgentMessage> {
        let mut msgs = self.messages.write().unwrap();
        let (matching, remaining): (Vec<_>, Vec<_>) =
            msgs.drain(..).partition(|m| m.kind == kind);
        *msgs = remaining.into_iter().collect();
        matching
    }

    /// Number of pending messages.
    pub fn len(&self) -> usize {
        self.messages.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.read().unwrap().is_empty()
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

// ---------------------------------------------------------------------------
// MessageBus — global message bus connecting all agent mailboxes
// ---------------------------------------------------------------------------

/// A global message bus that routes messages between agent mailboxes.
/// Each agent has its own `AgentMailbox`. Sending a message looks up
/// the target agent's mailbox and delivers the message there.
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
        Self {
            mailboxes: Arc::new(RwLock::new(HashMap::new())),
            default_capacity,
        }
    }

    /// Register a new agent mailbox. Returns false if already registered.
    pub fn register(&self, agent_id: &str) -> bool {
        let mut mbs = self.mailboxes.write().unwrap();
        if mbs.contains_key(agent_id) {
            return false;
        }
        mbs.insert(
            agent_id.to_string(),
            AgentMailbox::new(agent_id.to_string(), self.default_capacity),
        );
        true
    }

    /// Register with a custom capacity.
    pub fn register_with_capacity(&self, agent_id: &str, capacity: usize) -> bool {
        let mut mbs = self.mailboxes.write().unwrap();
        if mbs.contains_key(agent_id) {
            return false;
        }
        mbs.insert(
            agent_id.to_string(),
            AgentMailbox::new(agent_id.to_string(), capacity),
        );
        true
    }

    /// Unregister an agent's mailbox.
    pub fn unregister(&self, agent_id: &str) {
        let mut mbs = self.mailboxes.write().unwrap();
        mbs.remove(agent_id);
    }

    /// Send a message from one agent to another.
    /// Returns `Ok(())` if delivered, `Err` if the target mailbox is
    /// full or not registered.
    pub fn send(&self, message: AgentMessage) -> Result<(), AgentMessageError> {
        let mbs = self.mailboxes.read().unwrap();
        let mailbox = mbs
            .get(&message.to_agent)
            .ok_or_else(|| AgentMessageError::MailboxNotFound(message.to_agent.clone()))?;
        if !mailbox.deliver(message.clone()) {
            Err(AgentMessageError::MailboxFull(message.to_agent.clone()))
        } else {
            Ok(())
        }
    }

    /// Receive the next message for an agent.
    pub fn receive(&self, agent_id: &str) -> Option<AgentMessage> {
        let mbs = self.mailboxes.read().unwrap();
        mbs.get(agent_id).and_then(|mb| mb.receive())
    }

    /// Receive all messages of a specific kind for an agent.
    pub fn receive_by_kind(&self, agent_id: &str, kind: AgentMessageKind) -> Vec<AgentMessage> {
        let mbs = self.mailboxes.read().unwrap();
        mbs.get(agent_id)
            .map(|mb| mb.receive_by_kind(kind))
            .unwrap_or_default()
    }

    /// Peek at all pending messages for an agent (non-consuming).
    pub fn peek_all(&self, agent_id: &str) -> Vec<AgentMessage> {
        let mbs = self.mailboxes.read().unwrap();
        mbs.get(agent_id)
            .map(|mb| mb.peek_all())
            .unwrap_or_default()
    }

    /// Number of pending messages for an agent.
    pub fn pending_count(&self, agent_id: &str) -> usize {
        let mbs = self.mailboxes.read().unwrap();
        mbs.get(agent_id).map(|mb| mb.len()).unwrap_or(0)
    }

    /// List all registered agent IDs.
    pub fn registered_agents(&self) -> Vec<String> {
        let mbs = self.mailboxes.read().unwrap();
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
        Self::new().expect("Failed to create default SubAgentRegistry")
    }
}

impl SubAgentRegistry {
    pub fn new() -> Result<Self> {
        let storage_path = Self::get_storage_path()?;
        Self::new_with_path(&storage_path)
    }

    pub fn new_with_path(storage_path: &PathBuf) -> Result<Self> {
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

        // Register mailboxes for all loaded agents
        for agent in &registry.agents {
            registry.message_bus.register(&agent.id);
        }

        Ok(registry)
    }

    fn get_storage_path() -> Result<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "clawcode", "trajectory") {
            let path = proj_dirs.data_dir().join("sub_agents.json");
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

    pub fn create_agent(&mut self, agent: SubAgent) {
        self.message_bus.register(&agent.id);
        self.agents.push(agent);
        self.dirty = true;
    }

    pub fn push(&mut self, agent: SubAgent) {
        self.message_bus.register(&agent.id);
        self.agents.push(agent);
        self.dirty = true;
    }

    pub fn create(
        &mut self,
        name: String,
        description: String,
        parent_id: Option<String>,
    ) -> SubAgent {
        let agent = SubAgent::new(name, description, parent_id.clone());
        self.message_bus.register(&agent.id);
        // If there's a parent, add this agent as a child
        if let Some(ref pid) = parent_id {
            if let Some(parent) = self.agents.iter_mut().find(|a| a.id == *pid) {
                parent.add_child(agent.id.clone());
            }
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
                    if let Some(ref parent_id) = q.parent_id {
                        if a.parent_id.as_ref() != Some(parent_id) {
                            return false;
                        }
                    }
                    if let Some(ref status) = q.status {
                        if &a.status != status {
                            return false;
                        }
                    }
                    if let Some(ref agent_type) = q.agent_type {
                        if &a.metadata.agent_type != agent_type {
                            return false;
                        }
                    }
                }
                true
            })
            .collect()
    }

    pub fn list_all(&self) -> Vec<&SubAgent> {
        self.agents.iter().collect()
    }

    pub fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.agents.iter().position(|a| a.id == id) {
            self.message_bus.unregister(id);
            self.agents.remove(pos);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn get_children(&self, parent_id: &str) -> Vec<&SubAgent> {
        self.agents
            .iter()
            .filter(|a| a.parent_id.as_deref() == Some(parent_id))
            .collect()
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

    pub fn clear(&mut self) {
        for agent in &self.agents {
            self.message_bus.unregister(&agent.id);
        }
        self.agents.clear();
        self.dirty = true;
    }

    pub fn reload(&mut self) -> Result<()> {
        if self.storage_path.exists() {
            let content = std::fs::read_to_string(&self.storage_path)?;
            // Unregister old agents
            for agent in &self.agents {
                self.message_bus.unregister(&agent.id);
            }
            self.agents = serde_json::from_str(&content)?;
            // Register new agents
            for agent in &self.agents {
                self.message_bus.register(&agent.id);
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
    pub fn dispatch_task(
        &mut self,
        parent_id: &str,
        child_id: &str,
        task: String,
    ) -> Result<(), AgentMessageError> {
        // P4-2: Check for duplicate task
        if let Some(similarity) = self.task_deduplicator.check_duplicate(&task) {
            // Task is too similar to an existing one — skip dispatch
            let msg = AgentMessage::new(
                parent_id,
                child_id,
                AgentMessageKind::TaskError,
                format!("Duplicate task detected (similarity: {:.2}), skipping dispatch", similarity),
            );
            let _ = self.message_bus.send(msg);
            return Err(AgentMessageError::MailboxFull(child_id.to_string()));
        }

        // Register the task for future dedup checks
        self.task_deduplicator.register_task(&task);

        // Send TaskAssign message FIRST — if mailbox is full, don't update child state
        let msg = AgentMessage::new(
            parent_id,
            child_id,
            AgentMessageKind::TaskAssign,
            task.clone(),
        );
        if let Err(e) = self.message_bus.send(msg) {
            // Mailbox full — do NOT update child state to avoid inconsistency
            return Err(e);
        }

        // Only update child agent state after successful message delivery
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
    pub fn dispatch_tasks_parallel(
        &mut self,
        parent_id: &str,
        tasks: Vec<(&str, String)>, // (child_id, task_description)
    ) -> usize {
        let mut dispatched = 0;
        for (child_id, task) in tasks {
            if self.dispatch_task(parent_id, child_id, task).is_ok() {
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
                }
                SubAgentStatus::Pending | SubAgentStatus::Running => {
                    pending.push(child.id.clone());
                }
                SubAgentStatus::Cancelled => {
                    // Treat cancelled as completed with no result
                    results.push(SubAgentResult::from(child));
                }
            }
        }

        (results, pending)
    }

    /// Check if all children of a parent have finished (completed, failed, or cancelled).
    pub fn all_children_finished(&self, parent_id: &str) -> bool {
        self.get_children(parent_id)
            .iter()
            .all(|c| matches!(c.status,
                SubAgentStatus::Completed | SubAgentStatus::Failed | SubAgentStatus::Cancelled
            ))
    }

    // -----------------------------------------------------------------------
    // P2-13: Progress reporting (child → parent progress notification)
    // -----------------------------------------------------------------------

    /// Report progress from a child agent to its parent.
    /// Updates the child's progress field and sends a ProgressReport
    /// message through the message bus.
    pub fn report_progress(
        &mut self,
        child_id: &str,
        progress: f32,
    ) -> Result<(), AgentMessageError> {
        let parent_id = self.agents
            .iter_mut()
            .find(|a| a.id == child_id)
            .map(|child| {
                child.update_progress(progress);
                self.dirty = true;
                child.parent_id.clone()
            })
            .flatten();

        if let Some(pid) = parent_id {
            let msg = AgentMessage::new(
                child_id,
                &pid,
                AgentMessageKind::ProgressReport,
                serde_json::to_string(&progress).unwrap_or_default(),
            );
            self.message_bus.send(msg)
        } else {
            // No parent to notify — just update local progress
            Ok(())
        }
    }

    /// Report task completion from a child agent to its parent.
    /// Updates the child's status and result, sends a TaskResult message.
    /// If the message delivery fails (mailbox full), the child state is still updated
    /// but a warning is logged — the parent can poll the registry for completed children.
    pub fn report_completion(
        &mut self,
        child_id: &str,
        result: String,
    ) -> Result<(), AgentMessageError> {
        let parent_id = self.agents
            .iter_mut()
            .find(|a| a.id == child_id)
            .map(|child| {
                child.complete(result.clone());
                self.dirty = true;
                child.parent_id.clone()
            })
            .flatten();

        if let Some(pid) = parent_id {
            let msg = AgentMessage::new(
                child_id,
                &pid,
                AgentMessageKind::TaskResult,
                result,
            );
            if let Err(e) = self.message_bus.send(msg) {
                tracing::warn!("Failed to deliver completion notification for child {}: {:?}. Parent can poll registry.", child_id, e);
            }
        }
        Ok(())
    }

    /// Report task error from a child agent to its parent.
    pub fn report_error(
        &mut self,
        child_id: &str,
        error: String,
    ) -> Result<(), AgentMessageError> {
        let parent_id = self.agents
            .iter_mut()
            .find(|a| a.id == child_id)
            .map(|child| {
                child.fail(error.clone());
                self.dirty = true;
                child.parent_id.clone()
            })
            .flatten();

        if let Some(pid) = parent_id {
            let msg = AgentMessage::new(
                child_id,
                &pid,
                AgentMessageKind::TaskError,
                error,
            );
            self.message_bus.send(msg)
        } else {
            Ok(())
        }
    }

    /// Cancel a child's task from the parent.
    pub fn cancel_child(
        &mut self,
        parent_id: &str,
        child_id: &str,
    ) -> Result<(), AgentMessageError> {
        if let Some(child) = self.agents.iter_mut().find(|a| a.id == child_id) {
            child.cancel();
            self.dirty = true;
        }

        let msg = AgentMessage::new(
            parent_id,
            child_id,
            AgentMessageKind::TaskCancel,
            String::new(),
        );
        self.message_bus.send(msg)
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
}

impl Default for TaskDeduplicator {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.6,
            known_tasks: Vec::new(),
        }
    }
}

impl TaskDeduplicator {
    pub fn new(similarity_threshold: f64) -> Self {
        Self {
            similarity_threshold,
            known_tasks: Vec::new(),
        }
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
        if union == 0.0 { 0.0 } else { intersection / union }
    }

    /// Check if a new task is a duplicate of any known task.
    /// Returns `Some(similarity_score)` if duplicate found, `None` if unique.
    pub fn check_duplicate(&self, task: &str) -> Option<f64> {
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
    pub fn register_task(&mut self, task: &str) {
        let tokens = Self::tokenize(task);
        self.known_tasks.push((task.to_string(), tokens));
    }

    /// Remove a task from the known set.
    pub fn unregister_task(&mut self, task: &str) {
        self.known_tasks.retain(|(t, _)| t != task);
    }

    /// Number of known tasks.
    pub fn len(&self) -> usize {
        self.known_tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.known_tasks.is_empty()
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

    #[test]
    fn test_message_bus_basic() {
        let bus = MessageBus::new(64);
        bus.register("parent");
        bus.register("child");

        let msg = AgentMessage::new("parent", "child", AgentMessageKind::TaskAssign, "Do X".to_string());
        assert!(bus.send(msg).is_ok());

        let received = bus.receive("child");
        assert!(received.is_some());
        let received = received.unwrap();
        assert_eq!(received.kind, AgentMessageKind::TaskAssign);
        assert_eq!(received.payload, "Do X");
    }

    #[test]
    fn test_message_bus_mailbox_not_found() {
        let bus = MessageBus::new(64);
        let msg = AgentMessage::new("a", "b", AgentMessageKind::Data, "hello".to_string());
        let result = bus.send(msg);
        assert!(matches!(result, Err(AgentMessageError::MailboxNotFound(_))));
    }

    #[test]
    fn test_dispatch_and_collect() {
        let mut registry = SubAgentRegistry::new_with_path(
            &std::path::PathBuf::from("test_dispatch_agents.json"),
        ).unwrap();

        // Create parent and children
        let parent = registry.create("coordinator".to_string(), "Parent agent".to_string(), None);
        let child1 = registry.create("worker1".to_string(), "Worker 1".to_string(), Some(parent.id.clone()));
        let child2 = registry.create("worker2".to_string(), "Worker 2".to_string(), Some(parent.id.clone()));

        // Dispatch tasks (use distinct descriptions to avoid dedup)
        registry.dispatch_task(&parent.id, &child1.id, "Analyze the codebase structure".to_string()).unwrap();
        registry.dispatch_task(&parent.id, &child2.id, "Write unit tests for module".to_string()).unwrap();

        // Verify tasks assigned
        assert_eq!(registry.get(&child1.id).unwrap().task.as_deref(), Some("Analyze the codebase structure"));
        assert_eq!(registry.get(&child2.id).unwrap().task.as_deref(), Some("Write unit tests for module"));

        // Simulate child1 completing
        registry.report_completion(&child1.id, "Result A".to_string()).unwrap();
        assert_eq!(registry.get(&child1.id).unwrap().status, SubAgentStatus::Completed);

        // Collect results — child1 done, child2 pending
        let (results, pending) = registry.collect_results(&parent.id);
        assert_eq!(results.len(), 1);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0], child2.id);

        // Simulate child2 completing
        registry.report_completion(&child2.id, "Result B".to_string()).unwrap();

        // Now all children finished
        assert!(registry.all_children_finished(&parent.id));
        let (results, pending) = registry.collect_results(&parent.id);
        assert_eq!(results.len(), 2);
        assert!(pending.is_empty());

        // Cleanup
        let _ = std::fs::remove_file("test_dispatch_agents.json");
    }

    #[test]
    fn test_progress_reporting() {
        let mut registry = SubAgentRegistry::new_with_path(
            &std::path::PathBuf::from("test_progress_agents.json"),
        ).unwrap();

        let parent = registry.create("coordinator".to_string(), "Parent".to_string(), None);
        let child = registry.create("worker".to_string(), "Worker".to_string(), Some(parent.id.clone()));

        // Report progress
        registry.report_progress(&child.id, 0.5).unwrap();
        assert!((registry.get(&child.id).unwrap().progress - 0.5).abs() < 1.0);

        // Parent should have received a ProgressReport message
        let msgs = registry.message_bus().receive_by_kind(&parent.id, AgentMessageKind::ProgressReport);
        assert_eq!(msgs.len(), 1);
        let progress: f32 = serde_json::from_str(&msgs[0].payload).unwrap();
        assert!((progress - 0.5).abs() < 1.0);

        // Cleanup
        let _ = std::fs::remove_file("test_progress_agents.json");
    }

    #[test]
    fn test_error_reporting() {
        let mut registry = SubAgentRegistry::new_with_path(
            &std::path::PathBuf::from("test_error_agents.json"),
        ).unwrap();

        let parent = registry.create("coordinator".to_string(), "Parent".to_string(), None);
        let child = registry.create("worker".to_string(), "Worker".to_string(), Some(parent.id.clone()));

        registry.report_error(&child.id, "Something went wrong".to_string()).unwrap();
        assert_eq!(registry.get(&child.id).unwrap().status, SubAgentStatus::Failed);

        let msgs = registry.message_bus().receive_by_kind(&parent.id, AgentMessageKind::TaskError);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].payload, "Something went wrong");

        // Cleanup
        let _ = std::fs::remove_file("test_error_agents.json");
    }

    #[test]
    fn test_task_deduplication() {
        let mut dedup = TaskDeduplicator::new(0.6);

        // Register first task
        dedup.register_task("Analyze the codebase structure and find bugs");

        // Same task should be detected as duplicate
        let dup = dedup.check_duplicate("Analyze the codebase structure and find bugs");
        assert!(dup.is_some());
        assert!(dup.unwrap() >= 0.6);

        // Similar task should also be detected
        let similar = dedup.check_duplicate("Analyze the codebase structure and find issues");
        assert!(similar.is_some());

        // Very different task should not be detected
        let different = dedup.check_duplicate("Write documentation for the API endpoints");
        assert!(different.is_none());
    }
}
