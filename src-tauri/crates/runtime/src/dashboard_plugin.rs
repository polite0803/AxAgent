use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPanel {
    pub id: String,
    pub title: String,
    pub component_name: String,
    pub props: HashMap<String, serde_json::Value>,
    pub position: PanelPosition,
    pub size: PanelSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelPosition {
    Main,
    Sidebar,
    Header,
    Footer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelSize {
    Small,
    Medium,
    Large,
    FullWidth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub panels: Vec<DashboardPanel>,
    pub permissions: Vec<String>,
    pub frontend_entry: Option<String>,
}

#[async_trait]
pub trait DashboardPlugin: Send + Sync {
    fn manifest(&self) -> &DashboardPluginManifest;
    async fn on_load(&self) -> Result<(), String>;
    async fn on_unload(&self) -> Result<(), String>;
    async fn render_panel(
        &self,
        panel_id: &str,
        props: HashMap<String, serde_json::Value>,
    ) -> Result<String, String>;
}

pub struct DashboardPluginAdapter {
    manifest: DashboardPluginManifest,
    render_fn: Box<dyn Fn(&str, HashMap<String, serde_json::Value>) -> String + Send + Sync>,
}

impl DashboardPluginAdapter {
    pub fn new<F>(manifest: DashboardPluginManifest, render_fn: F) -> Self
    where
        F: Fn(&str, HashMap<String, serde_json::Value>) -> String + Send + Sync + 'static,
    {
        Self {
            manifest,
            render_fn: Box::new(render_fn),
        }
    }
}

#[async_trait]
impl DashboardPlugin for DashboardPluginAdapter {
    fn manifest(&self) -> &DashboardPluginManifest {
        &self.manifest
    }

    async fn on_load(&self) -> Result<(), String> {
        tracing::info!("Dashboard plugin loaded: {}", self.manifest.name);
        Ok(())
    }

    async fn on_unload(&self) -> Result<(), String> {
        tracing::info!("Dashboard plugin unloaded: {}", self.manifest.name);
        Ok(())
    }

    async fn render_panel(
        &self,
        panel_id: &str,
        props: HashMap<String, serde_json::Value>,
    ) -> Result<String, String> {
        Ok((self.render_fn)(panel_id, props))
    }
}
