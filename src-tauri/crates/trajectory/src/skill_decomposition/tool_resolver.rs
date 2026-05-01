use serde::{Deserialize, Serialize};

/// Tool dependency status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDependencyStatus {
    /// Tool is already installed and available
    Satisfied,
    /// Tool can be auto-installed from MCP server or plugin marketplace
    AutoInstallable,
    /// Tool requires manual configuration (API keys, environment, etc.)
    ManualInstallable,
    /// Tool cannot be installed and must be generated
    NeedsGeneration,
}

/// A tool dependency from composite skill decomposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDependency {
    pub name: String,
    pub tool_type: String,
    pub source_info: Option<String>,
    pub status: ToolDependencyStatus,
}

/// Result of checking a single tool dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDependencyCheckResult {
    pub dependency: ToolDependency,
    pub install_instructions: Option<String>,
    pub config_requirements: Option<String>,
}

/// Tool resolver that checks dependency status and provides resolution strategies
pub struct ToolResolver;

impl ToolResolver {
    /// Check tool dependencies against locally installed tools.
    ///
    /// Classifies each dependency as:
    /// - Satisfied: tool is already installed and available
    /// - AutoInstallable: tool can be auto-installed (MCP/plugin)
    /// - ManualInstallable: tool requires manual configuration
    /// - NeedsGeneration: tool must be generated
    pub fn check_tool_dependencies(
        deps: &[ToolDependency],
        installed_mcp_tools: &[String],
        installed_local_tools: &[String],
        installed_plugin_tools: &[String],
    ) -> Vec<ToolDependencyCheckResult> {
        deps.iter()
            .map(|dep| {
                let (status, instructions, config) = match dep.tool_type.as_str() {
                    "mcp" => {
                        if installed_mcp_tools.contains(&dep.name) {
                            (ToolDependencyStatus::Satisfied, None, None)
                        } else {
                            (
                                ToolDependencyStatus::AutoInstallable,
                                Some(format!("Install MCP server providing tool: {}", dep.name)),
                                None,
                            )
                        }
                    },
                    "local" => {
                        if installed_local_tools.contains(&dep.name) {
                            (ToolDependencyStatus::Satisfied, None, None)
                        } else {
                            // Local tools that aren't installed need generation
                            (ToolDependencyStatus::NeedsGeneration, None, None)
                        }
                    },
                    "plugin" => {
                        if installed_plugin_tools.contains(&dep.name) {
                            (ToolDependencyStatus::Satisfied, None, None)
                        } else {
                            (
                                ToolDependencyStatus::AutoInstallable,
                                Some(format!("Install plugin providing tool: {}", dep.name)),
                                None,
                            )
                        }
                    },
                    "builtin" => {
                        // Builtin tools are always available
                        (ToolDependencyStatus::Satisfied, None, None)
                    },
                    _ => {
                        // Unknown tool type: needs generation
                        (ToolDependencyStatus::NeedsGeneration, None, None)
                    },
                };

                ToolDependencyCheckResult {
                    dependency: ToolDependency {
                        name: dep.name.clone(),
                        tool_type: dep.tool_type.clone(),
                        source_info: dep.source_info.clone(),
                        status,
                    },
                    install_instructions: instructions,
                    config_requirements: config,
                }
            })
            .collect()
    }
}
