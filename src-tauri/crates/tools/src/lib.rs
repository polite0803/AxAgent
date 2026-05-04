//! AxAgent Tool System - 统一工具接口与执行引擎
//!
//! 提供 Tool trait、ToolRegistry、编排器、流式执行器等核心组件。

pub mod agent_def_loader;
pub mod agent_def_types;
pub mod bash;
pub mod builtin_handlers;
pub mod builtin_tools;
pub mod hooks;
pub mod mcp;
pub mod orchestration;
pub mod permissions;
pub mod recorder;
pub mod registry;
pub mod stats;
pub mod streaming;
pub mod tools;

pub use builtin_tools::{
    get_all_builtin_server_definitions, get_all_builtin_tools_flat,
    get_handler as get_builtin_handler, register_builtin_handler, BoxedToolHandler,
    FlatBuiltinTool,
};
pub use recorder::ToolExecutionRecorder;
pub use stats::{StatCategory, ToolMetadata, ToolUsageStats};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::sync::Arc;

/// 工具所属类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    /// 只读文件操作 (read, glob, grep, list)
    FileRead,
    /// 写入文件操作 (write, edit, delete)
    FileWrite,
    /// Shell 命令执行
    Shell,
    /// 网络请求
    Network,
    /// 系统操作
    System,
    /// Agent 相关 (子 agent、工作流)
    Agent,
}

impl ToolCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolCategory::FileRead => "file_read",
            ToolCategory::FileWrite => "file_write",
            ToolCategory::Shell => "shell",
            ToolCategory::Network => "network",
            ToolCategory::System => "system",
            ToolCategory::Agent => "agent",
        }
    }

    pub fn is_read_only(&self) -> bool {
        matches!(self, ToolCategory::FileRead | ToolCategory::Network)
    }
}

/// 工具执行上下文
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// 工作目录
    pub working_dir: String,
    /// 会话 ID
    pub conversation_id: Option<String>,
    /// 消息 ID
    pub message_id: Option<String>,
    /// 是否可写模式
    pub allow_write: bool,
    /// 是否允许执行 shell
    pub allow_execute: bool,
    /// 是否允许网络请求
    pub allow_network: bool,
    /// 中止信号（用于流式执行）
    pub abort_signal: Option<Arc<tokio::sync::Notify>>,
    /// 自定义配置
    pub extra: std::collections::HashMap<String, String>,
}

impl ToolContext {
    pub fn new(working_dir: impl Into<String>) -> Self {
        Self {
            working_dir: working_dir.into(),
            conversation_id: None,
            message_id: None,
            allow_write: true,
            allow_execute: true,
            allow_network: true,
            abort_signal: None,
            extra: std::collections::HashMap::new(),
        }
    }

    pub fn with_conversation(mut self, id: impl Into<String>) -> Self {
        self.conversation_id = Some(id.into());
        self
    }
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 输出内容（文本、JSON 等）
    pub content: String,
    /// 是否被截断
    pub truncated: bool,
    /// 是否出错
    pub is_error: bool,
    /// 额外的结构化数据
    pub metadata: Option<Value>,
    /// 执行耗时（毫秒）
    pub duration_ms: Option<u64>,
}

impl ToolResult {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            truncated: false,
            is_error: false,
            metadata: None,
            duration_ms: None,
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            truncated: false,
            is_error: true,
            metadata: None,
            duration_ms: None,
        }
    }

    pub fn truncated(content: impl Into<String>, max_chars: usize) -> Self {
        let content = content.into();
        let (content, truncated) = if content.len() > max_chars {
            (
                content[..max_chars].to_string()
                    + &format!(
                        "\n\n[输出被截断，已显示 {max_chars}/{total} 字符]",
                        total = content.len()
                    ),
                true,
            )
        } else {
            (content, false)
        };
        Self {
            content,
            truncated,
            is_error: false,
            metadata: None,
            duration_ms: None,
        }
    }
}

/// 工具错误
#[derive(Debug, Clone)]
pub struct ToolError {
    pub message: String,
    pub kind: ToolErrorKind,
    /// i18n 错误码，格式 "tool.{name}.{kind}" 或 "tool.{name}.{specific}"
    /// 前端通过 t(error_code, { default: message }) 翻译
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
    #[allow(dead_code)]
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

    /// 兼容旧 API：简单字符串构造
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: String::new(),
        }
    }

    pub fn not_found(tool_name: &str) -> Self {
        Self {
            message: format!("工具 '{}' 未找到", tool_name),
            kind: ToolErrorKind::NotFound,
            error_code: format!("tool.{}.notFound", tool_name),
        }
    }

    pub fn permission_denied(tool_name: &str, reason: &str) -> Self {
        Self {
            message: format!("工具 '{}' 权限被拒绝: {}", tool_name, reason),
            kind: ToolErrorKind::PermissionDenied,
            error_code: format!("tool.{}.permissionDenied", tool_name),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::InvalidInput,
            error_code: String::new(),
        }
    }

    pub fn invalid_input_for(tool_name: &str, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::InvalidInput,
            error_code: format!("tool.{}.invalidInput", tool_name),
        }
    }

    pub fn execution_failed(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: String::new(),
        }
    }

    pub fn execution_failed_for(tool_name: &str, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToolErrorKind::ExecutionFailed,
            error_code: format!("tool.{}.executionFailed", tool_name),
        }
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolError {}

/// 权限检查结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResult {
    /// 允许执行
    Allow,
    /// 拒绝执行，附原因
    Deny(String),
    /// 需要用户确认
    Ask(String),
}

/// 统一工具接口
///
/// 所有内置工具、MCP 工具、生成工具都必须实现此 trait。
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具名称（主名）
    fn name(&self) -> &str;

    /// 工具描述（给 LLM 看）
    fn description(&self) -> &str;

    /// 输入参数的 JSON Schema
    fn input_schema(&self) -> Value;

    /// 别名列表
    fn aliases(&self) -> &[&str] {
        &[]
    }

    /// 工具类别
    fn category(&self) -> ToolCategory;

    /// 是否可以并发执行
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    /// 是否只读操作
    fn is_read_only(&self) -> bool {
        self.category().is_read_only()
    }

    /// 是否不可逆操作（删除、覆盖、发送）
    fn is_destructive(&self) -> bool {
        false
    }

    /// 输出结果最大字符数（超过则截断）
    fn max_result_chars(&self) -> usize {
        100_000
    }

    /// 是否启用
    fn is_enabled(&self) -> bool {
        true
    }

    /// 核心执行逻辑
    async fn call(
        &self,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError>;

    /// 输入验证（在执行前调用）
    async fn validate(
        &self,
        input: &serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<(), ToolError> {
        // 默认: 检查 required 字段
        let schema = self.input_schema();
        if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
            for field in required {
                let key = field.as_str().unwrap_or("");
                if input.get(key).is_none() || input.get(key) == Some(&serde_json::Value::Null) {
                    return Err(ToolError::invalid_input(format!("缺少必需参数: {}", key)));
                }
            }
        }
        Ok(())
    }

    /// 权限检查（在执行前调用）
    fn check_permissions(
        &self,
        _input: &serde_json::Value,
        _ctx: &ToolContext,
    ) -> PermissionResult {
        PermissionResult::Allow
    }
}

// ============================================================
// 工具信息（用于注册表和前端展示）
// ============================================================

/// 工具元信息（用于注册和发现）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub aliases: Vec<String>,
    pub category: ToolCategory,
    pub is_concurrency_safe: bool,
    pub is_read_only: bool,
    pub is_destructive: bool,
}

impl ToolInfo {
    pub fn from_tool(tool: &dyn Tool) -> Self {
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            input_schema: tool.input_schema(),
            aliases: tool.aliases().iter().map(|s| s.to_string()).collect(),
            category: tool.category(),
            is_concurrency_safe: tool.is_concurrency_safe(),
            is_read_only: tool.is_read_only(),
            is_destructive: tool.is_destructive(),
        }
    }
}
