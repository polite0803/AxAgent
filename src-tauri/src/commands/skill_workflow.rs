// SPDX-License-Identifier: AGPL-3.0-only

//! 技能分解工作流保存（v126）—— 补齐前端 `workflowEditorStore.saveSkillWorkflowFromLlm` 的后端。
//!
//! 功能：把 LLM 技能分解产出的节点/边持久化为工作流模板（`workflow_templates`），
//! tags 标记 `skill-workflow` 与来源技能，便于技能库检索与后续去重审查。
//!
//! 响应契约对齐前端 `SaveSkillWorkflowResponse`：
//! `{ needsReview, workflowId, similarWorkflows: [{ workflowId, name, skillIds, similarity }] }`。
//! 存在相似度 ≥ 0.6 的既有模板时返回 `needsReview = true` 并不落库；
//! 调用方确认后带 `forceSave = true` 重试即可强制保存。

use axagent_agent_macro::agent_command;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::AppState;
use crate::commands::error::ErrorResponse;

/// 保存请求（对齐前端 `saveSkillWorkflowFromLlm` 传参，camelCase）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSkillWorkflowRequest {
    pub skill_id: String,
    pub skill_name: String,
    pub workflow_name: String,
    pub description: Option<String>,
    pub nodes: serde_json::Value,
    pub edges: serde_json::Value,
    /// 相似审查确认后强制保存。首次调用不传（false）。
    #[serde(default)]
    pub force_save: bool,
}

/// 相似工作流摘要（对齐前端 `SimilarWorkflow`：workflowId / name / skillIds / similarity）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarSkillWorkflow {
    pub workflow_id: String,
    pub name: String,
    pub skill_ids: Vec<String>,
    pub similarity: f64,
}

/// 保存响应（对齐前端 `SaveSkillWorkflowResponse`）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSkillWorkflowResponse {
    pub needs_review: bool,
    pub workflow_id: Option<String>,
    pub similar_workflows: Vec<SimilarSkillWorkflow>,
}

fn err(e: impl std::fmt::Display) -> String {
    String::from(ErrorResponse::from_error(e, crate::commands::error::ErrorCategory::Unrecoverable))
}

#[tauri::command]
#[agent_command(
    domain = workflow,
    safety = Caution,
    call_mode = StateInput,
    description = "把 LLM 技能分解结果保存为工作流模板（相似审查 + 落库）"
)]
pub async fn save_skill_workflow_from_llm(
    state: State<'_, AppState>,
    request: SaveSkillWorkflowRequest,
) -> Result<SaveSkillWorkflowResponse, String> {
    use axagent_harness::workflow_types::WorkflowNode;

    if request.workflow_name.trim().is_empty() {
        return Err(String::from(ErrorResponse::from_error(
            "工作流名称不能为空",
            crate::commands::error::ErrorCategory::Validation,
        )));
    }
    let db = state.harness.db();

    // 节点反序列化失败不阻断保存 —— 相似检查是增强能力，保存是主流程。
    let nodes: Vec<WorkflowNode> = match serde_json::from_value(request.nodes.clone()) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[save_skill_workflow] 节点反序列化失败，跳过相似检查: {e}");
            Vec::new()
        },
    };

    // 相似审查：Jaccard ≥ 0.6 视为相似（与 create_workflow_template 同一实现）。
    let similar = if nodes.is_empty() {
        Vec::new()
    } else {
        crate::commands::workflow_template::find_similar_workflows(db, &nodes).await.map_err(err)?
    };

    let similar_infos: Vec<SimilarSkillWorkflow> = similar
        .iter()
        .map(|s| SimilarSkillWorkflow {
            workflow_id: s.workflow_id.clone(),
            name: s.name.clone(),
            // 既有 SimilarWorkflow 无 skill 来源信息，保留扩展位
            skill_ids: Vec::new(),
            similarity: s.similarity,
        })
        .collect();

    if !similar_infos.is_empty() && !request.force_save {
        return Ok(SaveSkillWorkflowResponse {
            needs_review: true,
            workflow_id: None,
            similar_workflows: similar_infos,
        });
    }

    // 落库为工作流模板：tags 标记技能来源，与普通模板同表存储、同一套
    // list/get/update 通道复用，不引入第二套持久化。
    let now = chrono::Utc::now().timestamp_millis();
    let id = uuid::Uuid::new_v4().to_string();
    let tags = serde_json::to_string(&vec![
        "skill-workflow".to_string(),
        request.skill_id.clone(),
        request.skill_name.clone(),
    ])
    .unwrap_or_default();

    let active = axagent_entities::workflow_template::ActiveModel {
        id: Set(id.clone()),
        name: Set(request.workflow_name.clone()),
        description: Set(request.description.clone()),
        icon: Set(String::new()),
        tags: Set(Some(tags)),
        version: Set(1),
        is_preset: Set(false),
        is_editable: Set(true),
        is_public: Set(false),
        trigger_config: Set(None),
        nodes: Set(serde_json::to_string(&request.nodes).unwrap_or_default()),
        edges: Set(serde_json::to_string(&request.edges).unwrap_or_default()),
        input_schema: Set(None),
        output_schema: Set(None),
        variables: Set(Some("[]".to_string())),
        error_config: Set(None),
        composite_source: Set(None),
        tool_defs: Set(None),
        mission_hash: Set(None),
        cluster_id: Set(None),
        route_path: Set(None),
        hooks_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    axagent_dao::repo::workflow_template::insert_workflow_template(db, active)
        .await
        .map_err(err)?;

    // 回灌能力索引：技能转工作流产物是新模板，不索引则本会话内不可路由。
    // 此处手上是 entity ActiveModel，故按 ID 从库里读回完整模型再派生护照。
    crate::commands::workflow_template::sync_template_index_by_id(&state, &id).await;

    tracing::info!(
        "[save_skill_workflow] 已保存技能工作流: id={id} skill={} name={}",
        request.skill_id,
        request.workflow_name
    );

    Ok(SaveSkillWorkflowResponse {
        needs_review: false,
        workflow_id: Some(id),
        similar_workflows: Vec::new(),
    })
}
