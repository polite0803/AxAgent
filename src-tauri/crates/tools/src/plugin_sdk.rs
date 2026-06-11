// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub homepage: Option<String>,
    pub category: PluginCategory,
    pub permissions: Vec<PluginPermission>,
    pub tools: Vec<PluginToolDef>,
    pub min_app_version: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub min_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginCategory {
    Productivity,
    Development,
    Data,
    Communication,
    Automation,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginPermission {
    FileSystemRead,
    FileSystemWrite,
    NetworkAccess,
    SubprocessExecution,
    ClipboardAccess,
    NotificationAccess,
}

impl PluginPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileSystemRead => "file_system_read",
            Self::FileSystemWrite => "file_system_write",
            Self::NetworkAccess => "network_access",
            Self::SubprocessExecution => "subprocess_execution",
            Self::ClipboardAccess => "clipboard_access",
            Self::NotificationAccess => "notification_access",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub plugin_id: String,
    pub workspace_path: Option<String>,
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

#[async_trait]
pub trait AxAgentPlugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;

    async fn initialize(&mut self, ctx: &PluginContext) -> Result<(), String>;

    async fn execute_tool(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        ctx: &PluginContext,
    ) -> Result<PluginToolResult, String>;

    async fn shutdown(&mut self) -> Result<(), String>;
}

pub struct PluginBuilder {
    manifest: PluginManifest,
}

impl PluginBuilder {
    pub fn new(id: &str, name: &str, version: &str) -> Self {
        Self {
            manifest: PluginManifest {
                id: id.to_string(),
                name: name.to_string(),
                version: version.to_string(),
                description: String::new(),
                author: String::new(),
                homepage: None,
                category: PluginCategory::Custom("general".to_string()),
                permissions: Vec::new(),
                tools: Vec::new(),
                min_app_version: None,
                dependencies: Vec::new(),
            },
        }
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.manifest.description = desc.to_string();
        self
    }

    pub fn author(mut self, author: &str) -> Self {
        self.manifest.author = author.to_string();
        self
    }

    pub fn category(mut self, cat: PluginCategory) -> Self {
        self.manifest.category = cat;
        self
    }

    pub fn permission(mut self, perm: PluginPermission) -> Self {
        self.manifest.permissions.push(perm);
        self
    }

    pub fn tool(mut self, tool: PluginToolDef) -> Self {
        self.manifest.tools.push(tool);
        self
    }

    pub fn min_version(mut self, version: &str) -> Self {
        self.manifest.min_app_version = Some(version.to_string());
        self
    }

    pub fn dependency(mut self, plugin_id: &str, min_version: Option<&str>) -> Self {
        self.manifest.dependencies.push(PluginDependency {
            plugin_id: plugin_id.to_string(),
            min_version: min_version.map(|v| v.to_string()),
        });
        self
    }

    pub fn build(self) -> PluginManifest {
        self.manifest
    }
}

impl PluginToolDef {
    pub fn simple(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
            output_schema: None,
        }
    }

    pub fn with_input_schema(name: &str, description: &str, schema: serde_json::Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: schema,
            output_schema: None,
        }
    }
}

struct SdkPluginEntry {
    plugin: Arc<RwLock<Box<dyn AxAgentPlugin>>>,
    initialized: bool,
}

pub struct SdkPluginRegistry {
    plugins: RwLock<HashMap<String, SdkPluginEntry>>,
}

fn infer_permissions_for_tool(tool: &PluginToolDef) -> Vec<PluginPermission> {
    let schema_str = tool.input_schema.to_string().to_lowercase();
    let mut perms = Vec::new();
    if schema_str.contains("path")
        || schema_str.contains("file")
        || schema_str.contains("directory")
    {
        perms.push(PluginPermission::FileSystemRead);
    }
    if schema_str.contains("write")
        || schema_str.contains("save")
        || schema_str.contains("delete")
        || schema_str.contains("create")
    {
        perms.push(PluginPermission::FileSystemWrite);
    }
    if schema_str.contains("url")
        || schema_str.contains("fetch")
        || schema_str.contains("request")
        || schema_str.contains("api")
    {
        perms.push(PluginPermission::NetworkAccess);
    }
    if schema_str.contains("command")
        || schema_str.contains("exec")
        || schema_str.contains("shell")
        || schema_str.contains("run")
    {
        perms.push(PluginPermission::SubprocessExecution);
    }
    perms
}

impl SdkPluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, plugin: Box<dyn AxAgentPlugin>) -> Result<(), String> {
        let id = plugin.manifest().id.clone();
        let mut plugins = self.plugins.write().await;
        if plugins.contains_key(&id) {
            return Err(format!("SDK plugin '{}' already registered", id));
        }
        plugins.insert(
            id,
            SdkPluginEntry {
                plugin: Arc::new(RwLock::new(plugin)),
                initialized: false,
            },
        );
        Ok(())
    }

    pub async fn unregister(&self, plugin_id: &str) -> Result<(), String> {
        let mut plugins = self.plugins.write().await;
        if let Some(entry) = plugins.remove(plugin_id) {
            let mut plugin = entry.plugin.write().await;
            plugin.shutdown().await.ok();
            Ok(())
        } else {
            Err(format!("SDK plugin '{}' not found", plugin_id))
        }
    }

    pub async fn initialize(&self, plugin_id: &str, ctx: &PluginContext) -> Result<(), String> {
        let plugins = self.plugins.read().await;
        let entry = plugins
            .get(plugin_id)
            .ok_or_else(|| format!("SDK plugin '{}' not found", plugin_id))?;
        if entry.initialized {
            return Ok(());
        }
        drop(plugins);

        let mut plugins = self.plugins.write().await;
        let entry = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| format!("SDK plugin '{}' not found", plugin_id))?;
        let mut plugin = entry.plugin.write().await;
        plugin.initialize(ctx).await?;
        entry.initialized = true;
        Ok(())
    }

    pub async fn execute_tool(
        &self,
        plugin_id: &str,
        tool_name: &str,
        input: &serde_json::Value,
        ctx: &PluginContext,
    ) -> Result<PluginToolResult, String> {
        let plugins = self.plugins.read().await;
        let entry = plugins
            .get(plugin_id)
            .ok_or_else(|| format!("SDK plugin '{}' not found", plugin_id))?;
        if !entry.initialized {
            return Err(format!("SDK plugin '{}' is not initialized", plugin_id));
        }
        let plugin = entry.plugin.read().await;
        let tool_def = plugin.manifest().tools.iter().find(|t| t.name == tool_name);
        if let Some(tool) = tool_def {
            let required_perms: Vec<PluginPermission> = infer_permissions_for_tool(tool);
            self.check_permissions(plugin_id, &required_perms).await?;
        }
        plugin.execute_tool(tool_name, input, ctx).await
    }

    pub async fn list_plugins(&self) -> Vec<PluginManifest> {
        let plugins = self.plugins.read().await;
        let mut result = Vec::new();
        for entry in plugins.values() {
            let plugin = entry.plugin.read().await;
            result.push(plugin.manifest().clone());
        }
        result
    }

    pub async fn contains(&self, plugin_id: &str) -> bool {
        self.plugins.read().await.contains_key(plugin_id)
    }

    pub async fn check_permissions(
        &self,
        plugin_id: &str,
        required: &[PluginPermission],
    ) -> Result<(), String> {
        let plugins = self.plugins.read().await;
        let entry = plugins
            .get(plugin_id)
            .ok_or_else(|| format!("SDK plugin '{}' not found", plugin_id))?;
        let plugin = entry.plugin.read().await;
        let declared = &plugin.manifest().permissions;
        for perm in required {
            if !declared.contains(perm) {
                return Err(format!(
                    "SDK plugin '{}' lacks required permission: {}",
                    plugin_id,
                    perm.as_str()
                ));
            }
        }
        Ok(())
    }
}

impl Default for SdkPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(test))]
static GLOBAL_SDK_PLUGINS: std::sync::LazyLock<SdkPluginRegistry> =
    std::sync::LazyLock::new(SdkPluginRegistry::default);

#[cfg(not(test))]
pub fn global_sdk_plugins() -> &'static SdkPluginRegistry {
    &GLOBAL_SDK_PLUGINS
}

#[cfg(test)]
thread_local! {
    static TEST_SDK_PLUGINS: std::cell::RefCell<Option<SdkPluginRegistry>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub fn global_sdk_plugins() -> &'static SdkPluginRegistry {
    static FALLBACK: std::sync::LazyLock<SdkPluginRegistry> =
        std::sync::LazyLock::new(SdkPluginRegistry::default);
    TEST_SDK_PLUGINS.with(|cell| {
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
pub fn set_test_sdk_plugins(registry: SdkPluginRegistry) {
    TEST_SDK_PLUGINS.with(|cell| {
        *cell.borrow_mut() = Some(registry);
    });
}

#[cfg(test)]
pub fn reset_test_sdk_plugins() {
    TEST_SDK_PLUGINS.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

pub async fn register_sdk_plugin(plugin: Box<dyn AxAgentPlugin>) -> Result<(), String> {
    global_sdk_plugins().register(plugin).await
}

pub async fn unregister_sdk_plugin(plugin_id: &str) -> Result<(), String> {
    global_sdk_plugins().unregister(plugin_id).await
}

pub async fn execute_sdk_plugin_tool(
    plugin_id: &str,
    tool_name: &str,
    input: &serde_json::Value,
    ctx: &PluginContext,
) -> Result<PluginToolResult, String> {
    global_sdk_plugins()
        .execute_tool(plugin_id, tool_name, input, ctx)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_builder() {
        let manifest = PluginBuilder::new("my-plugin", "My Plugin", "1.0.0")
            .description("A test plugin")
            .author("Test Author")
            .category(PluginCategory::Development)
            .permission(PluginPermission::FileSystemRead)
            .tool(PluginToolDef::simple("my_tool", "Does something"))
            .min_version("1.4.0")
            .dependency("other-plugin", Some("2.0.0"))
            .build();

        assert_eq!(manifest.id, "my-plugin");
        assert_eq!(manifest.permissions.len(), 1);
        assert_eq!(manifest.tools.len(), 1);
        assert_eq!(manifest.dependencies.len(), 1);
    }

    #[test]
    fn test_tool_def_simple() {
        let tool = PluginToolDef::simple("read_file", "Read a file");
        assert_eq!(tool.name, "read_file");
    }

    #[test]
    fn test_tool_def_with_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            }
        });
        let tool = PluginToolDef::with_input_schema("read_file", "Read a file", schema);
        assert!(tool.input_schema["properties"]["path"].is_object());
    }
}
