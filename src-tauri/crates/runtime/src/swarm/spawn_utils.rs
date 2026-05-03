//! 队友进程启动工具
//!
//! 根据 BackendType 选择合适的启动方式。
//! - `SubProcess`: 启动自身可执行文件作为子进程（--teammate 模式）
//! - `InProcess`: 同进程模式，返回错误提示应使用 InProcessTeammateTask
//! - `Tmux`: 在 tmux 会话中运行（仅 Unix）

use std::io::{BufRead, Write};
use std::process::{Child, Command, Stdio};

use crate::swarm::team_helpers::{BackendType, TeammateMessage};

/// 启动队友进程
///
/// 根据后端类型启动队友进程并返回子进程句柄。
/// - `SubProcess`: 启动当前可执行文件的 --teammate 模式子进程
/// - `InProcess`: 返回错误（同进程模式应在当前 runtime 中创建）
/// - `Tmux`: 在 tmux 会话中启动（仅 Unix）
pub fn spawn_teammate_process(
    agent_name: &str,
    team_name: &str,
    backend: BackendType,
) -> std::io::Result<Child> {
    match backend {
        BackendType::SubProcess => {
            let mut cmd = Command::new(std::env::current_exe()?);
            cmd.arg("--teammate")
                .arg("--agent-name")
                .arg(agent_name)
                .arg("--team-name")
                .arg(team_name)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            cmd.spawn()
        }
        BackendType::InProcess => {
            // 同进程模式由 InProcessTeammateTask 处理
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "InProcess 队友应在当前进程中创建",
            ))
        }
        BackendType::Tmux => {
            // tmux 仅 Unix 支持
            #[cfg(unix)]
            {
                let session_name = format!("{}-{}", team_name, agent_name);
                let mut cmd = Command::new("tmux");
                cmd.arg("new-session")
                    .arg("-d")
                    .arg("-s")
                    .arg(&session_name)
                    .arg(
                        std::env::current_exe()
                            .unwrap_or_else(|_| std::path::PathBuf::from("axagent")),
                    )
                    .arg("--teammate")
                    .arg("--agent-name")
                    .arg(agent_name)
                    .arg("--team-name")
                    .arg(team_name);
                cmd.spawn()
            }
            #[cfg(not(unix))]
            {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "tmux 后端在 Windows 上不可用，请使用 SubProcess",
                ))
            }
        }
    }
}

/// 向队友进程发送 JSON 行消息
///
/// 将 TeammateMessage 序列化为 JSON 并通过 stdin 发送，
/// 末尾追加换行符作为消息分隔符。
pub fn send_message(process: &mut Child, message: &TeammateMessage) -> std::io::Result<()> {
    if let Some(stdin) = process.stdin.as_mut() {
        let json = serde_json::to_string(message)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        stdin.write_all(json.as_bytes())?;
        stdin.write_all(&[b'\n'])?; // JSON 行分隔符
        stdin.flush()?;
    }
    Ok(())
}

/// 从队友进程读取 JSON 行消息
///
/// 从 stdout 读取一行 JSON 并反序列化为 TeammateMessage。
/// 返回 `None` 表示 EOF（进程已退出）。
pub fn read_message(process: &mut Child) -> std::io::Result<Option<TeammateMessage>> {
    if let Some(stdout) = process.stdout.as_mut() {
        let mut reader = std::io::BufReader::new(stdout);
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => Ok(None), // EOF
            Ok(_) => {
                let msg: TeammateMessage = serde_json::from_str(line.trim())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(Some(msg))
            }
            Err(e) => Err(e),
        }
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swarm::team_helpers::TeammateStatus;

    #[test]
    fn test_inprocess_not_supported() {
        let result = spawn_teammate_process("test", "team", BackendType::InProcess);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("InProcess"));
    }

    #[test]
    fn test_send_message_serialization() {
        // 验证消息可以正常序列化（不需要真实的子进程）
        let msg = TeammateMessage::Heartbeat {
            from: "test@team".into(),
            status: TeammateStatus::Idle,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("heartbeat"));
        assert!(json.contains("test@team"));
    }

    #[test]
    fn test_read_message_deserialization() {
        let json = r#"{"type":"heartbeat","from":"Alice@T","status":"Idle"}"#;
        let msg: TeammateMessage = serde_json::from_str(json).unwrap();
        match msg {
            TeammateMessage::Heartbeat { from, status } => {
                assert_eq!(from, "Alice@T");
                assert_eq!(status, TeammateStatus::Idle);
            }
            _ => panic!("应为 Heartbeat 消息"),
        }
    }

    #[test]
    fn test_task_result_deserialization() {
        let json =
            r#"{"type":"task_result","task_id":"t1","success":true,"content":"完成","from":"Bob@T"}"#;
        let msg: TeammateMessage = serde_json::from_str(json).unwrap();
        match msg {
            TeammateMessage::TaskResult {
                task_id,
                success,
                content,
                from,
            } => {
                assert_eq!(task_id, "t1");
                assert!(success);
                assert_eq!(content, "完成");
                assert_eq!(from, "Bob@T");
            }
            _ => panic!("应为 TaskResult 消息"),
        }
    }
}
