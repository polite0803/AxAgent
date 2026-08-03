// SPDX-License-Identifier: AGPL-3.0-only

//! PlaywrightClient → BrowserHttpFetch 适配器。
//!
//! 将浏览器自动化客户端的 HTTP 请求能力包装为 astock-data 中定义的
//! `BrowserHttpFetch` trait 接口，供 `browser_eastmoney` vendor
//! 在浏览器内核中执行请求以绕过 EastMoney JA3 TLS 封锁。
//!
//! 移动端（android / iOS）不支持 Playwright，整个模块被门控掉，
//! 由 `state.rs` 在移动端注入 `None` fetcher。

#[cfg(not(mobile))]
use std::sync::Arc;

use async_trait::async_trait;
use axagent_astock_data::vendors::browser_eastmoney::BrowserHttpFetch;
#[cfg(not(mobile))]
use axagent_kit::browser_automation::PlaywrightClient;
use serde_json::Value;
#[cfg(not(mobile))]
use tokio::sync::Mutex;

/// 将 `Arc<Mutex<Option<PlaywrightClient>>>` 包装为 `BrowserHttpFetch`。
///
/// `PlaywrightClient` 的方法需要 `&mut self`（内部管理浏览器页面状态），
/// 此适配器通过外部 `Mutex` 提供内部可变性，与 trait 的 `&self` 签名兼容。
///
/// **懒启动**：首次调用 `fetch_json` / `fetch_text` 时自动通过 `PlaywrightClient::launch()`
/// 启动浏览器实例，与 `commands::browser::ensure_browser_client()` 行为一致。
/// 这保证了 vendor_health_prober 等后台路径也能触发浏览器初始化。
#[cfg(not(mobile))]
pub struct PlaywrightBrowserFetcher {
    client: Arc<Mutex<Option<PlaywrightClient>>>,
}

#[cfg(not(mobile))]
impl PlaywrightBrowserFetcher {
    pub fn new(client: Arc<Mutex<Option<PlaywrightClient>>>) -> Self {
        Self { client }
    }

    /// 确保 Playwright 客户端已初始化（懒启动）。
    /// 与 commands::browser::ensure_browser_client 逻辑一致，但无需 AppState 上下文。
    async fn ensure_initialized(&self) -> Result<(), String> {
        let mut guard = self.client.lock().await;
        let needs_launch = match guard.as_mut() {
            Some(c) => !c.is_alive(),
            None => true,
        };
        if needs_launch {
            let client = PlaywrightClient::launch()
                .await
                .map_err(|e| format!("playwright launch failed: {e}"))?;
            *guard = Some(client);
        }
        Ok(())
    }
}

#[cfg(not(mobile))]
#[async_trait]
impl BrowserHttpFetch for PlaywrightBrowserFetcher {
    /// 通过浏览器 fetch API 发送 HTTP GET 请求。
    /// 使用 `fetch_json_via_browser`，内部通过 `page.evaluate()` 调用浏览器 fetch()。
    async fn fetch_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<Value, String> {
        self.ensure_initialized().await?;
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or_else(|| "browser not initialized".to_string())?;
        client.fetch_json_via_browser(url, headers).await.map_err(|e| e.to_string())
    }

    /// 通过浏览器页面导航发送 HTTP GET 请求。
    /// 使用 `http_get_via_browser`，内部通过 `page.goto()` 导航绕过 CORS 限制。
    async fn fetch_text(&self, url: &str) -> Result<Value, String> {
        self.ensure_initialized().await?;
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or_else(|| "browser not initialized".to_string())?;
        client.http_get_via_browser(url).await.map_err(|e| e.to_string())
    }
}

/// 移动端（android / iOS）的浏览器 fetch 空实现：直接返回"不可用"错误。
///
/// 供 `state.rs` 在移动端注入 `AStockClient`，保持 `BrowserHttpFetch` trait
/// 注入路径一致，运行时 vendor 若尝试使用浏览器内核会得到明确的错误提示。
#[cfg(mobile)]
pub struct NoopBrowserFetcher;

#[cfg(mobile)]
#[async_trait]
impl BrowserHttpFetch for NoopBrowserFetcher {
    async fn fetch_json(&self, _url: &str, _headers: &[(&str, &str)]) -> Result<Value, String> {
        Err("移动端不支持浏览器内核 fetch（Playwright 不可用）".to_string())
    }

    async fn fetch_text(&self, _url: &str) -> Result<Value, String> {
        Err("移动端不支持浏览器内核 fetch（Playwright 不可用）".to_string())
    }
}
