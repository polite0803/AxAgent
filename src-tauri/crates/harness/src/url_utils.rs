// SPDX-License-Identifier: AGPL-3.0-only

//! LLM Provider URL 解析工具函数。
//!
//! 提供 base URL / chat URL 的拼接、版本化路径追加、双重路径去重等功能。
//! 放在 `axagent-harness`（harness 契约层）供所有 crate 通过 `axagent_harness::*` 使用，
//! `axagent-providers` 仅做 re-export 转发，保留向后兼容。
//!
//! 迁移历史：
//! - 2026-06: 首次从 harness 迁出到 providers。
//! - 2026-06: 再次迁回 harness（因 `axagent_harness::resolve_base_url_for_type` 是
//!   跨多个 crate 大量使用的统一 import 路径；迁移到 providers 破坏了调用方期望）。

use crate::types::ProviderType;

/// Default version path for a given provider type.
pub fn default_version_for_type(provider_type: &ProviderType) -> &'static str {
    match provider_type {
        ProviderType::Gemini => "/v1beta",
        ProviderType::Ollama => "/v1",
        _ => "/v1",
    }
}

/// Resolve `api_host` into a usable base URL, using the provider type to
/// determine the default version path (e.g. `/v1` for OpenAI, `/v1beta` for Gemini).
///
/// - Trailing `!` → force mode: strip `!`, return as-is.
/// - Already ends with a versioned path (e.g. `/v1`, `/v1beta`) → return as-is.
/// - Otherwise → append the default version path for this provider type.
pub fn resolve_base_url_for_type(api_host: &str, provider_type: &ProviderType) -> String {
    let default_version = default_version_for_type(provider_type);
    resolve_base_url_inner(api_host, default_version)
}

/// Resolve `api_host` into a usable base URL (defaults to `/v1`).
pub fn resolve_base_url(api_host: &str) -> String {
    resolve_base_url_inner(api_host, "/v1")
}

fn resolve_base_url_inner(api_host: &str, default_version: &str) -> String {
    let trimmed = api_host.trim_end_matches('/');
    if let Some(forced) = trimmed.strip_suffix('!') {
        forced.trim_end_matches('/').to_string()
    } else if has_version_suffix(trimmed) {
        trimmed.to_string()
    } else {
        format!("{}{}", trimmed, default_version)
    }
}

fn has_version_suffix(url: &str) -> bool {
    let last_seg = url.rsplit('/').next().unwrap_or("");
    let bytes = last_seg.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'v' {
        return false;
    }
    let rest = &last_seg[1..];
    rest.starts_with(|c: char| c.is_ascii_digit())
}

/// Build the full chat/completion URL from resolved `base_url` and optional `api_path`.
pub fn resolve_chat_url(
    resolved_base: &str,
    api_path: Option<&str>,
    default_suffix: &str,
) -> String {
    let base = resolved_base.trim_end_matches('/');
    match api_path {
        Some(path) if !path.is_empty() => {
            if let Some(forced) = path.strip_suffix('!') {
                format!("{}/{}", base, forced.trim_matches('/'))
            } else {
                let path = path.trim_matches('/');
                if let Some(common) = find_common_version_prefix(base, path) {
                    let remainder = path[common.len()..].trim_start_matches('/');
                    if remainder.is_empty() {
                        base.to_string()
                    } else {
                        format!("{}/{}", base, remainder)
                    }
                } else {
                    format!("{}/{}", base, path)
                }
            }
        },
        _ => format!("{}{}", base, default_suffix),
    }
}

fn find_common_version_prefix<'a>(base: &str, path: &'a str) -> Option<&'a str> {
    let base_last = base.rsplit('/').next().unwrap_or("");
    if path.starts_with(base_last) && !base_last.is_empty() && base_last.starts_with('v') {
        Some(&path[..base_last.len()])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_base_url_appends_default_version() {
        assert_eq!(resolve_base_url("https://api.openai.com"), "https://api.openai.com/v1");
    }

    #[test]
    fn test_resolve_base_url_already_has_version() {
        assert_eq!(resolve_base_url("https://api.openai.com/v1"), "https://api.openai.com/v1");
    }

    #[test]
    fn test_resolve_base_url_with_trailing_slash() {
        assert_eq!(resolve_base_url("https://api.openai.com/"), "https://api.openai.com/v1");
    }

    #[test]
    fn test_resolve_base_url_force_mode_strips_bang() {
        assert_eq!(resolve_base_url("https://api.openai.com!"), "https://api.openai.com");
    }

    #[test]
    fn test_resolve_base_url_force_mode_with_path() {
        assert_eq!(resolve_base_url("https://api.openai.com/v2!"), "https://api.openai.com/v2");
    }

    #[test]
    fn test_resolve_base_url_for_type_gemini() {
        assert_eq!(
            resolve_base_url_for_type("https://api.google.com", &ProviderType::Gemini),
            "https://api.google.com/v1beta"
        );
    }

    #[test]
    fn test_resolve_base_url_for_type_openai() {
        assert_eq!(
            resolve_base_url_for_type("https://api.openai.com", &ProviderType::OpenAI),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn test_resolve_base_url_v2_version() {
        assert_eq!(resolve_base_url("https://api.example.com/v2"), "https://api.example.com/v2");
    }

    #[test]
    fn test_resolve_base_url_v1beta_version() {
        assert_eq!(
            resolve_base_url("https://api.example.com/v1beta"),
            "https://api.example.com/v1beta"
        );
    }

    #[test]
    fn test_resolve_chat_url_default_suffix() {
        assert_eq!(
            resolve_chat_url("https://api.openai.com/v1", None, "/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_resolve_chat_url_with_api_path() {
        assert_eq!(
            resolve_chat_url(
                "https://api.openai.com/v1",
                Some("/v1/messages"),
                "/chat/completions"
            ),
            "https://api.openai.com/v1/messages"
        );
    }

    #[test]
    fn test_resolve_chat_url_force_mode_with_bang() {
        assert_eq!(
            resolve_chat_url("https://api.openai.com", Some("/v1/messages!"), "/chat/completions"),
            "https://api.openai.com/v1/messages"
        );
    }

    #[test]
    fn test_resolve_chat_url_auto_dedup() {
        assert_eq!(
            resolve_chat_url(
                "https://api.openai.com/v1",
                Some("/v1/messages"),
                "/chat/completions"
            ),
            "https://api.openai.com/v1/messages"
        );
    }

    #[test]
    fn test_resolve_chat_url_empty_path() {
        assert_eq!(
            resolve_chat_url("https://api.openai.com/v1", Some(""), "/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_resolve_chat_url_path_without_slash() {
        assert_eq!(
            resolve_chat_url("https://api.openai.com/v1", Some("messages"), "/chat/completions"),
            "https://api.openai.com/v1/messages"
        );
    }
}
