// SPDX-License-Identifier: AGPL-3.0-only

//! 后端 i18n 模块 — 将面向用户的错误消息从硬编码字符串迁移到 locale 管理。
//!
//! 当前仅支持中文（zh-CN）和英文（en-US），但架构支持扩展。
//! 各 crate 通过 `Locale::current()` 获取当前 locale，`msg(key)` 获取对应消息。

use std::fmt;

/// 支持的语言环境
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    /// 简体中文
    ZhCn,
    /// 美式英语
    EnUs,
}

impl Locale {
    /// 从环境变量 `AXAGENT_LOCALE` 或系统 locale 检测当前语言。
    pub fn from_env() -> Self {
        if let Ok(val) = std::env::var("AXAGENT_LOCALE") {
            match val.as_str() {
                "en" | "en-US" | "en_US" => return Locale::EnUs,
                "zh" | "zh-CN" | "zh_CN" => return Locale::ZhCn,
                _ => {},
            }
        }
        // 默认中文
        Locale::ZhCn
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Locale::ZhCn => write!(f, "zh-CN"),
            Locale::EnUs => write!(f, "en-US"),
        }
    }
}

/// 消息键枚举 — 每个键对应一条面向用户的消息。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum I18nKey {
    // ── Providers ──
    /// 无法构建 {provider} HTTP 客户端: {error}，降级为默认客户端
    ProviderHttpClientBuildFailed,
    /// 无效的 HTTP 方法 '{method}': {error}
    ProviderInvalidHttpMethod,

    // ── Agent / Action ──
    /// 沙箱安全检查未通过: {error}
    AgentSandboxCheckFailed,
    /// 未配置工具注册表（Harness 未注入 registry）
    AgentToolRegistryNotConfigured,
    /// 工具 '{tool}' 不允许访问绝对路径: {path}
    AgentToolAbsolutePathDenied,
    /// 工具 '{tool}' 不允许路径回溯: {path}
    AgentToolPathTraversalDenied,
    /// {adapter} 未配置工具执行器（Harness 未注入 executor）
    AgentExecutorNotConfigured,

    // ── Agent / Coordinator ──
    /// {field} 不能为空
    AgentFieldRequired,
    /// 早期步骤摘要
    AgentEarlyStepSummary,
    /// ... (更早的步骤已省略)
    AgentOlderStepsOmitted,

    // ── Agent / Feedback ──
    /// 累积 {count} 条负面反馈（评级 1-2），已达到阈值 {threshold}
    AgentNegativeFeedbackThreshold,
    /// 累积 {count} 条正向反馈（评级 4-5），已达到阈值 {threshold}
    AgentPositiveFeedbackThreshold,
}

/// 根据当前 locale 获取消息文本。
/// 不支持格式化的简单消息直接返回 &'static str；
/// 需要格式化的消息使用 `fmt_msg()` / `fmt_msg_with()`。
pub fn msg(key: I18nKey) -> &'static str {
    match key {
        I18nKey::AgentToolRegistryNotConfigured => match Locale::from_env() {
            Locale::ZhCn => "未配置工具注册表（Harness 未注入 registry）",
            Locale::EnUs => "Tool registry not configured (Harness did not inject registry)",
        },
        I18nKey::AgentExecutorNotConfigured => match Locale::from_env() {
            Locale::ZhCn => "未配置工具执行器（Harness 未注入 executor）",
            Locale::EnUs => "Tool executor not configured (Harness did not inject executor)",
        },
        I18nKey::AgentEarlyStepSummary => match Locale::from_env() {
            Locale::ZhCn => "早期步骤摘要",
            Locale::EnUs => "Early step summary",
        },
        I18nKey::AgentOlderStepsOmitted => match Locale::from_env() {
            Locale::ZhCn => "... (更早的步骤已省略)",
            Locale::EnUs => "... (older steps omitted)",
        },
        _ => match Locale::from_env() {
            Locale::ZhCn => "处理中...",
            Locale::EnUs => "Processing...",
        },
    }
}

/// 获取可格式化的消息模板（含 {placeholder}）。
/// 调用方使用 `replace` 填充占位符。
pub fn fmt_msg(key: I18nKey, param: &str) -> String {
    match key {
        I18nKey::ProviderHttpClientBuildFailed => match Locale::from_env() {
            Locale::ZhCn => format!("无法构建 HTTP 客户端: {param}，降级为默认客户端"),
            Locale::EnUs => {
                format!("Failed to build HTTP client: {param}, falling back to default client")
            },
        },
        I18nKey::ProviderInvalidHttpMethod => match Locale::from_env() {
            Locale::ZhCn => format!("无效的 HTTP 方法: {param}"),
            Locale::EnUs => format!("Invalid HTTP method: {param}"),
        },
        I18nKey::AgentSandboxCheckFailed => match Locale::from_env() {
            Locale::ZhCn => format!("沙箱安全检查未通过: {param}"),
            Locale::EnUs => format!("Sandbox security check failed: {param}"),
        },
        I18nKey::AgentToolAbsolutePathDenied => match Locale::from_env() {
            Locale::ZhCn => format!("工具 '{param}' 不允许访问绝对路径"),
            Locale::EnUs => format!("Tool '{param}' absolute path access denied"),
        },
        I18nKey::AgentToolPathTraversalDenied => match Locale::from_env() {
            Locale::ZhCn => format!("工具 '{param}' 不允许路径回溯"),
            Locale::EnUs => format!("Tool '{param}' path traversal denied"),
        },
        I18nKey::AgentFieldRequired => match Locale::from_env() {
            Locale::ZhCn => format!("{param} 不能为空"),
            Locale::EnUs => format!("{param} cannot be empty"),
        },
        _ => format!("{key:?}: {param}"),
    }
}

/// 两个参数的可格式化消息模板。
pub fn fmt_msg_with(key: I18nKey, p1: &str, p2: &str) -> String {
    match key {
        I18nKey::AgentNegativeFeedbackThreshold => match Locale::from_env() {
            Locale::ZhCn => format!("累积 {p1} 条负面反馈（评级 1-2），已达到阈值 {p2}"),
            Locale::EnUs => {
                format!("Accumulated {p1} negative feedback (rating 1-2), threshold {p2} reached")
            },
        },
        I18nKey::AgentPositiveFeedbackThreshold => match Locale::from_env() {
            Locale::ZhCn => format!("累积 {p1} 条正向反馈（评级 4-5），已达到阈值 {p2}"),
            Locale::EnUs => {
                format!("Accumulated {p1} positive feedback (rating 4-5), threshold {p2} reached")
            },
        },
        _ => format!("{key:?}: {p1}, {p2}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msg_registry_not_configured() {
        let s = msg(I18nKey::AgentToolRegistryNotConfigured);
        // Both locales should produce non-empty strings
        assert!(!s.is_empty());
    }

    #[test]
    fn test_fmt_msg_sandbox() {
        let s = fmt_msg(I18nKey::AgentSandboxCheckFailed, "test error");
        assert!(s.contains("test error"));
    }

    #[test]
    fn test_fmt_msg_with_feedback() {
        let s = fmt_msg_with(I18nKey::AgentNegativeFeedbackThreshold, "5", "3");
        assert!(s.contains("5"));
        assert!(s.contains("3"));
    }

    #[test]
    fn test_locale_from_env_en() {
        std::env::set_var("AXAGENT_LOCALE", "en-US");
        let s = msg(I18nKey::AgentToolRegistryNotConfigured);
        assert!(s.contains("Tool registry"));
        std::env::remove_var("AXAGENT_LOCALE");
    }
}
