//! AxAgent Tool System - 统一工具接口与执行引擎
//!
//! 提供 Tool trait、ToolRegistry、编排器、流式执行器等核心组件。

pub mod agent_def_loader;
pub mod agent_def_types;
pub mod audit;
pub mod bash;
pub mod context_keys;
pub mod global_state;
pub mod hooks;
pub mod knowledge_callback;
pub mod markdown;
pub mod mcp;
pub mod orchestration;
pub mod permissions;
pub mod plugin_sdk;
pub mod recorder;
pub mod registry;
pub mod rhai_engine;
pub mod sandbox;
pub mod stats;
pub mod streaming;
pub mod tools;
pub mod utils;

pub use global_state::{get_db_path, get_sea_db, set_db_path, set_sea_db};
pub use plugin_sdk::{
    AxAgentPlugin, PluginBuilder, PluginCategory, PluginContext, PluginManifest, PluginPermission,
    PluginToolDef, PluginToolResult,
};
pub use recorder::ToolExecutionRecorder;
pub use sandbox::{
    SandboxConfig, SandboxPlatform, SandboxViolation, SandboxViolationType, SecuritySandbox,
};
pub use stats::{StatCategory, ToolMetadata, ToolUsageStats};

// 工具体系类型统一来自 axagent-harness 契约层
#[doc(hidden)]
pub use axagent_harness::parse_tool_name;
#[doc(hidden)]
pub use axagent_harness::{
    PermissionResult, ProgressEntry, Tool, ToolCategory, ToolContext, ToolInfo, ToolResult,
};
#[doc(hidden)]
pub use axagent_harness::{ToolError, ToolErrorKind};

use async_trait::async_trait;

// 所有工具体系类型已统一由 axagent_harness 定义，上方已 re-export
// 以下为 tools crate 特有的扩展实现

#[async_trait]
impl tools::rpc::ToolExecutorAccess for registry::ToolRegistry {
    async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let tool = self
            .find(name)
            .ok_or_else(|| format!("Tool '{}' not found", name))?;
        let ctx = ToolContext::new(
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        );
        let result = tool.call(input, &ctx).await.map_err(|e| e.message)?;
        if result.is_error {
            Err(result.content)
        } else {
            Ok(serde_json::json!({
                "content": result.content,
                "metadata": result.metadata,
            }))
        }
    }
}
