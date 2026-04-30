//! Unified module switch framework for AxAgent feature modules.
//!
//! Implements the `ModuleSwitch` trait and `ModuleLifecycle` state machine
//! so that every extension module can independently enable/disable, sleep/wake,
//! and report its resource cost. When disabled or sleeping, modules release
//! all CPU and memory resources.
//!
//! # Architecture
//!
//! Each module implements `ModuleSwitch` and is registered with a `ModuleRegistry`.
//! The registry tracks state and can bulk-enable/disable modules (e.g., for
//! "Speed Mode" vs "General Mode" switching).
//!
//! - ModuleRegistry <--> ModuleSwitch (enable/disable/sleep/wake)
//! - ModuleSwitch implementations: LSP Server, AST Index, RL Optimizer, etc.
//! - ModuleRegistry can enter Speed Mode (essential only) or General Mode (all)
//!

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Estimated resource cost for a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceCost {
    pub cpu_percent: u8,
    pub memory_mb: u64,
    pub disk_mb: u64,
}

impl ResourceCost {
    pub const ZERO: Self = Self { cpu_percent: 0, memory_mb: 0, disk_mb: 0 };
    pub const LOW: Self = Self { cpu_percent: 5, memory_mb: 10, disk_mb: 5 };
    pub const MEDIUM: Self = Self { cpu_percent: 20, memory_mb: 100, disk_mb: 50 };
    pub const HIGH: Self = Self { cpu_percent: 50, memory_mb: 500, disk_mb: 200 };
}

/// The lifecycle state of a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleState {
    /// Module is enabled and actively running.
    Active,
    /// Module is enabled but temporarily sleeping (low-power, no CPU).
    Sleeping,
    /// Module is fully disabled — no resources consumed, not discoverable.
    Disabled,
}

/// A module that can be enabled/disabled and report its resource cost.
///
/// Implementors should release all held resources (connections, caches, tasks)
/// when `disable()` is called, and re-acquire them on `enable()`.
#[async_trait::async_trait]
pub trait ModuleSwitch: Send + Sync {
    /// Unique identifier for this module.
    fn module_id(&self) -> &'static str;

    /// Human-readable name.
    fn module_name(&self) -> &'static str;

    /// Estimated resource cost when active.
    fn resource_cost(&self) -> ResourceCost;

    /// Enable the module (allocate resources, start background tasks).
    async fn enable(&self) -> Result<(), String>;

    /// Disable the module (release all resources).
    async fn disable(&self) -> Result<(), String>;

    /// Put the module to sleep (suspend background tasks, keep state).
    async fn sleep(&self) -> Result<(), String>;

    /// Wake the module from sleep.
    async fn wake(&self) -> Result<(), String>;

    /// Get current module state.
    fn state(&self) -> ModuleState;
}

/// A module entry in the registry.
struct ModuleEntry {
    module: Arc<dyn ModuleSwitch>,
    #[allow(dead_code)]
    state: ModuleState,
}

/// Registry tracking all registered modules and their states.
pub struct ModuleRegistry {
    entries: RwLock<Vec<ModuleEntry>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    /// Register a module with the registry.
    pub async fn register(&self, module: Arc<dyn ModuleSwitch>) {
        let mut entries = self.entries.write().await;
        let state = module.state();
        entries.push(ModuleEntry { module, state });
    }

    /// Enable a module by ID.
    pub async fn enable_module(&self, module_id: &str) -> Result<(), String> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.iter().find(|e| e.module.module_id() == module_id) {
            entry.module.enable().await
        } else {
            Err(format!("Module '{module_id}' not found"))
        }
    }

    /// Disable a module by ID.
    pub async fn disable_module(&self, module_id: &str) -> Result<(), String> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.iter().find(|e| e.module.module_id() == module_id) {
            entry.module.disable().await
        } else {
            Err(format!("Module '{module_id}' not found"))
        }
    }

    /// Put a module to sleep.
    pub async fn sleep_module(&self, module_id: &str) -> Result<(), String> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.iter().find(|e| e.module.module_id() == module_id) {
            entry.module.sleep().await
        } else {
            Err(format!("Module '{module_id}' not found"))
        }
    }

    /// Wake a sleeping module.
    pub async fn wake_module(&self, module_id: &str) -> Result<(), String> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.iter().find(|e| e.module.module_id() == module_id) {
            entry.module.wake().await
        } else {
            Err(format!("Module '{module_id}' not found"))
        }
    }

    /// Get the state of a module.
    pub async fn module_state(&self, module_id: &str) -> Option<ModuleState> {
        let entries = self.entries.read().await;
        entries.iter().find(|e| e.module.module_id() == module_id).map(|e| e.module.state())
    }

    /// List all registered modules.
    pub async fn list_modules(&self) -> Vec<ModuleInfo> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .map(|e| ModuleInfo {
                id: e.module.module_id().to_string(),
                name: e.module.module_name().to_string(),
                state: e.module.state(),
                resource_cost: e.module.resource_cost(),
            })
            .collect()
    }

    /// Calculate total resource cost of all active modules.
    pub async fn total_resource_cost(&self) -> ResourceCost {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.module.state() == ModuleState::Active)
            .fold(ResourceCost::ZERO, |acc, e| {
                let cost = e.module.resource_cost();
                ResourceCost {
                    cpu_percent: acc.cpu_percent.saturating_add(cost.cpu_percent),
                    memory_mb: acc.memory_mb.saturating_add(cost.memory_mb),
                    disk_mb: acc.disk_mb.saturating_add(cost.disk_mb),
                }
            })
    }

    /// Enable "Speed Mode": disable all non-essential modules, keep only code engine modules.
    ///
    /// `essential_ids` lists the module IDs to keep active. All others are disabled.
    pub async fn enter_speed_mode(&self, essential_ids: &[&str]) -> Result<usize, String> {
        let entries = self.entries.read().await;
        let mut disabled = 0;
        for entry in entries.iter() {
            if !essential_ids.contains(&entry.module.module_id()) {
                entry.module.disable().await?;
                disabled += 1;
            }
        }
        Ok(disabled)
    }

    /// Enable "General Mode": re-enable all registered modules.
    pub async fn enter_general_mode(&self) -> Result<usize, String> {
        let entries = self.entries.read().await;
        let mut enabled = 0;
        for entry in entries.iter() {
            if entry.module.state() == ModuleState::Disabled {
                entry.module.enable().await?;
                enabled += 1;
            }
        }
        Ok(enabled)
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a registered module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub name: String,
    pub state: ModuleState,
    pub resource_cost: ResourceCost,
}

// ── Simple toggle switch (non-async, for config fields) ──────────────────────

/// A simple boolean toggle that can be used as a module switch for
/// synchronous modules that don't need async lifecycle management.
#[derive(Debug, Clone)]
pub struct SimpleToggle {
    id: &'static str,
    name: &'static str,
    cost: ResourceCost,
    enabled: Arc<RwLock<bool>>,
}

impl SimpleToggle {
    pub fn new(id: &'static str, name: &'static str, cost: ResourceCost) -> Self {
        Self {
            id,
            name,
            cost,
            enabled: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    pub async fn set_enabled(&self, value: bool) {
        *self.enabled.write().await = value;
    }
}

#[async_trait::async_trait]
impl ModuleSwitch for SimpleToggle {
    fn module_id(&self) -> &'static str {
        self.id
    }

    fn module_name(&self) -> &'static str {
        self.name
    }

    fn resource_cost(&self) -> ResourceCost {
        self.cost
    }

    async fn enable(&self) -> Result<(), String> {
        *self.enabled.write().await = true;
        Ok(())
    }

    async fn disable(&self) -> Result<(), String> {
        *self.enabled.write().await = false;
        Ok(())
    }

    async fn sleep(&self) -> Result<(), String> {
        self.disable().await
    }

    async fn wake(&self) -> Result<(), String> {
        self.enable().await
    }

    fn state(&self) -> ModuleState {
        if self.enabled.try_read().is_ok_and(|e| *e) {
            ModuleState::Active
        } else {
            ModuleState::Disabled
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_register_and_list() {
        let registry = ModuleRegistry::new();
        let toggle = Arc::new(SimpleToggle::new("test_mod", "Test Module", ResourceCost::LOW));
        registry.register(toggle).await;

        let modules = registry.list_modules().await;
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].id, "test_mod");
    }

    #[tokio::test]
    async fn test_registry_enable_disable() {
        let registry = ModuleRegistry::new();
        let toggle = Arc::new(SimpleToggle::new("test_mod", "Test Module", ResourceCost::LOW));
        registry.register(toggle).await;

        assert_eq!(registry.module_state("test_mod").await, Some(ModuleState::Disabled));

        registry.enable_module("test_mod").await.unwrap();
        assert_eq!(registry.module_state("test_mod").await.unwrap(), ModuleState::Active);

        registry.disable_module("test_mod").await.unwrap();
        let state = registry.module_state("test_mod").await.unwrap();
        assert_eq!(state, ModuleState::Disabled);
    }

    #[tokio::test]
    async fn test_speed_mode() {
        let registry = ModuleRegistry::new();
        let code = Arc::new(SimpleToggle::new("code_engine", "Code Engine", ResourceCost::MEDIUM));
        let vision = Arc::new(SimpleToggle::new("screen_vision", "Screen Vision", ResourceCost::HIGH));
        let research = Arc::new(SimpleToggle::new("deep_research", "Deep Research", ResourceCost::LOW));
        registry.register(code.clone()).await;
        registry.register(vision.clone()).await;
        registry.register(research.clone()).await;

        code.enable().await.unwrap();
        vision.enable().await.unwrap();
        research.enable().await.unwrap();

        let disabled = registry.enter_speed_mode(&["code_engine"]).await.unwrap();
        assert_eq!(disabled, 2);

        assert_eq!(registry.module_state("code_engine").await, Some(ModuleState::Active));
        assert_eq!(registry.module_state("screen_vision").await, Some(ModuleState::Disabled));
    }

    #[tokio::test]
    async fn test_resource_cost_accumulation() {
        let registry = ModuleRegistry::new();
        let a = Arc::new(SimpleToggle::new("a", "A", ResourceCost { cpu_percent: 10, memory_mb: 50, disk_mb: 0 }));
        let b = Arc::new(SimpleToggle::new("b", "B", ResourceCost { cpu_percent: 20, memory_mb: 100, disk_mb: 0 }));
        registry.register(a.clone()).await;
        registry.register(b.clone()).await;

        a.enable().await.unwrap();
        b.enable().await.unwrap();

        let total = registry.total_resource_cost().await;
        assert_eq!(total.cpu_percent, 30);
        assert_eq!(total.memory_mb, 150);
    }
}
