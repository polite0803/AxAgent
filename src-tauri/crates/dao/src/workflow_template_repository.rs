// SPDX-License-Identifier: AGPL-3.0-only

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use axagent_harness::repo_dtos::WorkflowTemplateData;
use axagent_harness::repositories::WorkflowTemplateRepository;

pub struct DaoWorkflowTemplateRepository {
    pub db: Arc<DatabaseConnection>,
}

#[async_trait]
impl WorkflowTemplateRepository for DaoWorkflowTemplateRepository {
    async fn get_workflow_template(
        &self,
        id: &str,
    ) -> Result<Option<WorkflowTemplateData>, String> {
        let model = crate::repo::workflow_template::get_workflow_template(&self.db, id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(model.map(|m| WorkflowTemplateData {
            id: m.id,
            name: m.name,
            description: m.description,
            icon: m.icon,
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
}
