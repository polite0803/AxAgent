// SPDX-License-Identifier: AGPL-3.0-only
//! 开发者体验契约
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentInfo {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub rust_version: Option<String>,
    pub node_version: Option<String>,
    pub ide: Option<String>,
    pub workspace_path: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

#[async_trait]
pub trait DevExperienceProvider: Send + Sync {
    async fn get_env_info(&self) -> Result<EnvironmentInfo, String>;
    async fn set_log_level(&self, level: LogLevel);
    async fn get_log_level(&self) -> Result<LogLevel, String>;
    fn version(&self) -> &'static str;
    async fn check_update(&self) -> Result<Option<String>, String>;
}
