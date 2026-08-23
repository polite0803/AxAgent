// SPDX-License-Identifier: AGPL-3.0-only

//! 工具权限检查器
//!
//! 从 `tool.rs` 中的 `ToolPermissions::check_tool_allowed()` 提取。
//! 包含黑名单/白名单/类别/次数四层权限检查逻辑。

use crate::tool::{PermissionResult, ToolCategory};
use serde::{Deserialize, Serialize};
use tracing::warn;

/// 权限范围定义
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPermissions {
    /// 允许调用的工具名白名单（空 = 允许全部）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    /// 明确禁止的工具名
    pub forbidden_tools: Vec<String>,
    /// 允许的 ToolCategory 白名单（空 = 允许全部）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_categories: Option<Vec<ToolCategory>>,
    /// 最大调用次数（会话级），None = 不限
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolCategory as Tc;

    #[test]
    fn test_deny_list_blocks_tool() {
        let perms =
            ToolPermissions { forbidden_tools: vec!["dangerous".into()], ..Default::default() };
        match perms.check_tool_allowed("dangerous", Tc::System, 0) {
            PermissionResult::Deny(msg) => assert!(msg.contains("禁止")),
            _ => panic!("Expected Deny"),
        }
    }

    #[test]
    fn test_allow_list_permits_only_listed() {
        let perms =
            ToolPermissions { allowed_tools: Some(vec!["safe".into()]), ..Default::default() };
        assert!(matches!(perms.check_tool_allowed("safe", Tc::System, 0), PermissionResult::Allow));
        assert!(matches!(
            perms.check_tool_allowed("other", Tc::System, 0),
            PermissionResult::Deny(_)
        ));
    }

    #[test]
    fn test_category_restriction() {
        let perms = ToolPermissions {
            allowed_categories: Some(vec![Tc::FileRead, Tc::Network]),
            ..Default::default()
        };
        assert!(matches!(perms.check_tool_allowed("r", Tc::FileRead, 0), PermissionResult::Allow));
        assert!(matches!(
            perms.check_tool_allowed("w", Tc::FileWrite, 0),
            PermissionResult::Deny(_)
        ));
    }

    #[test]
    fn test_max_calls_exceeded() {
        let perms = ToolPermissions { max_calls_per_session: Some(3), ..Default::default() };
        assert!(matches!(perms.check_tool_allowed("t", Tc::System, 2), PermissionResult::Allow));
        assert!(matches!(perms.check_tool_allowed("t", Tc::System, 3), PermissionResult::Deny(_)));
    }

    #[test]
    fn test_empty_permissions_allow_all() {
        let perms = ToolPermissions::default();
        assert!(matches!(
            perms.check_tool_allowed("x", Tc::FileWrite, 999),
            PermissionResult::Allow
        ));
    }
}
