// SPDX-License-Identifier: AGPL-3.0-only

//! 审批策略（PLAN-codex-parity P0-2）
//!
//! 对标 codex approval policy 四档语义，决策消费方为 Shell 类工具
//! （`tools/src/approval.rs` 的 `decide`）：
//! - `Untrusted`：最严格，非 Safe 一律问用户；
//! - `OnFailure`：沙箱内先跑，失败且疑似沙箱限制 → 问用户批准后沙箱外重试一次；
//! - `OnRequest`：可疑/危险 → 问用户（默认档）；
//! - `Never`：永不询问，除硬危险外自动执行（用户自担风险）。

use serde::{Deserialize, Serialize};

/// 审批策略（对标 codex approval policy）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalPolicy {
    /// 最严格：非 Safe 命令一律询问用户
    Untrusted,
    /// 沙箱内先跑；失败且疑似沙箱限制时询问用户批准后沙箱外重试一次
    OnFailure,
    /// 可疑/危险命令询问用户（默认档，与既有行为最接近）
    #[default]
    OnRequest,
    /// 永不询问：除硬危险（始终拒绝）外自动执行
    Never,
}

impl ApprovalPolicy {
    /// 解析 settings 存储的策略字符串（kebab-case），未识别值回退 `OnRequest`。
    #[must_use]
    pub fn from_policy_str(policy: &str) -> Self {
        match policy {
            "untrusted" => Self::Untrusted,
            "on-failure" => Self::OnFailure,
            "never" => Self::Never,
            _ => Self::OnRequest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ApprovalPolicy;

    #[test]
    fn serializes_kebab_case() {
        assert_eq!(serde_json::to_string(&ApprovalPolicy::Untrusted).unwrap(), "\"untrusted\"");
        assert_eq!(serde_json::to_string(&ApprovalPolicy::OnFailure).unwrap(), "\"on-failure\"");
        assert_eq!(serde_json::to_string(&ApprovalPolicy::OnRequest).unwrap(), "\"on-request\"");
        assert_eq!(serde_json::to_string(&ApprovalPolicy::Never).unwrap(), "\"never\"");
        let back: ApprovalPolicy = serde_json::from_str("\"on-failure\"").unwrap();
        assert_eq!(back, ApprovalPolicy::OnFailure);
    }

    #[test]
    fn from_policy_str_maps_and_falls_back() {
        assert_eq!(ApprovalPolicy::from_policy_str("untrusted"), ApprovalPolicy::Untrusted);
        assert_eq!(ApprovalPolicy::from_policy_str("on-failure"), ApprovalPolicy::OnFailure);
        assert_eq!(ApprovalPolicy::from_policy_str("on-request"), ApprovalPolicy::OnRequest);
        assert_eq!(ApprovalPolicy::from_policy_str("never"), ApprovalPolicy::Never);
        // 默认档与未识别值都回退 OnRequest
        assert_eq!(ApprovalPolicy::from_policy_str(""), ApprovalPolicy::OnRequest);
        assert_eq!(ApprovalPolicy::from_policy_str("garbage"), ApprovalPolicy::OnRequest);
    }

    #[test]
    fn default_is_on_request() {
        assert_eq!(ApprovalPolicy::default(), ApprovalPolicy::OnRequest);
    }
}
