// SPDX-License-Identifier: AGPL-3.0-only

//! 消息平台适配器契约 — 「一切皆插件」platform.adapter 接缝的权威定义。
//!
//! `MessagePlatformAdapter` 是消息平台（telegram / discord / wechat / ...）的统一
//! 接入契约。与 harness 中 gateway 数据访问层的 `PlatformAdapter`（facade trait）
//! 是**两个不同概念**：后者聚合 dao/crypto 子 trait，本模块面向消息平台的
//! 生命周期与收发能力。
//!
//! 权威定义在此（harness，foundation），具体实现（telegram/discord/... 8 个内置
//! 平台）位于 `axagent-rt-messaging`（hybrid）。wiring 层将内置平台注册到能力
//! 注册表的 `platform.adapter` 接缝，外部插件可经 `register_external_platform_adapter`
//! 替换/新增同一接缝（内置与插件平权）。
//!
//! `MediaAttachment`/`MediaType`/`DeliveryMode` 为纯数据 DTO，一并上沉，
//! 供 trait 方法与消费方共享，避免 rt-messaging 内部重复定义。

use crate::platform_config::PlatformConfig;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 媒体 DTO（纯数据，上沉自 rt-messaging::media_types） ──

/// 媒体投递方式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    Native,
    Voice,
    Document,
}

impl DeliveryMode {
    /// 媒体投递方式的字符串表示。
    pub fn as_str(&self) -> &'static str {
        match self {
            DeliveryMode::Native => "native",
            DeliveryMode::Voice => "voice",
            DeliveryMode::Document => "document",
        }
    }
}

/// 媒体类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Image,
    Audio,
    Video,
    Document,
}

impl MediaType {
    /// 媒体类型的字符串表示。
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Image => "image",
            MediaType::Audio => "audio",
            MediaType::Video => "video",
            MediaType::Document => "document",
        }
    }
}

/// 一条待发送的媒体附件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAttachment {
    pub path: String,
    pub media_type: MediaType,
    pub delivery_mode: DeliveryMode,
}

// ── 消息平台适配器 trait ──

/// 消息平台适配器：实现平台接入的生命周期（start/stop/connected）与收发能力。
///
/// 实现方：`axagent_rt_messaging` 的 8 个内置平台（telegram/discord/slack/whatsapp/
/// wechat/feishu/qq/dingtalk）。
/// 调用方：`PlatformManager`（集中管理平台生命周期）、`api_server`（对外发送）。
#[async_trait]
pub trait MessagePlatformAdapter: Send + Sync + std::any::Any {
    /// 平台唯一名称（如 `"telegram"`）。
    fn name(&self) -> &'static str;

    /// 该平台是否在当前配置下启用。
    fn is_enabled(&self, config: &PlatformConfig) -> bool;

    /// 启动平台（轮询/建立连接/webhook 注册等）。
    async fn start(&self, config: &PlatformConfig) -> anyhow::Result<()>;

    /// 停止平台并释放资源。
    async fn stop(&self) -> anyhow::Result<()>;

    /// 是否已连接。
    async fn is_connected(&self) -> bool;

    /// 发送一条文本消息。
    async fn send_message(
        &self,
        config: &PlatformConfig,
        chat_id: &str,
        text: &str,
        parse_mode: Option<&str>,
    ) -> anyhow::Result<()>;

    /// 发送一条媒体附件（默认实现：仅记录日志）。
    async fn send_media(
        &self,
        _config: &PlatformConfig,
        _chat_id: &str,
        _attachment: &MediaAttachment,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "[{}] send_media: path={} type={} mode={} (not yet implemented)",
            self.name(),
            _attachment.path,
            _attachment.media_type.as_str(),
            _attachment.delivery_mode.as_str()
        );
        Ok(())
    }
}
