// SPDX-License-Identifier: AGPL-3.0-only

//! MemoryScanner 契约 — 本地日历/消息扫描器的 trait 定义。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    pub ical_paths: Vec<String>,
    pub file_paths: Vec<String>,
    pub scan_interval_secs: u64,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self { ical_paths: Vec::new(), file_paths: Vec::new(), scan_interval_secs: 300 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedItem {
    pub external_id: String,
    pub title: String,
    pub content: String,
    pub source: String,
    pub priority: String,
    pub tags: Vec<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanResult {
    pub items: Vec<ScannedItem>,
    pub errors: Vec<String>,
}

#[async_trait]
pub trait MemoryScanner: Send + Sync {
    async fn scan(&self, config: &ScannerConfig) -> Result<ScanResult, String>;
}
