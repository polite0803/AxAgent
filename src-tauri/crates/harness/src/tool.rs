// SPDX-License-Identifier: AGPL-3.0-only

//! 工具系统契约 — 从 axagent-tools 提取的接口层
//!
//! 所有工具必须实现 `Tool` trait。在 Harness 架构中，
//! agent crate 仅通过此契约依赖工具系统，不依赖具体实现。

use crate::error::ToolError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::warn;

/// 工具所处功能域 — 用于按需加载工具 schema 给 LLM
///
/// - `Core`：文件/Shell/网络/Agent 等绝对必备工具，永远随请求发送
/// - `General`：常用开发工具（Git/浏览器/文档），默认启用，非必要场景可跳过
/// - `Devops`：CI/CD、安全审计、打包分析等运维工具
/// - `AiMedia`：图片生成、媒体投递等 AI 媒体工具
/// - `Invest`：投资分析业务工具（AxInvest）
/// - `Opc`：一人公司业务工具（AxOPC）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolDomain {
    Core,
    General,
    Devops,
    AiMedia,
    Invest,
    Opc,
}

impl ToolDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolDomain::Core => "core",
            ToolDomain::General => "general",
            ToolDomain::Devops => "devops",
            ToolDomain::AiMedia => "ai_media",
            ToolDomain::Invest => "invest",
            ToolDomain::Opc => "opc",
        }
    }
}

/// 工具所属类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    /// 只读文件操作 (read, glob, grep, list)
    FileRead,
    /// 写入文件操作 (write, edit, delete)
    FileWrite,
    /// Shell 命令执行
    Shell,
    /// 网络请求（WebFetch, WebSearch）
    Network,
    /// 系统操作
    System,
    /// Agent 相关 (子 agent、工作流)
    Agent,
    /// 版本控制 (Git)
    Vcs,
    /// 自动化（定时任务、后台任务、工作流）
    Automation,
    /// 通信（消息、通知、团队）
    Communication,
    /// AI 媒体（图片生成、图表、推理）
    AiMedia,
    /// 外部集成（Dify, Obsidian 等）
    Integration,
    /// 存储管理
    Storage,
    /// 知识库
    Knowledge,
    /// 浏览器自动化
    Browser,
    /// 桌面控制
    Desktop,
    /// 金融计算（ATR、KDJ、回撤、VaR 等）
    Finance,
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
            ToolCategory::Vcs => "vcs",
            ToolCategory::Automation => "automation",
            ToolCategory::Communication => "communication",
            ToolCategory::AiMedia => "ai_media",
            ToolCategory::Integration => "integration",
            ToolCategory::Storage => "storage",
            ToolCategory::Knowledge => "knowledge",
            ToolCategory::Browser => "browser",
            ToolCategory::Desktop => "desktop",
            ToolCategory::Finance => "finance",
        }
    }

    pub fn is_read_only(&self) -> bool {
        matches!(self, ToolCategory::FileRead | ToolCategory::Network | ToolCategory::Knowledge)
    }

    /// 返回该类工具对应的默认工具组 ID。
    /// 统一 category → group 映射，消除多处重复定义。
    pub fn default_group(&self) -> &'static str {
        match self {
            ToolCategory::FileRead => "builtin-file-read",
            ToolCategory::FileWrite => "builtin-file-write",
            ToolCategory::Shell => "builtin-shell",
            ToolCategory::Network => "builtin-network",
            ToolCategory::System => "builtin-system-tools",
            ToolCategory::Agent => "builtin-agent",
            ToolCategory::Vcs => "builtin-vcs",
            ToolCategory::Automation => "builtin-automation",
            ToolCategory::Communication => "builtin-communication",
            ToolCategory::AiMedia => "builtin-ai-media",
            ToolCategory::Integration => "builtin-integration",
            ToolCategory::Storage => "builtin-storage",
            ToolCategory::Knowledge => "builtin-knowledge",
            ToolCategory::Browser => "builtin-browser",
            ToolCategory::Desktop => "builtin-desktop",
            ToolCategory::Finance => "builtin-finance",
        }
    }
}

// ToolPermissions 结构体及其 check_tool_allowed() 方法已提取至
// `tool_permissions` 模块。保留此处的 re-export 以保证向后兼容。
pub use crate::tool_permissions::ToolPermissions;

/// 用户提问桥接器 — 工具调用此接口向用户提问并阻塞等待回复。
/// 由 wiring 层注入具体实现（emit 前端事件 + 阻塞等待 oneshot 回复）。
pub trait AskUserBridge: Send + Sync + std::fmt::Debug {
    /// 向用户提问并阻塞等待回复。
    /// `ask_id` 是唯一标识符，用于匹配前端响应。
    /// `questions_json` 是 `AskUserQuestionTool` 的输入 JSON。
    /// `conversation_id` 是当前对话 ID。
    /// 返回用户的回答字符串，或错误信息。
    fn ask_user_blocking(
        &self,
        ask_id: String,
        questions_json: serde_json::Value,
        conversation_id: &str,
    ) -> Result<String, String>;
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
    /// 自定义配置（通过 ToolContext.extra 传递给工具）
    pub extra: std::collections::HashMap<String, String>,
    /// 工具级权限约束（可选，None 表示不施加额外约束）
    pub permissions: Option<Arc<ToolPermissions>>,
    /// 输出脱敏器（可选，None 不过滤）
    pub output_sanitizer: Option<Arc<dyn OutputSanitizer>>,
    /// 用户提问桥接器（可选，None 表示 AskUserQuestion 工具降级为纯文本输出）
    pub ask_user_bridge: Option<Arc<dyn AskUserBridge>>,
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
            permissions: None,
            output_sanitizer: None,
            ask_user_bridge: None,
        }
    }

    /// 设置会话 ID（链式调用）
    pub fn with_conversation(mut self, id: impl Into<String>) -> Self {
        self.conversation_id = Some(id.into());
        self
    }
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    pub truncated: bool,
    pub is_error: bool,
    pub metadata: Option<Value>,
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub progress: Vec<ProgressEntry>,
}

impl ToolResult {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            truncated: false,
            is_error: false,
            metadata: None,
            duration_ms: None,
            progress: Vec::new(),
        }
    }

    /// 创建成功结果并执行脱敏
    pub fn sanitized(
        content: impl Into<String>,
        ctx: &SanitizeContext,
        sanitizer: &dyn OutputSanitizer,
    ) -> Self {
        let raw = content.into();
        let content = sanitizer.sanitize(&raw, ctx);
        if content != raw {
            warn!(tool_name = %ctx.tool_name, "OutputSanitizer: 已脱敏敏感信息");
        }
        Self {
            content,
            truncated: false,
            is_error: false,
            metadata: None,
            duration_ms: None,
            progress: Vec::new(),
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            truncated: false,
            is_error: true,
            metadata: None,
            duration_ms: None,
            progress: Vec::new(),
        }
    }

    /// 创建截断结果
    pub fn truncated(content: impl Into<String>, max_chars: usize) -> Self {
        let content = content.into();
        let (content, truncated) = if content.len() > max_chars {
            (
                format!(
                    "{}\n\n[输出被截断，已显示 {max_chars}/{} 字符]",
                    &content[..max_chars],
                    content.len(),
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
            progress: Vec::new(),
        }
    }

    /// 追加一条进度条目
    pub fn with_progress(mut self, entry: ProgressEntry) -> Self {
        self.progress.push(entry);
        self
    }
}

/// 流式进度报告条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEntry {
    /// 阶段标识: "searching"|"fetching"|"rendering"|"cleaning"|"done"
    pub phase: String,
    /// 人类可读描述
    pub message: String,
    /// 进度百分比 (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
    /// 距开始时间的毫秒数
    pub timestamp_ms: u64,
}

/// 工具元信息（用于注册表和前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub aliases: Vec<String>,
    pub category: ToolCategory,
    pub domain: ToolDomain,
    pub is_concurrency_safe: bool,
    pub is_read_only: bool,
    pub is_destructive: bool,
    pub enabled: bool,
}

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

    /// 工具所属功能域（默认 Core，业务工具应覆盖此方法）
    fn domain(&self) -> ToolDomain {
        ToolDomain::Core
    }

    /// 核心执行逻辑
    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError>;

    /// 输入验证（在执行前调用）
    ///
    /// 默认实现委托给 `ToolValidator`，校验 required / type / enum /
    /// minimum / maximum / minLength / maxLength。
    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ToolError> {
        crate::tool_validation::ToolValidator.validate(input, &self.input_schema())
    }

    /// 权限检查（在执行前调用）
    fn check_permissions(&self, _input: &Value, _ctx: &ToolContext) -> PermissionResult {
        PermissionResult::Allow
    }
}

/// 拆分复合工具名 `"server/tool"` → `("server", "tool")`。
/// 无 `/` 时返回 `("", full_name)`。
pub fn parse_tool_name(full_name: &str) -> (&str, &str) {
    if let Some(idx) = full_name.find('/') {
        (&full_name[..idx], &full_name[idx + 1..])
    } else {
        ("", full_name)
    }
}

// ── 敏感数据过滤 ──────────────────────────────────────

/// 脱敏上下文
#[derive(Debug, Clone)]
pub struct SanitizeContext {
    pub tool_name: String,
    pub tool_category: ToolCategory,
    pub conversation_id: Option<String>,
}

/// 输出脱敏器 — 对工具结果中的敏感信息做自动替换
pub trait OutputSanitizer: Send + Sync + std::fmt::Debug {
    fn sanitize(&self, output: &str, ctx: &SanitizeContext) -> String;
}

// 默认脱敏器和 Noop 实现已提取至 `output_sanitizer` 模块。
// 保留此处的 re-export 以保证向后兼容。
pub use crate::output_sanitizer::{DefaultInputSanitizer, DefaultOutputSanitizer};
pub use crate::test_support::NoopOutputSanitizer;

// ─────────────────────────────────────────────────────────

impl ToolInfo {
    pub fn from_tool(tool: &dyn Tool) -> Self {
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            input_schema: tool.input_schema(),
            aliases: tool.aliases().iter().map(|s| s.to_string()).collect(),
            category: tool.category(),
            domain: tool.domain(),
            is_concurrency_safe: tool.is_concurrency_safe(),
            is_read_only: tool.is_read_only(),
            is_destructive: tool.is_destructive(),
            enabled: true,
        }
    }
}

// ── 工具排名契约 ──────────────────────────────────────

/// 工具排名器 — 用 RL 策略学习的权重对工具列表重排。
///
/// 在 `get_chat_tools()` 发送给 LLM 之前调用，高权重工具排在前面，
/// 间接影响 LLM 的工具选择偏好。
pub trait ToolRanker: Send + Sync {
    fn rank_tools(&self, tools: Vec<crate::types::ChatTool>) -> Vec<crate::types::ChatTool>;
}

// ── 输入脱敏 ──────────────────────────────────────────

/// 输入脱敏器 — 对 LLM 输入（用户消息）中的敏感信息做屏蔽
pub trait InputSanitizer: Send + Sync + std::fmt::Debug {
    fn sanitize_input(&self, input: &str, context: &str) -> String;
}

// 默认输入脱敏器已提取至 `output_sanitizer` 模块。
