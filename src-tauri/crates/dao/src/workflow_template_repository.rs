// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::Set;

use axagent_entities::workflow_template;
use axagent_harness::WorkflowTemplateRepo;
use axagent_harness::capability::Visibility;
use axagent_harness::repo_dtos::WorkflowTemplateData;
use axagent_harness::repositories::WorkflowTemplateRepository;
use axagent_harness::workflow_types::WorkflowTemplateData as Wtd;
use axagent_harness::workflow_types::{
    ErrorConfig, JsonSchema, RhaiToolDef, TriggerConfig, Variable, WorkflowEdge, WorkflowNode,
};

use crate::repo::workflow_template as repo_wt;

pub struct DaoWorkflowTemplateRepository {
    pub db: Arc<sea_orm::DatabaseConnection>,
}

// ── 旧 trait: repo_dtos::WorkflowTemplateData（JSON String） ──

fn data_to_active_model(d: WorkflowTemplateData) -> workflow_template::ActiveModel {
    workflow_template::ActiveModel {
        id: Set(d.id.clone()),
        name: Set(d.name),
        description: Set(d.description),
        icon: Set(d.icon),
        tags: Set(d.tags),
        version: Set(d.version),
        is_preset: Set(d.is_preset),
        is_editable: Set(d.is_editable),
        is_public: Set(d.is_public),
        trigger_config: Set(d.trigger_config),
        nodes: Set(d.nodes),
        edges: Set(d.edges),
        input_schema: Set(d.input_schema),
        output_schema: Set(d.output_schema),
        variables: Set(d.variables),
        error_config: Set(d.error_config),
        composite_source: Set(None),
        tool_defs: Set(None),
        mission_hash: Set(None),
        cluster_id: Set(d.cluster_id),
        route_path: Set(d.route_path),
        created_at: Set(axagent_harness::util_fns::now_ms()),
        updated_at: Set(axagent_harness::util_fns::now_ms()),
    }
}

#[async_trait]
impl WorkflowTemplateRepository for DaoWorkflowTemplateRepository {
    async fn get_workflow_template(
        &self,
        id: &str,
    ) -> Result<Option<WorkflowTemplateData>, String> {
        let model =
            repo_wt::get_workflow_template(&self.db, id).await.map_err(|e| e.to_string())?;
        Ok(model.map(|m| WorkflowTemplateData {
            id: m.id,
            name: m.name,
            description: m.description,
            icon: m.icon,
            cluster_id: None,
            route_path: None,
            tags: m.tags,
            version: m.version,
            is_preset: m.is_preset,
            is_editable: m.is_editable,
            is_public: m.is_public,
            trigger_config: m.trigger_config,
            nodes: m.nodes,
            edges: m.edges,
            input_schema: m.input_schema,
            output_schema: m.output_schema,
            variables: m.variables,
            error_config: m.error_config,
        }))
    }

    async fn create_workflow_template(
        &self,
        template: WorkflowTemplateData,
    ) -> Result<String, String> {
        let id = template.id.clone();
        let active = data_to_active_model(template);
        repo_wt::upsert_workflow_template(&self.db, active).await.map_err(|e| e.to_string())?;
        Ok(id)
    }

    async fn update_workflow_template(&self, template: WorkflowTemplateData) -> Result<(), String> {
        let active = data_to_active_model(template);
        repo_wt::upsert_workflow_template(&self.db, active).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 新 trait: axagent_harness::WorkflowTemplateRepo
//
// 用于 Phase 5 WorkflowEvolutionTick 后台 tick。
// 消费方 trajectory crate 需要的是 workflow_types::WorkflowTemplateData
// （nodes/edges 为 Vec 结构化类型,便于 WorkflowOptimizer 直接遍历）。
// ═══════════════════════════════════════════════════════════════════

// ── Entity::Model → workflow_types::WorkflowTemplateData ──
// (不用 impl From,避免 orphan rule:两个类型都不在本 crate)

fn model_to_wtd(m: workflow_template::Model) -> Wtd {
    let tags: Vec<String> =
        m.tags.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();

    let nodes: Vec<WorkflowNode> = serde_json::from_str(&m.nodes).unwrap_or_default();

    let edges: Vec<WorkflowEdge> = serde_json::from_str(&m.edges).unwrap_or_default();

    let trigger_config: Option<TriggerConfig> =
        m.trigger_config.as_deref().and_then(|s| serde_json::from_str(s).ok());

    let input_schema: Option<JsonSchema> =
        m.input_schema.as_deref().and_then(|s| serde_json::from_str(s).ok());

    let output_schema: Option<JsonSchema> =
        m.output_schema.as_deref().and_then(|s| serde_json::from_str(s).ok());

    let variables: Vec<Variable> =
        m.variables.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();

    let error_config: Option<ErrorConfig> =
        m.error_config.as_deref().and_then(|s| serde_json::from_str(s).ok());

    let tool_defs: Vec<RhaiToolDef> =
        m.tool_defs.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();

    Wtd {
        id: m.id,
        name: m.name,
        description: m.description,
        icon: m.icon,
        tags,
        version: m.version,
        is_preset: m.is_preset,
        is_editable: m.is_editable,
        is_public: m.is_public,
        visibility: Visibility::Public,
        trigger_config,
        nodes,
        edges,
        input_schema,
        output_schema,
        variables,
        error_config,
        error_workflow_id: None,
        tool_defs,
        mission_hash: m.mission_hash,
        cluster_id: m.cluster_id,
        route_path: m.route_path,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

// ── workflow_types::WorkflowTemplateData → Entity::ActiveModel ──

fn wtd_to_active_model(t: Wtd) -> workflow_template::ActiveModel {
    workflow_template::ActiveModel {
        id: Set(t.id.clone()),
        name: Set(t.name),
        description: Set(t.description),
        icon: Set(t.icon),
        tags: Set(serde_json::to_string(&t.tags).ok()),
        version: Set(t.version),
        is_preset: Set(t.is_preset),
        is_editable: Set(t.is_editable),
        is_public: Set(t.is_public),
        trigger_config: Set(t.trigger_config.and_then(|c| serde_json::to_string(&c).ok())),
        nodes: Set(serde_json::to_string(&t.nodes).unwrap_or_default()),
        edges: Set(serde_json::to_string(&t.edges).unwrap_or_default()),
        input_schema: Set(t.input_schema.and_then(|s| serde_json::to_string(&s).ok())),
        output_schema: Set(t.output_schema.and_then(|s| serde_json::to_string(&s).ok())),
        variables: Set(Some(serde_json::to_string(&t.variables).unwrap_or_default())),
        error_config: Set(t.error_config.and_then(|e| serde_json::to_string(&e).ok())),
        composite_source: Set(None),
        tool_defs: Set(Some(serde_json::to_string(&t.tool_defs).unwrap_or_default())),
        mission_hash: Set(t.mission_hash),
        cluster_id: Set(t.cluster_id),
        route_path: Set(t.route_path),
        created_at: Set(t.created_at),
        updated_at: Set(axagent_harness::util_fns::now_ms()),
    }
}

// ── 新 trait impl: WorkflowTemplateRepo ──

#[async_trait]
impl WorkflowTemplateRepo for DaoWorkflowTemplateRepository {
    async fn list_templates(&self) -> Result<Vec<Wtd>, String> {
        let models =
            repo_wt::list_workflow_templates(&self.db, None).await.map_err(|e| e.to_string())?;
        Ok(models.into_iter().map(model_to_wtd).collect())
    }

    async fn get_template(&self, id: &str) -> Result<Option<Wtd>, String> {
        let model =
            repo_wt::get_workflow_template(&self.db, id).await.map_err(|e| e.to_string())?;
        Ok(model.map(model_to_wtd))
    }

    async fn save_template(&self, template: &Wtd) -> Result<(), String> {
        let active = wtd_to_active_model(template.clone());
        repo_wt::upsert_workflow_template(&self.db, active).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
