// SPDX-License-Identifier: AGPL-3.0-only

//! `axagent_harness::search_sources::DocumentParser` trait 的默认实现。
//!
//! 把现有 `extract_text` free function 包成 struct impl，
//! 让 search crate 通过 trait object 注入使用，而不用直接依赖本 crate。

use std::path::Path;

use axagent_harness::core_error::Result;
use axagent_harness::search_sources::DocumentParser;

pub struct DefaultDocumentParser;

impl DocumentParser for DefaultDocumentParser {
    fn extract_text(&self, file_path: &Path, mime_type: &str) -> Result<String> {
        // 转发给同 crate 内的 free function
        crate::extract_text(file_path, mime_type)
    }
}
