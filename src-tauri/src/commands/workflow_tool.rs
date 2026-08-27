// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流运行时工具命令 —— workflow_tools 表的 CRUD 与启停。
//!
//! 用途：
//! - 运行时发现/LLM 生成的工具写回持久化（pending → active 审批流）
//! - 前端工具面板查看/新增/启停工具
//! - 执行反馈回写成功率（支撑工具进化证据）

use crate::AppState;
use crate::commands::error::{ErrorCategory, ErrorResponse};
use crate::commands::error_code::workflow as workflow_err;
use axagent_agent_macro::agent_command;
use axagent_dao::repo::workflow_tool as db_repo;
use axagent_harness::workflow_types::WorkflowToolResponse;
use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertWorkflowToolInput {
    /// 工具归属工作流（必填）
    pub workflow_id: String,
    /// 工具名（运行时注册名，工作流内唯一）
    pub tool_name: String,
    /// rhai_script | workflow_dag | llm_function
    #[serde(default = "default_tool_type")]
    pub tool_type: String,
    pub description: Option<String>,
    /// 实现体：Rhai 源码 / DAG JSON / LLM 函数定义
    pub code: Option<String>,
    /// 输入 JSON Schema（字符串）
    #[serde(rename = "inputSchema")]
    pub input_schema: Option<String>,
    /// 来源标记
    #[serde(default = "default_source")]
    pub source: String,
    /// pending | active | disabled
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_tool_type() -> String {
    db_repo::TYPE_RHAI_SCRIPT.to_string()
}

fn default_source() -> String {
    "manual".to_string()
}

fn default_status() -> String {
    db_repo::STATUS_ACTIVE.to_string()
}

/// entities Model → harness DTO 转换（harness 不依赖 entities，转换在应用层）
fn to_response(m: axagent_entities::workflow_tools::Model) -> WorkflowToolResponse {
    WorkflowToolResponse {
        id: m.id,
        workflow_id: m.workflow_id,
        tool_name: m.tool_name,
        tool_type: m.tool_type,
        description: m.description,
        code: m.code,
        input_schema: m.input_schema,
        source: m.source,
        status: m.status,
        usage_count: m.usage_count,
        success_rate: m.success_rate,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn err(e: impl std::fmt::Display) -> String {
    String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable))
}

/// 列出工作流的所有运行时工具（可按状态过滤）
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "列出工作流的运行时工具")]
#[tauri::command]
pub async fn list_workflow_tools(
    state: State<'_, AppState>,
    workflow_id: String,
    status: Option<String>,
) -> Result<Vec<WorkflowToolResponse>, String> {
    let db = state.harness.db();
    let tools =
        db_repo::list_by_workflow(db, &workflow_id, status.as_deref()).await.map_err(err)?;
    Ok(tools.into_iter().map(to_response).collect())
}

/// 新增/覆盖工作流运行时工具（(workflow_id, tool_name) 冲突时更新定义、保留统计）
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "新增或覆盖工作流运行时工具")]
#[tauri::command]
pub async fn upsert_workflow_tool(
    state: State<'_, AppState>,
    input: UpsertWorkflowToolInput,
) -> Result<WorkflowToolResponse, String> {
    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp_millis();

    // 幂等键：同工作流内工具名稳定映射到同一 id，跨工作流各自独立
    let existing =
        db_repo::get_by_name(db, &input.workflow_id, &input.tool_name).await.map_err(err)?;
    let id = existing.as_ref().map(|t| t.id.clone()).unwrap_or_else(|| Uuid::new_v4().to_string());

    db_repo::upsert(
        db,
        &id,
        &input.workflow_id,
        &input.tool_name,
        &input.tool_type,
        input.description.as_deref(),
        input.code.as_deref(),
        input.input_schema.as_deref(),
        &input.source,
        &input.status,
        now,
    )
    .await
    .map_err(err)?;

    let model = db_repo::get_by_id(db, &id).await.map_err(err)?;
    model.map(to_response).ok_or_else(|| err("workflow tool not found after upsert"))
}

/// 启停工具（pending → active / active → disabled）
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "启停工作流运行时工具")]
#[tauri::command]
pub async fn update_workflow_tool_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<bool, String> {
    if !matches!(status.as_str(), "pending" | "active" | "disabled") {
        return Err(String::from(
            ErrorResponse::new(workflow_err::TOOL_INVALID_STATUS)
                .with_category(ErrorCategory::Unrecoverable),
        ));
    }
    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp_millis();
    db_repo::update_status(db, &id, &status, now).await.map_err(err)
}

/// 删除工具（工作流删除时前端可先行级联调用）
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "删除工作流运行时工具")]
#[tauri::command]
pub async fn delete_workflow_tool(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let db = state.harness.db();
    db_repo::delete(db, &id).await.map_err(err)
}

/// 记录一次真实执行反馈（成功/失败），回写 usage_count 与 success_rate。
/// 由运行时执行回调在工具真实执行后调用；非真实执行不上报。
#[agent_command(domain = workflow, safety = Safe, call_mode = StateInput, description = "记录工作流工具执行反馈")]
#[tauri::command]
pub async fn record_workflow_tool_feedback(
    state: State<'_, AppState>,
    id: String,
    success: bool,
) -> Result<bool, String> {
    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp_millis();
    db_repo::record_execution_feedback(db, &id, success, now).await.map_err(err)
}

/// LLM 生成工作流工具（发现闭环）—— 描述需求 → LLM 生成 Rhai 脚本 →
/// 沙箱编译验证 → 写入 workflow_tools(pending，待人工确认启用)。
///
/// 安全红线：生成产物仅写入 pending 状态，不自动注册；用户在前端工具面板
/// 确认（status → active）后才在下次工作流启动时注册生效。
#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "LLM 生成工作流运行时工具")]
#[tauri::command]
pub async fn generate_workflow_tool(
    state: State<'_, AppState>,
    workflow_id: String,
    description: String,
    available_tools: Option<Vec<String>>,
) -> Result<WorkflowToolResponse, String> {
    // 1. 构造 LLM 工具生成器（未配置 provider 时返回明确错误）
    let provider =
        crate::init::llm_providers::build_llm_tool_provider_from_db(state.harness.master_key())
            .await
            .ok_or_else(|| {
                String::from(
                    ErrorResponse::new(workflow_err::TOOL_PROVIDER_NOT_CONFIGURED)
                        .with_category(ErrorCategory::Retryable),
                )
            })?;

    // 2. LLM 生成 Rhai 脚本工具（纯计算约束由 ProviderLlmBridge 系统提示保证）
    let request = axagent_harness::trajectory_types::ToolCreationRequest::new(
        &description,
        &format!("为工作流 {workflow_id} 生成一个可复用的计算工具"),
        available_tools.unwrap_or_default(),
    );
    let generated = provider.generate_tool_code(&request).await.map_err(|e| {
        tracing::warn!("[workflow_tool] LLM 生成工具失败: {e}");
        String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable))
    })?;

    // 3. 沙箱编译验证：Rhai 引擎编译不通过 → 拒绝落地（不写库）
    let engine = axagent_tools::rhai_engine::create_rhai_engine();
    axagent_tools::rhai_engine::compile_script(&engine, &generated.code).map_err(|e| {
        tracing::warn!("[workflow_tool] 生成工具沙箱编译失败: {e}");
        String::from(
            ErrorResponse::new(workflow_err::TOOL_SANDBOX_REJECTED)
                .with_category(ErrorCategory::Unrecoverable),
        )
    })?;

    // 4. 写入 workflow_tools（pending 状态，source=ai_generated）
    let db = state.harness.db();
    let now = chrono::Utc::now().timestamp_millis();
    db_repo::upsert(
        db,
        &Uuid::new_v4().to_string(),
        &workflow_id,
        &generated.name,
        db_repo::TYPE_RHAI_SCRIPT,
        Some(&generated.description),
        Some(&generated.code),
        None,
        "ai_generated",
        db_repo::STATUS_PENDING,
        now,
    )
    .await
    .map_err(err)?;

    let model = db_repo::get_by_name(db, &workflow_id, &generated.name)
        .await
        .map_err(err)?
        .ok_or_else(|| err("workflow tool not found after generate"))?;
    Ok(to_response(model))
}
