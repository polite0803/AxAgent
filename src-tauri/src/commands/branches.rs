// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_harness::types::{BranchComparison, ConversationBranch, WorkspaceSnapshot};
use tauri::State;

use crate::commands::error::{ErrorCategory, ErrorResponse};

/// 把 harness 错误转换为 String(Tauri command 返回类型要求)。
fn err_to_string(e: axagent_harness::core_error::AxAgentError) -> String {
    String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable))
}

#[tauri::command]
pub async fn list_branches(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<ConversationBranch>, String> {
    axagent_dao::repo::conversation_branch::list_branches(state.harness.db(), &conversation_id)
        .await
        .map_err(err_to_string)
}

#[tauri::command]
pub async fn fork_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
    message_id: String,
) -> Result<ConversationBranch, String> {
    axagent_dao::repo::conversation_branch::create_branch(
        state.harness.db(),
        &conversation_id,
        &message_id,
        "Branch",
    )
    .await
    .map_err(err_to_string)
}

/// 对比两个分支的消息差异。
///
/// 返回 `BranchComparison`,包含:
/// - `common_prefix`:两条分支共享的前缀消息(从会话起点到分叉点)
/// - `only_in_a` / `only_in_b`:仅在某条分支中存在的消息
/// - `diverge_at`:分叉点消息 ID
#[tauri::command]
pub async fn compare_branches(
    state: State<'_, AppState>,
    branch_a: String,
    branch_b: String,
) -> Result<BranchComparison, String> {
    axagent_dao::repo::conversation_branch::compare_branches(
        state.harness.db(),
        &branch_a,
        &branch_b,
    )
    .await
    .map_err(err_to_string)
}

/// 读取会话工作区快照。
///
/// 返回的 `WorkspaceSnapshot` 包含:
/// - `context_sources` / `active_tools` / `knowledge_bindings` /
///   `memory_policy` / `search_policy` / `artifacts`:从
///   `conversations.workspace_snapshot_json` 反序列化
/// - `branches`:从 `conversation_branches` 表实时拼装
/// - `active_branch_id`:从 `conversations.active_branch_id` 读取
#[tauri::command]
pub async fn get_workspace_snapshot(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<WorkspaceSnapshot, String> {
    let db = state.harness.db();

    // 1. 读取持久化的 snapshot JSON
    let raw_json =
        axagent_dao::repo::conversation::get_workspace_snapshot_json(db, &conversation_id)
            .await
            .map_err(err_to_string)?;

    // 2. 反序列化为 WorkspaceSnapshot(空 JSON "{}" 也能反序列化为默认值)
    let mut snapshot: WorkspaceSnapshot = serde_json::from_str(&raw_json).map_err(|e| {
        String::from(ErrorResponse::from_error(
            axagent_harness::core_error::AxAgentError::Validation(format!(
                "Invalid workspace_snapshot_json: {e}"
            )),
            ErrorCategory::Unrecoverable,
        ))
    })?;

    // 3. 实时拼装 branches 列表
    snapshot.branches = axagent_dao::repo::conversation_branch::list_branches(db, &conversation_id)
        .await
        .map_err(err_to_string)?;

    // 4. 实时读取 active_branch_id(以数据库为准,JSON 中的值仅作参考)
    snapshot.active_branch_id =
        axagent_dao::repo::conversation::get_active_branch_id(db, &conversation_id)
            .await
            .map_err(err_to_string)?;

    Ok(snapshot)
}

/// 更新会话工作区快照。
///
/// 调用方传入完整的 `WorkspaceSnapshot`,本命令会:
/// 1. 清空 `branches` 字段(分支列表由 `conversation_branches` 表实时拼装,
///    持久化进 JSON 会与表数据脱节)
/// 2. 序列化剩余字段为 JSON,写入 `conversations.workspace_snapshot_json`
/// 3. 若 `active_branch_id` 字段不为 None,同步更新 `conversations.active_branch_id`
#[tauri::command]
pub async fn update_workspace_snapshot(
    state: State<'_, AppState>,
    conversation_id: String,
    snapshot: WorkspaceSnapshot,
) -> Result<(), String> {
    let db = state.harness.db();

    // 1. 取出 active_branch_id,单独走字段更新
    let active_branch_id = snapshot.active_branch_id.clone();

    // 2. 清空 branches 后序列化(branches 由表实时拼装,不持久化)
    let mut snapshot_to_persist = snapshot.clone();
    snapshot_to_persist.branches = Vec::new();
    // active_branch_id 已单独走字段,JSON 中也清空避免冗余
    snapshot_to_persist.active_branch_id = None;

    let json = serde_json::to_string(&snapshot_to_persist).map_err(|e| {
        String::from(ErrorResponse::from_error(
            axagent_harness::core_error::AxAgentError::Validation(format!(
                "Failed to serialize WorkspaceSnapshot: {e}"
            )),
            ErrorCategory::Unrecoverable,
        ))
    })?;

    // 3. 更新 workspace_snapshot_json
    axagent_dao::repo::conversation::update_workspace_snapshot_json(db, &conversation_id, &json)
        .await
        .map_err(err_to_string)?;

    // 4. 更新 active_branch_id(若提供)
    if let Some(branch_id) = active_branch_id {
        axagent_dao::repo::conversation::set_active_branch_id(
            db,
            &conversation_id,
            Some(&branch_id),
        )
        .await
        .map_err(err_to_string)?;
    }

    Ok(())
}
