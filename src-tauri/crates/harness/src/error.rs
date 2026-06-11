//! 工具错误类型 — 从 axagent-runtime-core 提取的契约接口
//!
//! `ToolError` 和 `ToolErrorKind` 是工具系统的核心错误类型，
//! 供 `axagent-runtime-core`、`axagent-tools`、`axagent-agent` 跨 crate 共享。
//! 各 crate 通过 `pub use axagent_harness::error::*` 重导出保持兼容。

use std::fmt::{Display, Formatter};

/// 工具调用失败时返回的错误
#[derive(Debug, Clone)]
pub struct ToolError {
    pub message: String,
    pub kind: ToolErrorKind,
    /// i18n 错误码，格式 "tool.{name}.{kind}" 或 "tool.{name}.{specific}"
    pub error_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolErrorKind {
    NotFound,
    PermissionDenied,
    InvalidInput,
    ExecutionFailed,
    Timeout,
    Cancelled,
}

impl ToolError {
    fn kind_str(kind: &ToolErrorKind) -> &'static str {
        match kind {
            ToolErrorKind::NotFound => "notFound",
            ToolErrorKind::PermissionDenied => "permissionDenied",
            ToolErrorKind::InvalidInput => "invalidInput",
            ToolErrorKind::ExecutionFailed => "executionFailed",
            ToolErrorKind::Timeout => "timeout",
            ToolErrorKind::Cancelled => "cancelled",
        }
    }

    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: String::new(),
        }
    }

    #[must_use]
    pub fn not_found(tool_name: &str) -> Self {
        Self {
            message: format!("工具 '{}' 未找到", tool_name),
            kind: ToolErrorKind::NotFound,
            error_code: format!("tool.{}.notFound", tool_name),
        }
    }

    #[must_use]
    pub fn permission_denied(tool_name: &str, reason: &str) -> Self {
        Self {
            message: format!("工具 '{tool_name}' 权限被拒绝: {reason}"),
            kind: ToolErrorKind::PermissionDenied,
            error_code: format!("tool.{tool_name}.permissionDenied"),
        }
    }

    #[must_use]
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::InvalidInput,
            error_code: String::new(),
        }
    }

    #[must_use]
    pub fn invalid_input_for(tool_name: &str, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::InvalidInput,
            error_code: format!("tool.{tool_name}.invalidInput"),
        }
    }

    #[must_use]
    pub fn execution_failed(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: String::new(),
        }
    }

    #[must_use]
    pub fn execution_failed_for(tool_name: &str, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: format!("tool.{tool_name}.executionFailed"),
        }
    }

    #[must_use]
    pub fn timeout_for(tool_name: &str, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::Timeout,
            error_code: format!("tool.{tool_name}.timeout"),
        }
    }
}

impl Display for ToolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", Self::kind_str(&self.kind), self.message)
    }
}

impl std::error::Error for ToolError {}
