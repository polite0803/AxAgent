// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流反思 / 进化 / 优化命令层(阶段 5:wiring)
//!
//! 暴露 trajectory crate 三个 Impl(WorkflowReflector / WorkflowEvolver / WorkflowOptimizer)
//! 的能力为 Tauri 命令,供前端手动触发:
//! - 反思由 WorkEngine 钩子(阶段 3)自动触发,本模块不暴露手动反思命令(输入复杂,前端构造 `WorkflowExecutionRecord` 不现实)
//! - 暴露优化器命令:基于历史反思生成 / 应用建议
//! - 暴露进化器命令:基于反思批量触发模板进化,查询进化统计与运行状态
//!
//! 错误处理:统一通过 `ErrorResponse::from_error` 包装,前端通过 `t("error.${code}")` 翻译。
//! 参考实现:`commands/workflow_ai_diagnose.rs`。

use crate::AppState;
use axagent_harness::reflection_types::Reflection;
use axagent_harness::workflow_evolution::{EvolutionStats, WorkflowModification};
use axagent_harness::workflow_optimization::WorkflowSuggestion;
use axagent_harness::workflow_types::WorkflowTemplateData;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::error::{ErrorCategory, ErrorResponse};

// ── 命令请求 / 响应 DTO ──

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowOptimizeSuggestRequest {
    pub template: WorkflowTemplateData,
    pub reflection: Reflection,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowOptimizeApplyRequest {
    pub template: WorkflowTemplateData,
    pub suggestions: Vec<WorkflowSuggestion>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowEvolveRequest {
    pub template_id: String,
    pub reflections: Vec<Reflection>,
}

// ── 命令实现 ──

/// 基于单次反思生成工作流优化建议。
///
/// 输入:工作流模板 + 反思结果(由 WorkEngine 钩子产生并持久化,前端从 trajectory_storage 查询)。
/// 输出:`Vec<WorkflowSuggestion>`,前端可展示或选择性 apply。
#[tauri::command]
pub async fn workflow_optimize_suggest(
    state: State<'_, AppState>,
    request: WorkflowOptimizeSuggestRequest,
) -> Result<Vec<WorkflowSuggestion>, String> {
    state
        .workflow_optimizer
        .suggest(&request.template, &request.reflection)
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

/// 批量应用优化建议到模板,返回新模板(不修改原模板)。
///
/// 调用方决定是否持久化:可由人工审核后调用 `workflow_template::update_workflow_template`,
/// 或由 wiring 层自动应用(需配置 auto_apply_threshold,此处 MVP 不自动应用)。
#[tauri::command]
pub async fn workflow_optimize_apply(
    state: State<'_, AppState>,
    request: WorkflowOptimizeApplyRequest,
) -> Result<WorkflowTemplateData, String> {
    state
        .workflow_optimizer
        .apply_suggestions(&request.template, &request.suggestions)
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

/// 触发工作流模板进化(基于反思批量进化,返回最终修改结果)。
///
/// 建议在前端以异步任务形式调用(可能耗时较长),返回 `WorkflowModification` 含:
/// - evolved genome(进化后的工作流基因组)
/// - changes(变异操作列表)
/// - validation(沙箱验证结果,MVP 不实际验证)
#[tauri::command]
pub async fn workflow_evolve_template(
    state: State<'_, AppState>,
    request: WorkflowEvolveRequest,
) -> Result<WorkflowModification, String> {
    state
        .workflow_evolver
        .run(&request.template_id, &request.reflections)
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

/// 查询工作流进化器的统计信息(当前代数、最佳 / 平均适应度、是否收敛)。
#[tauri::command]
pub async fn workflow_evolution_stats(
    state: State<'_, AppState>,
) -> Result<EvolutionStats, String> {
    state
        .workflow_evolver
        .get_stats()
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

/// 查询进化器是否正在执行(用于前端防重入)。
#[tauri::command]
pub async fn workflow_evolution_is_running(state: State<'_, AppState>) -> Result<bool, String> {
    state
        .workflow_evolver
        .is_running()
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

/// 查询是否应自动触发进化(基于近期失败率与使用次数,阈值由 `EvolutionConfig` 配置)。
///
/// 注意:`should_auto_evolve` 依赖 evolver 内部的 `recent_reflections` 历史,
/// 该历史目前由 wiring 层在 WorkEngine 反思钩子中调用 `record_reflection` 写入。
/// 若该机制未启用,本命令始终返回 false。
#[tauri::command]
pub async fn workflow_should_auto_evolve(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<bool, String> {
    state
        .workflow_evolver
        .should_auto_evolve(&template_id)
        .await
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))
}

// ── 单元测试 ──
//
// 命令层是薄包装,核心逻辑在 trajectory crate 已测试(19 个单元测试覆盖)。
// 此处不重复集成测试,仅验证 DTO 序列化 / 反序列化往返。

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolve_request_roundtrip() {
        let req = WorkflowEvolveRequest {
            template_id: "wf-1".to_string(),
            reflections: vec![
                Reflection::new("exec-1".to_string()),
                Reflection::new("exec-2".to_string()),
            ],
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: WorkflowEvolveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.template_id, "wf-1");
        assert_eq!(back.reflections.len(), 2);
    }

    #[test]
    fn test_apply_request_roundtrip() {
        // 空模板 + 空建议列表,仅验证序列化往返
        let template = WorkflowTemplateData {
            id: "wf-empty".to_string(),
            name: "Empty".to_string(),
            description: None,
            icon: String::new(),
            tags: Vec::new(),
            version: 1,
            is_preset: false,
            is_editable: true,
            is_public: false,
            trigger_config: None,
            nodes: Vec::new(),
            edges: Vec::new(),
            input_schema: None,
            output_schema: None,
            variables: Vec::new(),
            error_config: None,
            error_workflow_id: None,
            tool_defs: Vec::new(),
            mission_hash: None,
            created_at: 0,
            updated_at: 0,
        };
        let req = WorkflowOptimizeApplyRequest { template, suggestions: Vec::new() };
        let json = serde_json::to_string(&req).unwrap();
        let back: WorkflowOptimizeApplyRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.template.id, "wf-empty");
        assert!(back.suggestions.is_empty());
    }
}
