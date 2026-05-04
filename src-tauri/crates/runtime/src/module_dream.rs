//! DreamConsolidator 的 ModuleSwitch 适配器。
//!
//! 将 DreamConsolidator 包装为可被 ModuleRegistry 管理的模块，
//! 支持 enable/disable/sleep/wake 生命周期。

use crate::module_switch::{ModuleState, ModuleSwitch, ResourceCost};
use axagent_trajectory::{DreamConsolidationConfig, DreamConsolidator};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DreamModule {
    consolidator: Arc<DreamConsolidator>,
    name: &'static str,
    state: Arc<Mutex<ModuleState>>,
    active_config: DreamConsolidationConfig,
}

impl DreamModule {
    pub fn new(
        consolidator: Arc<DreamConsolidator>,
        name: &'static str,
    ) -> Self {
        let default_config = DreamConsolidationConfig::default();
        Self {
            consolidator,
            name,
            state: Arc::new(Mutex::new(ModuleState::Active)),
            active_config: default_config,
        }
    }
}

#[async_trait::async_trait]
impl ModuleSwitch for DreamModule {
    fn module_id(&self) -> &'static str {
        "dream_consolidation"
    }

    fn module_name(&self) -> &'static str {
        self.name
    }

    fn resource_cost(&self) -> ResourceCost {
        ResourceCost::LOW
    }

    async fn enable(&self) -> Result<(), String> {
        let mut cfg = self.active_config.clone();
        cfg.enabled = true;
        self.consolidator.update_config(cfg).await;
        *self.state.lock().await = ModuleState::Active;
        tracing::info!("[ModuleSwitch] Dream 巩固已启用");
        Ok(())
    }

    async fn disable(&self) -> Result<(), String> {
        let mut cfg = self.active_config.clone();
        cfg.enabled = false;
        self.consolidator.update_config(cfg).await;
        *self.state.lock().await = ModuleState::Disabled;
        tracing::info!("[ModuleSwitch] Dream 巩固已禁用");
        Ok(())
    }

    async fn sleep(&self) -> Result<(), String> {
        let mut cfg = self.active_config.clone();
        cfg.enabled = false;
        self.consolidator.update_config(cfg).await;
        *self.state.lock().await = ModuleState::Sleeping;
        tracing::info!("[ModuleSwitch] Dream 巩固已休眠");
        Ok(())
    }

    async fn wake(&self) -> Result<(), String> {
        let mut cfg = self.active_config.clone();
        cfg.enabled = true;
        self.consolidator.update_config(cfg).await;
        *self.state.lock().await = ModuleState::Active;
        tracing::info!("[ModuleSwitch] Dream 巩固已唤醒");
        Ok(())
    }

    fn state(&self) -> ModuleState {
        self.state.try_lock().map(|s| *s).unwrap_or(ModuleState::Active)
    }
}
