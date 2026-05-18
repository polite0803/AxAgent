use crate::dashboard_plugin::{
    DashboardPlugin, DashboardPluginAdapter, DashboardPluginManifest, PanelPosition, RenderOutput,
};
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

struct PluginEntry {
    plugin: Arc<dyn DashboardPlugin>,
    enabled: bool,
}

pub struct DashboardRegistry {
    entries: RwLock<HashMap<String, PluginEntry>>,
    config: DashboardRegistryConfig,
}

impl Default for DashboardRegistry {
    fn default() -> Self {
        Self::new_with_config(DashboardRegistryConfig::default())
    }
}

impl DashboardRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_config(config: DashboardRegistryConfig) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            config,
        }
    }

    pub fn config(&self) -> &DashboardRegistryConfig {
        &self.config
    }

    pub async fn register(&self, plugin: Box<dyn DashboardPlugin>) -> Result<(), String> {
        let manifest = plugin.manifest().clone();
        let id = manifest.id.clone();
        if self.entries.read().await.contains_key(&id) {
            return Err(format!("Plugin '{}' already registered", id));
        }
        plugin.on_load().await?;
        self.entries.write().await.insert(
            id,
            PluginEntry {
                plugin: Arc::from(plugin),
                enabled: true,
            },
        );
        tracing::info!("Plugin registered: {}", manifest.name);
        Ok(())
    }

    pub async fn unregister(&self, plugin_id: &str) -> Result<(), String> {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.remove(plugin_id) {
            entry.plugin.on_unload().await?;
            tracing::info!("Plugin unregistered: {}", plugin_id);
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    pub async fn get_plugin(&self, plugin_id: &str) -> Option<Arc<dyn DashboardPlugin>> {
        self.entries
            .read()
            .await
            .get(plugin_id)
            .map(|e| e.plugin.clone())
    }

    pub async fn list_plugins(&self) -> Vec<DashboardPluginInfo> {
        self.entries
            .read()
            .await
            .iter()
            .map(|(id, entry)| {
                let manifest = entry.plugin.manifest();
                DashboardPluginInfo {
                    id: id.clone(),
                    name: manifest.name.clone(),
                    version: manifest.version.clone(),
                    description: manifest.description.clone(),
                    author: manifest.author.clone(),
                    panels: manifest.panels.iter().map(|p| p.id.clone()).collect(),
                    enabled: entry.enabled,
                }
            })
            .collect()
    }

    pub async fn list_panels(
        &self,
        position: Option<PanelPosition>,
    ) -> Vec<DashboardPanelWithPlugin> {
        let mut result = Vec::new();
        let entries = self.entries.read().await;
        for (plugin_id, entry) in entries.iter() {
            if !entry.enabled {
                continue;
            }
            for panel in &entry.plugin.manifest().panels {
                if let Some(pos) = position
                    && panel.position != pos
                {
                    continue;
                }
                result.push(DashboardPanelWithPlugin {
                    plugin_id: plugin_id.clone(),
                    plugin_name: entry.plugin.manifest().name.clone(),
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
    ) -> Result<RenderOutput, String> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(plugin_id) {
            if !entry.enabled {
                return Err(format!("Plugin '{}' is disabled", plugin_id));
            }
            entry.plugin.render_panel(panel_id, props).await
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    pub async fn fetch_panel_data(
        &self,
        plugin_id: &str,
        panel_id: &str,
        query: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(plugin_id) {
            if !entry.enabled {
                return Err(format!("Plugin '{}' is disabled", plugin_id));
            }
            entry.plugin.fetch_data(panel_id, query).await
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    pub async fn enable(&self, plugin_id: &str) -> Result<(), String> {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(plugin_id) {
            entry.enabled = true;
            tracing::info!("Plugin enabled: {}", plugin_id);
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    pub async fn disable(&self, plugin_id: &str) -> Result<(), String> {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(plugin_id) {
            entry.enabled = false;
            tracing::info!("Plugin disabled: {}", plugin_id);
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    pub async fn reload(&self) -> Result<(), String> {
        tracing::info!(
            "Reloading dashboard plugins from {} director(ies)",
            self.config.plugin_dirs.len()
        );

        let old_entries = {
            let mut entries = self.entries.write().await;
            let old: HashMap<String, (Arc<dyn DashboardPlugin>, bool)> = entries
                .drain()
                .map(|(id, entry)| (id, (entry.plugin, entry.enabled)))
                .collect();
            old
        };

        let mut new_entries: HashMap<String, PluginEntry> = HashMap::new();

        for dir in &self.config.plugin_dirs {
            if !dir.exists() {
                tracing::warn!("Plugin directory does not exist: {:?}", dir);
                continue;
            }
            let read_dir = std::fs::read_dir(dir)
                .map_err(|e| format!("Failed to read plugin dir {:?}: {}", dir, e))?;
            for entry in read_dir {
                let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let manifest_path = path.join("manifest.json");
                if !manifest_path.exists() {
                    continue;
                }
                let manifest_str = std::fs::read_to_string(&manifest_path)
                    .map_err(|e| format!("Failed to read manifest {:?}: {}", manifest_path, e))?;
                let manifest: DashboardPluginManifest = serde_json::from_str(&manifest_str)
                    .map_err(|e| format!("Failed to parse manifest {:?}: {}", manifest_path, e))?;

                let plugin_dir = path.clone();
                let panel_id_prefix = manifest.id.clone();
                let plugin = DashboardPluginAdapter::new(manifest, move |panel_id, props| {
                    crate::dashboard_plugin::RenderOutput::Directive(
                        crate::dashboard_plugin::RenderDirective {
                            panel_id: panel_id.to_string(),
                            component: format!("{}_{}", panel_id_prefix, panel_id),
                            props,
                            data_endpoint: Some(format!(
                                "/api/plugins/{}/data/{}",
                                plugin_dir.to_string_lossy(),
                                panel_id
                            )),
                            refresh_interval_ms: None,
                        },
                    )
                });

                let id = plugin.manifest().id.clone();
                let preserved_enabled = old_entries
                    .get(&id)
                    .map(|(_, enabled)| *enabled)
                    .unwrap_or(self.config.auto_load);

                if !old_entries.contains_key(&id) {
                    plugin.on_load().await.ok();
                }

                new_entries.insert(
                    id,
                    PluginEntry {
                        plugin: Arc::from(plugin),
                        enabled: preserved_enabled,
                    },
                );
                tracing::info!("Loaded plugin from: {:?}", path);
            }
        }

        for (id, (plugin, _)) in &old_entries {
            if !new_entries.contains_key(id) {
                plugin.on_unload().await.ok();
            }
        }

        let mut entries = self.entries.write().await;
        *entries = new_entries;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPanelWithPlugin {
    pub plugin_id: String,
    pub plugin_name: String,
    pub panel: crate::dashboard_plugin::DashboardPanel,
}
