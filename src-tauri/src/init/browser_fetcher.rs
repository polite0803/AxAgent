// SPDX-License-Identifier: AGPL-3.0-only

//! PlaywrightClient → BrowserHttpFetch 适配器。
//!
//! 将浏览器自动化客户端的 HTTP 请求能力包装为 astock-data 中定义的
//! `BrowserHttpFetch` trait 接口，供 `browser_eastmoney` vendor
//! 在浏览器内核中执行请求以绕过 EastMoney JA3 TLS 封锁。

use std::sync::Arc;

use async_trait::async_trait;
use axagent_astock_data::vendors::browser_eastmoney::BrowserHttpFetch;
use axagent_kit::browser_automation::PlaywrightClient;
use serde_json::Value;
use tokio::sync::Mutex;

/// 将 `Arc<Mutex<Option<PlaywrightClient>>>` 包装为 `BrowserHttpFetch`。
///
/// `PlaywrightClient` 的方法需要 `&mut self`（内部管理浏览器页面状态），
/// 此适配器通过外部 `Mutex` 提供内部可变性，与 trait 的 `&self` 签名兼容。
pub struct PlaywrightBrowserFetcher {
    client: Arc<Mutex<Option<PlaywrightClient>>>,
}

impl PlaywrightBrowserFetcher {
    pub fn new(client: Arc<Mutex<Option<PlaywrightClient>>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl BrowserHttpFetch for PlaywrightBrowserFetcher {
    /// 通过浏览器 fetch API 发送 HTTP GET 请求。
    /// 使用 `fetch_json_via_browser`，内部通过 `page.evaluate()` 调用浏览器 fetch()。
    async fn fetch_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<Value, String> {
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or_else(|| "browser not initialized".to_string())?;
        client.fetch_json_via_browser(url, headers).await.map_err(|e| e.to_string())
    }

    /// 通过浏览器页面导航发送 HTTP GET 请求。
    /// 使用 `http_get_via_browser`，内部通过 `page.goto()` 导航绕过 CORS 限制。
    async fn fetch_text(&self, url: &str) -> Result<Value, String> {
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or_else(|| "browser not initialized".to_string())?;
        client.http_get_via_browser(url).await.map_err(|e| e.to_string())
    }
}
