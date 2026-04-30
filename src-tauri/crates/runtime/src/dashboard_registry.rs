use crate::dashboard_plugin::{DashboardPlugin, PanelPosition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub panels: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardRegistryConfig {
    pub plugin_dirs: Vec<PathBuf>,
    pub auto_load: bool,
}

impl Default for DashboardRegistryConfig {
    fn default() -> Self {
        Self {
            plugin_dirs: vec![],
            auto_load: true,
        }
    }
}

pub struct DashboardRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn DashboardPlugin>>>,
}

impl Default for DashboardRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, plugin: Box<dyn DashboardPlugin>) -> Result<(), String> {
        let manifest = plugin.manifest().clone();
        let id = manifest.id.clone();
        if self.plugins.read().await.contains_key(&id) {
            return Err(format!("Plugin '{}' already registered", id));
        }
        plugin.on_load().await?;
        self.plugins.write().await.insert(id, Arc::from(plugin));
        tracing::info!("Plugin registered: {}", manifest.name);
        Ok(())
    }

    pub async fn unregister(&self, plugin_id: &str) -> Result<(), String> {
        let mut plugins = self.plugins.write().await;
        if let Some(plugin) = plugins.remove(plugin_id) {
            plugin.on_unload().await?;
            tracing::info!("Plugin unregistered: {}", plugin_id);
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    pub async fn get_plugin(&self, plugin_id: &str) -> Option<Arc<dyn DashboardPlugin>> {
        self.plugins.read().await.get(plugin_id).cloned()
    }

    pub async fn list_plugins(&self) -> Vec<DashboardPluginInfo> {
        self.plugins
            .read()
            .await
            .iter()
            .map(|(id, plugin)| {
                let manifest = plugin.manifest();
                DashboardPluginInfo {
                    id: id.clone(),
                    name: manifest.name.clone(),
                    version: manifest.version.clone(),
                    description: manifest.description.clone(),
                    author: manifest.author.clone(),
                    panels: manifest.panels.iter().map(|p| p.id.clone()).collect(),
                    enabled: true,
                }
            })
            .collect()
    }

    pub async fn list_panels(
        &self,
        position: Option<PanelPosition>,
    ) -> Vec<DashboardPanelWithPlugin> {
        let mut result = Vec::new();
        let plugins = self.plugins.read().await;
        for (plugin_id, plugin) in plugins.iter() {
            for panel in &plugin.manifest().panels {
                if let Some(pos) = position {
                    if panel.position != pos {
                        continue;
                    }
                }
                result.push(DashboardPanelWithPlugin {
                    plugin_id: plugin_id.clone(),
                    plugin_name: plugin.manifest().name.clone(),
                    panel: panel.clone(),
                });
            }
        }
        result
    }

    pub async fn render_panel(
        &self,
        plugin_id: &str,
        panel_id: &str,
        props: HashMap<String, serde_json::Value>,
    ) -> Result<String, String> {
        let plugins = self.plugins.read().await;
        if let Some(plugin) = plugins.get(plugin_id) {
            plugin.render_panel(panel_id, props).await
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    pub async fn enable(&self, plugin_id: &str) -> Result<(), String> {
        let plugins = self.plugins.read().await;
        if !plugins.contains_key(plugin_id) {
            return Err(format!("Plugin '{}' not found", plugin_id));
        }
        Ok(())
    }

    pub async fn disable(&self, plugin_id: &str) -> Result<(), String> {
        let plugins = self.plugins.read().await;
        if !plugins.contains_key(plugin_id) {
            return Err(format!("Plugin '{}' not found", plugin_id));
        }
        Ok(())
    }

    pub async fn reload(&self) -> Result<(), String> {
        tracing::info!("Reloading dashboard plugins");
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPanelWithPlugin {
    pub plugin_id: String,
    pub plugin_name: String,
    pub panel: crate::dashboard_plugin::DashboardPanel,
}
