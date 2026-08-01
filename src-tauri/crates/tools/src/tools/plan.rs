// SPDX-License-Identifier: AGPL-3.0-only

//! PlanMode Tools - 计划模式管理
//!
//! 这些工具是告知性存根（informational stubs），仅向 LLM 返回文本响应。
//! 它们**不**修改状态、不强制执行约束、不与后端 Plan 系统交互。
//!
//! 实际的 Plan 管理通过以下 Tauri 命令完成：
//! - `plan_generate` / `plan_execute` / `plan_cancel` / `plan_activate`
//! - 前端通过 `work_strategy` 字段控制 Plan-First 执行路径
//!
//! 参见: `src-tauri/src/commands/plan.rs`, `src/stores/feature/planStore.ts`

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

/// 告知性工具：声明进入 Plan 模式。
///
/// 仅返回成功文本，不修改任何状态。
/// 实际的 Plan 模式由前端 work_strategy 和后端 plan_generate 命令协调控制。
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

    async fn call(&self, _input: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        // 反映当前会话的真实权限状态，避免返回与实际情况冲突的文本
        let write_note = if ctx.allow_write {
            "⚠️ 当前会话允许写文件，请自觉遵守计划模式约束"
        } else {
            "✅ 当前会话已限制文件写入"
        };
        let exec_note = if ctx.allow_execute {
            "⚠️ 当前会话允许执行 Shell，请自觉遵守计划模式约束"
        } else {
            "✅ 当前会话已限制 Shell 执行"
        };
        Ok(ToolResult::success(format!(
            "✅ 已进入计划模式。\n\
             在此模式下：\n\
             - ✅ 可以探索代码库\n\
             - ✅ 可以设计方案\n\
             - ❌ 不能修改文件\n\
             - ❌ 不能执行 Shell 命令\n\
             {write_note}\n\
             {exec_note}\n\
             完成后使用 ExitPlanMode 退出。"
        )))
    }
}

/// 告知性工具：声明退出 Plan 模式并提交审批。
///
/// 返回成功文本并回显 allowedPrompts 数量。
/// 说明（2026-08-02 审计）：allowedPrompts 的逐条授权由前端 PlanCard + planStore.approvePlan()
/// 在审批环节接管（工具层无权限存储），故此处仅回显数量供 LLM 确认，不做权限写入。
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

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let prompt_count = input["allowedPrompts"].as_array().map(|a| a.len()).unwrap_or(0);
        let extra = if prompt_count > 0 {
            format!("\n\n（附带 {} 条预授权提示，将在审批时逐条确认）", prompt_count)
        } else {
            String::new()
        };
        Ok(ToolResult::success(format!("📤 计划已提交审批。等待用户确认后进入实施阶段。{}", extra)))
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
        let steps = i["steps_completed"].as_array().map(|a| a.len()).unwrap_or(0);
        Ok(ToolResult::success(format!("✅ 计划验证完成 — {} 个步骤已确认\n\n{}", steps, summary)))
    }
}
