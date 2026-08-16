// SPDX-License-Identifier: AGPL-3.0-only

//! 自指工具（`system_evolution_*`）— 系统进化工具的实现。
//!
//! 定义四个自指工具，通过 `RuntimeMutationAccess` trait 访问运行时工具注册表，
//! 由 wiring 层（`commands/evolution_engine.rs`）注入具体实现。
//!
//! # 架构
//! ```
//! tools (本文件)          harness                wiring (commands)
//! ┌──────────────┐       ┌──────────────────┐    ┌─────────────────────┐
//! │ InspectTool  │──────→│RuntimeMutationAccess│←───│EvolutionEngineAccess│
//! │ DefineTool   │ calls │  (trait)          │ impl│ (持有 AppState)     │
//! │ DeployTool   │──────→│ MutationResult    │    └─────────────────────┘
//! │ UndeployTool │       └──────────────────┘
//! └──────────────┘
//! ```
//!
//! # 安全约束
//! - 全部设置 `category = System` + `is_read_only` 按工具性质区分；
//! - inspect 是只读操作（`is_read_only = true`）；
//! - define/deploy/undeploy 是写操作（`is_read_only = false`）；
//! - 实际安全由 `RuntimeMutationAccess` 实现方保证（仅操作 `runtime_tool_sources`）。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_harness::runtime_types::runtime_mutation::{MutationResult, RuntimeMutationAccess};
use serde_json::Value;
use std::sync::Arc;

// ── 全局注入点 ──

/// 静态全局注入点：wiring 层在初始化时调用 `set_mutation_access` 注入。
static MUTATION_ACCESS: std::sync::LazyLock<
    parking_lot::Mutex<Option<Arc<dyn RuntimeMutationAccess>>>,
> = std::sync::LazyLock::new(|| parking_lot::Mutex::new(None));

/// 设置运行时变异访问器（由 wiring 层在初始化时调用一次）。
///
/// 多次调用会覆盖前值，但实际项目中仅初始化时调用一次。
pub fn set_mutation_access(access: Arc<dyn RuntimeMutationAccess>) {
    *MUTATION_ACCESS.lock() = Some(access);
}

/// 获取运行时变异访问器（工具内部调用）。
fn get_mutation_access() -> Result<Arc<dyn RuntimeMutationAccess>, ToolError> {
    MUTATION_ACCESS
        .lock()
        .clone()
        .ok_or_else(|| ToolError::new("运行时变异访问器未注入，请确认 wiring 层已初始化"))
}

// ── 辅助函数 ──

/// 从 MutationResult 转换为 ToolResult
fn mutation_to_tool_result(result: MutationResult) -> Result<ToolResult, ToolError> {
    if result.success {
        let content = serde_json::to_string_pretty(&result.data)
            .unwrap_or_else(|_| "序列化结果失败".to_string());
        Ok(ToolResult::success(content))
    } else {
        Err(ToolError::new(result.error.unwrap_or_else(|| "未知错误".to_string())))
    }
}

// ── SystemEvolutionInspectTool ──

/// 检查运行时进化能力状态。
///
/// 只读操作：列出当前运行时已注册的进化工具（名称 / 来源 / 描述）。
/// 在 ReadOnly 权限模式下可见。
pub struct SystemEvolutionInspectTool;

#[async_trait]
impl Tool for SystemEvolutionInspectTool {
    fn name(&self) -> &str {
        "system_evolution_inspect"
    }

    fn description(&self) -> &str {
        "检查当前运行时已注册的进化能力状态，包括工具、工作流和技能。\
         返回工具列表（名称、来源、描述）及运行时环境信息。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "capability_type": {
                    "type": "string",
                    "enum": ["tool", "workflow", "skill", "all"],
                    "description": "要检查的能力类型，默认为 all"
                }
            }
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    /// 只读操作，ReadOnly 模式下可见
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let capability_type =
            input.get("capability_type").and_then(|v| v.as_str()).unwrap_or("all");
        let access = get_mutation_access()?;
        let result = access.inspect(capability_type).await.map_err(ToolError::new)?;
        mutation_to_tool_result(result)
    }
}

// ── SystemEvolutionDefineTool ──

/// 定义一个新进化工具（仅生成定义，不注册）。
///
/// 写操作：生成工具定义 JSON，供 Agent 审查确认后调用 `deploy` 完成注册。
/// 不在 ReadOnly 模式下暴露。
pub struct SystemEvolutionDefineTool;

#[async_trait]
impl Tool for SystemEvolutionDefineTool {
    fn name(&self) -> &str {
        "system_evolution_define"
    }

    fn description(&self) -> &str {
        "定义一个新工具（仅生成工具定义，不注册到运行时）。\
         `spec` 至少包含 name / description / code / input_schema。\
         返回工具定义 JSON，供审查；确认后调用 system_evolution_deploy 完成注册。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "spec": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "工具名称" },
                        "description": { "type": "string", "description": "工具描述" },
                        "code": { "type": "string", "description": "工具实现代码" },
                        "input_schema": { "type": "object", "description": "输入参数 JSON Schema" }
                    },
                    "required": ["name", "description", "code"]
                }
            },
            "required": ["spec"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    /// 写操作，非 ReadOnly
    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let spec =
            input.get("spec").ok_or_else(|| ToolError::invalid_input("缺少必填参数 'spec'"))?;
        let access = get_mutation_access()?;
        let result = access.define(spec.clone()).await.map_err(ToolError::new)?;
        mutation_to_tool_result(result)
    }
}

// ── SystemEvolutionDeployTool ──

/// 部署（注册）工具到运行时注册表。
///
/// 写操作：将工具注册到 UnifiedToolRegistry（来源 runtime_evolution），
/// 注册后 Agent 立即可见可调用。重复注册返回 REGISTRATION_DUPLICATE。
pub struct SystemEvolutionDeployTool;

#[async_trait]
impl Tool for SystemEvolutionDeployTool {
    fn name(&self) -> &str {
        "system_evolution_deploy"
    }

    fn description(&self) -> &str {
        "部署（注册）工具到运行时注册表。\
         注册后 Agent 立即可发现并调用该工具。\
         `spec` 至少包含 name / description / code / input_schema。\
         重复注册会返回错误，不会覆盖已有工具。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "spec": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "工具名称" },
                        "description": { "type": "string", "description": "工具描述" },
                        "code": { "type": "string", "description": "工具实现代码" },
                        "input_schema": { "type": "object", "description": "输入参数 JSON Schema" }
                    },
                    "required": ["name", "description", "code"]
                }
            },
            "required": ["spec"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let spec =
            input.get("spec").ok_or_else(|| ToolError::invalid_input("缺少必填参数 'spec'"))?;
        let access = get_mutation_access()?;
        let result = access.deploy(spec.clone()).await.map_err(ToolError::new)?;
        mutation_to_tool_result(result)
    }
}

// ── SystemEvolutionUndeployTool ──

/// 卸载运行时注册的工具。
///
/// 写操作：仅允许卸载 `runtime_tool_sources` 中登记的工具。
/// 内置工具与 MCP 工具不受影响。
pub struct SystemEvolutionUndeployTool;

#[async_trait]
impl Tool for SystemEvolutionUndeployTool {
    fn name(&self) -> &str {
        "system_evolution_undeploy"
    }

    fn description(&self) -> &str {
        "卸载一个运行时注册的进化工具。\
         仅允许卸载此前通过 system_evolution_deploy 注册的工具，\
         内置工具与 MCP 工具不受影响。"
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "要卸载的工具名称"
                }
            },
            "required": ["name"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必填参数 'name'"))?;
        let access = get_mutation_access()?;
        let result = access.undeploy(name).await.map_err(ToolError::new)?;
        mutation_to_tool_result(result)
    }
}

// ── 便捷注册函数 ──

/// 注册全部四个自指工具到给定的注册接收器。
///
/// 由 wiring 层调用，将自指工具实例化并注册到 ToolRegistry。
/// 返回四个工具的 `Arc<dyn Tool>` 引用，供调用方决定注册方式。
pub fn create_all_self_referential_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(SystemEvolutionInspectTool) as Arc<dyn Tool>,
        Arc::new(SystemEvolutionDefineTool) as Arc<dyn Tool>,
        Arc::new(SystemEvolutionDeployTool) as Arc<dyn Tool>,
        Arc::new(SystemEvolutionUndeployTool) as Arc<dyn Tool>,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T2.1：四个自指工具元数据正确（名称 / 分类 / 只读性质）
    #[test]
    fn test_self_referential_tools_metadata() {
        let tools = create_all_self_referential_tools();
        assert_eq!(tools.len(), 4, "测试：应有四个自指工具");

        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert_eq!(
            names,
            vec![
                "system_evolution_inspect",
                "system_evolution_define",
                "system_evolution_deploy",
                "system_evolution_undeploy",
            ]
        );

        for tool in &tools {
            assert_eq!(tool.category(), ToolCategory::System, "测试：自指工具应归系统类");
            assert!(
                tool.description().contains("system_evolution")
                    || tool.description().contains("运行时")
                    || tool.description().contains("工具"),
                "测试：自指工具应有描述"
            );
        }

        // 只读性质：inspect 只读，其余写操作
        assert!(tools[0].is_read_only(), "测试：inspect 应为只读");
        assert!(!tools[1].is_read_only(), "测试：define 应为写操作");
        assert!(!tools[2].is_read_only(), "测试：deploy 应为写操作");
        assert!(!tools[3].is_read_only(), "测试：undeploy 应为写操作");
    }

    /// T2.1：未注入 access 时调用应返回明确的未注入错误（防误用）
    #[tokio::test]
    async fn test_inspect_without_injected_access_errors() {
        let tool = SystemEvolutionInspectTool;
        let ctx = ToolContext::new(".");
        let err = tool
            .call(serde_json::json!({ "capability_type": "tool" }), &ctx)
            .await
            .expect_err("测试：未注入访问器时应返回错误");
        assert!(
            err.message.contains("未注入") || err.message.contains("未初始化"),
            "测试：错误信息应提示未注入，实际: {}",
            err.message
        );
    }
}
