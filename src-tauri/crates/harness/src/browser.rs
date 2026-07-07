// SPDX-License-Identifier: AGPL-3.0-only

//! BrowserController 契约 — 浏览器自动化接口。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserNavigateResult {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserScreenshotResult {
    pub data: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Default)]
pub struct NoopBrowserController;

#[async_trait]
impl BrowserController for NoopBrowserController {
    async fn navigate(&self, _url: &str) -> Result<BrowserNavigateResult, String> {
        Err("browser not configured".to_string())
    }
    async fn screenshot(&self) -> Result<BrowserScreenshotResult, String> {
        Err("browser not configured".to_string())
    }
    async fn extract_elements(&self, _selector: &str) -> Result<Vec<ExtractedElement>, String> {
        Ok(Vec::new())
    }
    async fn click(&self, _selector: &str) -> Result<(), String> {
        Err("browser not configured".to_string())
    }
    async fn type_text(&self, _selector: &str, _text: &str) -> Result<(), String> {
        Err("browser not configured".to_string())
    }
    async fn close(&self) -> Result<(), String> {
        Ok(())
    }
}
