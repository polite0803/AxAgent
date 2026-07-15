// SPDX-License-Identifier: AGPL-3.0-only

//! Real-time 流式管道基础设施 —— 多 Agent 实时协作场景的契约层。
//!
//! 当前 `OrchestratorExecutor` 是 DAG 批处理式：子任务完成后才通过
//! `OrchestrationEvent` 通知外部。本模块定义 `AgentStreamReporter` trait，
//! 让 orchestrator 在子任务执行过程中也能向上游推送流式 chunk
//! （文本增量、工具调用、进度等），为未来多 Agent 实时协作打基础。
//!
//! 设计原则：
//! - 仅定义 trait 和 DTO，零实现逻辑（NoopStreamReporter 除外）
//! - 不依赖任何 axagent-* crate，符合 harness foundation 定位
//! - consumer crate（orchestrator / agent）通过 `Arc<dyn AgentStreamReporter>` 注入

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// 流式 chunk 类型 —— 标识 chunk 的语义类别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamChunkKind {
    /// LLM 文本输出增量
    TextDelta,
    /// 工具调用请求
    ToolUse,
    /// 工具调用结果
    ToolResult,
    /// 进度通知
    Progress,
    /// 子任务完成
    Completed,
    /// 子任务失败
    Failed,
}

/// Agent 流式输出 chunk —— 单个流式事件的载体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStreamChunk {
    /// 产生此 chunk 的 agent 标识
    pub agent_id: String,
    /// 关联的子任务标识
    pub sub_task_id: String,
    /// chunk 类型
    pub kind: StreamChunkKind,
    /// chunk 负载（具体内容因 kind 而异，如 TextDelta 时为 {"text": "..."}）
    pub payload: serde_json::Value,
    /// 毫秒级 Unix 时间戳
    pub timestamp: i64,
}

/// Agent 流式报告器 trait —— 用于多 Agent 实时协作场景。
///
/// 让 `OrchestratorExecutor` 能在子任务执行过程中接收流式输出，
/// 而非仅依赖完成后的 `OrchestrationEvent`。
///
/// 典型用法：
/// 1. consumer crate 创建 reporter 实例并注入到 `OrchestratorExecutor`
/// 2. 上游（如 Tauri 前端）通过 `subscribe(agent_id)` 获取 receiver 持续接收 chunk
/// 3. orchestrator 在子任务执行中调用 `report_chunk` 向所有订阅者推送
pub trait AgentStreamReporter: Send + Sync {
    /// 报告一个流式 chunk，将广播给对应 agent 的所有订阅者
    fn report_chunk(&self, chunk: AgentStreamChunk);

    /// 订阅指定 agent 的流式输出，返回 receiver 持续接收 chunk
    ///
    /// 注意：返回 `mpsc::Receiver<AgentStreamChunk>`，调用方需在异步上下文中消费
    fn subscribe(&self, agent_id: &str) -> mpsc::Receiver<AgentStreamChunk>;
}

/// Noop 实现 —— 用于不需要流式的场景（如单测、离线批处理）
pub struct NoopStreamReporter;

impl AgentStreamReporter for NoopStreamReporter {
    fn report_chunk(&self, _chunk: AgentStreamChunk) {
        // no-op：直接丢弃
    }

    fn subscribe(&self, _agent_id: &str) -> mpsc::Receiver<AgentStreamChunk> {
        // 返回一个永远不会收到消息的 receiver（channel 容量为 1，sender 被丢弃）
        // 调用方 await recv() 会阻塞直到被取消
        let (_tx, rx) = mpsc::channel(1);
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_serialization() {
        let chunk = AgentStreamChunk {
            agent_id: "agent-1".to_string(),
            sub_task_id: "task-1".to_string(),
            kind: StreamChunkKind::TextDelta,
            payload: serde_json::json!({"text": "hello"}),
            timestamp: 1700000000,
        };
        let json = serde_json::to_string(&chunk).unwrap();
        let deserialized: AgentStreamChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.agent_id, "agent-1");
        assert_eq!(deserialized.sub_task_id, "task-1");
        match deserialized.kind {
            StreamChunkKind::TextDelta => {},
            _ => panic!("expected TextDelta"),
        }
    }

    #[test]
    fn test_noop_reporter() {
        let reporter = NoopStreamReporter;
        let chunk = AgentStreamChunk {
            agent_id: "agent-1".to_string(),
            sub_task_id: "task-1".to_string(),
            kind: StreamChunkKind::Progress,
            payload: serde_json::json!({"percent": 50}),
            timestamp: 1700000000,
        };
        // 不应 panic
        reporter.report_chunk(chunk);
    }
}
