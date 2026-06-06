//! InProcessTeammate 任务 — 同进程 Swarm 队友任务载体
//! Feature flag: SWARM_MODE
//!
//! 与 SubAgent 的区别：
//! - SubAgent: 单次任务执行，完成后销毁
//! - InProcessTeammate: 持续运行的队友，可接收多个任务，支持消息通信

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
    Custom { from: String, content: String },
}

/// 同进程队友任务
///
/// 通道方向（以队友自身视角命名）：
/// - `cmd_rx`：从 leader 接收命令（leader → teammate）
/// - `evt_tx`：向 leader 发送事件（teammate → leader）
pub struct InProcessTeammateTask {
    pub task_id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub team_name: String,
    pub status: TeammateTaskStatus,
    pub created_at: DateTime<Utc>,
    /// 命令接收通道（leader → 队友）
    pub cmd_rx: mpsc::Receiver<TeammateMessage>,
    /// 事件发送通道（队友 → leader）
    pub evt_tx: mpsc::Sender<TeammateMessage>,
    /// 当前执行的任务
    pub current_task: Option<String>,
    /// 已完成的任务计数
    pub completed_tasks: usize,
    /// 失败的任务计数
    pub failed_tasks: usize,
}

impl InProcessTeammateTask {
    /// 创建队友
    ///
    /// 返回三元组：
    /// - `task`：队友自身，持有 `cmd_rx`（收命令）和 `evt_tx`（发事件）
    /// - `cmd_tx`：返回给 leader，用于向队友发送命令
    /// - `evt_rx`：返回给 leader，用于接收队友的事件
    pub fn new(
        agent_name: &str,
        team_name: &str,
    ) -> (Self, mpsc::Sender<TeammateMessage>, mpsc::Receiver<TeammateMessage>) {
        // 通道 1：命令通道，leader → teammate
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        // 通道 2：事件通道，teammate → leader
        let (evt_tx, evt_rx) = mpsc::channel(64);

        let task = Self {
            task_id: uuid::Uuid::new_v4().to_string(),
            agent_id: super::super::swarm::team_helpers::teammate_id(agent_name, team_name),
            agent_name: agent_name.to_string(),
            team_name: team_name.to_string(),
            status: TeammateTaskStatus::Created,
            created_at: Utc::now(),
            cmd_rx, // 队友从 leader 收命令
            evt_tx, // 队友向 leader 发事件
            current_task: None,
            completed_tasks: 0,
            failed_tasks: 0,
        };

        (task, cmd_tx, evt_rx)
    }

    /// 获取队友的确定性 ID（直接复用已存的 agent_id，避免重复格式化）
    pub fn teammate_id(&self) -> String {
        self.agent_id.clone()
    }

    /// 向 leader 发送事件
    pub async fn send_event(
        &self,
        message: TeammateMessage,
    ) -> Result<(), mpsc::error::SendError<TeammateMessage>> {
        self.evt_tx.send(message).await
    }

    /// 接收 leader 的命令
    pub async fn recv_command(&mut self) -> Option<TeammateMessage> {
        self.cmd_rx.recv().await
    }

    /// 分配任务给队友
    pub async fn assign_task(
        &mut self,
        task_id: &str,
        description: &str,
    ) -> Result<(), mpsc::error::SendError<TeammateMessage>> {
        self.current_task = Some(task_id.to_string());
        self.status = TeammateTaskStatus::Running;
        self.send_event(TeammateMessage::TaskAssign {
            task_id: task_id.to_string(),
            description: description.to_string(),
        })
        .await
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
        let _ = self.send_event(TeammateMessage::Shutdown).await;
        self.status = TeammateTaskStatus::Completed;
    }
}
