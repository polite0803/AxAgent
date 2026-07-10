// SPDX-License-Identifier: AGPL-3.0-only

//! 输出/输入脱敏服务
//!
//! 从 `tool.rs` 中的 `DefaultOutputSanitizer` 提取，
//! 包含 6 组正则脱敏模式：API key、内网 IP、邮箱、Token 等。
//!
//! `OutputSanitizer` / `InputSanitizer` trait 定义保留在 `tool.rs`，
//! 本模块提供默认实现和 Noop 实现。

use crate::tool::{InputSanitizer, OutputSanitizer, SanitizeContext, ToolCategory};
use regex::Regex;

/// 默认脱敏器 — 支持正则模式匹配替换。
///
/// 包含 6 组正则模式：
/// - API key: sk-/pk- 前缀
/// - 内网 IP: 192.168.x.x / 10.x.x.x / 172.16-31.x.x
/// - 邮箱
/// - 常见 token/secret/password 模式
#[derive(Debug, Clone)]
pub struct DefaultOutputSanitizer {
    patterns: Vec<(Regex, &'static str)>,
}

impl DefaultOutputSanitizer {
    pub fn new() -> Self {
        let patterns = vec![
            (Regex::new(r"(?i)(sk|pk)-[a-zA-Z0-9]{20,}").expect("static regex"), "${1}-****"),
            (Regex::new(r"\b192\.168\.\d{1,3}\.\d{1,3}\b").expect("static regex"), "192.168.*.*"),
            (Regex::new(r"\b10\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").expect("static regex"), "10.*.*.*"),
            (
                Regex::new(r"\b172\.(1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}\b").expect("static regex"),
                "172.*.*.*",
            ),
            (
                Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
                    .expect("static regex"),
                "***@***",
            ),
            (
                Regex::new(r"(?i)(token|secret|password)\s*[:=]\s*\S{8,}").expect("static regex"),
                "${1}=****",
            ),
        ];
        Self { patterns }
    }

    pub fn with_custom_patterns(patterns: Vec<(Regex, &'static str)>) -> Self {
        Self { patterns }
    }
}

impl Default for DefaultOutputSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputSanitizer for DefaultOutputSanitizer {
    fn sanitize(&self, output: &str, _ctx: &SanitizeContext) -> String {
        let mut result = output.to_string();
        for (re, replacement) in &self.patterns {
            result = re.replace_all(&result, *replacement).to_string();
        }
        result
    }
}

/// 默认输入脱敏器 — 复用 `DefaultOutputSanitizer` 的正则模式。
#[derive(Debug, Clone)]
pub struct DefaultInputSanitizer {
    output_sanitizer: DefaultOutputSanitizer,
}

impl DefaultInputSanitizer {
    pub fn new() -> Self {
        Self { output_sanitizer: DefaultOutputSanitizer::new() }
    }
}

impl Default for DefaultInputSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl InputSanitizer for DefaultInputSanitizer {
    fn sanitize_input(&self, input: &str, _context: &str) -> String {
        let ctx = SanitizeContext {
            tool_name: "__input_sanitizer__".into(),
            tool_category: ToolCategory::System,
            conversation_id: None,
        };
        self.output_sanitizer.sanitize(input, &ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::NoopOutputSanitizer;

    #[test]
    fn test_api_key_redaction() {
        let sanitizer = DefaultOutputSanitizer::new();
        let ctx = SanitizeContext {
            tool_name: "test".into(),
            tool_category: ToolCategory::Network,
            conversation_id: None,
        };
        let input = "My key is sk-abc123def456ghi789jkl012mno345pqr678";
        let result = sanitizer.sanitize(input, &ctx);
        assert!(result.contains("****"));
        assert!(!result.contains("sk-abc"));
    }

    #[test]
    fn test_internal_ip_redaction() {
        let sanitizer = DefaultOutputSanitizer::new();
        let ctx = SanitizeContext {
            tool_name: "test".into(),
            tool_category: ToolCategory::Network,
            conversation_id: None,
        };
        let input = "Host is 192.168.1.100";
        let result = sanitizer.sanitize(input, &ctx);
        assert!(result.contains("192.168.*.*"));
    }

    #[test]
    fn test_email_redaction() {
        let sanitizer = DefaultOutputSanitizer::new();
        let ctx = SanitizeContext {
            tool_name: "test".into(),
            tool_category: ToolCategory::Network,
            conversation_id: None,
        };
        let input = "Contact: user@example.com";
        let result = sanitizer.sanitize(input, &ctx);
        assert!(result.contains("***@***"));
    }

    #[test]
    fn test_no_redaction_for_safe_content() {
        let sanitizer = DefaultOutputSanitizer::new();
        let ctx = SanitizeContext {
            tool_name: "test".into(),
            tool_category: ToolCategory::Network,
            conversation_id: None,
        };
        let content = "This is a normal message with no secrets";
        let result = sanitizer.sanitize(content, &ctx);
        assert_eq!(content, result);
    }

    #[test]
    fn test_noop_sanitizer() {
        let sanitizer = NoopOutputSanitizer;
        let ctx = SanitizeContext {
            tool_name: "test".into(),
            tool_category: ToolCategory::FileWrite,
            conversation_id: None,
        };
        let content = "sk-sensitive-key and user@email.com";
        let result = sanitizer.sanitize(content, &ctx);
        assert_eq!(content, result);
    }
}
