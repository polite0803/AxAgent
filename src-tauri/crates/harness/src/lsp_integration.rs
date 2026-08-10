// SPDX-License-Identifier: AGPL-3.0-only

//! LSP 集成数据模型 (P2-18)
//!
//! Language Server Protocol 集成相关 DTO，
//! 用于与代码编辑器的 LSP 服务器交互

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// LSP 配置
// ---------------------------------------------------------------------------

/// LSP 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspConfig {
    /// 是否启用 LSP 集成
    pub enabled: bool,
    /// 服务器列表
    pub servers: Vec<LspServer>,
    /// 默认服务器 ID
    pub default_server_id: Option<String>,
    /// 自动启动
    pub auto_start: bool,
    /// 连接超时（毫秒）
    pub connection_timeout_ms: u64,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            servers: Vec::new(),
            default_server_id: None,
            auto_start: false,
            connection_timeout_ms: 5000,
        }
    }
}

/// LSP 服务器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServer {
    /// 服务器 ID
    pub id: String,
    /// 语言
    pub language: String,
    /// 服务器类型
    pub server_type: LspServerType,
    /// 连接方式
    pub connection_type: LspConnectionType,
    /// 连接地址
    pub connection_address: String,
    /// 启动命令（用于 stdio 模式）
    pub start_command: Option<String>,
    /// 启动参数
    pub start_args: Vec<String>,
    /// 是否启用
    pub enabled: bool,
}

/// LSP 服务器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspServerType {
    /// 官方服务器
    Official,
    /// 社区服务器
    Community,
    /// 自定义服务器
    Custom,
    /// 内嵌服务器
    Embedded,
}

/// LSP 连接方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspConnectionType {
    /// Stdio 标准输入输出
    Stdio,
    /// TCP 套接字
    Tcp,
    /// 管道
    Pipe,
    /// HTTP
    Http,
}

// ---------------------------------------------------------------------------
// LSP 请求/响应
// ---------------------------------------------------------------------------

/// LSP 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRequest {
    /// 请求 ID
    pub id: String,
    /// 服务器 ID
    pub server_id: String,
    /// 请求方法
    pub method: LspMethod,
    /// 参数
    pub params: serde_json::Value,
    /// 超时（毫秒）
    pub timeout_ms: u64,
}

/// LSP 方法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspMethod {
    Initialize,
    Initialized,
    Shutdown,
    TextDocumentDidOpen,
    TextDocumentDidChange,
    TextDocumentDidClose,
    TextDocumentCompletion,
    TextDocumentHover,
    TextDocumentDefinition,
    TextDocumentReferences,
    TextDocumentFormatting,
    TextDocumentDiagnostic,
    TextDocumentCodeAction,
    WorkspaceDidChangeConfiguration,
    WorkspaceSymbol,
}

/// LSP 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspResponse {
    /// 响应 ID
    pub id: String,
    /// 请求 ID
    pub request_id: String,
    /// 服务器 ID
    pub server_id: String,
    /// 响应结果
    pub result: Option<serde_json::Value>,
    /// 错误
    pub error: Option<LspError>,
    /// 耗时（毫秒）
    pub elapsed_ms: u64,
}

/// LSP 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// LSP 代码上下文
// ---------------------------------------------------------------------------

/// LSP 代码上下文（用于增强 AI 的代码理解能力）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCodeContext {
    /// 文件路径
    pub file_path: String,
    /// 光标位置
    pub cursor_position: CursorPosition,
    /// 选择的代码
    pub selected_code: Option<String>,
    /// 语言
    pub language: String,
    /// 诊断信息
    pub diagnostics: Vec<LspDiagnostic>,
    /// 可用的补全
    pub completions: Vec<LspCompletionItem>,
    /// 悬停信息
    pub hover_info: Option<LspHoverInfo>,
    /// 定义跳转
    pub definition: Option<LspLocation>,
    /// 引用
    pub references: Vec<LspLocation>,
}

/// 光标位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub character: u32,
}

/// LSP 诊断信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<String>,
}

/// 诊断严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// LSP 范围
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LspRange {
    pub start: CursorPosition,
    pub end: CursorPosition,
}

/// LSP 补全项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspCompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

/// 补全项类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionItemKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// LSP 悬停信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspHoverInfo {
    pub content: String,
    pub range: Option<LspRange>,
}

/// LSP 位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}
