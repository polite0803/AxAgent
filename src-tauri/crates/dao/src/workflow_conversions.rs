// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流模板转换函数
//!
//! 将 `axagent_entities::workflow_template::Model`（SeaORM 模型）
//! 转换为 `axagent_harness::workflow_types::WorkflowTemplateResponse`。
//!
//! 注意：不能使用 `impl From<Model> for WorkflowTemplateResponse`，
//! 因为两个类型都不属于本 crate（E0117 orphan rule），故使用自由函数。

use axagent_entities::workflow_template;
use axagent_harness::workflow_types::{
    ErrorConfig, JsonSchema, TriggerConfig, Variable, WorkflowEdge, WorkflowNode,
    WorkflowTemplateResponse,
};

/// 将 SeaORM `workflow_template::Model` 转换为 `WorkflowTemplateResponse`。
///
/// 返回的 `tool_defs` 字段固定为 `None`（当前模型不存储该字段）。
pub fn workflow_template_response_from_model(
    model: workflow_template::Model,
) -> WorkflowTemplateResponse {
    let tags: Vec<String> =
        model.tags.as_ref().and_then(|t| serde_json::from_str(t).ok()).unwrap_or_default();

    let trigger_config: Option<TriggerConfig> =
        model.trigger_config.as_ref().and_then(|t| serde_json::from_str(t).ok());

    let nodes: Vec<WorkflowNode> = match serde_json::from_str(&model.nodes) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "[workflow_conversions] nodes 反序列化失败 template_id={}: {} | 前200字符: {}",
                model.id,
                e,
                &model.nodes[..model.nodes.len().min(500)]
            );
            Vec::new()
        },
    };
    let edges: Vec<WorkflowEdge> = match serde_json::from_str(&model.edges) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "[workflow_conversions] edges 反序列化失败 template_id={}: {} | 前200字符: {}",
                model.id,
                e,
                &model.edges[..model.edges.len().min(500)]
            );
            Vec::new()
        },
    };
    let input_schema: Option<JsonSchema> =
        model.input_schema.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let output_schema: Option<JsonSchema> =
        model.output_schema.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let variables_vec: Vec<Variable> =
        model.variables.as_ref().and_then(|v| serde_json::from_str(v).ok()).unwrap_or_default();
    let error_config: Option<ErrorConfig> =
        model.error_config.as_ref().and_then(|e| serde_json::from_str(e).ok());

    // 系统模板判定复用 harness 权威方法（is_preset + cognitive_router 标签），
    // 前端据此区分系统模板页与业务模板页。
    let is_system =
        crate::repo::workflow_template::template_model_to_data(&model).is_system_template();

    WorkflowTemplateResponse {
        id: model.id,
        name: model.name,
        description: model.description,
        icon: model.icon,
        tags,
        version: model.version,
        is_preset: model.is_preset,
        is_editable: model.is_editable,
        is_public: model.is_public,
        is_system,
        trigger_config,
        nodes,
        edges,
        input_schema,
        output_schema,
        variables: variables_vec,
        error_config,
        tool_defs: None,
        mission_hash: model.mission_hash,
        cluster_id: model.cluster_id,
        route_path: model.route_path,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}
