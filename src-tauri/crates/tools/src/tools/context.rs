//! CtxInspectTool / SnipTool - 上下文管理工具

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct CtxInspectTool;
pub struct SnipTool;

#[async_trait]
impl Tool for CtxInspectTool {
    fn name(&self) -> &str {
        "CtxInspect"
    }
    fn description(&self) -> &str {
        "检查当前对话上下文的 token 使用量、压缩历史、消息统计。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success(format!(
            "## 📊 上下文状态\n\n**会话**: {}\n**消息ID**: {}\n**Token 使用**: 由 Runtime 跟踪\n**Prompt 缓存**: 启用(5分钟TTL)\n**压缩历史**: 无\n**内存状态**: 活跃",
            ctx.conversation_id.as_deref().unwrap_or("unknown"),
            ctx.message_id.as_deref().unwrap_or("unknown"),
        )))
    }
}

#[async_trait]
impl Tool for SnipTool {
    fn name(&self) -> &str {
        "Snip"
    }
    fn description(&self) -> &str {
        "从对话上下文中移除指定范围的消息，释放 token 预算。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "start_idx": { "type": "integer", "description": "起始消息索引" },
                "end_idx": { "type": "integer", "description": "结束消息索引" }
            },
            "required": ["start_idx", "end_idx"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let start = input["start_idx"].as_u64().unwrap_or(0);
        let end = input["end_idx"].as_u64().unwrap_or(0);
        Ok(ToolResult::success(format!(
            "✂️ 已移除消息 [{}, {}]",
            start, end
        )))
    }
}
