use std::collections::HashMap;
// SAFETY: PluginAgentRegistry methods (register, unregister, all) are all
// synchronous. The RwLock is never held across .await points. Callers
// (register_plugin_agents, unregister_plugin_agents) are also sync.
use std::sync::RwLock;

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

    pub fn register(&self, def: PluginAgentDef) {
        match self.agents.write() {
            Ok(mut guard) => {
                guard.insert(def.agent_type.clone(), def);
            },
            Err(e) => {
                tracing::error!(
                    "PluginAgentRegistry: lock poisoned during register, recovering: {}",
                    e
                );
                let mut guard = e.into_inner();
                guard.insert(def.agent_type.clone(), def);
            },
        }
    }

    pub fn unregister(&self, agent_type: &str) {
        match self.agents.write() {
            Ok(mut guard) => {
                guard.remove(agent_type);
            },
            Err(e) => {
                tracing::error!(
                    "PluginAgentRegistry: lock poisoned during unregister, recovering: {}",
                    e
                );
                let mut guard = e.into_inner();
                guard.remove(agent_type);
            },
        }
    }

    pub fn all(&self) -> Vec<PluginAgentDef> {
        match self.agents.read() {
            Ok(guard) => guard.values().cloned().collect(),
            Err(e) => {
                tracing::error!("PluginAgentRegistry: lock poisoned during all, recovering: {}", e);
                e.into_inner().values().cloned().collect()
            },
        }
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

pub fn register_plugin_agents(plugin_id: &str, agents: &[PluginAgentDefInternal]) {
    let registry = global_plugin_agents();
    for agent in agents {
        registry.register(PluginAgentDef {
            agent_type: format!("{}/{}", plugin_id, agent.agent_type),
            description: agent.description.clone(),
            tools: agent.tools.clone(),
            disallowed_tools: agent.disallowed_tools.clone(),
            model: agent.model.clone(),
            background: agent.background,
            system_prompt: agent.system_prompt.clone(),
        });
    }
}

pub fn unregister_plugin_agents(plugin_id: &str) {
    let registry = global_plugin_agents();
    let prefix = format!("{}/", plugin_id);
    let to_remove: Vec<String> = registry
        .all()
        .into_iter()
        .filter(|a| a.agent_type.starts_with(&prefix))
        .map(|a| a.agent_type)
        .collect();
    for agent_type in to_remove {
        registry.unregister(&agent_type);
    }
}
