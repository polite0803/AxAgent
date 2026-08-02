// SPDX-License-Identifier: AGPL-3.0-only

//! WorkItem 状态机（Self-Run 机制核心）。
//!
//! 纯函数状态机：输入当前 phase + 事件 → 输出新 phase，无 DB 依赖，
//! 便于全流转单测。DB 持久化在 dao 层（opc_work_items 表），
//! 本模块只定义状态转换规则与依赖传播。
//!
//! ```ignore
//! QUEUED → IN_PROGRESS ⇄ BLOCKED → REVIEW → APPROVED → DONE
//!     ↘ WAITING_FOR_CHILDREN                ↘ FAILED / CANCELLED（终态）
//! ```
//!
//! 状态转换规则（纯函数）：
//! ```ignore
//! transition(Phase::Queued, Transition::Start) == Phase::InProgress
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// WorkItem 生命周期阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Phase {
    /// 排队中（可被认领）
    Queued,
    /// 执行中
    InProgress,
    /// 依赖项未完成（等待子项）
    WaitingForChildren,
    /// 阻塞（团队内无法解决，等待升级）
    Blocked,
    /// 评审中
    Review,
    /// 已批准（终态）
    Approved,
    /// 已完成（终态）
    Done,
    /// 失败（终态，触发依赖传播 doomed）
    Failed,
    /// 已取消（终态，触发依赖传播 doomed）
    Cancelled,
}

/// 终态集合：进入后不可再转换（Approved 可继续到 Done，不是终态）。
pub const DONE_PHASES: &[Phase] = &[Phase::Done, Phase::Failed, Phase::Cancelled];

/// 阻塞/失败终态：依赖该 work item 的下游应标记 doomed（不可认领）。
pub fn is_failure_phase(p: Phase) -> bool {
    matches!(p, Phase::Failed | Phase::Cancelled)
}

/// 五种管理模式（Manager 驱动）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementMode {
    /// 亲自执行
    Execute,
    /// 委托给下属
    Delegate,
    /// 交给上级/他人评审
    Review,
    /// 集成多个子项
    Integrate,
    /// 打回返工
    Rework,
}

/// 状态转换事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// 认领开始执行
    Start,
    /// 执行完成，提交评审
    SubmitForReview,
    /// 评审通过
    Approve,
    /// 评审打回返工
    Reject,
    /// 遇到阻塞（团队内无法解决）
    Block,
    /// 阻塞解除，恢复执行
    Unblock,
    /// 依赖项全部完成（子项通知）
    ChildrenDone,
    /// 等待依赖（子项未完成时自动进入）
    WaitChildren,
    /// 执行失败
    Fail,
    /// 取消
    Cancel,
}

/// 转换结果：成功的新 phase，或非法转换错误。
pub type TransitionResult = Result<Phase, TransitionError>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransitionError {
    #[error("非法转换: {from:?} → {event:?}")]
    Illegal { from: Phase, event: Transition },
    #[error("终态不可转换: {phase:?}")]
    Terminal { phase: Phase },
    #[error("无法解析 phase: {input:?}")]
    Parse { input: String },
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Phase {
    /// 序列化名（DB 存储用，SCREAMING_SNAKE_CASE）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Queued => "QUEUED",
            Phase::InProgress => "IN_PROGRESS",
            Phase::WaitingForChildren => "WAITING_FOR_CHILDREN",
            Phase::Blocked => "BLOCKED",
            Phase::Review => "REVIEW",
            Phase::Approved => "APPROVED",
            Phase::Done => "DONE",
            Phase::Failed => "FAILED",
            Phase::Cancelled => "CANCELLED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        DONE_PHASES.contains(self)
    }

    /// 看板列名（前端 Kanban 投影）。
    pub fn kanban_column(&self) -> &'static str {
        match self {
            Phase::Queued | Phase::WaitingForChildren => "待办",
            Phase::InProgress => "进行中",
            Phase::Blocked => "阻塞",
            Phase::Review => "评审",
            Phase::Approved | Phase::Done => "已完成",
            Phase::Failed | Phase::Cancelled => "终止",
        }
    }
}

impl FromStr for Phase {
    type Err = TransitionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "QUEUED" => Ok(Phase::Queued),
            "IN_PROGRESS" => Ok(Phase::InProgress),
            "WAITING_FOR_CHILDREN" => Ok(Phase::WaitingForChildren),
            "BLOCKED" => Ok(Phase::Blocked),
            "REVIEW" => Ok(Phase::Review),
            "APPROVED" => Ok(Phase::Approved),
            "DONE" => Ok(Phase::Done),
            "FAILED" => Ok(Phase::Failed),
            "CANCELLED" => Ok(Phase::Cancelled),
            _ => Err(TransitionError::Parse { input: s.to_string() }),
        }
    }
}

/// 纯函数状态机：phase + 事件 → 新 phase。
pub fn transition(current: Phase, event: Transition) -> TransitionResult {
    if current.is_terminal() {
        return Err(TransitionError::Terminal { phase: current });
    }
    let next = match (current, event) {
        // 认领执行
        (Phase::Queued, Transition::Start) => Phase::InProgress,
        (Phase::Queued, Transition::WaitChildren) => Phase::WaitingForChildren,
        (Phase::Queued, Transition::Cancel) => Phase::Cancelled,
        (Phase::Queued, Transition::Fail) => Phase::Failed,
        // 执行中
        (Phase::InProgress, Transition::SubmitForReview) => Phase::Review,
        (Phase::InProgress, Transition::Block) => Phase::Blocked,
        (Phase::InProgress, Transition::WaitChildren) => Phase::WaitingForChildren,
        (Phase::InProgress, Transition::Cancel) => Phase::Cancelled,
        (Phase::InProgress, Transition::Fail) => Phase::Failed,
        // 等待依赖
        (Phase::WaitingForChildren, Transition::ChildrenDone) => Phase::InProgress,
        (Phase::WaitingForChildren, Transition::Cancel) => Phase::Cancelled,
        (Phase::WaitingForChildren, Transition::Fail) => Phase::Failed,
        // 阻塞
        (Phase::Blocked, Transition::Unblock) => Phase::InProgress,
        (Phase::Blocked, Transition::Cancel) => Phase::Cancelled,
        (Phase::Blocked, Transition::Fail) => Phase::Failed,
        // 评审
        (Phase::Review, Transition::Approve) => Phase::Approved,
        (Phase::Review, Transition::Reject) => Phase::InProgress, // 返工
        (Phase::Review, Transition::Block) => Phase::Blocked,
        (Phase::Review, Transition::Cancel) => Phase::Cancelled,
        (Phase::Review, Transition::Fail) => Phase::Failed,
        // 已批准 → 完成
        (Phase::Approved, Transition::Start) => Phase::Done,
        _ => return Err(TransitionError::Illegal { from: current, event }),
    };
    Ok(next)
}

/// 管理模式 → 推荐状态机路径（供 Manager 决策参考）。
pub fn mode_transition_hint(mode: ManagementMode) -> &'static [Transition] {
    match mode {
        ManagementMode::Execute => &[Transition::Start, Transition::SubmitForReview],
        ManagementMode::Delegate => &[Transition::Start, Transition::SubmitForReview],
        ManagementMode::Review => &[Transition::SubmitForReview, Transition::Approve],
        ManagementMode::Integrate => {
            &[Transition::WaitChildren, Transition::ChildrenDone, Transition::SubmitForReview]
        },
        ManagementMode::Rework => &[Transition::Reject, Transition::Start],
    }
}

/// 依赖传播：上游失败/取消 → 下游 doomed（不可认领）。
///
/// 返回 true 表示该 work item 因依赖失败而不可执行。
pub fn dependency_doomed(upstream_phases: &[Phase]) -> bool {
    upstream_phases.iter().any(|p| is_failure_phase(*p))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 全主路径流转：QUEUED → IN_PROGRESS → REVIEW → APPROVED → DONE
    #[test]
    fn happy_path_full_cycle() {
        let p = transition(Phase::Queued, Transition::Start).unwrap();
        assert_eq!(p, Phase::InProgress);
        let p = transition(p, Transition::SubmitForReview).unwrap();
        assert_eq!(p, Phase::Review);
        let p = transition(p, Transition::Approve).unwrap();
        assert_eq!(p, Phase::Approved);
        let p = transition(p, Transition::Start).unwrap();
        assert_eq!(p, Phase::Done);
        assert!(p.is_terminal());
    }

    /// 阻塞环：IN_PROGRESS ⇄ BLOCKED
    #[test]
    fn blocked_cycle() {
        let p = transition(Phase::InProgress, Transition::Block).unwrap();
        assert_eq!(p, Phase::Blocked);
        let p = transition(p, Transition::Unblock).unwrap();
        assert_eq!(p, Phase::InProgress);
    }

    /// 依赖等待：WAITING_FOR_CHILDREN → IN_PROGRESS
    #[test]
    fn waiting_children_cycle() {
        let p = transition(Phase::InProgress, Transition::WaitChildren).unwrap();
        assert_eq!(p, Phase::WaitingForChildren);
        let p = transition(p, Transition::ChildrenDone).unwrap();
        assert_eq!(p, Phase::InProgress);
    }

    /// 评审打回返工：REVIEW → IN_PROGRESS
    #[test]
    fn review_reject_rework() {
        let p = transition(Phase::Review, Transition::Reject).unwrap();
        assert_eq!(p, Phase::InProgress);
    }

    /// 终态不可转换
    #[test]
    fn terminal_phase_immutable() {
        for terminal in DONE_PHASES {
            assert!(matches!(
                transition(*terminal, Transition::Start),
                Err(TransitionError::Terminal { .. })
            ));
        }
    }

    /// 非法转换报错
    #[test]
    fn illegal_transition() {
        assert!(matches!(
            transition(Phase::Queued, Transition::Approve),
            Err(TransitionError::Illegal { .. })
        ));
    }

    /// 失败/取消 → 依赖 doomed
    #[test]
    fn dependency_doomed_propagation() {
        assert!(dependency_doomed(&[Phase::Failed]));
        assert!(dependency_doomed(&[Phase::Cancelled, Phase::Done]));
        assert!(!dependency_doomed(&[Phase::Done, Phase::Approved]));
        assert!(!dependency_doomed(&[Phase::InProgress]));
    }

    /// 管理模式提示
    #[test]
    fn mode_hints() {
        assert_eq!(
            mode_transition_hint(ManagementMode::Execute),
            &[Transition::Start, Transition::SubmitForReview]
        );
        assert!(!mode_transition_hint(ManagementMode::Integrate).is_empty());
    }

    /// phase 序列化往返
    #[test]
    fn phase_serde_roundtrip() {
        for p in [
            Phase::Queued,
            Phase::InProgress,
            Phase::Blocked,
            Phase::Review,
            Phase::Approved,
            Phase::Done,
            Phase::Failed,
            Phase::Cancelled,
            Phase::WaitingForChildren,
        ] {
            let s = p.as_str();
            assert_eq!(s.parse::<Phase>().ok(), Some(p));
        }
        assert_eq!("BOGUS".parse::<Phase>().ok(), None);
    }

    /// 看板列映射
    #[test]
    fn kanban_column_mapping() {
        assert_eq!(Phase::Queued.kanban_column(), "待办");
        assert_eq!(Phase::Blocked.kanban_column(), "阻塞");
        assert_eq!(Phase::Done.kanban_column(), "已完成");
    }
}
