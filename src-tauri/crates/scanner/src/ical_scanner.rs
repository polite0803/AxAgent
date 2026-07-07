// SPDX-License-Identifier: AGPL-3.0-only

//! ICalScanner — 解析 .ics 日历文件的 MemoryScanner 实现。

use axagent_harness::scanner::{MemoryScanner, ScanResult, ScannerConfig};

pub struct ICalScanner;

#[async_trait::async_trait]
impl MemoryScanner for ICalScanner {
    async fn scan(&self, config: &ScannerConfig) -> Result<ScanResult, String> {
        Ok(ScanResult::default())
    }
}
