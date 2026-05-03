//! InProcessTeammate 任务 — 同进程 Swarm 队友任务载体
//! Feature flag: SWARM_MODE
//!
//! 与 SubAgent 的区别：
//! - SubAgent: 单次任务执行，完成后销毁
//! - InProcessTeammate: 持续运行的队友，可接收多个任务，支持消息通信

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

/// 队友任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeammateTaskStatus {
    Created,
    Initializing,
    Running,
    Idle,
    Completed,
    Failed,
}

/// 队友消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TeammateMessage {
    /// 任务分配
    TaskAssign {
        task_id: String,
        description: String,
    },
    /// 任务结果
    TaskResult {
        task_id: String,
        result: String,
        success: bool,
    },
    /// 进度报告
    ProgressReport {
        task_id: String,
        progress: f64,
        message: String,
    },
    /// 心跳
    Heartbeat,
    /// 关闭
    Shutdown,
    /// 自定义消息
    Custom {
        from: String,
        content: String,
    },
}

/// 同进程队友任务
pub struct InProcessTeammateTask {
    pub task_id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub team_name: String,
    pub status: TeammateTaskStatus,
    pub created_at: DateTime<Utc>,
    /// 消息发送通道（队友 → leader）
    pub outgoing_tx: mpsc::Sender<TeammateMessage>,
    /// 消息接收通道（leader → 队友）
    pub incoming_rx: mpsc::Receiver<TeammateMessage>,
    /// 当前执行的任务
    pub current_task: Option<String>,
    /// 已完成的任务计数
    pub completed_tasks: usize,
    /// 失败的任务计数
    pub failed_tasks: usize,
}

impl InProcessTeammateTask {
    /// 创建队友
    pub fn new(
        agent_name: &str,
        team_name: &str,
    ) -> (Self, mpsc::Sender<TeammateMessage>, mpsc::Receiver<TeammateMessage>) {
        let (outgoing_tx, outgoing_rx) = mpsc::channel(64);
        let (incoming_tx, incoming_rx) = mpsc::channel(64);

        // 注意：这里 outgoing 和 incoming 的视角是相对于 leader
        // leader 通过 outgoing_tx 发送消息给队友
        // leader 通过 incoming_rx 接收队友的消息
        let task = Self {
            task_id: uuid::Uuid::new_v4().to_string(),
            agent_id: super::super::swarm::team_helpers::teammate_id(agent_name, team_name),
            agent_name: agent_name.to_string(),
            team_name: team_name.to_string(),
            status: TeammateTaskStatus::Created,
            created_at: Utc::now(),
            outgoing_tx: incoming_tx, // 队友的发送通道 = leader 的接收通道
            incoming_rx: outgoing_rx, // 队友的接收通道 = leader 的发送通道
            current_task: None,
            completed_tasks: 0,
            failed_tasks: 0,
        };

        (task, outgoing_tx, incoming_rx)
    }

    /// 获取队友的确定性 ID
    pub fn teammate_id(&self) -> String {
        format!("{}@{}", self.agent_name, self.team_name)
    }

    /// 发送消息给队友
    pub async fn send_to_teammate(
        &mut self,
        message: TeammateMessage,
    ) -> Result<(), mpsc::error::SendError<TeammateMessage>> {
        self.outgoing_tx.send(message).await
    }

    /// 接收队友的消息
    pub async fn recv_from_teammate(
        &mut self,
    ) -> Option<TeammateMessage> {
        self.incoming_rx.recv().await
    }

    /// 分配任务给队友
    pub async fn assign_task(&mut self, task_id: &str, description: &str) -> Result<(), mpsc::error::SendError<TeammateMessage>> {
        self.current_task = Some(task_id.to_string());
        self.status = TeammateTaskStatus::Running;
        self.send_to_teammate(TeammateMessage::TaskAssign {
            task_id: task_id.to_string(),
            description: description.to_string(),
        }).await
    }

    /// 标记任务完成
    pub fn complete_task(&mut self, success: bool) {
        if success {
            self.completed_tasks += 1;
        } else {
            self.failed_tasks += 1;
        }
        self.current_task = None;
        self.status = TeammateTaskStatus::Idle;
    }

    /// 关闭队友
    pub async fn shutdown(&mut self) {
        let _ = self.send_to_teammate(TeammateMessage::Shutdown).await;
        self.status = TeammateTaskStatus::Completed;
    }
}
