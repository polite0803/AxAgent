//! 系统信息工具
//!
//! 将 builtin_handlers 中的 get_system_info、list_processes 迁移为 Tool trait 实现。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct GetSystemInfoTool;

#[async_trait]
impl Tool for GetSystemInfoTool {
    fn name(&self) -> &str {
        "GetSystemInfo"
    }

    fn description(&self) -> &str {
        "获取系统信息，包括操作系统、架构、主目录和运行时间。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let uptime = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| format!("{} 秒", d.as_secs()))
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(ToolResult::success(format!(
            "系统信息:\n  OS: {}\n  架构: {}\n  主目录: {}\n  运行时间: {}",
            os, arch, home, uptime
        )))
    }
}

pub struct ListProcessesTool;

#[async_trait]
impl Tool for ListProcessesTool {
    fn name(&self) -> &str {
        "ListProcesses"
    }

    fn description(&self) -> &str {
        "列出系统正在运行的进程。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "最多返回的进程数",
                    "default": 20
                }
            }
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(20);

        #[cfg(windows)]
        let output = tokio::process::Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .output()
            .await;

        #[cfg(not(windows))]
        let output = tokio::process::Command::new("ps")
            .args(["aux"])
            .output()
            .await;

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let lines: Vec<&str> = stdout.lines().take(limit).collect();
                let content = if lines.is_empty() {
                    "没有找到进程".to_string()
                } else {
                    lines.join("\n")
                };
                Ok(ToolResult::success(content))
            },
            Err(e) => Ok(ToolResult::error(format!("列出进程时出错: {}", e))),
        }
    }
}
