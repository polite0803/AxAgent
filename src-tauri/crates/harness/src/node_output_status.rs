// SPDX-License-Identifier: AGPL-3.0-only

//! 节点输出状态枚举
//!
//! 用于结构化解析节点输出中的状态字段，替代脆弱的字符串比较。
//! 阶段 D: 锁定隐式协议 — 将 JSON Strings 改为 Structured Enum。

use serde::{Deserialize, Serialize};

/// 节点输出中的状态标记
///
/// 用于解析审批、循环控制等节点的输出状态。
/// 通过 serde tag 功能实现严格解析，避免字符串比较的脆弱性。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NodeOutputStatus {
    /// 等待审批（审批节点挂起等待用户输入）
    WaitingForApproval {
        /// 审批请求详情（可选）
        #[serde(skip_serializing_if = "Option::is_none")]
        approval_request: Option<serde_json::Value>,
        /// 附加消息
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        /// 超时时间（秒）
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout_secs: Option<u64>,
    },

    /// 挂起执行（循环节点等待外部信号）
    Paused {
        /// 挂起原因
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    /// 跳过当前步骤
    Skipped {
        /// 跳过原因
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    /// 需要人工干预
    NeedsIntervention {
        /// 干预原因
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        /// 需要的信息
        #[serde(skip_serializing_if = "Option::is_none")]
        required_info: Option<Vec<String>>,
    },

    /// 自定义状态（扩展点）
    Custom {
        /// 自定义状态名称
        custom_status: String,
    },
}

impl NodeOutputStatus {
    /// 从 JSON 值解析状态
    ///
    /// 使用 serde tag 功能严格解析，无法识别的状态会返回错误。
    pub fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to parse NodeOutputStatus: {e}"))
    }

    /// 从状态字符串判断是否需要中断（触发挂起）
    ///
    /// 返回 true 表示该状态需要触发中断（如审批等待、挂起等）。
    pub fn should_trigger_interrupt(&self) -> bool {
        matches!(
            self,
            Self::WaitingForApproval { .. } | Self::Paused { .. } | Self::NeedsIntervention { .. }
        )
    }

    /// 判断是否是 pending 类状态（兼容旧逻辑）
    ///
    /// 某些节点输出 "pending" 作为状态标记，此方法用于向后兼容。
    pub fn is_pending(&self) -> bool {
        matches!(
            self,
            Self::WaitingForApproval { .. } | Self::Paused { .. } | Self::NeedsIntervention { .. }
        )
    }

    /// 获取状态的字符串表示（用于日志和向后兼容）
    pub fn status_str(&self) -> &'static str {
        match self {
            Self::WaitingForApproval { .. } => "waiting_for_approval",
            Self::Paused { .. } => "paused",
            Self::Skipped { .. } => "skipped",
            Self::NeedsIntervention { .. } => "needs_intervention",
            Self::Custom { .. } => "custom",
        }
    }
}

/// 便捷构造器：创建等待审批状态
impl NodeOutputStatus {
    pub fn waiting_for_approval(
        approval_request: Option<serde_json::Value>,
        message: Option<String>,
        timeout_secs: Option<u64>,
    ) -> Self {
        Self::WaitingForApproval { approval_request, message, timeout_secs }
    }

    pub fn paused(reason: Option<String>) -> Self {
        Self::Paused { reason }
    }

    pub fn skipped(reason: Option<String>) -> Self {
        Self::Skipped { reason }
    }

    pub fn needs_intervention(reason: Option<String>, required_info: Option<Vec<String>>) -> Self {
        Self::NeedsIntervention { reason, required_info }
    }

    pub fn custom(status: String) -> Self {
        Self::Custom { custom_status: status }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_waiting_for_approval() {
        let json = serde_json::json!({
            "status": "waiting_for_approval",
            "approval_request": {"node_id": "test"},
            "message": "Please approve",
            "timeout_secs": 300
        });
        let status = NodeOutputStatus::from_json(&json).unwrap();
        assert!(status.should_trigger_interrupt());
        assert_eq!(status.status_str(), "waiting_for_approval");
    }

    #[test]
    fn test_parse_paused() {
        let json = serde_json::json!({
            "status": "paused",
            "reason": "Waiting for input"
        });
        let status = NodeOutputStatus::from_json(&json).unwrap();
        assert!(status.should_trigger_interrupt());
        assert_eq!(status.status_str(), "paused");
    }

    #[test]
    fn test_parse_skipped() {
        let json = serde_json::json!({
            "status": "skipped",
            "reason": "Not applicable"
        });
        let status = NodeOutputStatus::from_json(&json).unwrap();
        assert!(!status.should_trigger_interrupt());
        assert_eq!(status.status_str(), "skipped");
    }

    #[test]
    fn test_parse_unknown_status_error() {
        let json = serde_json::json!({
            "status": "unknown_status"
        });
        let result = NodeOutputStatus::from_json(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_interrupt_detection() {
        assert!(
            NodeOutputStatus::waiting_for_approval(None, None, None).should_trigger_interrupt()
        );
        assert!(NodeOutputStatus::paused(None).should_trigger_interrupt());
        assert!(NodeOutputStatus::needs_intervention(None, None).should_trigger_interrupt());
        assert!(!NodeOutputStatus::skipped(None).should_trigger_interrupt());
        assert!(!NodeOutputStatus::custom("test".to_string()).should_trigger_interrupt());
    }

    #[test]
    fn test_is_pending() {
        assert!(NodeOutputStatus::waiting_for_approval(None, None, None).is_pending());
        assert!(NodeOutputStatus::paused(None).is_pending());
        assert!(NodeOutputStatus::needs_intervention(None, None).is_pending());
        assert!(!NodeOutputStatus::skipped(None).is_pending());
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let status = NodeOutputStatus::waiting_for_approval(
            Some(serde_json::json!({"node_id": "test"})),
            Some("Approve this".to_string()),
            Some(300),
        );
        let json = serde_json::to_value(&status).unwrap();
        let parsed = NodeOutputStatus::from_json(&json).unwrap();
        assert_eq!(parsed.status_str(), "waiting_for_approval");
    }
}
