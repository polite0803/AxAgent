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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderDirective {
    pub panel_id: String,
    pub component: String,
    pub props: HashMap<String, serde_json::Value>,
    pub data_endpoint: Option<String>,
    pub refresh_interval_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenderOutput {
    Directive(RenderDirective),
    Html { content: String },
    Data { payload: serde_json::Value },
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
    ) -> Result<RenderOutput, String>;
    async fn fetch_data(
        &self,
        panel_id: &str,
        query: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let _ = (panel_id, query);
        Err("fetch_data not implemented".to_string())
    }
}

type RenderFn = Box<dyn Fn(&str, HashMap<String, serde_json::Value>) -> RenderOutput + Send + Sync>;

pub struct DashboardPluginAdapter {
    manifest: DashboardPluginManifest,
    render_fn: RenderFn,
}

impl DashboardPluginAdapter {
    pub fn new<F>(manifest: DashboardPluginManifest, render_fn: F) -> Self
    where
        F: Fn(&str, HashMap<String, serde_json::Value>) -> RenderOutput + Send + Sync + 'static,
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
    ) -> Result<RenderOutput, String> {
        Ok((self.render_fn)(panel_id, props))
    }
}
