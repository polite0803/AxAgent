// SPDX-License-Identifier: AGPL-3.0-only
//! 开发者体验契约
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub rust_version: Option<String>,
    pub node_version: Option<String>,
    pub ide: Option<String>,
    pub workspace_path: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[async_trait]
pub trait DevExperienceProvider: Send + Sync {
    async fn get_env_info(&self) -> Result<EnvironmentInfo, String>;
    async fn set_log_level(&self, level: LogLevel);
    async fn get_log_level(&self) -> Result<LogLevel, String>;
    fn version(&self) -> &'static str;
    async fn check_update(&self) -> Result<Option<String>, String>;
}
#[derive(Default)]
pub struct NoopDevExperienceProvider;
#[async_trait]
impl DevExperienceProvider for NoopDevExperienceProvider {
    async fn get_env_info(&self) -> Result<EnvironmentInfo, String> {
        Ok(EnvironmentInfo {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            hostname: "unknown".into(),
            rust_version: None,
            node_version: None,
            ide: None,
            workspace_path: None,
        })
    }
    async fn set_log_level(&self, _: LogLevel) {}
    async fn get_log_level(&self) -> Result<LogLevel, String> {
        Ok(LogLevel::Info)
    }
    fn version(&self) -> &'static str {
        "0.0.0"
    }
    async fn check_update(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
}
