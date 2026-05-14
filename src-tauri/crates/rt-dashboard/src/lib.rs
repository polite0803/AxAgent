//! Dashboard plugin and registry system.

pub mod dashboard_plugin;
pub mod dashboard_registry;

pub use dashboard_plugin::{
    DashboardPlugin, DashboardPluginAdapter, DashboardPluginManifest, PanelPosition,
};
pub use dashboard_registry::{DashboardPluginInfo, DashboardRegistry, DashboardRegistryConfig};
