//! Dual-mode switching — one-click toggle between Speed Mode and General Mode.
//!
//! Integrates with the `ModuleSwitch` framework to provide two preset
//! configurations:
//!
//! - **Speed Mode** (极速专用模式): Only the code engine and essential modules
//!   are active. Strict token controls, aggressive output processing,
//!   code-optimized chunking, minimal prompt. All non-code modules (document
//!   parser, web search, screen vision, RL optimizer, deep research, message
//!   gateways) are disabled and fully released.
//!
//! - **General Mode** (全能通用模式): All modules are active. Full document
//!   processing, web search, message gateways, screen perception — the
//!   complete feature set with relaxed token constraints.
//!
//! # Usage
//!
//! ```ignore
//! let mode_selector = ModeSelector::new(module_registry);
//! mode_selector.enter_speed_mode().await?;
//! // ... code session ...
//! mode_selector.enter_general_mode().await?;
//! ```

use crate::module_switch::{ModuleRegistry, ModuleState, ResourceCost};
use serde::{Deserialize, Serialize};

/// The active mode of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveMode {
    Speed,
    General,
}

impl ActiveMode {
    pub fn as_str(&self) -> &str {
        match self {
            ActiveMode::Speed => "speed",
            ActiveMode::General => "general",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            ActiveMode::Speed => "极速专用模式",
            ActiveMode::General => "全能通用模式",
        }
    }
}

/// Modules considered essential and kept active in Speed Mode.
pub const SPEED_MODE_ESSENTIAL: &[&str] = &[
    "code_engine",
    "file_index",
    "ast_index",
    "recall_pipeline",
    "lsp_client",
    "git_tools",
    "compact",
    "session",
];

/// Modules disabled in Speed Mode (non-code overhead).
pub const SPEED_MODE_DISABLED: &[&str] = &[
    "document_parser",
    "screen_vision",
    "screen_capture",
    "web_search",
    "academic_search",
    "deep_research",
    "rl_optimizer",
    "lora_finetune",
    "message_gateway",
    "browser_automation",
    "ui_automation",
    "sandbox_runner",
];

/// Mode switching result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeSwitchResult {
    pub from: ActiveMode,
    pub to: ActiveMode,
    pub modules_enabled: usize,
    pub modules_disabled: usize,
    pub active_modules: Vec<String>,
    pub total_resource_cost: ResourceCost,
}

pub struct ModeSelector {
    registry: std::sync::Arc<ModuleRegistry>,
    current_mode: tokio::sync::RwLock<ActiveMode>,
}

impl ModeSelector {
    pub fn new(registry: std::sync::Arc<ModuleRegistry>) -> Self {
        Self {
            registry,
            current_mode: tokio::sync::RwLock::new(ActiveMode::General),
        }
    }

    /// Enter Speed Mode — disable all non-essential modules.
    pub async fn enter_speed_mode(&self) -> Result<ModeSwitchResult, String> {
        let registry = &*self.registry;

        // First enable essential modules
        for id in SPEED_MODE_ESSENTIAL {
            let _ = registry.enable_module(id).await;
        }

        // Then disable non-essential modules
        let mut disabled = 0;
        for id in SPEED_MODE_DISABLED {
            if registry.disable_module(id).await.is_ok() {
                disabled += 1;
            }
        }

        *self.current_mode.write().await = ActiveMode::Speed;

        let modules = registry.list_modules().await;
        let active: Vec<String> = modules
            .iter()
            .filter(|m| m.state == ModuleState::Active)
            .map(|m| m.name.clone())
            .collect();

        let cost = registry.total_resource_cost().await;
        let enabled = active.len();

        Ok(ModeSwitchResult {
            from: ActiveMode::General,
            to: ActiveMode::Speed,
            modules_enabled: enabled,
            modules_disabled: disabled,
            active_modules: active,
            total_resource_cost: cost,
        })
    }

    /// Enter General Mode — enable all registered modules.
    pub async fn enter_general_mode(&self) -> Result<ModeSwitchResult, String> {
        let registry = &*self.registry;

        let modules = registry.list_modules().await;
        let mut enabled = 0;

        for m in &modules {
            if registry.enable_module(&m.id).await.is_ok() {
                enabled += 1;
            }
        }

        *self.current_mode.write().await = ActiveMode::General;

        let modules_after = registry.list_modules().await;
        let active: Vec<String> = modules_after
            .iter()
            .filter(|m| m.state == ModuleState::Active)
            .map(|m| m.name.clone())
            .collect();

        let cost = registry.total_resource_cost().await;

        Ok(ModeSwitchResult {
            from: ActiveMode::Speed,
            to: ActiveMode::General,
            modules_enabled: enabled,
            modules_disabled: 0,
            active_modules: active,
            total_resource_cost: cost,
        })
    }

    /// Toggle between Speed and General modes.
    pub async fn toggle_mode(&self) -> Result<ModeSwitchResult, String> {
        let current = *self.current_mode.read().await;
        match current {
            ActiveMode::Speed => self.enter_general_mode().await,
            ActiveMode::General => self.enter_speed_mode().await,
        }
    }

    /// Get the current active mode.
    pub async fn current_mode(&self) -> ActiveMode {
        *self.current_mode.read().await
    }

    /// Get a human-readable mode description.
    pub async fn mode_description(&self) -> String {
        let mode = self.current_mode().await;
        match mode {
            ActiveMode::Speed => format!(
                "{} — 仅代码引擎 + 严格Token管控 + 非必要模块休眠",
                mode.display_name()
            ),
            ActiveMode::General => {
                format!("{} — 全功能开启，保留完整通用能力", mode.display_name())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_switch::SimpleToggle;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_mode_switching() {
        let registry = Arc::new(ModuleRegistry::new());

        // Register test modules
        let code = Arc::new(SimpleToggle::new(
            "code_engine",
            "Code Engine",
            ResourceCost::MEDIUM,
        ));
        let doc = Arc::new(SimpleToggle::new(
            "document_parser",
            "Document Parser",
            ResourceCost::LOW,
        ));
        let vision = Arc::new(SimpleToggle::new(
            "screen_vision",
            "Screen Vision",
            ResourceCost::HIGH,
        ));
        let deep = Arc::new(SimpleToggle::new(
            "deep_research",
            "Deep Research",
            ResourceCost::LOW,
        ));

        registry.register(code).await;
        registry.register(doc).await;
        registry.register(vision).await;
        registry.register(deep.clone()).await;

        let selector = ModeSelector::new(registry.clone());

        // Start in General mode - all enabled
        let result = selector.enter_general_mode().await.unwrap();
        assert_eq!(result.to, ActiveMode::General);

        // Switch to Speed mode
        let result = selector.enter_speed_mode().await.unwrap();
        assert_eq!(result.to, ActiveMode::Speed);

        // Toggle back
        let result = selector.toggle_mode().await.unwrap();
        assert_eq!(result.to, ActiveMode::General);

        assert_eq!(selector.current_mode().await, ActiveMode::General);
    }
}
