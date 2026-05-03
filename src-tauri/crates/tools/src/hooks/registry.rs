//! Hook 注册表

use super::{HookConfig, HookEventType};
use std::collections::HashMap;

/// Hook 注册表
///
/// 管理所有已注册 Hook 的生命周期。
pub struct HookRegistry {
    hooks: Vec<HookConfig>,
    /// 事件类型 -> Hook 列表索引
    event_index: HashMap<HookEventType, Vec<usize>>,
    /// 工具模式 -> Hook 列表索引（缓存匹配）
    pattern_cache: HashMap<String, Vec<usize>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: Vec::new(),
            event_index: HashMap::new(),
            pattern_cache: HashMap::new(),
        }
    }

    /// 注册 Hook
    pub fn register(&mut self, hook: HookConfig) {
        let idx = self.hooks.len();
        self.event_index
            .entry(hook.event.clone())
            .or_default()
            .push(idx);
        self.hooks.push(hook);
        self.pattern_cache.clear(); // 缓存失效
    }

    /// 获取匹配事件的 Hook（按优先级排序）
    pub fn get_matching(&self, event: &HookEventType, tool_name: &str) -> Vec<&HookConfig> {
        let mut result = Vec::new();

        if let Some(indices) = self.event_index.get(event) {
            for &idx in indices {
                let hook = &self.hooks[idx];
                if !hook.enabled {
                    continue;
                }
                if match_tool_pattern(&hook.tool_pattern, tool_name) {
                    result.push(hook);
                }
            }
        }

        // 按优先级排序
        result.sort_by_key(|h| h.priority);
        result
    }

    /// 注销 Hook
    pub fn unregister(&mut self, hook_id: &str) -> bool {
        if let Some(pos) = self.hooks.iter().position(|h| h.id == hook_id) {
            self.hooks.remove(pos);
            // 重建索引
            self.rebuild_index();
            self.pattern_cache.clear();
            true
        } else {
            false
        }
    }

    /// 启用/禁用 Hook
    pub fn set_enabled(&mut self, hook_id: &str, enabled: bool) -> bool {
        if let Some(hook) = self.hooks.iter_mut().find(|h| h.id == hook_id) {
            hook.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// 获取所有 Hook
    pub fn all(&self) -> &[HookConfig] {
        &self.hooks
    }

    /// 按事件类型筛选
    pub fn by_event(&self, event: &HookEventType) -> Vec<&HookConfig> {
        self.event_index
            .get(event)
            .map(|indices| indices.iter().map(|&i| &self.hooks[i]).collect())
            .unwrap_or_default()
    }

    fn rebuild_index(&mut self) {
        self.event_index.clear();
        for (i, hook) in self.hooks.iter().enumerate() {
            self.event_index
                .entry(hook.event.clone())
                .or_default()
                .push(i);
        }
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 匹配工具名与 Hook 模式
fn match_tool_pattern(pattern: &str, tool_name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return tool_name.starts_with(prefix);
    }
    pattern == tool_name
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::{HookConfig, HookEventType, HookExecutor, ShellHookExec};

    fn make_hook(id: &str, tool_pattern: &str) -> HookConfig {
        HookConfig {
            id: id.into(),
            event: HookEventType::PreToolUse,
            tool_pattern: tool_pattern.into(),
            executor: HookExecutor::Shell(ShellHookExec {
                command: "echo".into(),
                args: vec!["hook".into()],
                working_dir: None,
            }),
            enabled: true,
            timeout_secs: 10,
            priority: 0,
        }
    }

    #[test]
    fn test_exact_match() {
        let mut reg = HookRegistry::new();
        reg.register(make_hook("h1", "FileRead"));
        reg.register(make_hook("h2", "FileWrite"));

        let matched = reg.get_matching(&HookEventType::PreToolUse, "FileRead");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].id, "h1");
    }

    #[test]
    fn test_wildcard_match() {
        let mut reg = HookRegistry::new();
        reg.register(make_hook("h1", "File*"));

        assert_eq!(
            reg.get_matching(&HookEventType::PreToolUse, "FileRead")
                .len(),
            1
        );
        assert_eq!(
            reg.get_matching(&HookEventType::PreToolUse, "FileWrite")
                .len(),
            1
        );
        assert_eq!(
            reg.get_matching(&HookEventType::PreToolUse, "Bash").len(),
            0
        );
    }

    #[test]
    fn test_disabled_hook() {
        let mut reg = HookRegistry::new();
        let mut hook = make_hook("h1", "FileRead");
        hook.enabled = false;
        reg.register(hook);

        assert_eq!(
            reg.get_matching(&HookEventType::PreToolUse, "FileRead")
                .len(),
            0
        );
    }
}
