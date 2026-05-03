//! Hook 系统
//!
//! PreToolUse / PostToolUse / PostToolUseFailure 钩子系统。
//! 支持 Shell 命令、HTTP 回调、Prompt 注入三种执行方式。

pub mod executors;
pub mod registry;

use serde::{Deserialize, Serialize};

/// Hook 触发事件类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEventType {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
}

/// Hook 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Hook 唯一标识
    pub id: String,
    /// 触发事件类型
    pub event: HookEventType,
    /// 匹配模式（工具名，支持 * 通配符）
    pub tool_pattern: String,
    /// 执行方式
    pub executor: HookExecutor,
    /// 是否启用
    pub enabled: bool,
    /// 超时时间（秒）
    pub timeout_secs: u64,
    /// 优先级（数字越小越先执行）
    pub priority: i32,
}

/// Hook 执行方式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HookExecutor {
    /// 执行 Shell 命令
    #[serde(rename = "shell")]
    Shell(ShellHookExec),
    /// HTTP 回调
    #[serde(rename = "http")]
    Http(HttpHookExec),
    /// Prompt 注入（将 Hook 输出注入对话）
    #[serde(rename = "prompt")]
    Prompt(PromptHookExec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellHookExec {
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHookExec {
    pub url: String,
    pub method: String, // GET, POST
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptHookExec {
    pub template: String,
}

/// Hook 执行结果
#[derive(Debug, Clone)]
pub struct HookResult {
    /// Hook ID
    pub hook_id: String,
    /// 决策行为
    pub action: HookAction,
    /// 修改后的输入（仅 PreToolUse 有效）
    pub modified_input: Option<serde_json::Value>,
    /// 附加上下文（注入到对话中）
    pub additional_context: Option<String>,
    /// 决策原因
    pub reason: Option<String>,
    /// 是否出错
    pub is_error: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookAction {
    Allow,
    Deny,
    Ask,
}

impl HookResult {
    pub fn allowed() -> Self {
        Self {
            hook_id: String::new(),
            action: HookAction::Allow,
            modified_input: None,
            additional_context: None,
            reason: None,
            is_error: false,
        }
    }

    pub fn denied(reason: impl Into<String>) -> Self {
        Self {
            hook_id: String::new(),
            action: HookAction::Deny,
            modified_input: None,
            additional_context: None,
            reason: Some(reason.into()),
            is_error: false,
        }
    }
}
