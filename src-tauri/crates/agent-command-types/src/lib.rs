// SPDX-License-Identifier: AGPL-3.0-only

//! 命令类型定义
//!
//! 定义 agent_command 宏使用的元数据类型和命令注册表。

use serde::{Deserialize, Serialize};

// 使用 inventory 收集所有命令元数据
inventory::collect!(CommandMetadata);

/// 命令元数据
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CommandMetadata {
    /// 命令名称（函数名）
    pub name: &'static str,
    /// 所属功能域
    pub domain: &'static str,
    /// 安全级别
    pub safety: CommandSafety,
    /// 调用模式
    pub call_mode: CallMode,
    /// 命令描述
    pub description: &'static str,
}

// 允许 inventory 存储 CommandMetadata
impl CommandMetadata {
    pub const fn new(
        name: &'static str,
        domain: &'static str,
        safety: CommandSafety,
        call_mode: CallMode,
        description: &'static str,
    ) -> Self {
        Self {
            name,
            domain,
            safety,
            call_mode,
            description,
        }
    }
}

/// 命令安全级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandSafety {
    /// 只读查询，Agent 可直接调用
    Safe,
    /// 写入操作，需用户确认
    Caution,
    /// 危险操作，需显式授权
    Dangerous,
}

impl CommandSafety {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommandSafety::Safe => "safe",
            CommandSafety::Caution => "caution",
            CommandSafety::Dangerous => "dangerous",
        }
    }

    pub fn severity(&self) -> u8 {
        match self {
            CommandSafety::Safe => 0,
            CommandSafety::Caution => 1,
            CommandSafety::Dangerous => 2,
        }
    }

    pub fn requires_confirmation(&self) -> bool {
        matches!(self, CommandSafety::Caution)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, CommandSafety::Dangerous)
    }
}

/// 命令调用模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallMode {
    /// 只有 state 参数（只读查询）
    StateOnly,
    /// 有 state + input 参数（写入操作）
    StateInput,
    /// 复杂命令，需要手动实现 Handler
    Manual,
}

impl CallMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CallMode::StateOnly => "state_only",
            CallMode::StateInput => "state_input",
            CallMode::Manual => "manual",
        }
    }
}

/// 命令注册表
///
/// 使用 inventory 收集所有带有 #[agent_command] 宏的命令元数据。
/// 在程序启动时调用 `register_all_commands()` 来收集命令。
pub mod registry {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    /// 命令注册表单例
    static COMMAND_REGISTRY: RwLock<Option<HashMap<&'static str, CommandMetadata>>> = RwLock::new(None);

    /// 注册所有命令元数据
    pub fn init() {
        let mut registry = HashMap::new();

        // 从 inventory 收集所有命令元数据
        for meta in inventory::iter::<CommandMetadata> {
            registry.insert(meta.name, *meta);
        }

        let mut lock = COMMAND_REGISTRY.write().expect("获取写锁失败");
        *lock = Some(registry);
    }

    /// 获取所有命令元数据
    pub fn get_all() -> Vec<CommandMetadata> {
        let lock = COMMAND_REGISTRY.read().expect("获取读锁失败");
        match lock.as_ref() {
            Some(registry) => registry.values().copied().collect(),
            None => {
                // 如果未初始化，返回空列表
                Vec::new()
            }
        }
    }

    /// 按名称查找命令元数据
    pub fn get_by_name(name: &str) -> Option<CommandMetadata> {
        let lock = COMMAND_REGISTRY.read().expect("获取读锁失败");
        lock.as_ref().and_then(|r| r.get(name).copied())
    }

    /// 按域筛选命令
    pub fn get_by_domain(domain: &str) -> Vec<CommandMetadata> {
        let lock = COMMAND_REGISTRY.read().expect("获取读锁失败");
        match lock.as_ref() {
            Some(registry) => {
                registry.values().filter(|m| m.domain == domain).copied().collect()
            }
            None => Vec::new(),
        }
    }

    /// 检查命令是否存在
    pub fn contains(name: &str) -> bool {
        let lock = COMMAND_REGISTRY.read().expect("获取读锁失败");
        lock.as_ref().map(|r| r.contains_key(name)).unwrap_or(false)
    }

    /// 获取命令数量
    pub fn count() -> usize {
        let lock = COMMAND_REGISTRY.read().expect("获取读锁失败");
        lock.as_ref().map(|r| r.len()).unwrap_or(0)
    }

    /// 检查注册表是否已初始化
    pub fn is_initialized() -> bool {
        let lock = COMMAND_REGISTRY.read().expect("获取读锁失败");
        lock.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_metadata_new() {
        let meta = CommandMetadata::new(
            "test_command",
            "test_domain",
            CommandSafety::Safe,
            CallMode::StateOnly,
            "测试命令",
        );

        assert_eq!(meta.name, "test_command");
        assert_eq!(meta.domain, "test_domain");
        assert_eq!(meta.safety, CommandSafety::Safe);
        assert_eq!(meta.call_mode, CallMode::StateOnly);
        assert_eq!(meta.description, "测试命令");
    }

    #[test]
    fn test_command_safety_methods() {
        assert_eq!(CommandSafety::Safe.as_str(), "safe");
        assert_eq!(CommandSafety::Caution.as_str(), "caution");
        assert_eq!(CommandSafety::Dangerous.as_str(), "dangerous");

        assert_eq!(CommandSafety::Safe.severity(), 0);
        assert_eq!(CommandSafety::Caution.severity(), 1);
        assert_eq!(CommandSafety::Dangerous.severity(), 2);

        assert!(!CommandSafety::Safe.requires_confirmation());
        assert!(CommandSafety::Caution.requires_confirmation());
        assert!(!CommandSafety::Dangerous.requires_confirmation());

        assert!(!CommandSafety::Safe.is_blocked());
        assert!(!CommandSafety::Caution.is_blocked());
        assert!(CommandSafety::Dangerous.is_blocked());
    }

    #[test]
    fn test_call_mode_as_str() {
        assert_eq!(CallMode::StateOnly.as_str(), "state_only");
        assert_eq!(CallMode::StateInput.as_str(), "state_input");
        assert_eq!(CallMode::Manual.as_str(), "manual");
    }

    #[test]
    fn test_registry_not_initialized() {
        // 注意：由于 COMMAND_REGISTRY 是全局静态的，多个测试可能共享状态
        // 这里我们只验证 get_all() 方法可用
        let _all = registry::get_all();
        // 验证 get_by_name 对不存在的命令返回 None
        assert!(registry::get_by_name("nonexistent_command_xyz").is_none());
    }

    #[test]
    fn test_registry_init_with_no_commands() {
        // 即使没有命令注册，init() 也应该初始化注册表
        registry::init();
        assert!(registry::is_initialized());
    }

    #[test]
    fn test_registry_get_by_name_not_found() {
        registry::init();
        assert!(registry::get_by_name("nonexistent_command").is_none());
    }

    #[test]
    fn test_registry_contains() {
        registry::init();
        assert!(!registry::contains("nonexistent_command"));
    }
}
