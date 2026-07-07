// SPDX-License-Identifier: AGPL-3.0-only
//! 平台管理器契约
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct PlatformConnectionInfo {
    pub name: String,
    pub enabled: bool,
    pub connected: bool,
    pub error: Option<String>,
}

#[async_trait]
pub trait PlatformManager: Send + Sync {
    async fn start_all(&self) -> Result<(), String>;
    async fn stop_all(&self) -> Result<(), String>;
    async fn start_platform(&self, name: &str) -> Result<(), String>;
    async fn stop_platform(&self, name: &str) -> Result<(), String>;
    async fn get_connections(&self) -> Result<Vec<PlatformConnectionInfo>, String>;
    async fn send_message(&self, platform: &str, chat_id: &str, text: &str) -> Result<(), String>;
}
#[derive(Default)]
pub struct NoopPlatformManager;
#[async_trait]
impl PlatformManager for NoopPlatformManager {
    async fn start_all(&self) -> Result<(), String> {
        Err("not configured".into())
    }
    async fn stop_all(&self) -> Result<(), String> {
        Ok(())
    }
    async fn start_platform(&self, _: &str) -> Result<(), String> {
        Err("not configured".into())
    }
    async fn stop_platform(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    async fn get_connections(&self) -> Result<Vec<PlatformConnectionInfo>, String> {
        Ok(Vec::new())
    }
    async fn send_message(&self, _: &str, _: &str, _: &str) -> Result<(), String> {
        Err("not configured".into())
    }
}

#[async_trait]
pub trait PlatformMessageHandler: Send + Sync {
    async fn handle_message(
        &self,
        platform: &str,
        chat_id: &str,
        text: &str,
    ) -> Result<String, String>;
}
