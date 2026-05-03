//! Plugin Agent 提供者 — 插件可以注册自定义 agent 定义

use std::sync::RwLock;
use std::collections::HashMap;

/// Agent 定义（简化的，避免循环依赖）
#[derive(Debug, Clone)]
pub struct PluginAgentDef {
    pub agent_type: String,
    pub description: String,
    pub tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub model: Option<String>,
    pub background: bool,
    pub system_prompt: Option<String>,
}

/// Plugin Agent 注册表
pub struct PluginAgentRegistry {
    agents: RwLock<HashMap<String, PluginAgentDef>>,
}

impl PluginAgentRegistry {
    pub fn new() -> Self {
        Self { agents: RwLock::new(HashMap::new()) }
    }

    /// 插件注册一个 agent
    pub fn register(&self, def: PluginAgentDef) {
        self.agents.write().unwrap().insert(def.agent_type.clone(), def);
    }

    /// 插件注销一个 agent
    pub fn unregister(&self, agent_type: &str) {
        self.agents.write().unwrap().remove(agent_type);
    }

    /// 获取所有已注册的插件 agent
    pub fn all(&self) -> Vec<PluginAgentDef> {
        self.agents.read().unwrap().values().cloned().collect()
    }
}

impl Default for PluginAgentRegistry {
    fn default() -> Self { Self::new() }
}

/// 全局单例
static GLOBAL_PLUGIN_AGENTS: std::sync::LazyLock<PluginAgentRegistry> =
    std::sync::LazyLock::new(PluginAgentRegistry::default);

/// 获取全局 Plugin Agent 注册表
pub fn global_plugin_agents() -> &'static PluginAgentRegistry {
    &GLOBAL_PLUGIN_AGENTS
}
