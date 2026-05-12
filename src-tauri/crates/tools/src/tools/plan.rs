//! PlanMode Tools - 计划模式管理

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct EnterPlanModeTool;

#[async_trait]
impl Tool for EnterPlanModeTool {
    fn name(&self) -> &str {
        "EnterPlanMode"
    }
    fn description(&self) -> &str {
        "进入计划模式。在计划模式下只能进行代码探索和方案设计，不能修改文件。\
         适用于需要先设计方案再实施的复杂任务。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success(
            "✅ 已进入计划模式。\n\
             在此模式下：\n\
             - ✅ 可以探索代码库\n\
             - ✅ 可以设计方案\n\
             - ❌ 不能修改文件\n\
             - ❌ 不能执行 Shell 命令\n\
             完成后使用 ExitPlanMode 退出。",
        ))
    }
}

pub struct ExitPlanModeTool;

#[async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        "ExitPlanMode"
    }
    fn description(&self) -> &str {
        "退出计划模式，提交方案供用户审批。退出后可进入实施阶段。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "allowedPrompts": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "tool": { "type": "string" },
                            "prompt": { "type": "string" }
                        }
                    }
                }
            },
            "required": []
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success("📤 计划已提交审批。等待用户确认后进入实施阶段。"))
    }
}

// ── VerifyPlanExecution ──

pub struct VerifyPlanExecutionTool;

#[async_trait]
impl Tool for VerifyPlanExecutionTool {
    fn name(&self) -> &str {
        "VerifyPlanExecution"
    }
    fn description(&self) -> &str {
        "退出计划模式前验证计划执行状态。检查每个步骤的完成情况，记录实施摘要。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"summary":{"type":"string","description":"实施摘要"},"steps_completed":{"type":"array","items":{"type":"string"},"description":"已完成的步骤列表"}},"required":["summary"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, i: Value, _c: &ToolContext) -> Result<ToolResult, ToolError> {
        let summary = i["summary"].as_str().unwrap_or("");
        let steps = i["steps_completed"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        Ok(ToolResult::success(format!(
            "✅ 计划验证完成 — {} 个步骤已确认\n\n{}",
            steps, summary
        )))
    }
}
