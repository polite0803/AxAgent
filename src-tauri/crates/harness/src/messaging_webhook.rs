// SPDX-License-Identifier: AGPL-3.0-only

//! 消息平台 Webhook 契约层。
//!
//! 让 `rt-webhook`（HTTP 服务器层）不直接依赖 `rt-messaging::platforms::{wechat, whatsapp}`
////! 的具体模块。HTTP 处理器只依赖这些 trait，trait impl 在 `axagent-rt-messaging` 中。

use async_trait::async_trait;

use crate::platform_config::PlatformConfig;

/// 微信（official_account 模式）webhook 处理器。
#[async_trait]
pub trait WeChatWebhookHandler: Send + Sync {
    /// GET 端点：处理微信服务器验证请求。
    /// `echostr` 校验通过时回显给微信，完成 URL 配置握手。
    fn verify_server(
        &self,
        token: &str,
        signature: &str,
        timestamp: &str,
        nonce: &str,
        echostr: &str,
    ) -> Result<String, String>;

    /// POST 端点：处理微信服务器推送的消息 XML。
    /// 解析后通过回调把消息转给 Agent；返回要回给微信的 XML 字符串。
    async fn handle_message(
        &self,
        config: &PlatformConfig,
        xml_body: &str,
    ) -> Result<String, String>;
}

/// WhatsApp Cloud API webhook 处理器。
#[async_trait]
pub trait WhatsAppWebhookHandler: Send + Sync {
    /// GET 端点：处理 Meta 的 webhook 验证握手。
    /// `mode=subscribe` + token 匹配时回显 `challenge`。
    fn verify_challenge(
        &self,
        config: &PlatformConfig,
        mode: &str,
        token: &str,
        challenge: &str,
    ) -> Result<String, String>;

    /// POST 端点：处理 Meta 推送的消息事件 JSON。
    /// 解析后通过回调把消息转给 Agent。
    async fn handle_notification(
        &self,
        config: &PlatformConfig,
        body: &serde_json::Value,
    ) -> Result<(), String>;
}
