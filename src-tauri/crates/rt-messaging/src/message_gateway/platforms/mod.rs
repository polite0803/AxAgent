pub mod dingtalk;
pub mod discord;
pub mod feishu;
pub mod qq;
pub mod slack;
pub mod telegram;
pub mod wechat;
pub mod whatsapp;

use std::sync::Arc;
use std::sync::OnceLock;

use crate::message_gateway::platform_config::PlatformConfig;
use crate::message_gateway::platform_manager::PlatformMessageCallback;

static MESSAGE_CALLBACK: OnceLock<Arc<dyn PlatformMessageCallback>> = OnceLock::new();

pub fn set_message_callback(callback: Arc<dyn PlatformMessageCallback>) {
    let _ = MESSAGE_CALLBACK.set(callback);
}

pub fn get_message_callback() -> Option<Arc<dyn PlatformMessageCallback>> {
    MESSAGE_CALLBACK.get().cloned()
}

#[async_trait::async_trait]
pub trait PlatformAdapter: Send + Sync {
    fn name(&self) -> &'static str;

    fn is_enabled(&self, config: &PlatformConfig) -> bool;

    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()>;

    async fn stop(&self) -> anyhow::Result<()>;

    async fn is_connected(&self) -> bool;

    async fn send_message(
        &self,
        config: &PlatformConfig,
        chat_id: &str,
        text: &str,
        parse_mode: Option<&str>,
    ) -> anyhow::Result<()>;
}
