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
        Self {
            agents: RwLock::new(HashMap::new()),
        }
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
thread_local! {
    static TEST_PLUGIN_AGENTS: std::cell::RefCell<Option<PluginAgentRegistry>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub fn global_plugin_agents() -> &'static PluginAgentRegistry {
    static FALLBACK: std::sync::LazyLock<PluginAgentRegistry> =
        std::sync::LazyLock::new(PluginAgentRegistry::default);
    TEST_PLUGIN_AGENTS.with(|cell| {
        let ptr = cell.as_ptr();
        unsafe {
            match &*ptr {
                Some(_) => &*(*ptr).as_ref().unwrap(),
                None => &*FALLBACK,
            }
        }
    })
}

#[cfg(test)]
pub fn set_test_plugin_agents(registry: PluginAgentRegistry) {
    TEST_PLUGIN_AGENTS.with(|cell| {
        *cell.borrow_mut() = Some(registry);
    });
}

#[cfg(test)]
pub fn reset_test_plugin_agents() {
    TEST_PLUGIN_AGENTS.with(|cell| {
        *cell.borrow_mut() = None;
    });
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
    let to_remove: Vec<String> = agents
        .keys()
        .filter(|k| k.starts_with(&prefix))
        .cloned()
        .collect();
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
