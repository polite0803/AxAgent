// SPDX-License-Identifier: AGPL-3.0-only

//! 多 Agent 协作的全局工作记忆（Blackboard 模式）。
//!
//! 提供共享状态、决策记录、冲突仲裁和消息广播功能。
//!
//! ## 架构位置
//!
//! 本模块的 DTO(`AgentDecision` / `BlackboardMessage` / `ConflictResolution` /
//! `ConflictRecord`)和 trait(`SharedBlackboard`)权威定义在 `axagent-harness::multi_agent`。
//! 本模块提供具体内存实现 `SharedBlackboard` struct,并为其 `Arc<RwLock<...>>` 包装
//! 实现 harness trait,完成 P0 阶段的收口接入。

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

// 从 harness 引入共享 DTO 和 trait(收口标志 — 不再重复定义)
pub use axagent_harness::multi_agent::{
    AgentDecision, BlackboardMessage, ConflictRecord, ConflictResolution, SharedBlackboard as _,
};

/// 多 Agent 协作的全局工作记忆(内存实现)。
///
/// 字段公开供直接访问(向后兼容 session_manager 中的 `bb.write().await.field = ...` 模式)。
/// 若需要 harness trait 接口,使用 `Arc<RwLock<SharedBlackboard>>` 并通过 trait 方法调用。
#[derive(Debug)]
pub struct SharedBlackboard {
    pub task_id: String,
    pub goal: String,
    pub shared_state: HashMap<String, String>,
    pub decisions: Vec<AgentDecision>,
    pub messages: Vec<BlackboardMessage>,
    pub conflicts: Vec<ConflictRecord>,
}

impl SharedBlackboard {
    /// 创建新的 Blackboard
    pub fn new(task_id: impl Into<String>, goal: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            goal: goal.into(),
            shared_state: HashMap::new(),
            decisions: Vec::new(),
            messages: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    /// 记录 Agent 决策
    pub fn record_decision(&mut self, agent_id: &str, task_id: &str, field: &str, value: &str) {
        let decision = AgentDecision {
            agent_id: agent_id.to_string(),
            timestamp_ms: now_ms(),
            task_id: task_id.to_string(),
            field: field.to_string(),
            value: value.to_string(),
        };
        self.decisions.push(decision);
    }

    /// 设置共享状态
    pub fn set_state(&mut self, key: &str, value: &str) {
        self.shared_state.insert(key.to_string(), value.to_string());
    }

    /// 读取共享状态
    pub fn get_state(&self, key: &str) -> Option<&String> {
        self.shared_state.get(key)
    }

    /// 获取所有 Agent 对某个 field 的共识值
    pub fn get_consensus(&self, field: &str) -> Option<String> {
        let relevant: Vec<&AgentDecision> =
            self.decisions.iter().filter(|d| d.field == field).collect();
        if relevant.is_empty() {
            return None;
        }
        let mut votes: HashMap<&str, usize> = HashMap::new();
        for d in &relevant {
            *votes.entry(&d.value).or_default() += 1;
        }
        votes.into_iter().max_by_key(|(_, count)| *count).map(|(value, _)| value.to_string())
    }

    /// 检测并解决冲突
    pub fn resolve_conflicts(&mut self) -> Vec<ConflictRecord> {
        let mut records = Vec::new();
        let mut groups: HashMap<(String, String), Vec<&AgentDecision>> = HashMap::new();
        for d in &self.decisions {
            groups.entry((d.task_id.clone(), d.field.clone())).or_default().push(d);
        }
        for ((task_id, field), decisions) in groups {
            if decisions.len() < 2 {
                continue;
            }
            let mut votes: HashMap<&str, usize> = HashMap::new();
            for d in &decisions {
                *votes.entry(&d.value).or_default() += 1;
            }
            let max_votes = votes.values().max().copied().unwrap_or(0);
            let winners: Vec<&&str> =
                votes.iter().filter(|(_, c)| **c == max_votes).map(|(v, _)| v).collect();
            let resolution = if winners.len() == 1 {
                ConflictResolution::MajorityVote {
                    winner: winners[0].to_string(),
                    vote_count: max_votes,
                }
            } else {
                let first = decisions
                    .iter()
                    .min_by_key(|d| d.timestamp_ms)
                    .expect("decisions is non-empty (len >= 2 checked above)");
                ConflictResolution::TieBreak {
                    chosen: first.value.clone(),
                    reason: "平局，选择首个完成者的决策".to_string(),
                }
            };
            records.push(ConflictRecord {
                task_id,
                field,
                conflicting_decisions: decisions.iter().map(|&d| d.clone()).collect(),
                resolution,
            });
        }
        self.conflicts.extend(records.clone());
        records
    }

    /// 广播消息到所有 Agent
    pub fn broadcast(&mut self, from: &str, content: &str) {
        self.messages.push(BlackboardMessage {
            from: from.to_string(),
            to: None,
            content: content.to_string(),
            timestamp_ms: now_ms(),
        });
    }

    /// 获取发给特定 Agent 的消息（含广播）
    pub fn get_messages_for(&self, agent_id: &str) -> Vec<&BlackboardMessage> {
        self.messages
            .iter()
            .filter(|m| m.to.is_none() || m.to.as_deref() == Some(agent_id))
            .collect()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================================
// harness SharedBlackboard trait 实现(收口接入)
// ============================================================================
//
// 由于孤儿规则(E0117),不能直接为 `Arc<RwLock<SharedBlackboard>>`(外部类型组合)
// 实现 harness trait。这里通过 newtype `BlackboardHandle` 包装,
// 让 trait impl 落在 agent crate 本地类型上。
//
// `BlackboardHandle` 通过 `Deref` 暴露底层 `Arc<RwLock<SharedBlackboard>>` 的所有方法,
// 因此与原 `Arc<RwLock<SharedBlackboard>>` API 完全兼容(session_manager 可无缝替换)。

/// 多 Agent 协作的黑板句柄(newtype 包装,用于实现 harness trait)。
///
/// 包装 `Arc<RwLock<SharedBlackboard>>`,通过 `Deref` 暴露底层 API,
/// 同时作为 harness `SharedBlackboard` trait 的实现载体。
///
/// ## 用法
///
/// ```ignore
/// use axagent_agent::shared_blackboard::{BlackboardHandle, SharedBlackboard};
///
/// let handle = BlackboardHandle::new("task-1", "test goal");
/// // 作为 trait 对象使用
/// let trait_obj: Arc<dyn axagent_harness::SharedBlackboard> = handle.clone().into();
/// // 作为底层 Arc<RwLock<...>> 使用(通过 Deref)
/// handle.write().await.set_state("k", "v");
/// ```
#[derive(Debug, Clone)]
pub struct BlackboardHandle(pub Arc<RwLock<SharedBlackboard>>);

impl BlackboardHandle {
    /// 创建新的 BlackboardHandle(内部新建 SharedBlackboard)。
    pub fn new(task_id: impl Into<String>, goal: impl Into<String>) -> Self {
        Self(Arc::new(RwLock::new(SharedBlackboard::new(task_id, goal))))
    }

    /// 从已有的 `Arc<RwLock<SharedBlackboard>>` 包装为 handle。
    pub fn from_arc(arc: Arc<RwLock<SharedBlackboard>>) -> Self {
        Self(arc)
    }

    /// 获取内部 `Arc<RwLock<SharedBlackboard>>` 的克隆(便于传统 API 调用)。
    pub fn inner(&self) -> Arc<RwLock<SharedBlackboard>> {
        self.0.clone()
    }
}

impl std::ops::Deref for BlackboardHandle {
    type Target = Arc<RwLock<SharedBlackboard>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<RwLock<SharedBlackboard>>> for BlackboardHandle {
    fn from(arc: Arc<RwLock<SharedBlackboard>>) -> Self {
        Self(arc)
    }
}

impl From<BlackboardHandle> for Arc<RwLock<SharedBlackboard>> {
    fn from(handle: BlackboardHandle) -> Self {
        handle.0
    }
}

#[async_trait::async_trait]
impl axagent_harness::SharedBlackboard for BlackboardHandle {
    async fn record_decision(
        &self,
        agent_id: &str,
        task_id: &str,
        field: &str,
        value: &str,
    ) -> Result<(), String> {
        let mut bb = self.0.write().await;
        bb.record_decision(agent_id, task_id, field, value);
        Ok(())
    }

    async fn set_state(&self, key: &str, value: &str) -> Result<(), String> {
        let mut bb = self.0.write().await;
        bb.set_state(key, value);
        Ok(())
    }

    async fn get_state(&self, key: &str) -> Option<String> {
        let bb = self.0.read().await;
        bb.get_state(key).cloned()
    }

    async fn get_consensus(&self, field: &str) -> Option<String> {
        let bb = self.0.read().await;
        bb.get_consensus(field)
    }

    async fn resolve_conflicts(&self) -> Result<Vec<ConflictRecord>, String> {
        let mut bb = self.0.write().await;
        Ok(bb.resolve_conflicts())
    }

    async fn broadcast(&self, from: &str, content: &str) -> Result<(), String> {
        let mut bb = self.0.write().await;
        bb.broadcast(from, content);
        Ok(())
    }

    async fn get_messages_for(&self, agent_id: &str) -> Vec<BlackboardMessage> {
        let bb = self.0.read().await;
        bb.get_messages_for(agent_id).into_iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_read_decision() {
        let mut bb = SharedBlackboard::new("task-1", "test goal");
        bb.record_decision("agent-a", "task-1", "next_action", "call_api");
        assert_eq!(bb.decisions.len(), 1);
        assert_eq!(bb.decisions[0].value, "call_api");
    }

    #[test]
    fn consensus_returns_majority_value() {
        let mut bb = SharedBlackboard::new("task-1", "test");
        bb.record_decision("a", "task-1", "result", "A");
        bb.record_decision("b", "task-1", "result", "A");
        bb.record_decision("c", "task-1", "result", "B");
        assert_eq!(bb.get_consensus("result"), Some("A".to_string()));
    }

    #[test]
    fn conflict_majority_vote_wins() {
        let mut bb = SharedBlackboard::new("task-1", "test");
        bb.record_decision("a", "task-1", "action", "deploy");
        bb.record_decision("b", "task-1", "action", "deploy");
        bb.record_decision("c", "task-1", "action", "rollback");
        let records = bb.resolve_conflicts();
        assert_eq!(records.len(), 1);
        match &records[0].resolution {
            ConflictResolution::MajorityVote { winner, vote_count } => {
                assert_eq!(winner, "deploy");
                assert_eq!(*vote_count, 2);
            },
            _ => panic!("expected MajorityVote"),
        }
    }

    #[test]
    fn conflict_tiebreak_chooses_first() {
        let mut bb = SharedBlackboard::new("task-1", "test");
        bb.record_decision("a", "task-1", "action", "X");
        std::thread::sleep(std::time::Duration::from_millis(2));
        bb.record_decision("b", "task-1", "action", "Y");
        let records = bb.resolve_conflicts();
        assert_eq!(records.len(), 1);
        match &records[0].resolution {
            ConflictResolution::TieBreak { chosen, .. } => {
                assert_eq!(chosen, "X");
            },
            _ => panic!("expected TieBreak"),
        }
    }

    #[test]
    fn broadcast_and_receive() {
        let mut bb = SharedBlackboard::new("task-1", "test");
        bb.broadcast("agent-a", "hello all");
        let msgs = bb.get_messages_for("agent-b");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "hello all");
    }

    #[test]
    fn shared_state_read_write() {
        let mut bb = SharedBlackboard::new("task-1", "test");
        bb.set_state("status", "in_progress");
        assert_eq!(bb.get_state("status"), Some(&"in_progress".to_string()));
    }

    // ── harness trait 接入测试 ──
    //
    // 这些测试通过 `BlackboardHandle` 调用 harness `SharedBlackboard` trait,
    // 验证收口接入正确性。`BlackboardHandle` 是 agent crate 本地 newtype,
    // 包装 `Arc<RwLock<SharedBlackboard>>` 并实现 harness trait(规避 E0117 孤儿规则)。

    #[tokio::test]
    async fn harness_trait_record_and_get_state() {
        let bb = BlackboardHandle::new("task-1", "test");
        bb.set_state("status", "running").await.expect("测试：异步操作应成功");
        let val = bb.get_state("status").await;
        assert_eq!(val, Some("running".to_string()));
    }

    #[tokio::test]
    async fn harness_trait_record_decision_and_consensus() {
        let bb = BlackboardHandle::new("task-1", "test");
        bb.record_decision("a", "task-1", "result", "A").await.expect("测试：异步操作应成功");
        bb.record_decision("b", "task-1", "result", "A").await.expect("测试：异步操作应成功");
        bb.record_decision("c", "task-1", "result", "B").await.expect("测试：异步操作应成功");
        let consensus = bb.get_consensus("result").await;
        assert_eq!(consensus, Some("A".to_string()));
    }

    #[tokio::test]
    async fn harness_trait_broadcast_and_get_messages() {
        let bb = BlackboardHandle::new("task-1", "test");
        bb.broadcast("agent-a", "hello all").await.expect("测试：异步操作应成功");
        let msgs = bb.get_messages_for("agent-b").await;
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "hello all");
    }

    #[tokio::test]
    async fn harness_trait_resolve_conflicts() {
        let bb = BlackboardHandle::new("task-1", "test");
        bb.record_decision("a", "task-1", "action", "deploy").await.expect("测试：异步操作应成功");
        bb.record_decision("b", "task-1", "action", "deploy").await.expect("测试：异步操作应成功");
        bb.record_decision("c", "task-1", "action", "rollback")
            .await
            .expect("测试：异步操作应成功");
        let records = bb.resolve_conflicts().await.expect("测试：异步操作应成功");
        assert_eq!(records.len(), 1);
    }

    /// 验证 `BlackboardHandle` 可作为 `dyn SharedBlackboard` trait 对象使用(收口验证)。
    ///
    /// 这是收口的关键验证点:harness 层定义的 trait 可以承载 agent crate 的具体实现,
    /// 从而允许 consumer crate(orchestrator/runtime-core/gateway)通过 trait 接口
    /// 操控多 Agent 协作,不依赖 agent crate 的具体类型。
    #[tokio::test]
    async fn harness_trait_object_compatibility() {
        // 直接通过 BlackboardHandle 调用 trait 方法(验证 trait impl 正确)
        let bb = BlackboardHandle::new("task-1", "test");
        bb.set_state("k", "v").await.expect("测试：异步操作应成功");
        let val = bb.get_state("k").await;
        assert_eq!(val, Some("v".to_string()));

        // 验证可作为 trait 对象使用:用 Arc<BlackboardHandle> coerce 到 Arc<dyn Trait>
        let bb_arc: Arc<BlackboardHandle> = Arc::new(BlackboardHandle::new("task-2", "test2"));
        let trait_obj: Arc<dyn axagent_harness::SharedBlackboard> = bb_arc;
        trait_obj.set_state("x", "y").await.expect("测试：异步操作应成功");
        let val = trait_obj.get_state("x").await;
        assert_eq!(val, Some("y".to_string()));
    }
}
