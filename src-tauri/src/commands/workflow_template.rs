use crate::AppState;
use axagent_core::repo::workflow_template as db_repo;
use axagent_core::workflow_types::*;
use sea_orm::{ActiveModelTrait, Set};
use serde::Deserialize;
use tauri::State;

fn model_to_active_model(
    template: &WorkflowTemplateData,
) -> axagent_core::entity::workflow_template::ActiveModel {
    let now = chrono::Utc::now().timestamp_millis();

    axagent_core::entity::workflow_template::ActiveModel {
        id: Set(template.id.clone()),
        name: Set(template.name.clone()),
        description: Set(template.description.clone()),
        icon: Set(template.icon.clone()),
        tags: Set(Some(
            serde_json::to_string(&template.tags).unwrap_or_default(),
        )),
        version: Set(template.version),
        is_preset: Set(template.is_preset),
        is_editable: Set(template.is_editable),
        is_public: Set(template.is_public),
        trigger_config: Set(template
            .trigger_config
            .as_ref()
            .and_then(|c| serde_json::to_string(c).ok())),
        nodes: Set(serde_json::to_string(&template.nodes).unwrap_or_default()),
        edges: Set(serde_json::to_string(&template.edges).unwrap_or_default()),
        input_schema: Set(template
            .input_schema
            .as_ref()
            .and_then(|s| serde_json::to_string(s).ok())),
        output_schema: Set(template
            .output_schema
            .as_ref()
            .and_then(|s| serde_json::to_string(s).ok())),
        variables: Set(Some(
            serde_json::to_string(&template.variables).unwrap_or_default(),
        )),
        error_config: Set(template
            .error_config
            .as_ref()
            .and_then(|e| serde_json::to_string(e).ok())),
        composite_source: Set(None),
        created_at: Set(template.created_at),
        updated_at: Set(now),
    }
}

#[tauri::command]
pub async fn list_workflow_templates(
    state: State<'_, AppState>,
    is_preset: Option<bool>,
) -> Result<Vec<WorkflowTemplateResponse>, String> {
    let db = &state.sea_db;
    let templates = db_repo::list_workflow_templates(db, is_preset)
        .await
        .map_err(|e| e.to_string())?;

    Ok(templates
        .into_iter()
        .map(WorkflowTemplateResponse::from)
        .collect())
}

#[tauri::command]
pub async fn get_workflow_template(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<WorkflowTemplateResponse>, String> {
    let db = &state.sea_db;
    let template = db_repo::get_workflow_template(db, &id)
        .await
        .map_err(|e| e.to_string())?;

    // Auto-migrate legacy Tool/Code nodes to Agent nodes on load
    if let Some(ref m) = template {
        let mut nodes: Vec<axagent_core::workflow_types::WorkflowNode> =
            serde_json::from_str(&m.nodes).map_err(|e| e.to_string())?;

        if axagent_core::workflow_types::WorkflowMigrator::has_legacy_nodes(&nodes) {
            axagent_core::workflow_types::WorkflowMigrator::migrate(&mut nodes);
            let updated_nodes_str = serde_json::to_string(&nodes).map_err(|e| e.to_string())?;

            // Persist the migrated nodes back
            let active_model = axagent_core::entity::workflow_template::ActiveModel {
                id: sea_orm::ActiveValue::Unchanged(m.id.clone()),
                nodes: sea_orm::ActiveValue::Set(updated_nodes_str),
                ..Default::default()
            };
            active_model.update(db).await.map_err(|e| e.to_string())?;

            // Re-fetch to get the updated template
            let updated = db_repo::get_workflow_template(db, &id)
                .await
                .map_err(|e| e.to_string())?;
            return Ok(updated.map(WorkflowTemplateResponse::from));
        }
    }

    Ok(template.map(WorkflowTemplateResponse::from))
}

#[tauri::command]
pub async fn create_workflow_template(
    state: State<'_, AppState>,
    input: WorkflowTemplateInput,
) -> Result<String, String> {
    let db = &state.sea_db;
    let now = chrono::Utc::now().timestamp_millis();

    let template = WorkflowTemplateData {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        description: input.description,
        icon: input.icon,
        tags: input.tags,
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        trigger_config: input.trigger_config,
        nodes: input.nodes,
        edges: input.edges,
        input_schema: input.input_schema,
        output_schema: input.output_schema,
        variables: input.variables,
        error_config: input.error_config,
        created_at: now,
        updated_at: now,
    };

    let active_model = model_to_active_model(&template);
    db_repo::insert_workflow_template(db, active_model)
        .await
        .map_err(|e| e.to_string())?;

    Ok(template.id)
}

#[tauri::command]
pub async fn update_workflow_template(
    state: State<'_, AppState>,
    id: String,
    input: WorkflowTemplateInput,
) -> Result<bool, String> {
    let db = &state.sea_db;

    let updated = db_repo::update_workflow_template(
        db,
        &id,
        input.name,
        input.description,
        input.icon,
        input.tags,
        input.trigger_config,
        input.nodes,
        input.edges,
        input.input_schema,
        input.output_schema,
        input.variables,
        input.error_config,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(updated)
}

#[tauri::command]
pub async fn delete_workflow_template(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let db = &state.sea_db;
    let deleted = db_repo::delete_workflow_template(db, &id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(deleted)
}

#[tauri::command]
pub async fn duplicate_workflow_template(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let db = &state.sea_db;

    let template = db_repo::get_workflow_template(db, &id)
        .await
        .map_err(|e| e.to_string())?;

    let template = template.ok_or("Template not found")?;
    let response = WorkflowTemplateResponse::from(template);

    let now = chrono::Utc::now().timestamp_millis();
    let new_template = WorkflowTemplateData {
        id: uuid::Uuid::new_v4().to_string(),
        name: format!("{} (Copy)", response.name),
        description: response.description,
        icon: response.icon,
        tags: response.tags,
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        trigger_config: response.trigger_config,
        nodes: response.nodes,
        edges: response.edges,
        input_schema: response.input_schema,
        output_schema: response.output_schema,
        variables: response.variables,
        error_config: response.error_config,
        created_at: now,
        updated_at: now,
    };

    let active_model = model_to_active_model(&new_template);
    db_repo::insert_workflow_template(db, active_model)
        .await
        .map_err(|e| e.to_string())?;

    Ok(new_template.id)
}

#[tauri::command]
pub async fn seed_preset_templates(state: State<'_, AppState>) -> Result<usize, String> {
    use axagent_core::preset_templates::{
        convert_preset_to_workflow_template, get_preset_templates,
    };

    let db = &state.sea_db;
    let presets = get_preset_templates();

    let mut count = 0;
    for preset in presets {
        let existing = db_repo::get_workflow_template(db, preset.id)
            .await
            .map_err(|e| e.to_string())?;

        match existing {
            // Template doesn't exist yet → insert full data (first run)
            None => {
                let mut template = convert_preset_to_workflow_template(&preset);
                template.is_preset = true;
                template.is_editable = true;
                template.is_public = true;

                let active_model = model_to_active_model(&template);
                db_repo::insert_workflow_template(db, active_model)
                    .await
                    .map_err(|e| e.to_string())?;
                count += 1;
            },
            // Template exists with empty nodes (upgrade from old data) → update with full data
            Some(ref t) if t.nodes == "[]" || t.nodes.is_empty() => {
                let mut template = convert_preset_to_workflow_template(&preset);
                template.is_preset = true;
                template.is_editable = true;
                template.is_public = true;

                let active_model = model_to_active_model(&template);
                db_repo::upsert_workflow_template(db, active_model)
                    .await
                    .map_err(|e| e.to_string())?;
                count += 1;
            },
            // Template exists with nodes → user may have edited it, keep as-is
            _ => {},
        }
    }

    Ok(count)
}

#[tauri::command]
pub async fn get_template_versions(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<i32>, String> {
    let db = &state.sea_db;
    let versions = db_repo::get_template_versions(db, &id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(versions)
}

#[tauri::command]
pub async fn get_template_by_version(
    state: State<'_, AppState>,
    id: String,
    version: i32,
) -> Result<Option<WorkflowTemplateResponse>, String> {
    let db = &state.sea_db;
    let template = db_repo::get_template_by_version(db, &id, version)
        .await
        .map_err(|e| e.to_string())?;
    Ok(template.map(WorkflowTemplateResponse::from))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateWorkflowInput {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

#[allow(clippy::unnecessary_filter_map)]
#[tauri::command]
pub async fn validate_workflow_template(
    _state: State<'_, AppState>,
    input: ValidateWorkflowInput,
) -> Result<ValidationResult, String> {
    let nodes = input.nodes;
    let edges = input.edges;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let node_ids: std::collections::HashSet<String> = nodes
        .iter()
        .filter_map(|n| match n {
            WorkflowNode::Trigger(t) => Some(t.base.id.clone()),
            WorkflowNode::Agent(t) => Some(t.base.id.clone()),
            WorkflowNode::Llm(t) => Some(t.base.id.clone()),
            WorkflowNode::Condition(t) => Some(t.base.id.clone()),
            WorkflowNode::Parallel(t) => Some(t.base.id.clone()),
            WorkflowNode::Loop(t) => Some(t.base.id.clone()),
            WorkflowNode::Merge(t) => Some(t.base.id.clone()),
            WorkflowNode::Delay(t) => Some(t.base.id.clone()),
            WorkflowNode::Validation(t) => Some(t.base.id.clone()),
            WorkflowNode::Tool(t) => Some(t.base.id.clone()),
            WorkflowNode::Code(t) => Some(t.base.id.clone()),
            WorkflowNode::SubWorkflow(t) => Some(t.base.id.clone()),
            WorkflowNode::DocumentParser(t) => Some(t.base.id.clone()),
            WorkflowNode::VectorRetrieve(t) => Some(t.base.id.clone()),
            WorkflowNode::End(t) => Some(t.base.id.clone()),
        })
        .collect();

    if nodes.is_empty() {
        errors.push(ValidationError {
            error_type: "empty_workflow".to_string(),
            node_id: None,
            message: "Workflow must have at least one node".to_string(),
            suggestion: Some("Add a trigger node to start the workflow".to_string()),
        });
    }

    let trigger_count = nodes
        .iter()
        .filter(|n| matches!(n, WorkflowNode::Trigger(_)))
        .count();
    if trigger_count == 0 {
        errors.push(ValidationError {
            error_type: "missing_trigger".to_string(),
            node_id: None,
            message: "Workflow must have at least one trigger node".to_string(),
            suggestion: Some(
                "Add a trigger node (manual, schedule, webhook, or event)".to_string(),
            ),
        });
    } else if trigger_count > 1 {
        warnings.push(ValidationWarning {
            warning_type: "multiple_triggers".to_string(),
            node_id: None,
            message: format!("Workflow has {} trigger nodes. Consider using a single trigger with conditional branching.", trigger_count),
        });
    }

    let end_count = nodes
        .iter()
        .filter(|n| matches!(n, WorkflowNode::End(_)))
        .count();
    if end_count == 0 {
        warnings.push(ValidationWarning {
            warning_type: "missing_end".to_string(),
            node_id: None,
            message:
                "Workflow has no End node. Consider adding one for proper workflow termination."
                    .to_string(),
        });
    }

    for edge in &edges {
        if !node_ids.contains(&edge.source) {
            errors.push(ValidationError {
                error_type: "invalid_edge_source".to_string(),
                node_id: Some(edge.id.clone()),
                message: format!(
                    "Edge '{}' references non-existent source node '{}'",
                    edge.id, edge.source
                ),
                suggestion: Some("Remove this edge or create the missing source node".to_string()),
            });
        }
        if !node_ids.contains(&edge.target) {
            errors.push(ValidationError {
                error_type: "invalid_edge_target".to_string(),
                node_id: Some(edge.id.clone()),
                message: format!(
                    "Edge '{}' references non-existent target node '{}'",
                    edge.id, edge.target
                ),
                suggestion: Some("Remove this edge or create the missing target node".to_string()),
            });
        }
    }

    let mut has_cycle = false;
    let mut visited = std::collections::HashSet::new();
    let mut rec_stack = std::collections::HashSet::new();
    let mut adjacency: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for edge in &edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
    }

    fn dfs(
        node: &str,
        adjacency: &std::collections::HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        if let Some(neighbors) = adjacency.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if dfs(neighbor, adjacency, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }
        rec_stack.remove(node);
        false
    }

    for node_id in &node_ids {
        if !visited.contains(node_id) && dfs(node_id, &adjacency, &mut visited, &mut rec_stack) {
            has_cycle = true;
            break;
        }
    }

    if has_cycle {
        errors.push(ValidationError {
            error_type: "cyclic_dependency".to_string(),
            node_id: None,
            message: "Workflow contains cyclic dependencies".to_string(),
            suggestion: Some(
                "Remove loops in the workflow graph or use a Loop node for iteration".to_string(),
            ),
        });
    }

    let is_valid = errors.is_empty();

    Ok(ValidationResult {
        is_valid,
        errors,
        warnings,
    })
}

#[tauri::command]
pub async fn export_workflow_template(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let db = &state.sea_db;
    let template = db_repo::get_workflow_template(db, &id)
        .await
        .map_err(|e| e.to_string())?;

    let template = template.ok_or("Template not found")?;
    let response = WorkflowTemplateResponse::from(template);

    serde_json::to_string_pretty(&response).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_workflow_template(
    state: State<'_, AppState>,
    json_data: String,
) -> Result<String, String> {
    let db = &state.sea_db;

    let template: WorkflowTemplateResponse =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON format: {}", e))?;

    // Auto-migrate legacy Tool/Code nodes to Agent nodes on import
    let mut nodes = template.nodes.clone();
    let migrated_nodes: Vec<axagent_core::workflow_types::WorkflowNode> =
        if axagent_core::workflow_types::WorkflowMigrator::has_legacy_nodes(&nodes) {
            axagent_core::workflow_types::WorkflowMigrator::migrate(&mut nodes);
            nodes
        } else {
            nodes
        };

    let now = chrono::Utc::now().timestamp_millis();
    let new_template = WorkflowTemplateData {
        id: uuid::Uuid::new_v4().to_string(),
        name: template.name,
        description: template.description,
        icon: template.icon,
        tags: template.tags,
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        trigger_config: template.trigger_config,
        nodes: migrated_nodes,
        edges: template.edges,
        input_schema: template.input_schema,
        output_schema: template.output_schema,
        variables: template.variables,
        error_config: template.error_config,
        created_at: now,
        updated_at: now,
    };

    let active_model = model_to_active_model(&new_template);
    db_repo::insert_workflow_template(db, active_model)
        .await
        .map_err(|e| e.to_string())?;

    Ok(new_template.id)
}
