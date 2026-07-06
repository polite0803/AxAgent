// SPDX-License-Identifier: AGPL-3.0-only

//! QR 绑定 token 存储（参考 nomifun-tauri 的 QrTokenStore 设计）。
//!
//! 用于 IM 渠道扫码绑定流程：
//! 1. Gateway WebUI 生成 QR Token → 前端展示二维码
//! 2. 用户手机扫码 → 发送绑定请求（含平台用户信息）
//! 3. 服务端将平台用户 ID 与 API Key 关联
//!
//! Token 一次性使用，5 分钟过期，后台定时清理。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use uuid::Uuid;

/// 一次性的 QR 绑定令牌。
#[derive(Clone, Debug)]
pub struct QrBindTicket {
    pub ticket_id: String,
    /// 与此令牌关联的 API key ID（生成时指定，消费时绑定平台用户到此 key）
    pub key_id: String,
    pub expires_at: Instant,
}

/// 内存 QR 绑定令牌存储。
///
/// 模式与 `TicketStore` 完全相同，但语义不同：
/// - TicketStore 用于 WebSocket 认证（短时、高频）
/// - QrBindStore 用于平台用户绑定（中时、低频）
#[derive(Clone)]
pub struct QrBindStore {
    ttl: Duration,
    inner: Arc<Mutex<HashMap<String, QrBindTicket>>>,
}

const QR_TTL: Duration = Duration::from_secs(300); // 5 分钟
const SWEEP_INTERVAL: Duration = Duration::from_secs(30);

impl Default for QrBindStore {
    fn default() -> Self {
        Self::new()
    }
}

impl QrBindStore {
    pub fn new() -> Self {
        let inner = Arc::new(Mutex::new(HashMap::new()));
        Self::spawn_sweeper(inner.clone(), SWEEP_INTERVAL);
        Self { ttl: QR_TTL, inner }
    }

    /// 生成一个新的绑定令牌。
    pub fn issue(&self, key_id: impl Into<String>) -> QrBindTicket {
        let ticket = QrBindTicket {
            ticket_id: Uuid::new_v4().to_string(),
            key_id: key_id.into(),
            expires_at: Instant::now() + self.ttl,
        };
        let mut map = self.inner.lock();
        map.insert(ticket.ticket_id.clone(), ticket.clone());
        ticket
    }

    /// 消费一个绑定令牌（一次性，过期返回 None）。
    pub fn consume(&self, ticket_id: &str) -> Option<QrBindTicket> {
        let mut map = self.inner.lock();
        let ticket = map.remove(ticket_id)?;
        if ticket.expires_at < Instant::now() {
            return None;
        }
        Some(ticket)
    }

    /// 清理过期令牌。
    pub fn sweep(&self) -> usize {
        let now = Instant::now();
        let mut map = self.inner.lock();
        let before = map.len();
        map.retain(|_, t| t.expires_at >= now);
        before - map.len()
    }

    fn spawn_sweeper(inner: Arc<Mutex<HashMap<String, QrBindTicket>>>, interval: Duration) {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(interval);
            tick.tick().await; // 跳过首次立即触发
            loop {
                tick.tick().await;
                let now = Instant::now();
                let mut map = inner.lock();
                map.retain(|_, t| t.expires_at >= now);
            }
        });
    }
}
