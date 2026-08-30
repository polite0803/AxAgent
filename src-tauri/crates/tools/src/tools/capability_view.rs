// SPDX-License-Identifier: AGPL-3.0-only
//! CapabilityView — 渐进式披露 L1「定义层」按需展开工具
//!
//! 系统提示里的 `<capability-index>` 只给摘要（L0）；模型确认要用某项能力后，
//! 凭目录里的 `capability_id` 调本工具取回完整定义：入参 schema、SOP 步骤、
//! 前置条件、附带知识片段、工具引用、工具链步骤、模板正文等。
//!
//! 与既有 `SkillView` 的分工：
//! - `CapabilityView` 面向**护照全集**（Tool / Toolchain / Template / Skill / KnowledgeBase）；
//! - `SkillView` 面向**文件系统里 SKILL.md 的正文**。两者不互相替代。
//!
//! 分层合规：tools 是 hybrid 角色，可依赖 harness 的 trait 与自身实现；
//! 共享依赖按项目既有惯例经 `OnceLock` + setter 注入（`ToolContext` 不是 DI 容器）。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolErrorKind, ToolResult};
use async_trait::async_trait;
use axagent_harness::CapabilityIndexer;
use axagent_harness::error_codes::capability::NOT_FOUND as CAPABILITY_NOT_FOUND;
use serde_json::{Value, json};
use std::sync::{Arc, OnceLock};

pub(crate) static CAPABILITY_INDEXER: OnceLock<Arc<dyn CapabilityIndexer>> = OnceLock::new();

/// 注入 `CapabilityIndexer` trait object（wiring 层初始化时调用一次）
pub fn set_capability_indexer(indexer: Arc<dyn CapabilityIndexer>) {
    let _ = CAPABILITY_INDEXER.set(indexer);
}

pub struct CapabilityViewTool;

#[async_trait]
impl Tool for CapabilityViewTool {
    fn name(&self) -> &str {
        "CapabilityView"
    }

    fn description(&self) -> &str {
        "按需展开某个能力的完整定义（渐进式披露 L1 — 定义层）。\
         入参 capability_id 取自系统提示的 <capability-index> 目录。\
         返回入参 schema、SOP 步骤、前置条件、附带知识、工具引用与工具链步骤等全部细节。\
         注意：查看文件系统里的 SKILL.md 正文请用 SkillView，本工具面向能力护照全集。"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "capability_id": {
                    "type": "string",
                    "description": "要展开的能力 ID（来自能力目录）"
                }
            },
            "required": ["capability_id"]
        })
    }

    /// Knowledge 属只读类别，`is_read_only` 由 `category().is_read_only()` 派生，
    /// 默认组 `builtin-knowledge` 也已在组表里存在 —— 无需改动权限白名单。
    fn category(&self) -> ToolCategory {
        ToolCategory::Knowledge
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let capability_id = input["capability_id"].as_str().unwrap_or("").trim();
        if capability_id.is_empty() {
            return Err(ToolError::invalid_input_for("CapabilityView", "capability_id 为必填参数"));
        }

        let Some(indexer) = CAPABILITY_INDEXER.get() else {
            return Err(not_found(format!("能力索引器尚未初始化，无法展开 {capability_id}")));
        };

        let Some(passport) = indexer.get_passport(capability_id).await else {
            return Err(not_found(format!(
                "能力 '{capability_id}' 未在索引中。请核对 <capability-index> 里的 id，\
                 或用 DiscoverSkills 检索。"
            )));
        };

        // 与索引层同口径：不可见的能力既不进目录，也不能被直接展开
        if !passport.is_user_visible() {
            return Err(ToolError {
                message: format!("能力 '{capability_id}' 不对当前上下文公开"),
                kind: ToolErrorKind::PermissionDenied,
                error_code: CAPABILITY_NOT_FOUND.to_string(),
            });
        }

        let view = json!({
            "capabilityId": passport.capability_id,
            "name": passport.name,
            "description": passport.description,
            "kind": passport.kind,
            "domain": passport.domain.as_str(),
            "subCategory": passport.sub_category,
            "level": passport.level,
            "exposure": passport.exposure,
            "securityLevel": passport.security_level,
            "version": passport.version,
            "owner": passport.owner,
            "agentProfileId": passport.agent_profile_id,
            "aliases": passport.aliases,
            "tags": passport.tags,
            "negativeScenarios": passport.negative_scenarios,
            "preconditions": passport.preconditions,
            "executionMode": passport.execution_mode,
            "timeoutMs": passport.timeout_ms,
            "inputSchema": passport.input_schema,
            "outputSchema": passport.output_schema,
            "implementation": passport.implementation,
            "toolRef": passport.tool_ref,
            "steps": passport.steps,
            "skillSteps": passport.skill_steps,
            "attachedSnippets": passport.attached_snippets,
            "templateBody": passport.template_body,
            "placeholders": passport.placeholders,
            "instantiatesTo": passport.instantiates_to,
            "exampleInstance": passport.example_instance,
            "upstream": passport.upstream,
            "downstream": passport.downstream
        });

        let body = serde_json::to_string_pretty(&view)
            .unwrap_or_else(|_| format!("能力 '{capability_id}' 定义序列化失败"));
        Ok(ToolResult::success(body))
    }
}

fn not_found(message: String) -> ToolError {
    ToolError { message, kind: ToolErrorKind::NotFound, error_code: CAPABILITY_NOT_FOUND.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_carries_capability_error_code() {
        let err = not_found("能力不存在".to_string());
        assert_eq!(err.kind, ToolErrorKind::NotFound);
        assert_eq!(err.error_code, CAPABILITY_NOT_FOUND);
    }

    #[test]
    fn category_is_read_only_knowledge() {
        let tool = CapabilityViewTool;
        assert_eq!(tool.category(), ToolCategory::Knowledge);
        assert!(
            tool.category().is_read_only(),
            "测试：CapabilityView 必须只读，否则编排阶段可借它产生副作用"
        );
        assert_eq!(tool.category().default_group(), "builtin-knowledge");
    }
}
