//! 多 Agent 协作的全局工作记忆（Blackboard 模式）。
//!
//! 提供共享状态、决策记录、冲突仲裁和消息广播功能。

use std::collections::HashMap;

/// Agent 的一次决策
#[derive(Debug, Clone)]
pub struct AgentDecision {
    pub agent_id: String,
    pub timestamp_ms: u64,
    pub task_id: String,
    pub field: String,
    pub value: String,
}

/// Blackboard 消息
#[derive(Debug, Clone)]
pub struct BlackboardMessage {
    pub from: String,
    pub to: Option<String>,
    pub content: String,
    pub timestamp_ms: u64,
}

/// 冲突解决结果
#[derive(Debug, Clone)]
pub enum ConflictResolution {
    MajorityVote { winner: String, vote_count: usize },
    TieBreak { chosen: String, reason: String },
}

/// 冲突记录
#[derive(Debug, Clone)]
pub struct ConflictRecord {
    pub task_id: String,
    pub field: String,
    pub conflicting_decisions: Vec<AgentDecision>,
    pub resolution: ConflictResolution,
}

/// 多 Agent 协作的全局工作记忆
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
        votes
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(value, _)| value.to_string())
    }

    /// 检测并解决冲突
    pub fn resolve_conflicts(&mut self) -> Vec<ConflictRecord> {
        let mut records = Vec::new();
        let mut groups: HashMap<(String, String), Vec<&AgentDecision>> = HashMap::new();
        for d in &self.decisions {
            groups
                .entry((d.task_id.clone(), d.field.clone()))
                .or_default()
                .push(d);
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
            let winners: Vec<&&str> = votes
                .iter()
                .filter(|(_, &c)| c == max_votes)
                .map(|(v, _)| v)
                .collect();
            let resolution = if winners.len() == 1 {
                ConflictResolution::MajorityVote {
                    winner: winners[0].to_string(),
                    vote_count: max_votes,
                }
            } else {
                let first = decisions.iter().min_by_key(|d| d.timestamp_ms).unwrap();
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
}
