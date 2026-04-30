use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutput {
    pub session_id: String,
    pub data: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalExit {
    pub session_id: String,
    pub exit_code: Option<i32>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendType {
    Local,
    Docker,
    Ssh,
}

impl BackendType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackendType::Local => "local",
            BackendType::Docker => "docker",
            BackendType::Ssh => "ssh",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnConfig {
    pub shell: Option<String>,
    pub cwd: Option<String>,
    pub env: HashMap<String, String>,
    pub rows: u16,
    pub cols: u16,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            shell: None,
            cwd: None,
            env: HashMap::new(),
            rows: 24,
            cols: 80,
        }
    }
}

#[async_trait]
pub trait TerminalBackend: Send + Sync {
    fn backend_type(&self) -> BackendType;

    async fn connect(&self) -> anyhow::Result<()>;

    async fn disconnect(&self) -> anyhow::Result<()>;

    async fn is_connected(&self) -> bool;

    async fn spawn_session(&self, session_id: &str, config: SpawnConfig) -> anyhow::Result<()>;

    async fn write_to_session(&self, session_id: &str, data: &[u8]) -> anyhow::Result<()>;

    async fn resize_session(&self, session_id: &str, rows: u16, cols: u16) -> anyhow::Result<()>;

    async fn kill_session(&self, session_id: &str) -> anyhow::Result<()>;

    async fn read_output(&self, session_id: &str) -> anyhow::Result<Vec<TerminalOutput>>;

    async fn wait_for_exit(&self, session_id: &str) -> anyhow::Result<TerminalExit>;

    async fn list_sessions(&self) -> anyhow::Result<Vec<String>>;
}
