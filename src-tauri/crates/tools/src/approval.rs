// SPDX-License-Identifier: AGPL-3.0-only

//! 审批决策层（PLAN-codex-parity P0-2）
//!
//! 复用现有两层分类（`HeuristicClassifier` + `SecurityAnalyzer`），不重写检测
//! 逻辑；本模块只做「分类结果 → 审批决策」的归并与决策，对标 codex 的
//! approval policy 语义。
//!
//! 决策矩阵（`decide`）：
//!
//! | policy     | Safe              | Suspicious        | Dangerous |
//! |------------|-------------------|-------------------|-----------|
//! | Untrusted  | 沙箱跑 / 问用户   | 问用户            | 拒绝      |
//! | OnFailure  | 沙箱跑 / 直通     | 沙箱跑 / 问用户   | 拒绝      |
//! | OnRequest  | 沙箱跑 / 直通     | 问用户            | 拒绝      |
//! | Never      | 沙箱跑 / 直通     | 沙箱跑 / 直通     | 拒绝      |
//!
//! 「沙箱跑 / 直通」按 `sandbox_active` 二选一；Dangerous 是硬拒底线，
//! 四档 policy 一致（`rm -rf /` 类命令不因 Never 放行）。

use axagent_harness::ApprovalPolicy;

/// 威胁级：两层分类的归一输出
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreatLevel {
    /// 安全：直接执行
    Safe,
    /// 可疑：按 policy 决定问用户 / 沙箱内跑
    Suspicious(String),
    /// 危险：所有 policy 一致硬拒（Heuristic suggest_deny / Security Blocked）
    Dangerous(String),
}

/// 审批决策
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// 在沙箱内执行（调用方 ctx.sandbox 必须可用）
    RunInsideSandbox,
    /// 沙箱外直通执行（行为与沙箱功能引入前一致）
    RunOutside,
    /// 询问用户
    AskUser { reason: String },
    /// 硬拒绝
    Deny { reason: String },
}

/// 归并两层分类结果为威胁级。
///
/// - Heuristic `suggest_deny` 或 Security `Blocked` → [`ThreatLevel::Dangerous`]；
/// - Security `Warning` 或 Heuristic `Medium`/`High` → [`ThreatLevel::Suspicious`]；
/// - 其余（Safe/Low 且无结构化警告）→ [`ThreatLevel::Safe`]。
pub fn merge_threat(
    suggest_deny: bool,
    heuristic_reason: &str,
    heuristic_risk_medium_or_high: bool,
    security_warning: Option<&str>,
    security_blocked: Option<&str>,
) -> ThreatLevel {
    if suggest_deny {
        return ThreatLevel::Dangerous(heuristic_reason.to_string());
    }
    if let Some(reason) = security_blocked {
        return ThreatLevel::Dangerous(reason.to_string());
    }
    if let Some(reason) = security_warning {
        return ThreatLevel::Suspicious(reason.to_string());
    }
    if heuristic_risk_medium_or_high {
        return ThreatLevel::Suspicious(heuristic_reason.to_string());
    }
    ThreatLevel::Safe
}

/// 审批决策（纯函数，矩阵见模块文档）。
pub fn decide(
    policy: ApprovalPolicy,
    threat: ThreatLevel,
    sandbox_active: bool,
) -> ApprovalDecision {
    match threat {
        ThreatLevel::Dangerous(reason) => ApprovalDecision::Deny { reason },
        ThreatLevel::Suspicious(reason) => match policy {
            ApprovalPolicy::Untrusted | ApprovalPolicy::OnRequest => {
                ApprovalDecision::AskUser { reason }
            },
            ApprovalPolicy::OnFailure => {
                if sandbox_active {
                    ApprovalDecision::RunInsideSandbox
                } else {
                    ApprovalDecision::AskUser { reason }
                }
            },
            // Never：永不询问；沙箱可用则沙箱内跑（更安全且不打扰），否则直通
            ApprovalPolicy::Never => {
                if sandbox_active {
                    ApprovalDecision::RunInsideSandbox
                } else {
                    ApprovalDecision::RunOutside
                }
            },
        },
        ThreatLevel::Safe => match (policy, sandbox_active) {
            // Untrusted：沙箱不可用时 Safe 命令也要问（用户显式选择了最严格档）
            (ApprovalPolicy::Untrusted, false) => {
                ApprovalDecision::AskUser { reason: "沙箱未启用".to_string() }
            },
            (_, true) => ApprovalDecision::RunInsideSandbox,
            (_, false) => ApprovalDecision::RunOutside,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{ApprovalDecision, ThreatLevel, decide, merge_threat};
    use axagent_harness::ApprovalPolicy;

    fn all_policies() -> Vec<ApprovalPolicy> {
        vec![
            ApprovalPolicy::Untrusted,
            ApprovalPolicy::OnFailure,
            ApprovalPolicy::OnRequest,
            ApprovalPolicy::Never,
        ]
    }

    // ── merge_threat ─────────────────────────────────────────────

    #[test]
    fn merge_heuristic_deny_wins() {
        let t = merge_threat(true, "critical", false, None, None);
        assert_eq!(t, ThreatLevel::Dangerous("critical".to_string()));
    }

    #[test]
    fn merge_security_blocked_wins_over_warning() {
        let t = merge_threat(false, "ok", false, Some("warn"), Some("blocked"));
        assert_eq!(t, ThreatLevel::Dangerous("blocked".to_string()));
    }

    #[test]
    fn merge_security_warning_is_suspicious() {
        let t = merge_threat(false, "ok", false, Some("重定向"), None);
        assert_eq!(t, ThreatLevel::Suspicious("重定向".to_string()));
    }

    #[test]
    fn merge_medium_high_risk_is_suspicious() {
        let t = merge_threat(false, "中风险", true, None, None);
        assert_eq!(t, ThreatLevel::Suspicious("中风险".to_string()));
    }

    #[test]
    fn merge_safe_low_is_safe() {
        assert_eq!(merge_threat(false, "", false, None, None), ThreatLevel::Safe);
        // Low 风险（heuristic_risk_medium_or_high=false 且非 deny）
        assert_eq!(merge_threat(false, "low", false, None, None), ThreatLevel::Safe);
    }

    // ── decide：Dangerous 硬拒底线（4 policy × 2 sandbox 全拒绝） ──

    #[test]
    fn dangerous_always_denied() {
        for policy in all_policies() {
            for sandbox_active in [true, false] {
                let d =
                    decide(policy, ThreatLevel::Dangerous("rm -rf /".to_string()), sandbox_active);
                assert_eq!(
                    d,
                    ApprovalDecision::Deny { reason: "rm -rf /".to_string() },
                    "policy={policy:?} sandbox_active={sandbox_active}"
                );
            }
        }
    }

    // ── decide：Suspicious ───────────────────────────────────────

    #[test]
    fn suspicious_untrusted_and_on_request_always_ask() {
        for policy in [ApprovalPolicy::Untrusted, ApprovalPolicy::OnRequest] {
            for sandbox_active in [true, false] {
                let d = decide(policy, ThreatLevel::Suspicious("警告".into()), sandbox_active);
                assert_eq!(d, ApprovalDecision::AskUser { reason: "警告".to_string() });
            }
        }
    }

    #[test]
    fn suspicious_on_failure_runs_in_sandbox_else_ask() {
        assert_eq!(
            decide(ApprovalPolicy::OnFailure, ThreatLevel::Suspicious("警告".into()), true),
            ApprovalDecision::RunInsideSandbox
        );
        assert_eq!(
            decide(ApprovalPolicy::OnFailure, ThreatLevel::Suspicious("警告".into()), false),
            ApprovalDecision::AskUser { reason: "警告".to_string() }
        );
    }

    #[test]
    fn suspicious_never_never_asks() {
        assert_eq!(
            decide(ApprovalPolicy::Never, ThreatLevel::Suspicious("警告".into()), true),
            ApprovalDecision::RunInsideSandbox
        );
        assert_eq!(
            decide(ApprovalPolicy::Never, ThreatLevel::Suspicious("警告".into()), false),
            ApprovalDecision::RunOutside
        );
    }

    // ── decide：Safe ─────────────────────────────────────────────

    #[test]
    fn safe_follows_sandbox_availability() {
        for policy in [ApprovalPolicy::OnFailure, ApprovalPolicy::OnRequest, ApprovalPolicy::Never]
        {
            assert_eq!(decide(policy, ThreatLevel::Safe, true), ApprovalDecision::RunInsideSandbox);
            assert_eq!(decide(policy, ThreatLevel::Safe, false), ApprovalDecision::RunOutside);
        }
    }

    #[test]
    fn safe_untrusted_asks_when_no_sandbox() {
        assert_eq!(
            decide(ApprovalPolicy::Untrusted, ThreatLevel::Safe, true),
            ApprovalDecision::RunInsideSandbox
        );
        assert!(matches!(
            decide(ApprovalPolicy::Untrusted, ThreatLevel::Safe, false),
            ApprovalDecision::AskUser { .. }
        ));
    }
}
