// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::PluginAgentDefInternal;

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

pub struct PluginAgentRegistry {
    agents: RwLock<HashMap<String, PluginAgentDef>>,
}

impl PluginAgentRegistry {
    pub fn new() -> Self {
        Self { agents: RwLock::new(HashMap::new()) }
    }

    pub async fn register(&self, def: PluginAgentDef) {
        let mut guard = self.agents.write().await;
        guard.insert(def.agent_type.clone(), def);
    }

    pub async fn unregister(&self, agent_type: &str) {
        let mut guard = self.agents.write().await;
        guard.remove(agent_type);
    }

    pub async fn all(&self) -> Vec<PluginAgentDef> {
        let guard = self.agents.read().await;
        guard.values().cloned().collect()
    }
}

impl Default for PluginAgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(test, allow(dead_code))]
static GLOBAL_PLUGIN_AGENTS: std::sync::LazyLock<PluginAgentRegistry> =
    std::sync::LazyLock::new(PluginAgentRegistry::default);

#[cfg(not(test))]
pub fn global_plugin_agents() -> &'static PluginAgentRegistry {
    &GLOBAL_PLUGIN_AGENTS
}

#[cfg(test)]
static TEST_PLUGIN_AGENTS: std::sync::Mutex<Option<&'static PluginAgentRegistry>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
pub fn global_plugin_agents() -> &'static PluginAgentRegistry {
    let guard = TEST_PLUGIN_AGENTS.lock().unwrap_or_else(|e| e.into_inner());
    match *guard {
        // 测试显式注册时，把 registry leak 成 'static 后存入 Mutex，
        // 引用生命周期独立于锁的临时 guard，可安全返回。
        Some(r) => r,
        // 未注册场景复用全局 LazyLock(static lifetime)。
        None => &GLOBAL_PLUGIN_AGENTS,
    }
}

#[cfg(test)]
pub fn set_test_plugin_agents(registry: PluginAgentRegistry) {
    // 把所有权转移给堆，再 leak 为 'static 引用，存入 Mutex。
    // 该进程的所有测试结束后会随进程退出而释放。
    *TEST_PLUGIN_AGENTS.lock().unwrap_or_else(|e| e.into_inner()) =
        Some(Box::leak(Box::new(registry)));
}

#[cfg(test)]
pub fn reset_test_plugin_agents() {
    *TEST_PLUGIN_AGENTS.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

pub async fn register_plugin_agents(plugin_id: &str, agents: &[PluginAgentDefInternal]) {
    let registry = global_plugin_agents();
    for agent in agents {
        registry
            .register(PluginAgentDef {
                agent_type: format!("{}/{}", plugin_id, agent.agent_type),
                description: agent.description.clone(),
                tools: agent.tools.clone(),
                disallowed_tools: agent.disallowed_tools.clone(),
                model: agent.model.clone(),
                background: agent.background,
                system_prompt: agent.system_prompt.clone(),
            })
            .await;
    }
}

pub async fn unregister_plugin_agents(plugin_id: &str) {
    let registry = global_plugin_agents();
    let prefix = format!("{}/", plugin_id);
    let to_remove: Vec<String> = registry
        .all()
        .await
        .into_iter()
        .filter(|a| a.agent_type.starts_with(&prefix))
        .map(|a| a.agent_type)
        .collect();
    for agent_type in to_remove {
        registry.unregister(&agent_type).await;
    }
}

// ── 同步版本（供非 async 上下文使用） ──

pub fn register_plugin_agents_sync(plugin_id: &str, agents: &[PluginAgentDefInternal]) {
    let registry = global_plugin_agents();
    for agent in agents {
        registry.agents.blocking_write().insert(
            format!("{}/{}", plugin_id, agent.agent_type),
            PluginAgentDef {
                agent_type: format!("{}/{}", plugin_id, agent.agent_type),
                description: agent.description.clone(),
                tools: agent.tools.clone(),
                disallowed_tools: agent.disallowed_tools.clone(),
                model: agent.model.clone(),
                background: agent.background,
                system_prompt: agent.system_prompt.clone(),
            },
        );
    }
}

pub fn unregister_plugin_agents_sync(plugin_id: &str) {
    let registry = global_plugin_agents();
    let prefix = format!("{}/", plugin_id);
    let mut agents = registry.agents.blocking_write();
    let to_remove: Vec<String> =
        agents.keys().filter(|k| k.starts_with(&prefix)).cloned().collect();
    for agent_type in to_remove {
        agents.remove(&agent_type);
    }
}

// ── `axagent_harness::PluginAgentProvider` trait impl ──
//
// 把 `global_plugin_agents()` 暴露成 `axagent_harness::PluginAgentProvider`，
// 让 `tools` crate 不用直接 import `axagent_plugins`，而是持有
// `Arc<dyn axagent_harness::PluginAgentProvider>`，由 wiring 层注入。

pub struct GlobalPluginAgentProvider;

impl axagent_harness::PluginAgentProvider for GlobalPluginAgentProvider {
    fn all(&self) -> Vec<axagent_harness::PluginAgentDescriptor> {
        // 使用blocking_read在同步上下文中访问异步锁
        let registry = global_plugin_agents();
        let agents = registry.agents.blocking_read();
        agents
            .values()
            .map(|a| axagent_harness::PluginAgentDescriptor {
                agent_type: a.agent_type.clone(),
                description: a.description.clone(),
                tools: a.tools.clone(),
                disallowed_tools: a.disallowed_tools.clone(),
                model: a.model.clone(),
                background: a.background,
                system_prompt: a.system_prompt.clone(),
                source: "plugin".to_string(),
            })
            .collect()
    }
}
