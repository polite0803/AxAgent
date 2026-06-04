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
        }
    }

    pub fn is_read_only(&self) -> bool {
        matches!(self, ToolCategory::FileRead | ToolCategory::Network | ToolCategory::Knowledge)
    }
}

/// 权限范围定义
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolPermissions {
    /// 允许调用的工具名白名单（空 = 允许全部）
    pub allowed_tools: Option<Vec<String>>,
    /// 明确禁止的工具名
    pub forbidden_tools: Vec<String>,
    /// 允许的 ToolCategory 白名单（空 = 允许全部）
    pub allowed_categories: Option<Vec<ToolCategory>>,
    /// 最大调用次数（会话级），None = 不限
    pub max_calls_per_session: Option<u32>,
    /// 是否启用严格模式（禁止 LLM 发散）
    pub strict_mode: bool,
}

impl ToolPermissions {
    /// 校验是否允许调用指定工具。
    ///
    /// 检查顺序：
    /// 1. `forbidden_tools` 黑名单
    /// 2. `allowed_tools` 白名单（若设置）
    /// 3. `allowed_categories` 类别白名单（若设置）
    /// 4. `max_calls_per_session` 调用次数限制
    ///
    /// `session_total_calls` 通常由调用方维护和传入。
    pub fn check_tool_allowed(
        &self,
        tool_name: &str,
        category: ToolCategory,
        session_total_calls: u32,
    ) -> PermissionResult {
        // 1. 检查黑名单
        if self.forbidden_tools.iter().any(|t| t == tool_name) {
            let reason = format!("工具 '{tool_name}' 在禁止调用列表中");
            warn!("权限拒绝: {reason}");
            return PermissionResult::Deny(reason);
        }

        // 2. 检查白名单
        if let Some(ref allowed) = self.allowed_tools
            && !allowed.iter().any(|t| t == tool_name)
        {
            let reason = format!("工具 '{tool_name}' 不在允许调用列表中（允许: {:?}）", allowed);
            warn!("权限拒绝: {reason}");
            return PermissionResult::Deny(reason);
        }

        // 3. 检查类别白名单
        if let Some(ref allowed_cats) = self.allowed_categories
            && !allowed_cats.contains(&category)
        {
            let reason =
                format!("工具类别 '{:?}' 不在允许类别中（允许: {:?}）", category, allowed_cats);
            warn!("权限拒绝: {reason}");
            return PermissionResult::Deny(reason);
        }

        // 4. 检查会话级调用次数限制
        if let Some(max_calls) = self.max_calls_per_session
            && session_total_calls >= max_calls
        {
            let reason = format!("工具调用次数已达上限（{max_calls}/{max_calls}）");
            warn!("权限拒绝: {reason}");
            return PermissionResult::Deny(reason);
        }

        PermissionResult::Allow
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
    /// 自定义配置（通过 ToolContext.extra 传递给工具）
    pub extra: std::collections::HashMap<String, String>,
    /// 工具级权限约束（可选，None 表示不施加额外约束）
    pub permissions: Option<Arc<ToolPermissions>>,
    /// 输出脱敏器（可选，None 不过滤）
    pub output_sanitizer: Option<Arc<dyn OutputSanitizer>>,
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
    pub is_concurrency_safe: bool,
    pub is_read_only: bool,
    pub is_destructive: bool,
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

    /// 核心执行逻辑
    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError>;

    /// 输入验证（在执行前调用）
    ///
    /// 默认实现会检查：
    /// - required 字段是否存在
    /// - 每个属性的 type、enum、minimum/maximum、minLength/maxLength
    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ToolError> {
        let schema = self.input_schema();

        // 必填字段检查
        if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
            for field in required {
                let key = field.as_str().unwrap_or("");
                if input.get(key).is_none() || input.get(key) == Some(&Value::Null) {
                    return Err(ToolError::invalid_input(format!("缺少必需参数: {key}")));
                }
            }
        }

        // 校验 properties 中每个字段的类型/格式/枚举值/范围
        if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
            for (prop_name, prop_schema) in properties {
                let val = match input.get(prop_name) {
                    Some(v) if !v.is_null() => v,
                    _ => continue, // 可选参数且未提供，跳过
                };

                // 类型校验
                if let Some(expected_type) = prop_schema.get("type").and_then(|t| t.as_str()) {
                    let type_ok = match expected_type {
                        "string" => val.is_string(),
                        "number" | "integer" => val.is_number(),
                        "boolean" => matches!(val, Value::Bool(_)),
                        "array" => val.is_array(),
                        "object" => val.is_object(),
                        _ => true,
                    };
                    if !type_ok {
                        return Err(ToolError::invalid_input(format!(
                            "参数 '{prop_name}' 应为 {expected_type} 类型"
                        )));
                    }
                    // 对 integer 额外检查必须是整数
                    if expected_type == "integer" && !val.as_f64().is_some_and(|f| f.fract() == 0.0)
                    {
                        return Err(ToolError::invalid_input(format!(
                            "参数 '{prop_name}' 应为整数"
                        )));
                    }
                }

                // 枚举值校验
                if let Some(enum_vals) = prop_schema.get("enum").and_then(|e| e.as_array())
                    && !enum_vals.contains(val)
                {
                    return Err(ToolError::invalid_input(format!(
                        "参数 '{prop_name}' 值不在允许范围内: {:?}",
                        enum_vals
                    )));
                }

                // 最小值/最大值校验（数值）
                if let Some(min) = prop_schema.get("minimum").and_then(|m| m.as_f64())
                    && let Some(n) = val.as_f64()
                    && n < min
                {
                    return Err(ToolError::invalid_input(format!(
                        "参数 '{prop_name}' 不能小于 {min}"
                    )));
                }
                if let Some(max) = prop_schema.get("maximum").and_then(|m| m.as_f64())
                    && let Some(n) = val.as_f64()
                    && n > max
                {
                    return Err(ToolError::invalid_input(format!(
                        "参数 '{prop_name}' 不能大于 {max}"
                    )));
                }

                // 最小长度/最大长度校验（字符串）
                if let Some(min_len) = prop_schema.get("minLength").and_then(|m| m.as_u64())
                    && let Some(s) = val.as_str()
                    && (s.len() as u64) < min_len
                {
                    return Err(ToolError::invalid_input(format!(
                        "参数 '{prop_name}' 长度不能少于 {min_len}"
                    )));
                }
                if let Some(max_len) = prop_schema.get("maxLength").and_then(|m| m.as_u64())
                    && let Some(s) = val.as_str()
                    && (s.len() as u64) > max_len
                {
                    return Err(ToolError::invalid_input(format!(
                        "参数 '{prop_name}' 长度不能超过 {max_len}"
                    )));
                }
            }
        }

        Ok(())
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

/// 默认脱敏器 — 支持正则模式匹配替换
#[derive(Debug, Clone)]
pub struct DefaultOutputSanitizer {
    patterns: Vec<(regex::Regex, &'static str)>,
}

impl DefaultOutputSanitizer {
    pub fn new() -> Self {
        let patterns = vec![
            // API key: sk-xxx
            (regex::Regex::new(r"(?i)(sk|pk)-[a-zA-Z0-9]{20,}").unwrap(), "${1}-****"),
            // 内部 IP: 192.168.x.x / 10.x.x.x
            (regex::Regex::new(r"\b192\.168\.\d{1,3}\.\d{1,3}\b").unwrap(), "192.168.*.*"),
            (regex::Regex::new(r"\b10\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(), "10.*.*.*"),
            (
                regex::Regex::new(r"\b172\.(1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}\b").unwrap(),
                "172.*.*.*",
            ),
            // 邮箱
            (
                regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
                "***@***",
            ),
            // 常见 token 模式: 需要分两步避免 raw string 中引号转义问题
            (
                regex::Regex::new(r"(?i)(token|secret|password)\s*[:=]\s*\S{8,}").unwrap(),
                "${1}=****",
            ),
        ];
        Self { patterns }
    }

    /// 使用自定义模式构建
    pub fn with_custom_patterns(patterns: Vec<(regex::Regex, &'static str)>) -> Self {
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

/// 空脱敏器 — 直接透传
#[derive(Debug, Clone)]
pub struct NoopOutputSanitizer;

impl OutputSanitizer for NoopOutputSanitizer {
    fn sanitize(&self, output: &str, _ctx: &SanitizeContext) -> String {
        output.to_string()
    }
}

// ─────────────────────────────────────────────────────────

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

// ── 输入脱敏 ──────────────────────────────────────────

/// 输入脱敏器 — 对 LLM 输入（用户消息）中的敏感信息做屏蔽
pub trait InputSanitizer: Send + Sync + std::fmt::Debug {
    fn sanitize_input(&self, input: &str, context: &str) -> String;
}

/// 默认输入脱敏器 — 复用 DefaultOutputSanitizer 的正则模式
#[derive(Debug, Clone)]
pub struct DefaultInputSanitizer {
    output_sanitizer: DefaultOutputSanitizer,
}

impl DefaultInputSanitizer {
    pub fn new() -> Self {
        Self {
            output_sanitizer: DefaultOutputSanitizer::new(),
        }
    }
}

impl Default for DefaultInputSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl InputSanitizer for DefaultInputSanitizer {
    fn sanitize_input(&self, input: &str, _context: &str) -> String {
        // 对 LLM 输入做脱敏（只坏不修：只替换敏感内容，不改变语义）
        let ctx = SanitizeContext {
            tool_name: "__input_sanitizer__".into(),
            tool_category: ToolCategory::System,
            conversation_id: None,
        };
        self.output_sanitizer.sanitize(input, &ctx)
    }
}
