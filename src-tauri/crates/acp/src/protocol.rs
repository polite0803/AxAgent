//! ACP 协议定义 — 基于 JSON-RPC 2.0

use serde::{Deserialize, Serialize};

/// ACP 方法名称
pub mod methods {
    // 会话管理
    pub const CREATE_SESSION: &str = "session.create";
    pub const RESUME_SESSION: &str = "session.resume";
    pub const CLOSE_SESSION: &str = "session.close";
    pub const LIST_SESSIONS: &str = "session.list";
    pub const GET_SESSION: &str = "session.get";

    // Prompt 交互
    pub const SEND_PROMPT: &str = "prompt.send";
    pub const INTERRUPT: &str = "prompt.interrupt";

    // 控制
    pub const SET_PERMISSION_MODE: &str = "control.setPermissionMode";
    pub const GET_STATUS: &str = "control.getStatus";
    pub const SET_MODEL: &str = "control.setModel";

    // Hook 管理
    pub const REGISTER_HOOK: &str = "hook.register";
    pub const UNREGISTER_HOOK: &str = "hook.unregister";

    // 工具
    pub const LIST_TOOLS: &str = "tool.list";
    pub const CALL_TOOL: &str = "tool.call";
}

/// 创建会话参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionParams {
    pub work_dir: String,
    pub model: Option<String>,
    pub permission_mode: Option<String>,
    pub system_prompt: Option<String>,
}

/// 发送 prompt 参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendPromptParams {
    pub session_id: String,
    pub prompt: String,
    pub max_turns: Option<u32>,
}

/// 注册 hook 参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterHookParams {
    pub session_id: String,
    pub event: String,
    pub callback_url: String,
}

/// 会话创建结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResult {
    pub session_id: String,
    pub work_dir: String,
    pub status: String,
}

/// Prompt 响应结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendPromptResult {
    pub session_id: String,
    pub content: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub turns: usize,
    pub tokens_used: u64,
}

/// 工具调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub tool_result: Option<String>,
    pub is_error: bool,
}

/// 会话状态结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResult {
    pub session_id: String,
    pub status: String,
    pub active_tasks: usize,
    pub tokens_used: u64,
    pub permission_mode: String,
}

/// ACP 协议错误码
pub mod error_codes {
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const SESSION_NOT_FOUND: i32 = -32000;
    pub const SESSION_CLOSED: i32 = -32001;
    pub const PERMISSION_DENIED: i32 = -32002;
    pub const FEATURE_DISABLED: i32 = -32003;
}
