// SPDX-License-Identifier: AGPL-3.0-only
//
// 权限系统 — 类型定义已上移至 `axagent-harness`。
// 本文件提供 RuntimePermissionRuleConfig 桥接扩展。

pub use axagent_harness::runtime_types::permissions::*;

use crate::config::RuntimePermissionRuleConfig;

/// 扩展 trait：为 PermissionPolicy 添加 runtime-core 特有的 `with_permission_rules`。
pub trait PermissionPolicyExt {
    fn with_permission_rules(self, config: &RuntimePermissionRuleConfig) -> Self;
}

impl PermissionPolicyExt for PermissionPolicy {
    fn with_permission_rules(self, config: &RuntimePermissionRuleConfig) -> Self {
        self.with_permission_rules_from_lists(
            config.allow().to_vec(),
            config.deny().to_vec(),
            config.ask().to_vec(),
        )
    }
}
