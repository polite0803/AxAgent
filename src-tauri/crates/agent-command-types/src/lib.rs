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
