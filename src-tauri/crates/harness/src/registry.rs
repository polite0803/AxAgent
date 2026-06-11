// SPDX-License-Identifier: AGPL-3.0-only

//! ToolRegistry 抽象接口
//!
//! 在 Harness 架构中，`agent` crate 只通过此 trait 查询工具注册表，
//! 不依赖 `axagent-tools` 的具体 `ToolRegistry` 实现。
//! 由 `axagent-runtime` 在启动时注入具体实现。

use crate::error::ToolError;
use crate::tool::{Tool, ToolCategory, ToolContext, ToolInfo, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Provider 注册表接口 — 抽象查找 LLM Provider 适配器
///
/// 实现方 (`axagent-providers::registry::ProviderRegistry`) 在运行时注入。
/// `rt-messaging` 和 `gateway` 仅依赖此 trait，不依赖具体实现。
pub trait ProviderRegistry: Send + Sync {
    /// 按 provider 类型名查找适配器
    fn get(&self, provider_type: &str) -> Option<Arc<dyn super::provider::ProviderAdapter>>;
}

/// 工具注册表抽象接口
///
/// 提供工具查找、列举、禁用状态查询、统一执行能力。
/// 实现方 (`axagent-tools::registry::ToolRegistry`) 在运行时注入。
#[async_trait]
pub trait ToolRegistry: Send + Sync {
    /// 按名称精确查找工具
    fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;

    /// 按名称查找工具（支持别名解析）
    fn find(&self, name: &str) -> Option<Arc<dyn Tool>>;

    /// 列举全部已注册工具
    fn list(&self) -> Vec<ToolInfo>;

    /// 按类别列举工具
    fn list_by_category(&self, category: ToolCategory) -> Vec<ToolInfo>;

    /// 检查工具是否被禁用
    fn is_disabled(&self, name: &str) -> bool;

    /// 统一执行工具 — 集成权限校验、输出脱敏、调用次数追踪
    ///
    /// 通过 ToolRegistry 执行工具可获得统一的权限检查、输出脱敏等能力。
    /// 默认实现通过 find() 查找工具并调用 tool.call()。
    async fn execute_tool(
        &self,
        name: &str,
        input: Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let tool = self.find(name).ok_or_else(|| ToolError::not_found(name))?;

        // 权限检查
        let perm = tool.check_permissions(&input, ctx);
        match perm {
            crate::tool::PermissionResult::Allow => {},
            crate::tool::PermissionResult::Deny(reason) => {
                return Err(ToolError::permission_denied(tool.name(), &reason));
            },
            crate::tool::PermissionResult::Ask(reason) => {
                return Err(ToolError::permission_denied(tool.name(), &reason));
            },
        }

        // 输入验证
        tool.validate(&input, ctx).await?;

        // 核心调用
        let mut result = tool.call(input, ctx).await?;

        // 输出脱敏
        if let Some(ref sanitizer) = ctx.output_sanitizer {
            let sanitize_ctx = crate::tool::SanitizeContext {
                tool_name: tool.name().to_string(),
                tool_category: tool.category(),
                conversation_id: ctx.conversation_id.clone(),
            };
            let sanitized = sanitizer.sanitize(&result.content, &sanitize_ctx);
            if sanitized != result.content {
                tracing::warn!(
                    tool_name = %tool.name(),
                    "ToolRegistry.execute_tool: OutputSanitizer 已脱敏敏感信息"
                );
                result.content = sanitized;
            }
        }

        Ok(result)
    }
}
