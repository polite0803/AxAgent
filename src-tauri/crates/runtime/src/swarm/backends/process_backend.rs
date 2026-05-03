//! Windows 子进程后端实现
//!
//! 通过 stdin/stdout JSON 行协议与队友进程通信。
//! 支持消息收发和进程生命周期管理。

use std::process::Child;

use crate::swarm::team_helpers::TeammateMessage;

/// 子进程后端管理器
///
/// 持有子进程句柄，提供消息收发和进程管理功能。
/// Drop 时自动终止子进程。
pub struct ProcessBackend {
    /// 子进程句柄
    pub child: Option<Child>,
    /// 队友名称
    pub agent_name: String,
    /// 团队名称
    pub team_name: String,
}

impl ProcessBackend {
    /// 创建新的进程后端（不启动进程）
    pub fn new(agent_name: &str, team_name: &str) -> Self {
        Self {
            child: None,
            agent_name: agent_name.to_string(),
            team_name: team_name.to_string(),
        }
    }

    /// 设置子进程句柄（由 spawn_utils 启动后赋值）
    pub fn set_child(&mut self, child: Child) {
        self.child = Some(child);
    }

    /// 发送消息到子进程
    ///
    /// 将消息序列化为 JSON 行并通过 stdin 发送。
    pub fn send(&mut self, message: &TeammateMessage) -> std::io::Result<()> {
        if let Some(ref mut child) = self.child {
            super::super::spawn_utils::send_message(child, message)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "队友进程未启动",
            ))
        }
    }

    /// 从子进程读取消息
    ///
    /// 从 stdout 读取一行 JSON 并反序列化为 TeammateMessage。
    /// 返回 `None` 表示没有消息或进程已退出。
    pub fn receive(&mut self) -> std::io::Result<Option<TeammateMessage>> {
        if let Some(ref mut child) = self.child {
            super::super::spawn_utils::read_message(child)
        } else {
            Ok(None)
        }
    }

    /// 检查子进程是否仍在运行
    pub fn is_alive(&mut self) -> bool {
        match self.child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(None) => true,  // 仍在运行
                Ok(Some(_)) => false, // 已退出
                Err(_) => false,
            },
            None => false,
        }
    }

    /// 等待子进程退出并获取退出状态
    pub fn wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        match self.child.as_mut() {
            Some(child) => child.wait().map(Some),
            None => Ok(None),
        }
    }

    /// 终止子进程
    ///
    /// 先尝试优雅终止（kill），再等待进程退出。
    pub fn kill(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.child = None;
    }
}

impl Drop for ProcessBackend {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_new() {
        let backend = ProcessBackend::new("Alice", "DreamTeam");
        assert_eq!(backend.agent_name, "Alice");
        assert_eq!(backend.team_name, "DreamTeam");
        assert!(backend.child.is_none());
    }

    #[test]
    fn test_send_without_child() {
        let mut backend = ProcessBackend::new("Test", "Team");
        let msg = TeammateMessage::Heartbeat {
            from: "Test@Team".into(),
            status: crate::swarm::team_helpers::TeammateStatus::Idle,
        };
        let result = backend.send(&msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_alive_no_child() {
        let mut backend = ProcessBackend::new("Test", "Team");
        assert!(!backend.is_alive());
    }
}
