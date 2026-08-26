// SPDX-License-Identifier: AGPL-3.0-only

//! BrowserController 契约 — 浏览器自动化接口。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserNavigateResult {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserScreenshotResult {
    pub data: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedElement {
    pub tag: String,
    pub text: String,
    pub attributes: std::collections::HashMap<String, String>,
}

#[async_trait]
pub trait BrowserController: Send + Sync {
    async fn navigate(&self, url: &str) -> Result<BrowserNavigateResult, String>;
    async fn screenshot(&self) -> Result<BrowserScreenshotResult, String>;
    async fn extract_elements(&self, selector: &str) -> Result<Vec<ExtractedElement>, String>;
    async fn click(&self, selector: &str) -> Result<(), String>;
    async fn type_text(&self, selector: &str, text: &str) -> Result<(), String>;
    async fn close(&self) -> Result<(), String>;
}
