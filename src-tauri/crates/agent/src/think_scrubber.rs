// SPDX-License-Identifier: AGPL-3.0-only
//! G10 Think Scrubbing
//!
//! 清理 LLM 思考链（thinking content）中的敏感信息和噪声，确保推送给
//! 客户端的 reasoning_content 是安全的、有价值的。
//!
//! ## 清理规则
//!
//! 1. **API Key / 密钥脱敏**：匹配 `sk-xxx`、`Bearer xxx`、`api_key=xxx` 等
//! 2. **内部提示词泄漏**：移除形如 `system prompt:` / `instructions:` 的泄漏
//! 3. **IP 地址 / 内部 URL**：脱敏内网地址（如 `127.0.0.1`、`localhost:8080`）
//! 4. **重复行去重**：移除连续重复的思考片段
//! 5. **长度限制**：超过阈值的思考链截断并附加 `[...]` 标记
//!
//! ## 使用方式
//!
//! ```ignore
//! use axagent_agent::think_scrubber::ThinkScrubber;
//!
//! let scrubber = ThinkScrubber::default();
//! let cleaned = scrubber.scrub("my api key is sk-abc123...");
//! assert!(cleaned.contains("sk-***"));
//! ```

use regex::Regex;
use std::sync::OnceLock;

/// 思考链清理配置
#[derive(Debug, Clone)]
pub struct ScrubberConfig {
    /// 最大长度（字符数），超过则截断
    pub max_length: usize,
    /// 是否脱敏 API key
    pub redact_api_keys: bool,
    /// 是否脱敏 IP 地址
    pub redact_ips: bool,
    /// 是否移除内部提示词泄漏
    pub remove_prompt_leaks: bool,
    /// 是否去重连续重复行
    pub dedup_consecutive_lines: bool,
}

impl Default for ScrubberConfig {
    fn default() -> Self {
        Self {
            max_length: 4096,
            redact_api_keys: true,
            redact_ips: true,
            remove_prompt_leaks: true,
            dedup_consecutive_lines: true,
        }
    }
}

/// 思考链清理器
pub struct ThinkScrubber {
    config: ScrubberConfig,
    // 预编译的正则表达式（OnceLock 保证只编译一次）
}

impl Default for ThinkScrubber {
    fn default() -> Self {
        Self::new(ScrubberConfig::default())
    }
}

impl ThinkScrubber {
    /// 创建新的清理器
    pub fn new(config: ScrubberConfig) -> Self {
        Self { config }
    }

    /// 清理思考链内容
    pub fn scrub(&self, content: &str) -> String {
        if content.is_empty() {
            return content.to_string();
        }

        let mut result = content.to_string();

        // 1. 脱敏 API key / 密钥
        if self.config.redact_api_keys {
            result = redact_api_keys(&result);
        }

        // 2. 脱敏 IP 地址（仅内网地址）
        if self.config.redact_ips {
            result = redact_internal_ips(&result);
        }

        // 3. 移除内部提示词泄漏
        if self.config.remove_prompt_leaks {
            result = remove_prompt_leaks(&result);
        }

        // 4. 去重连续重复行
        if self.config.dedup_consecutive_lines {
            result = dedup_consecutive_lines(&result);
        }

        // 5. 长度限制
        if result.len() > self.config.max_length {
            let truncated: String = result.chars().take(self.config.max_length).collect();
            result = format!("{truncated}\n[... truncated]");
        }

        result
    }

    /// 清理并返回是否发生了修改
    pub fn scrub_with_diff(&self, content: &str) -> (String, bool) {
        let cleaned = self.scrub(content);
        let modified = cleaned != content;
        (cleaned, modified)
    }
}

// ── 正则表达式（懒加载） ────────────────────────────────────────────────

/// API key 模式正则
fn api_key_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // 匹配常见 API key 格式：
        // - sk-xxx (OpenAI)
        // - Bearer xxx
        // - api_key=xxx / apikey=xxx
        // - key: xxx
        // - token: xxx
        // 注意：raw string 中包含引号需使用 r#"..."# 形式
        Regex::new(r#"(?i)(sk-[a-zA-Z0-9]{20,}|Bearer\s+[a-zA-Z0-9\-_\.]{20,}|api[_-]?key\s*[=:]\s*['"]?[a-zA-Z0-9\-_]{16,}|token\s*[=:]\s*['"]?[a-zA-Z0-9\-_]{16,})"#)
            .expect("invalid api_key regex")
    })
}

/// 内网 IP 地址正则
fn internal_ip_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // 匹配私网/回环 IPv4（4 段八位组）：
        // - 10.0.0.0/8、127.0.0.0/8
        // - 172.16.0.0/12（16-31）
        // - 192.168.0.0/16
        Regex::new(
            r"(?:(?:10|127)\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3})(?::\d{2,5})?",
        )
        .expect("invalid internal_ip regex")
    })
}

/// localhost URL 正则
fn localhost_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?i)https?://localhost(?::\d+)?(?:/[^\s]*)?").expect("invalid localhost regex")
    })
}

/// 提示词泄漏正则
fn prompt_leak_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?im)^\s*(system\s*prompt|instructions?|system\s*message)\s*[:：]\s*.*$")
            .expect("invalid prompt_leak regex")
    })
}

// ── 清理函数 ────────────────────────────────────────────────────────────

/// 脱敏 API key / 密钥
fn redact_api_keys(content: &str) -> String {
    api_key_regex().replace_all(content, "[REDACTED_KEY]").to_string()
}

/// 脱敏内网 IP 地址
fn redact_internal_ips(content: &str) -> String {
    let result = internal_ip_regex().replace_all(content, "[REDACTED_IP]");
    localhost_regex().replace_all(&result, "[REDACTED_LOCALHOST]").to_string()
}

/// 移除提示词泄漏行
fn remove_prompt_leaks(content: &str) -> String {
    prompt_leak_regex().replace_all(content, "").to_string()
}

/// 去重连续重复行
fn dedup_consecutive_lines(content: &str) -> String {
    let mut result = Vec::new();
    let mut prev_line: Option<&str> = None;

    for line in content.lines() {
        if Some(line) != prev_line {
            result.push(line);
            prev_line = Some(line);
        }
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_openai_key() {
        let scrubber = ThinkScrubber::default();
        let input = "my key is sk-abcdefghijklmnopqrstuvwxyz123456";
        let output = scrubber.scrub(input);
        assert!(output.contains("[REDACTED_KEY]"));
        assert!(!output.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));
    }

    #[test]
    fn test_redact_bearer_token() {
        let scrubber = ThinkScrubber::default();
        let input = "Authorization: Bearer abcdefghijklmnopqrstuvwxyz1234567890";
        let output = scrubber.scrub(input);
        assert!(output.contains("[REDACTED_KEY]"));
    }

    #[test]
    fn test_redact_api_key_assignment() {
        let scrubber = ThinkScrubber::default();
        let input = "api_key=abcdefghijklmnopqrstuvwxyz123456";
        let output = scrubber.scrub(input);
        assert!(output.contains("[REDACTED_KEY]"));
    }

    #[test]
    fn test_redact_internal_ip() {
        let scrubber = ThinkScrubber::default();
        let input = "connecting to 127.0.0.1:8080 and 192.168.1.100";
        let output = scrubber.scrub(input);
        assert!(output.contains("[REDACTED_IP]"));
        assert!(!output.contains("127.0.0.1:8080"));
        assert!(!output.contains("192.168.1.100"));
    }

    #[test]
    fn test_redact_localhost_url() {
        let scrubber = ThinkScrubber::default();
        let input = "fetching from http://localhost:3000/api/data";
        let output = scrubber.scrub(input);
        assert!(output.contains("[REDACTED_LOCALHOST]"));
    }

    #[test]
    fn test_remove_prompt_leak() {
        let scrubber = ThinkScrubber::default();
        let input = "system prompt: you are a helpful assistant\nthinking about the question";
        let output = scrubber.scrub(input);
        assert!(!output.contains("system prompt:"));
        assert!(output.contains("thinking about the question"));
    }

    #[test]
    fn test_dedup_consecutive_lines() {
        let scrubber = ThinkScrubber::default();
        let input = "line1\nline2\nline2\nline2\nline3";
        let output = scrubber.scrub(input);
        assert_eq!(output, "line1\nline2\nline3");
    }

    #[test]
    fn test_length_limit() {
        let config = ScrubberConfig { max_length: 10, ..Default::default() };
        let scrubber = ThinkScrubber::new(config);
        let input = "abcdefghijklmnopqrstuvwxyz";
        let output = scrubber.scrub(input);
        assert!(output.contains("[... truncated]"));
        assert!(output.len() < input.len() + 20);
    }

    #[test]
    fn test_empty_input() {
        let scrubber = ThinkScrubber::default();
        assert_eq!(scrubber.scrub(""), "");
    }

    #[test]
    fn test_no_modification() {
        let scrubber = ThinkScrubber::default();
        let input = "clean thinking content without any issues";
        let (output, modified) = scrubber.scrub_with_diff(input);
        assert_eq!(output, input);
        assert!(!modified);
    }

    #[test]
    fn test_with_modification() {
        let scrubber = ThinkScrubber::default();
        let input = "key is sk-abcdefghijklmnopqrstuvwxyz123456";
        let (output, modified) = scrubber.scrub_with_diff(input);
        assert_ne!(output, input);
        assert!(modified);
    }

    #[test]
    fn test_disabled_redaction() {
        let config = ScrubberConfig { redact_api_keys: false, ..Default::default() };
        let scrubber = ThinkScrubber::new(config);
        let input = "key is sk-abcdefghijklmnopqrstuvwxyz123456";
        let output = scrubber.scrub(input);
        assert!(output.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));
    }

    #[test]
    fn test_public_ip_not_redacted() {
        let scrubber = ThinkScrubber::default();
        let input = "connecting to 8.8.8.8 for DNS";
        let output = scrubber.scrub(input);
        assert!(output.contains("8.8.8.8"));
    }

    #[test]
    fn test_non_consecutive_duplicates_preserved() {
        let scrubber = ThinkScrubber::default();
        let input = "line1\nline2\nline1\nline2";
        let output = scrubber.scrub(input);
        // 非连续重复应保留
        assert_eq!(output, "line1\nline2\nline1\nline2");
    }
}
