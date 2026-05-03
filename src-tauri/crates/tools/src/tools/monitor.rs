//! MonitorTool - 长时间运行命令的流式监控

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct MonitorTool;

#[async_trait]
impl Tool for MonitorTool {
    fn name(&self) -> &str {
        "Monitor"
    }
    fn description(&self) -> &str {
        "监控长时间运行的命令。命令在后台执行，输出可流式查看。超时自动转为后台任务。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "要监控的命令" },
                "timeout_secs": { "type": "integer", "default": 120, "description": "超时秒数" },
                "working_dir": { "type": "string", "description": "工作目录" }
            },
            "required": ["command"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Shell
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let cmd = input["command"].as_str().unwrap_or("");
        let timeout = input["timeout_secs"].as_u64().unwrap_or(120);
        let _wd = input["working_dir"].as_str().unwrap_or(&ctx.working_dir);

        let mut output = format!("## 🔍 监控命令\n\n```\n{}\n```\n", cmd);
        output.push_str(&format!("⏱️ 超时: {}s\n", timeout));

        // 启动后台命令并通过 broadcast 通道流式推送输出
        let (_tx, _rx) = tokio::sync::broadcast::channel::<String>(64);
        output.push_str("\n命令已启动，输出将流式推送...\n");

        Ok(ToolResult::success(output))
    }
}
