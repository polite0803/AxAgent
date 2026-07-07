// SPDX-License-Identifier: AGPL-3.0-only

//! 工具组管理器
//!
//! 从 `UnifiedToolRegistry` 中拆分，独立管理工具组启用/禁用状态和显示名称。

use crate::ToolInfo;
use crate::registry::ToolGroupInfo;
use std::collections::{HashMap, HashSet};

/// 工具组管理器 —— 独立管理工具组启用状态、禁用列表和显示名称
#[derive(Debug, Clone, Default)]
pub struct ToolGroupManager {
    /// 工具组启用状态（从 DB 加载）
    pub group_enabled: HashMap<String, bool>,
    /// 单个工具禁用列表（从 DB 加载，空 = 全部启用）
    pub disabled_tools: HashSet<String>,
    /// 工具组显示名称
    pub group_names: HashMap<String, String>,
}

impl ToolGroupManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查工具是否可用（组启用 + 未在禁用列表中）
    pub fn is_tool_enabled(&self, info: &ToolInfo) -> bool {
        if self.disabled_tools.contains(&info.name) {
            return false;
        }
        let gid = info.category.default_group();
        self.group_enabled.get(gid).copied().unwrap_or(true)
    }

    /// 检查工具是否在禁用列表中
    pub fn is_name_disabled(&self, name: &str) -> bool {
        self.disabled_tools.contains(name)
    }

    /// 获取工具组摘要列表
    pub fn get_tool_groups(&self, tools: &[ToolInfo]) -> Vec<ToolGroupInfo> {
        let mut groups: HashMap<String, ToolGroupInfo> = HashMap::new();
        for t in tools {
            let group_key = t.category.default_group().to_string();
            let entry = groups.entry(group_key.clone()).or_insert_with(|| {
                let enabled = self.group_enabled.get(&group_key).copied().unwrap_or(true);
                let name =
                    self.group_names.get(&group_key).cloned().unwrap_or_else(|| group_key.clone());
                ToolGroupInfo { group_id: group_key, group_name: name, enabled, tools: Vec::new() }
            });
            entry.tools.push(t.clone());
        }
        groups.into_values().collect()
    }

    /// 获取所有启用的工具名
    pub fn enabled_tool_names(&self, tools: &[ToolInfo]) -> Vec<String> {
        tools.iter().filter(|t| self.is_tool_enabled(t)).map(|t| t.name.clone()).collect()
    }
}
