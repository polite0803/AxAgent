// SPDX-License-Identifier: AGPL-3.0-only

//! Hook 类型 — 从 `axagent-runtime-core::hooks` 搬迁的纯数据/trait 定义。
//!
//! 仅含不依赖 runtime-core 内部实现的类型。执行器逻辑（HookRunner 等）保留在 runtime-core。

use serde::{Deserialize, Serialize};

/// 可触发的 Hook 事件枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    // 工具生命周期
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    // 通用
    Notification,
    UserPromptSubmit,
    SessionStart,
    SessionEnd,
    Stop,
    // 子 Agent 生命周期
    SubagentStart,
    SubagentStop,
    // 上下文管理
    PreCompact,
    PostCompact,
    // 队友事件
    TeammateIdle,
    // 任务事件
    TaskCreated,
    TaskCompleted,
    // 交互事件
    Elicitation,
    ElicitationResult,
    // 配置事件
    ConfigChange,
    // 指令事件
    InstructionsLoaded,
    // 文件监控
    FileChanged,
    CwdChanged,
    // 权限事件
    PermissionRequest,
    PermissionDenied,
    // Worktree 事件
    WorktreeCreate,
    WorktreeRemove,
    // 失败事件
    StopFailure,
}

impl HookEvent {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::PostToolUseFailure => "PostToolUseFailure",
            Self::Notification => "Notification",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
            Self::Stop => "Stop",
            Self::StopFailure => "StopFailure",
            Self::SubagentStart => "SubagentStart",
            Self::SubagentStop => "SubagentStop",
            Self::PreCompact => "PreCompact",
            Self::PostCompact => "PostCompact",
            Self::TeammateIdle => "TeammateIdle",
            Self::TaskCreated => "TaskCreated",
            Self::TaskCompleted => "TaskCompleted",
            Self::Elicitation => "Elicitation",
            Self::ElicitationResult => "ElicitationResult",
            Self::ConfigChange => "ConfigChange",
            Self::InstructionsLoaded => "InstructionsLoaded",
            Self::FileChanged => "FileChanged",
            Self::CwdChanged => "CwdChanged",
            Self::PermissionRequest => "PermissionRequest",
            Self::PermissionDenied => "PermissionDenied",
            Self::WorktreeCreate => "WorktreeCreate",
            Self::WorktreeRemove => "WorktreeRemove",
        }
    }

    #[must_use]
    pub fn is_tool_event(self) -> bool {
        matches!(self, Self::PreToolUse | Self::PostToolUse | Self::PostToolUseFailure)
    }

    #[must_use]
    pub fn is_session_event(self) -> bool {
        matches!(self, Self::SessionStart | Self::SessionEnd)
    }

    #[must_use]
    pub fn is_subagent_event(self) -> bool {
        matches!(self, Self::SubagentStart | Self::SubagentStop)
    }
}

/// Hook 进度事件（纯数据，用于进度报告）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookProgressEvent {
    Started { event: HookEvent, tool_name: String, command: String, tool_use_id: Option<String> },
    Completed { event: HookEvent, tool_name: String, command: String, tool_use_id: Option<String> },
    Cancelled { event: HookEvent, tool_name: String, command: String, tool_use_id: Option<String> },
}

/// Hook 进度报告 trait。
pub trait HookProgressReporter: Send + Sync {
    fn on_event(&mut self, event: &HookProgressEvent);

    /// 周期性进度通知（默认 noop）。
    fn on_progress(&mut self, _message: &str, _iteration: usize, _total: usize) {}
}
