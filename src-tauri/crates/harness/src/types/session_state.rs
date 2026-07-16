// SPDX-License-Identifier: AGPL-3.0-only

//! 三层状态管理枚举（改进4：显式化三层状态管理）
//!
//! 在 harness 层统一定义 Session / Task / Step 三层状态枚举，
//! 消除散落在 agent / coordinator / orchestrator / 前端的不一致状态定义。
//!
//! ## 设计原则
//!
//! - **DB 兼容**：枚举序列化为 snake_case 字符串，与现有 DB `TEXT` 列兼容
//! - **向后兼容**：提供 `From<&str>` 解析，兼容历史遗留字符串值
//! - **类型安全**：编译期捕获拼写错误，替代裸 `String` 状态字段
//! - **权威来源**：AGENTS.md 规则12 — 所有 crate 通过 `pub use` 引用此处定义

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ============================================================================
// SessionStatus — 会话层状态（8 态）
// ============================================================================
//
// 合并来源：
// - coordinator::AgentStatus（7 态：Idle/Initializing/Running/WaitingForConfirmation/Paused/Completed/Failed）
// - 前端 AgentRuntimeStatus（5 值：idle/running/waiting_approval/completed/error）
// - 前端 ExecutionPhase（7 态：idle/planning/executing/waiting_permission/completed/failed/cancelled）
//
// 统一后：idle / initializing / running / waiting_approval / paused / completed / failed / cancelled

/// 会话层状态枚举（8 态状态机）
///
/// 表示一个 Agent 会话从创建到终止的完整生命周期。
/// 状态转换规则：
/// ```text
/// Idle → Initializing → Running ⇄ WaitingApproval
///                         ↓        ↓
///                       Paused   Completed/Failed/Cancelled
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// 空闲 — 会话已创建，等待用户输入
    Idle,
    /// 初始化中 — 正在加载上下文、准备工具
    Initializing,
    /// 运行中 — 正在执行推理/工具调用
    Running,
    /// 等待审批 — 需要用户确认才能继续（原 WaitingForConfirmation / waiting_approval）
    WaitingApproval,
    /// 暂停 — 用户主动暂停，可恢复
    Paused,
    /// 已完成 — 会话正常结束
    Completed,
    /// 已失败 — 发生错误（原 error / failed）
    Failed,
    /// 已取消 — 用户主动取消
    Cancelled,
}

impl SessionStatus {
    /// 转为 snake_case 字符串（用于 DB 存储 / API 传输）
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Initializing => "initializing",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// 是否为终态（不可再转换）
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// 是否为活跃态（正在消耗资源）
    pub fn is_active(self) -> bool {
        matches!(self, Self::Initializing | Self::Running | Self::WaitingApproval)
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SessionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idle" => Ok(Self::Idle),
            "initializing" | "init" => Ok(Self::Initializing),
            "running" | "processing" => Ok(Self::Running),
            "waiting_approval" | "waiting_for_confirmation" | "waiting_permission" => {
                Ok(Self::WaitingApproval)
            },
            "paused" => Ok(Self::Paused),
            "completed" | "ready" => Ok(Self::Completed),
            "failed" | "error" => Ok(Self::Failed),
            "cancelled" | "canceled" => Ok(Self::Cancelled),
            other => Err(format!("unknown SessionStatus: {other}")),
        }
    }
}

// ============================================================================
// TaskStatus — 任务层状态（6 态）
// ============================================================================
//
// 合并来源：
// - agent::AgentTaskStatus（5 态：Pending/Running/Completed/Failed/Skipped）
// - orchestrator::SubTaskStatus（6 态：Pending/Ready/Running/Completed/Failed/Skipped）
//
// 统一后：pending / ready / running / completed / failed / skipped

/// 任务层状态枚举（6 态）
///
/// 表示一个 Task/SubTask 在编排中的执行状态。
/// 状态转换规则：
/// ```text
/// Pending → Ready → Running → Completed/Failed/Skipped
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 等待依赖完成
    Pending,
    /// 依赖已满足，待派发
    Ready,
    /// 已派发，正在执行
    Running,
    /// 成功完成
    Completed,
    /// 失败，可能触发重规划
    Failed,
    /// 因上游失败或条件排除而跳过
    Skipped,
}

impl TaskStatus {
    /// 转为 snake_case 字符串
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    /// 是否为终态
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Skipped)
    }

    /// 是否可派发
    pub fn is_dispatchable(self) -> bool {
        matches!(self, Self::Ready)
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "ready" => Ok(Self::Ready),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            other => Err(format!("unknown TaskStatus: {other}")),
        }
    }
}

// ============================================================================
// StepStatus — 步骤层状态（6 态）
// ============================================================================
//
// 参考来源：
// - entities::tool_executions.status（5 值：pending/running/success/failed/cancelled）
// - 前端 ToolCallState.executionStatus（5 值：queued/running/success/failed/cancelled）
//
// 统一后：pending / running / success / failed / cancelled / skipped

/// 步骤层状态枚举（6 态）
///
/// 表示单个执行步骤（工具调用、推理步骤等）的状态。
/// 这是最细粒度的状态，一个 Task 包含多个 Step。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// 排队等待执行
    Pending,
    /// 正在执行
    Running,
    /// 执行成功
    Success,
    /// 执行失败
    Failed,
    /// 已取消
    Cancelled,
    /// 已跳过（条件不满足等）
    Skipped,
}

impl StepStatus {
    /// 转为 snake_case 字符串
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }

    /// 是否为终态
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Success | Self::Failed | Self::Cancelled | Self::Skipped)
    }

    /// 是否为成功终态
    pub fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }
}

impl fmt::Display for StepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for StepStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" | "queued" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            "cancelled" | "canceled" => Ok(Self::Cancelled),
            "skipped" => Ok(Self::Skipped),
            other => Err(format!("unknown StepStatus: {other}")),
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_status_roundtrip() {
        for status in [
            SessionStatus::Idle,
            SessionStatus::Initializing,
            SessionStatus::Running,
            SessionStatus::WaitingApproval,
            SessionStatus::Paused,
            SessionStatus::Completed,
            SessionStatus::Failed,
            SessionStatus::Cancelled,
        ] {
            let s = status.as_str();
            assert_eq!(SessionStatus::from_str(s).unwrap(), status);
        }
    }

    #[test]
    fn session_status_backward_compat() {
        // 历史遗留值必须能正确解析
        assert_eq!(
            SessionStatus::from_str("waiting_for_confirmation").unwrap(),
            SessionStatus::WaitingApproval
        );
        assert_eq!(
            SessionStatus::from_str("waiting_permission").unwrap(),
            SessionStatus::WaitingApproval
        );
        assert_eq!(SessionStatus::from_str("error").unwrap(), SessionStatus::Failed);
        assert_eq!(SessionStatus::from_str("processing").unwrap(), SessionStatus::Running);
    }

    #[test]
    fn task_status_roundtrip() {
        for status in [
            TaskStatus::Pending,
            TaskStatus::Ready,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Skipped,
        ] {
            let s = status.as_str();
            assert_eq!(TaskStatus::from_str(s).unwrap(), status);
        }
    }

    #[test]
    fn step_status_roundtrip() {
        for status in [
            StepStatus::Pending,
            StepStatus::Running,
            StepStatus::Success,
            StepStatus::Failed,
            StepStatus::Cancelled,
            StepStatus::Skipped,
        ] {
            let s = status.as_str();
            assert_eq!(StepStatus::from_str(s).unwrap(), status);
        }
    }

    #[test]
    fn step_status_queued_alias() {
        assert_eq!(StepStatus::from_str("queued").unwrap(), StepStatus::Pending);
    }

    #[test]
    fn session_status_terminal_check() {
        assert!(SessionStatus::Completed.is_terminal());
        assert!(SessionStatus::Failed.is_terminal());
        assert!(SessionStatus::Cancelled.is_terminal());
        assert!(!SessionStatus::Idle.is_terminal());
        assert!(!SessionStatus::Running.is_terminal());
    }

    #[test]
    fn session_status_active_check() {
        assert!(SessionStatus::Running.is_active());
        assert!(SessionStatus::Initializing.is_active());
        assert!(SessionStatus::WaitingApproval.is_active());
        assert!(!SessionStatus::Idle.is_active());
        assert!(!SessionStatus::Completed.is_active());
    }

    #[test]
    fn serde_snake_case() {
        let json = serde_json::to_string(&SessionStatus::WaitingApproval).unwrap();
        assert_eq!(json, "\"waiting_approval\"");

        let parsed: SessionStatus = serde_json::from_str("\"waiting_approval\"").unwrap();
        assert_eq!(parsed, SessionStatus::WaitingApproval);
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(SessionStatus::Idle.to_string(), "idle");
        assert_eq!(TaskStatus::Ready.to_string(), "ready");
        assert_eq!(StepStatus::Success.to_string(), "success");
    }
}

// ============================================================================
// AgentSession DTO 扩展方法
// ============================================================================

use crate::types::AgentSession;

impl AgentSession {
    /// 将 `runtime_status` 字符串解析为类型安全的 `SessionStatus` 枚举。
    ///
    /// 解析失败时返回 `None` 并记录警告日志（不中断业务流程）。
    /// 新代码应优先使用此方法而非直接比较 `runtime_status` 字符串。
    pub fn runtime_status_enum(&self) -> Option<SessionStatus> {
        match SessionStatus::from_str(&self.runtime_status) {
            Ok(status) => Some(status),
            Err(e) => {
                tracing::warn!(
                    "AgentSession {} runtime_status 无法解析: {} (原始值: {})",
                    self.id,
                    e,
                    self.runtime_status
                );
                None
            },
        }
    }

    /// 设置 `runtime_status` 为指定枚举值。
    pub fn set_runtime_status(&mut self, status: SessionStatus) {
        self.runtime_status = status.as_str().to_string();
    }
}
