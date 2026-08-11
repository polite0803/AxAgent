// SPDX-License-Identifier: AGPL-3.0-only

//! 多 Agent 协作契约 — Swarm / Debate / SharedBlackboard 三种模式的统一抽象。
//!
//! ## 设计动机
//!
//! AxAgent 当前存在三种多 Agent 协作实现:
//! - **SharedBlackboard**: 定义在 `agent` crate,违反铁律 4(共享类型权威应在 harness)
//! - **Swarm**: 双实现 — `rt-workflow/swarm_executor`(DAG 内)+ `runtime/swarm/`(跨进程)
//! - **Debate**: 双实现 — `rt-workflow/debate_executor`(空容器)+ `runtime/adversarial_debate`(完整引擎,死代码)
//!
//! 本模块在 harness 层定义:
//! 1. **共享 DTO** — `AgentDecision` / `BlackboardMessage` / `ConflictRecord` / `ConflictResolution`
//!    (从 agent crate 上移,解决铁律 4 违规)
//! 2. **`SharedBlackboard` trait** — Blackboard 模式的统一接口
//! 3. **`MultiAgentCoordination` trait + `CoordinationMode` 枚举** — Swarm/Debate/Blackboard
//!    三种协作模式的统一抽象,供 orchestrator 上层调用
//!
//! ## 实现方
//!
//! - `agent::SharedBlackboard` struct → 实现 `harness::SharedBlackboard` trait
//! - `rt-workflow::SwarmExecutor` → 未来实现 `MultiAgentCoordination` trait
//! - `rt-workflow::DebateExecutor` → 未来实现 `MultiAgentCoordination` trait
//! - `runtime::adversarial_debate::DebateManager` → 待迁移或删除

use serde::{Deserialize, Serialize};

// ============================================================================
// 共享 DTO(从 agent/shared_blackboard.rs 上移)
// ============================================================================

/// Agent 的一次决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDecision {
    pub agent_id: String,
    pub timestamp_ms: u64,
    pub task_id: String,
    pub field: String,
    pub value: String,
}

/// Blackboard 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboardMessage {
    pub from: String,
    pub to: Option<String>,
    pub content: String,
    pub timestamp_ms: u64,
}

/// 冲突解决结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    MajorityVote { winner: String, vote_count: usize },
    TieBreak { chosen: String, reason: String },
}

/// 冲突记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub task_id: String,
    pub field: String,
    pub conflicting_decisions: Vec<AgentDecision>,
    pub resolution: ConflictResolution,
}

// ============================================================================
// SharedBlackboard trait
// ============================================================================

/// 多 Agent 协作的全局工作记忆接口(Blackboard 模式)。
///
/// 提供共享状态、决策记录、冲突仲裁和消息广播功能的统一抽象。
///
/// ## 实现方
///
/// - `agent::SharedBlackboard`(原 agent crate 中的 struct) — 内存实现
/// - 未来可扩展:基于 Redis / 数据库的分布式实现
///
/// ## 异步设计
///
/// 所有方法都是 `async fn`,因为分布式实现(Redis / DB)需要 await I/O。
/// 内存实现可使用 `tokio::sync::RwLock` 提供内部可变性。
///
/// ## 错误处理
///
/// 返回 `Result<T, String>`(与 `WorkflowHookSink` 一致),实现方把内部错误
/// 转换为 `String` 返回,不传播 panic。
#[async_trait::async_trait]
pub trait SharedBlackboard: Send + Sync {
    /// 记录 Agent 决策
    async fn record_decision(
        &self,
        agent_id: &str,
        task_id: &str,
        field: &str,
        value: &str,
    ) -> Result<(), String>;

    /// 设置共享状态(需 `&self` + 内部可变性,实现方可使用 `RwLock`)
    async fn set_state(&self, key: &str, value: &str) -> Result<(), String>;

    /// 读取共享状态,返回克隆以避免生命周期耦合
    async fn get_state(&self, key: &str) -> Option<String>;

    /// 获取所有 Agent 对某个 field 的共识值(多数投票)
    async fn get_consensus(&self, field: &str) -> Option<String>;

    /// 检测并解决冲突,返回本次解决的冲突记录
    async fn resolve_conflicts(&self) -> Result<Vec<ConflictRecord>, String>;

    /// 广播消息到所有 Agent
    async fn broadcast(&self, from: &str, content: &str) -> Result<(), String>;

    /// 获取发给特定 Agent 的消息(含广播),返回克隆
    async fn get_messages_for(&self, agent_id: &str) -> Vec<BlackboardMessage>;
}

// ============================================================================
// MultiAgentCoordination trait + CoordinationMode 枚举
// ============================================================================

/// 多 Agent 协作模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationMode {
    /// Swarm — 任务派发式协作,中心化调度
    Swarm,
    /// Debate — 对抗式辩论,通过评分/反驳收敛
    Debate,
    /// SharedBlackboard — 共享黑板模式,Agent 通过共享状态协作
    Blackboard,
}

/// 多 Agent 协作统一抽象。
///
/// Swarm / Debate / SharedBlackboard 三种模式实现此 trait,
/// 让 orchestrator / agent 上层可以按统一接口调用,不关心具体协作模式。
///
/// ## 设计要点
///
/// - **异步**: 所有方法都是 `async fn`,实现方使用 `#[async_trait]`
/// - **错误隔离**: 内部错误转为 `String` 返回,不传播 panic
/// - **模式自描述**: 通过 `mode()` 方法让调用方知道当前协作模式
///
/// ## 接入计划
///
/// - **P0**: trait 定义 + SharedBlackboard 实现接入(本阶段)
/// - **P1**: SwarmExecutor 实现接入
/// - **P2**: DebateExecutor 实现接入 + adversarial_debate 收口
#[async_trait::async_trait]
pub trait MultiAgentCoordination: Send + Sync {
    /// 当前协作模式
    fn mode(&self) -> CoordinationMode;

    /// 启动一次协作会话,返回 session_id
    async fn start_session(
        &self,
        task_id: &str,
        goal: &str,
        participants: Vec<String>,
    ) -> Result<String, String>;

    /// 提交一次提案/决策到协作会话
    async fn propose(&self, session_id: &str, agent_id: &str, proposal: &str)
    -> Result<(), String>;

    /// 查询当前协作结果(共识/收敛状态)
    async fn current_result(&self, session_id: &str) -> Result<CoordinationOutcome, String>;

    /// 关闭协作会话
    async fn close_session(&self, session_id: &str) -> Result<(), String>;
}

/// 协作结果(共识/收敛状态)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationOutcome {
    pub session_id: String,
    pub mode: CoordinationMode,
    /// 是否已达成共识/收敛
    pub converged: bool,
    /// 共识内容(若 converged=true)
    pub consensus: Option<String>,
    /// 参与者列表
    pub participants: Vec<String>,
    /// 协作轮次
    pub rounds: u32,
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_decision_serialization() {
        let d = AgentDecision {
            agent_id: "a1".to_string(),
            timestamp_ms: 1000,
            task_id: "t1".to_string(),
            field: "action".to_string(),
            value: "deploy".to_string(),
        };
        let json = serde_json::to_string(&d).expect("测试：JSON序列化应成功");
        let parsed: AgentDecision = serde_json::from_str(&json).expect("测试：JSON反序列化应成功");
        assert_eq!(parsed.agent_id, "a1");
        assert_eq!(parsed.value, "deploy");
    }

    #[test]
    fn coordination_mode_serde() {
        let json = serde_json::to_string(&CoordinationMode::Swarm).expect("测试：JSON序列化应成功");
        assert_eq!(json, r#""swarm""#);
        let m: CoordinationMode =
            serde_json::from_str(r#""debate""#).expect("测试：JSON反序列化应成功");
        assert_eq!(m, CoordinationMode::Debate);
    }

    #[test]
    fn conflict_resolution_variants() {
        let majority = ConflictResolution::MajorityVote { winner: "A".to_string(), vote_count: 3 };
        let tie =
            ConflictResolution::TieBreak { chosen: "X".to_string(), reason: "平局".to_string() };
        let m_json = serde_json::to_string(&majority).expect("测试：JSON序列化应成功");
        let t_json = serde_json::to_string(&tie).expect("测试：JSON序列化应成功");
        assert!(m_json.contains("MajorityVote"));
        assert!(t_json.contains("TieBreak"));
    }
}
