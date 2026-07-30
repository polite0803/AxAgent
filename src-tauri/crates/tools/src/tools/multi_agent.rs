// SPDX-License-Identifier: AGPL-3.0-only

//! DelegateTaskTool — 将子任务委派给 Multi-Agent 固定角色。
//!
//! 委派给 G5 固定角色（analyst / implementer / reviewer），
//! 由 wiring 层注入的 `DelegateTaskRunner` 实现真实的 LLM 调用。
//!
//! 注册点：`tools/mod.rs::register_all()`
//! 注入点：`tools/mod.rs::set_delegate_task_runner()`

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_harness::DelegateTaskRunner;
use serde_json::Value;
use std::sync::{Arc, OnceLock};

static DELEGATE_RUNNER: OnceLock<Arc<dyn DelegateTaskRunner>> = OnceLock::new();

/// 注入 `DelegateTaskRunner` trait object（由 wiring 层初始化时调用一次）
pub fn set_delegate_task_runner(runner: Arc<dyn DelegateTaskRunner>) {
    let _ = DELEGATE_RUNNER.set(runner);
}

fn delegate_runner() -> &'static Arc<dyn DelegateTaskRunner> {
    DELEGATE_RUNNER
        .get()
        .expect("DelegateTaskRunner not initialized; call set_delegate_task_runner() at startup")
}

pub struct DelegateTaskTool;

#[async_trait]
impl Tool for DelegateTaskTool {
    fn name(&self) -> &str {
        "DelegateTask"
    }
    fn description(&self) -> &str {
        "将子任务委派给 Multi-Agent 固定角色（analyst/implementer/reviewer），\
         由专家角色完成并返回结果"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "roleName": {
                    "type": "string",
                    "enum": ["analyst", "implementer", "reviewer"],
                    "description": "目标角色：analyst（分析）、implementer（实施）、reviewer（审查）"
                },
                "task": {
                    "type": "string",
                    "description": "子任务描述"
                },
                "context": {
                    "type": "object",
                    "description": "上下文变量（可选）"
                },
                "providerId": {
                    "type": "string",
                    "description": "LLM 供应商 ID"
                },
                "modelId": {
                    "type": "string",
                    "description": "模型 ID"
                },
                "temperature": {
                    "type": "number",
                    "description": "温度（可选，默认 0.2）"
                },
                "maxTokens": {
                    "type": "integer",
                    "description": "最大输出 tokens（可选，默认 2048）"
                }
            },
            "required": ["roleName", "task", "providerId", "modelId"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let role_name = input
            .get("roleName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: roleName"))?;
        let task = input
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: task"))?;
        let provider_id = input
            .get("providerId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: providerId"))?;
        let model_id = input
            .get("modelId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: modelId"))?;

        let delegate_input = axagent_harness::DelegateTaskInput {
            role_name: role_name.to_string(),
            task: task.to_string(),
            context: input.get("context").cloned().unwrap_or(Value::Null),
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
            temperature: input.get("temperature").and_then(|v| v.as_f64()),
            max_tokens: input.get("maxTokens").and_then(|v| v.as_u64()).map(|v| v as u32),
        };

        let result = delegate_runner()
            .delegate(delegate_input)
            .await
            .map_err(|e| ToolError::execution_failed(e))?;

        Ok(ToolResult::success(format!(
            "## 委派结果（角色: {}）\n\n{}\n\n---\nPrompt: {} tokens · Completion: {} tokens · 耗时: {}ms",
            result.role_name,
            result.content,
            result.prompt_tokens,
            result.completion_tokens,
            result.duration_ms,
        )))
    }
}
