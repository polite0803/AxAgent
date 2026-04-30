use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

use super::backend_trait::{
    BackendType, SpawnConfig, TerminalBackend, TerminalExit, TerminalOutput,
};

#[allow(dead_code)]
pub struct SshBackend {
    host: String,
    port: u16,
    username: Option<String>,
    key_path: Option<String>,
    password: Option<String>,
    connected: Arc<RwLock<bool>>,
    sessions: Arc<RwLock<HashMap<String, tokio::process::Child>>>,
}

impl SshBackend {
    pub fn new(
        host: String,
        port: Option<u16>,
        username: Option<String>,
        key_path: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            host,
            port: port.unwrap_or(22),
            username,
            key_path,
            password,
            connected: Arc::new(RwLock::new(false)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn build_ssh_args(&self) -> Vec<String> {
        let mut args = vec![
            "-o".to_string(),
            "StrictHostKeyChecking=no".to_string(),
            "-o".to_string(),
            "UserKnownHostsFile=/dev/null".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
        ];

        if let Some(ref key) = self.key_path {
            args.push("-i".to_string());
            args.push(key.clone());
        }

        if let Some(ref user) = self.username {
            args.push(format!("{}@{}", user, self.host));
        } else {
            args.push(self.host.clone());
        }

        args
    }

    async fn run_ssh_command(
        &self,
        command: &str,
        timeout_secs: u64,
    ) -> anyhow::Result<(String, i32)> {
        let mut ssh_args = self.build_ssh_args();
        ssh_args.push(command.to_string());

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::process::Command::new("ssh")
                .args(&ssh_args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("SSH command timed out"))??;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        if exit_code != 0 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("SSH command failed ({}): {}", exit_code, stderr);
        }

        Ok((stdout, exit_code))
    }
}

#[async_trait]
impl TerminalBackend for SshBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Ssh
    }

    async fn connect(&self) -> anyhow::Result<()> {
        let (_, code) = self
            .run_ssh_command("echo connected", 10)
            .await
            .map_err(|e| anyhow::anyhow!("SSH connection failed: {}", e))?;

        if code != 0 {
            anyhow::bail!("SSH connection test failed with code {}", code);
        }

        let mut c = self.connected.write().await;
        *c = true;
        Ok(())
    }

    async fn disconnect(&self) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        for (_, mut child) in sessions.drain() {
            let _ = child.kill().await;
        }

        let mut c = self.connected.write().await;
        *c = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn spawn_session(&self, session_id: &str, config: SpawnConfig) -> anyhow::Result<()> {
        let shell = config.shell.unwrap_or_else(|| "/bin/bash".to_string());

        let mut ssh_args = self.build_ssh_args();
        if let Some(cwd) = &config.cwd {
            ssh_args.push(format!("cd {} && exec {}", cwd, shell));
        } else {
            ssh_args.push(format!("exec {}", shell));
        }

        let child = tokio::process::Command::new("ssh")
            .args(&ssh_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), child);

        Ok(())
    }

    async fn write_to_session(&self, session_id: &str, data: &[u8]) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        let child = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(data).await?;
            stdin.flush().await?;
        }

        Ok(())
    }

    async fn resize_session(
        &self,
        _session_id: &str,
        _rows: u16,
        _cols: u16,
    ) -> anyhow::Result<()> {
        anyhow::bail!("SSH PTY resize not supported via CLI-based backend");
    }

    async fn kill_session(&self, session_id: &str) -> anyhow::Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(mut child) = sessions.remove(session_id) {
            let _ = child.kill().await;
        }
        Ok(())
    }

    async fn read_output(&self, _session_id: &str) -> anyhow::Result<Vec<TerminalOutput>> {
        // For the SSH backend, output is handled by the child process pipes
        // The caller should poll stdout/stderr directly
        Ok(Vec::new())
    }

    async fn wait_for_exit(&self, session_id: &str) -> anyhow::Result<TerminalExit> {
        let mut sessions = self.sessions.write().await;
        let child = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let status = child.wait().await?;
        let exit_code = status.code();

        Ok(TerminalExit {
            session_id: session_id.to_string(),
            exit_code,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn list_sessions(&self) -> anyhow::Result<Vec<String>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.keys().cloned().collect())
    }
}
