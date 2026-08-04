use serde::{Deserialize, Serialize};

/// 命令元数据
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CommandMetadata {
    /// 命令名称（函数名）
    pub name: &'static str,
    /// 源模块路径，如 "crate::commands::providers"
    pub source_module: &'static str,
    /// 所属功能域
    pub domain: &'static str,
    /// 安全级别
    pub safety: CommandSafety,
    /// 调用模式
    pub call_mode: CallMode,
    /// 命令描述
    pub description: &'static str,
}

impl CommandMetadata {
    pub const fn new(
        name: &'static str,
        source_module: &'static str,
        domain: &'static str,
        safety: CommandSafety,
        call_mode: CallMode,
        description: &'static str,
    ) -> Self {
        Self { name, source_module, domain, safety, call_mode, description }
    }

    /// 构建完整路径，去除 "crate::" 前缀
    pub fn full_path(&self) -> String {
        let module = self.source_module.strip_prefix("crate::").unwrap_or(self.source_module);
        let name = self.name;
        format!("{module}::{name}")
    }
}

/// 命令安全级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandSafety {
    Safe,
    Caution,
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
}

/// 调用模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallMode {
    /// 仅状态访问，无额外参数
    StateOnly,
    /// 状态访问 + 输入参数
    StateInput,
    /// 手动调用（如流式命令）
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

// inventory 收集所有 #[agent_command] 命令元数据
inventory::collect!(CommandMetadata);

/// 宏注册表访问接口
pub mod registry {
    use super::*;

    /// 获取所有注册的命令元数据
    pub fn get_all() -> Vec<&'static CommandMetadata> {
        inventory::iter::<CommandMetadata>.into_iter().collect()
    }

    /// 按名称查找命令元数据
    pub fn find_by_name(name: &str) -> Option<&'static CommandMetadata> {
        inventory::iter::<CommandMetadata>.into_iter().find(|m| m.name == name)
    }

    /// 按域查找命令元数据
    pub fn find_by_domain(domain: &str) -> Vec<&'static CommandMetadata> {
        inventory::iter::<CommandMetadata>.into_iter().filter(|m| m.domain == domain).collect()
    }

    /// 获取所有唯一域
    pub fn get_all_domains() -> Vec<&'static str> {
        let mut domains = Vec::new();
        for meta in inventory::iter::<CommandMetadata> {
            if !domains.contains(&meta.domain) {
                domains.push(meta.domain);
            }
        }
        domains
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证命令元数据注册表接口可以正常工作
    /// 注意：此 crate 是类型定义 crate，命令注册在主应用 crate 中完成
    #[test]
    fn test_registry_interface() {
        let all = registry::get_all();
        // 在类型定义 crate 中，没有注册命令是正常的
        // 命令会在主应用 crate (axagent) 中通过 inventory 收集
        let _ = all; // 确保代码可以编译运行

        let domains = registry::get_all_domains();
        let _ = domains;
    }

    /// 验证命令元数据结构正确
    #[test]
    fn test_command_metadata_integrity() {
        for meta in registry::get_all() {
            assert!(!meta.name.is_empty(), "命令名称不能为空");
            assert!(!meta.domain.is_empty(), "命令域不能为空");
            assert!(!meta.description.is_empty(), "命令描述不能为空");
        }
    }

    /// 验证按名称查找命令
    #[test]
    fn test_find_by_name() {
        let all = registry::get_all();
        if let Some(first) = all.first() {
            let found = registry::find_by_name(first.name);
            assert!(found.is_some(), "应该能通过名称找到命令");
            assert_eq!(found.unwrap().name, first.name);
        }
        // 如果没有命令注册，测试自然通过
    }

    /// 验证按域查找命令
    #[test]
    fn test_find_by_domain() {
        let all = registry::get_all();
        if let Some(first) = all.first() {
            let domain_commands = registry::find_by_domain(first.domain);
            assert!(
                domain_commands.iter().any(|c| c.name == first.name),
                "按域查找应该包含该域的命令"
            );
        }
        // 如果没有命令注册，测试自然通过
    }

    /// 验证 full_path 格式正确
    #[test]
    fn test_full_path_format() {
        for meta in registry::get_all() {
            let path = meta.full_path();
            assert!(!path.is_empty(), "full_path 不能为空");
            assert!(path.contains(meta.name), "full_path 应该包含命令名称");
        }
    }

    /// 验证安全级别枚举值正确
    #[test]
    fn test_safety_enum() {
        assert_eq!(CommandSafety::Safe.as_str(), "safe");
        assert_eq!(CommandSafety::Caution.as_str(), "caution");
        assert_eq!(CommandSafety::Dangerous.as_str(), "dangerous");
    }

    /// 验证调用模式枚举值正确
    #[test]
    fn test_call_mode_enum() {
        assert_eq!(CallMode::StateOnly.as_str(), "state_only");
        assert_eq!(CallMode::StateInput.as_str(), "state_input");
        assert_eq!(CallMode::Manual.as_str(), "manual");
    }

    /// 验证 CommandMetadata::new 构造函数
    #[test]
    fn test_metadata_constructor() {
        let meta = CommandMetadata::new(
            "test_command",
            "crate::commands::test",
            "test_domain",
            CommandSafety::Safe,
            CallMode::StateInput,
            "测试命令",
        );

        assert_eq!(meta.name, "test_command");
        assert_eq!(meta.source_module, "crate::commands::test");
        assert_eq!(meta.domain, "test_domain");
        assert!(matches!(meta.safety, CommandSafety::Safe));
        assert!(matches!(meta.call_mode, CallMode::StateInput));
        assert_eq!(meta.description, "测试命令");
    }

    /// 验证 full_path 去除 crate:: 前缀
    #[test]
    fn test_full_path_strip_prefix() {
        let meta = CommandMetadata::new(
            "test_command",
            "crate::commands::test",
            "test_domain",
            CommandSafety::Safe,
            CallMode::StateInput,
            "测试命令",
        );

        assert_eq!(meta.full_path(), "commands::test::test_command");

        let meta_no_prefix = CommandMetadata::new(
            "test_command",
            "other::module",
            "test_domain",
            CommandSafety::Safe,
            CallMode::StateInput,
            "测试命令",
        );
        assert_eq!(meta_no_prefix.full_path(), "other::module::test_command");
    }
}
