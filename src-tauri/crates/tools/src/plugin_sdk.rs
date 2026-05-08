use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginPermission {
    FileSystemRead,
    FileSystemWrite,
    NetworkAccess,
    SubprocessExecution,
    ClipboardAccess,
    NotificationAccess,
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
            .build();

        assert_eq!(manifest.id, "my-plugin");
        assert_eq!(manifest.permissions.len(), 1);
        assert_eq!(manifest.tools.len(), 1);
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
