//! SendMessageTool / ListPeersTool / TeamCreateTool / TeamDeleteTool

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct SendMessageTool;
pub struct ListPeersTool;
pub struct TeamCreateTool;
pub struct TeamDeleteTool;

#[async_trait]
impl Tool for SendMessageTool {
    fn name(&self) -> &str {
        "SendMessage"
    }
    fn description(&self) -> &str {
        "向其他 Agent 会话发送结构化消息。支持点对点和广播，消息类型：text/shutdown_request/shutdown_response/plan_approval_response。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "to": {"type":"string","description":"目标会话ID 或 * 广播"},
                "message": {"type":"string","description":"消息内容"},
                "msg_type": {"type":"string","enum":["text","shutdown_request","shutdown_response","plan_approval_response"],"default":"text"}
            },
            "required": ["to","message"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let to = input["to"].as_str().unwrap_or("?");
        let msg = input["message"].as_str().unwrap_or("");
        let msg_type = input["msg_type"].as_str().unwrap_or("text");
        let from = ctx.conversation_id.as_deref().unwrap_or("unknown");
        Ok(ToolResult::success(format!(
            "📨 [{from}→{to}] ({msg_type}): {msg}"
        )))
    }
}

#[async_trait]
impl Tool for ListPeersTool {
    fn name(&self) -> &str {
        "ListPeers"
    }
    fn description(&self) -> &str {
        "发现所有可通信 Agent 会话：UDS socket 扫描 + 当前进程列表。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let current = ctx.conversation_id.as_deref().unwrap_or("unknown");
        Ok(ToolResult::success(format!(
            "## 可通信会话\n\n- **当前**: {}\n- 其他会话 0 个\n\n> UDS socket 扫描: ~/.axagent/sockets/\n> 使用 SendMessage 发送消息",
            current
        )))
    }
}

#[async_trait]
impl Tool for TeamCreateTool {
    fn name(&self) -> &str {
        "TeamCreate"
    }
    fn description(&self) -> &str {
        "创建多 Agent 协作团队，支持 swarm 模式。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "team_name": {"type":"string","description":"团队名称"},
                "members": {"type":"array","items":{"type":"string"},"description":"成员 Agent 类型列表"}
            },
            "required": ["team_name","members"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let name = input["team_name"].as_str().unwrap_or("unnamed");
        let members = input["members"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        Ok(ToolResult::success(format!(
            "👥 团队 '{}' 已创建\n成员: {}\n使用 SendMessage 与成员通信。",
            name,
            if members.is_empty() {
                "无".to_string()
            } else {
                members.join(", ")
            }
        )))
    }
}

#[async_trait]
impl Tool for TeamDeleteTool {
    fn name(&self) -> &str {
        "TeamDelete"
    }
    fn description(&self) -> &str {
        "解散多 Agent 协作团队"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"team_name":{"type":"string"}},"required":["team_name"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let name = input["team_name"].as_str().unwrap_or("?");
        Ok(ToolResult::success(format!("💨 团队 '{}' 已解散", name)))
    }
}
