// SPDX-License-Identifier: AGPL-3.0-only

//! FileScanner — 扫描本地文本/笔记文件的 MemoryScanner 实现。

use axagent_harness::scanner::{MemoryScanner, ScanResult, ScannerConfig};

pub struct FileScanner;

#[async_trait::async_trait]
impl MemoryScanner for FileScanner {
    async fn scan(&self, config: &ScannerConfig) -> Result<ScanResult, String> {
        Ok(ScanResult::default())
    }
}
