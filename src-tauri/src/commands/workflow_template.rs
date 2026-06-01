use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::workflow as workflow_err;
use axagent_core::repo::workflow_template as db_repo;
use axagent_core::workflow_types::*;
use axagent_runtime::work_engine::node_executor_trait::node_type_name;
use sea_orm::{DatabaseConnection, EntityTrait, Set};
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
        tags: Set(Some(serde_json::to_string(&template.tags).unwrap_or_default())),
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
        variables: Set(Some(serde_json::to_string(&template.variables).unwrap_or_default())),
        error_config: Set(template
            .error_config
            .as_ref()
            .and_then(|e| serde_json::to_string(e).ok())),
        composite_source: Set(None),
        tool_defs: Set(if template.tool_defs.is_empty() {
            None
        } else {
            serde_json::to_string(&template.tool_defs).ok()
        }),
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

    Ok(template.map(WorkflowTemplateResponse::from))
}

#[tauri::command]
pub async fn create_workflow_template(
    state: State<'_, AppState>,
    input: WorkflowTemplateInput,
) -> Result<String, String> {
    let db = &state.sea_db;

    // 节点组成相似性检查
    let similar = find_similar_workflows(db, &input.nodes).await?;
    if !similar.is_empty() {
        tracing::info!(
            "[workflow_template] 新建模板 '{}' 与 {} 个已有模板节点组成相似: {:?}",
            input.name,
            similar.len(),
            similar.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

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
        tool_defs: input.tool_defs.unwrap_or_default(),
        created_at: now,
        updated_at: now,
    };

    let active_model = model_to_active_model(&template);
    db_repo::insert_workflow_template(db, active_model)
        .await
        .map_err(|e| e.to_string())?;

    state
        .work_engine
        .precompile_tool_defs(&template.id, &template.tool_defs)
        .await;

    Ok(template.id)
}

#[tauri::command]
pub async fn update_workflow_template(
    state: State<'_, AppState>,
    id: String,
    mut input: WorkflowTemplateInput,
) -> Result<bool, String> {
    let db = &state.sea_db;

    // 保存前提取 tool_defs（确保移动后仍可引用）
    let tool_defs = input.tool_defs.take();

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
        tool_defs.clone(),
    )
    .await
    .map_err(|e| e.to_string())?;

    // 保存后立即预编译 Rhai 工具，Agent 节点可即时引用
    if let Some(ref tds) = tool_defs {
        state.work_engine.precompile_tool_defs(&id, tds).await;
    }

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

    let template = template.ok_or_else(|| {
        ErrorResponse::err_with_detail(workflow_err::NOT_FOUND, "Template not found")
    })?;
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
        tool_defs: vec![],
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
        .flat_map(|n| match n {
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
            WorkflowNode::HttpRequest(t) => Some(t.base.id.clone()),
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
        if edge.edge_type == EdgeType::LoopBack {
            continue;
        }
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

    // ── ConditionNode 专项校验 ──
    for node in &nodes {
        if let WorkflowNode::Condition(c) = node {
            // 1. 无 conditions 且未启用 LLM 路由
            if c.config.conditions.is_empty() && !c.config.judge_by_llm.unwrap_or(false) {
                errors.push(ValidationError {
                    error_type: "empty_conditions".to_string(),
                    node_id: Some(c.base.id.clone()),
                    message: format!(
                        "Condition node '{}' has no conditions and LLM routing is not enabled",
                        c.base.id
                    ),
                    suggestion: Some(
                        "Add at least one condition or enable LLM routing".to_string(),
                    ),
                });
            }

            // 2. 检查出边类型：condition 节点的出边必须是 conditionTrue/conditionFalse
            //    不允许 direct 类型出边
            for edge in &edges {
                if edge.source != c.base.id { continue; }
                match &edge.edge_type {
                    EdgeType::ConditionTrue | EdgeType::ConditionFalse => {},
                    _ => {
                        warnings.push(ValidationWarning {
                            warning_type: "invalid_condition_edge".to_string(),
                            node_id: Some(c.base.id.clone()),
                            message: format!(
                                "Condition node '{}' has an edge with type '{:?}'. Condition edges should be 'conditionTrue' (✓) or 'conditionFalse' (✗).",
                                c.base.id, edge.edge_type
                            ),
                        });
                    }
                }
            }

            // 3. 检查是否缺少 conditionTrue 或 conditionFalse 出边
            let has_true = edges.iter().any(|e| e.source == c.base.id && e.edge_type == EdgeType::ConditionTrue);
            let has_false = edges.iter().any(|e| e.source == c.base.id && e.edge_type == EdgeType::ConditionFalse);
            if !has_true {
                warnings.push(ValidationWarning {
                    warning_type: "missing_true_branch".to_string(),
                    node_id: Some(c.base.id.clone()),
                    message: format!(
                        "Condition node '{}' is missing a 'true' branch edge (✓ handle)",
                        c.base.id
                    ),
                });
            }
            if !has_false {
                warnings.push(ValidationWarning {
                    warning_type: "missing_false_branch".to_string(),
                    node_id: Some(c.base.id.clone()),
                    message: format!(
                        "Condition node '{}' is missing a 'false' branch edge (✗ handle)",
                        c.base.id
                    ),
                });
            }
        }
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

    let template = template.ok_or_else(|| {
        ErrorResponse::err_with_detail(workflow_err::NOT_FOUND, "Template not found")
    })?;
    let response = WorkflowTemplateResponse::from(template);

    serde_json::to_string_pretty(&response).map_err(|e| e.to_string())
}

/// 检测是否为 n8n 格式（存在 n8n-nodes-base 类型节点）
fn is_n8n_format(json: &serde_json::Value) -> bool {
    json.get("nodes")
        .and_then(|n| n.as_array())
        .map(|nodes| {
            nodes.iter().any(|n| {
                n.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t.starts_with("n8n-nodes-base."))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// n8n 节点类型 → (agent_profile_id, agent_role, expert_id, expert_system_prompt)
// i18n-exempt: Expert role descriptions are LLM system prompts — model interaction data, not UI
fn infer_agent_from_n8n(
    node_type: &str,
    node_name: &str,
) -> (&'static str, &'static str, &'static str, &'static str) {
    let t = node_type.to_lowercase();
    let n = node_name.to_lowercase();

    // Node name takes priority — handles generic n8n types (e.g. "n8n-nodes-base.noOp")
    if n.contains("review") || n.contains("check") || n.contains("validate") || n.contains("audit")
    {
        return (
            "code-reviewer",
            "reviewer",
            "code-reviewer",
            "Code Review Expert: Review code for correctness, security, performance, and maintainability. Provide specific improvement suggestions.",
        );
    }
    if n.contains("debug") || n.contains("fix") || n.contains("troubleshoot") {
        return (
            "debug-expert",
            "developer",
            "debug-expert",
            "Debug Expert: Systematically analyze error logs, identify root causes. Verify fix solutions.",
        );
    }
    if n.contains("test") || n.contains("qa") || n.contains("quality") {
        return (
            "debug-expert",
            "reviewer",
            "debug-expert",
            "Test Engineer: Write and execute test cases, verify functional correctness.",
        );
    }
    if n.contains("doc") || n.contains("report") || n.contains("summary") || n.contains("write") {
        return (
            "tech-writer",
            "synthesizer",
            "tech-writer",
            "Technical Writer: Write clear and accurate technical documentation and reports.",
        );
    }
    if n.contains("plan") || n.contains("design") || n.contains("architect") {
        return (
            "architect",
            "planner",
            "architect",
            "System Architect: Responsible for system design, technology selection, and architecture review.",
        );
    }
    if n.contains("monitor") || n.contains("alert") || n.contains("watch") {
        return (
            "devops-engineer",
            "executor",
            "devops-engineer",
            "DevOps Engineer: Monitor system status, handle alerts, and automate operations.",
        );
    }
    if n.contains("analyze") || n.contains("insight") || n.contains("report") {
        return (
            "data-analyst",
            "researcher",
            "data-analyst",
            "Data Analyst: Data cleaning, statistical analysis, and visualization.",
        );
    }

    if t.contains("http")
        || t.contains("api")
        || t.contains("rest")
        || t.contains("webhook")
        || t.contains("graphql")
        || t.contains("request")
    {
        (
            "devops-engineer",
            "executor",
            "devops-engineer",
            "DevOps Engineer: Responsible for API integration, CI/CD pipelines, HTTP request automation. Ensure reliability and error handling of interface calls.",
        )
    } else if t.contains("database")
        || t.contains("sql")
        || t.contains("postgres")
        || t.contains("mysql")
        || t.contains("mongo")
        || t.contains("redis")
    {
        (
            "sql-expert",
            "researcher",
            "sql-expert",
            "SQL Expert: Proficient in database query optimization, data modeling, and SQL writing. Consider indexing strategies and concurrency control.",
        )
    } else if t.contains("code")
        || t.contains("function")
        || t.contains("python")
        || t.contains("javascript")
        || t.contains("typescript")
    {
        (
            "senior-developer",
            "developer",
            "senior-developer",
            "Senior Developer: Proficient in multiple languages and frameworks, following best practices. Write clear, efficient, and maintainable code.",
        )
    } else if t.contains("email")
        || t.contains("slack")
        || t.contains("notify")
        || t.contains("telegram")
        || t.contains("discord")
    {
        (
            "product-manager",
            "coordinator",
            "product-manager",
            "Product Manager: Communication coordination, requirements analysis, and notification management.",
        )
    } else if t.contains("ai")
        || t.contains("llm")
        || t.contains("openai")
        || t.contains("anthropic")
        || t.contains("chat")
    {
        (
            "general-assistant",
            "coordinator",
            "general-assistant",
            "General AI Assistant: Versatile assistant handling various tasks and questions.",
        )
    } else if t.contains("file")
        || t.contains("csv")
        || t.contains("spreadsheet")
        || t.contains("xml")
        || t.contains("json")
        || t.contains("excel")
    {
        (
            "data-analyst",
            "researcher",
            "data-analyst",
            "Data Analyst: Data cleaning, statistical analysis, and visualization, skilled at extracting insights from data.",
        )
    } else if t.contains("security") || t.contains("auth") || t.contains("oauth") {
        (
            "security-auditor",
            "reviewer",
            "security-auditor",
            "Security Auditor: OWASP Top 10 review, authentication/authorization checks, data encryption, and privacy protection.",
        )
    } else if t.contains("transform")
        || t.contains("convert")
        || t.contains("merge")
        || t.contains("sort")
        || t.contains("filter")
    {
        (
            "tech-writer",
            "synthesizer",
            "tech-writer",
            "Technical Writer: Organize, transform, and aggregate data, output structured results.",
        )
    } else {
        (
            "debug-expert",
            "executor",
            "debug-expert",
            "Debug Expert: Systematic analysis, identify root causes, verify fix solutions.",
        )
    }
}

/// 确保 AgentRole 存在，不存在则创建
async fn ensure_agent_role(db: &DatabaseConnection, role_name: &str) -> Result<(), String> {
    use axagent_core::entity::agent_roles;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // 按 name 字段查重，避免仅按主键匹配导致同名字段创建重复记录
    let existing = agent_roles::Entity::find()
        .filter(agent_roles::Column::Name.eq(role_name))
        .one(db)
        .await
        .map_err(|e: sea_orm::DbErr| e.to_string())?;

    if existing.is_none() {
        let now = chrono::Utc::now().timestamp_millis();
        let am = agent_roles::ActiveModel {
            id: Set(role_name.to_string()),
            name: Set(role_name.to_string()),
            description: Set(Some(format!("Auto-created from n8n import: {}", role_name))),
            system_prompt: Set(String::new()),
            default_tools: Set(None),
            max_concurrent: Set(3),
            timeout_seconds: Set(600),
            source: Set("imported".to_string()),
            sort_order: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
        };
        agent_roles::Entity::insert(am)
            .exec(db)
            .await
            .map_err(|e| format!("Failed to create AgentRole {}: {}", role_name, e))?;
    }
    Ok(())
}

/// 从 n8n 导入时创建 Expert（技能）+ AgentRole（岗位）+ AgentProfile（组装体）
async fn ensure_agent_profile(
    db: &DatabaseConnection,
    profile_id: &str,
    profile_name: &str,
    agent_role: &str,
    expert_id: &str,
    expert_prompt: &str,
) -> Result<(), String> {
    use axagent_core::entity::{agency_experts, agent_profiles};
    use sea_orm::Set;

    let now = chrono::Utc::now().timestamp_millis();

    // 1. 确保 Expert（技能）存在
    let expert_exists = agency_experts::Entity::find_by_id(expert_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if !expert_exists {
        let expert_am = agency_experts::ActiveModel {
            id: Set(expert_id.to_string()),
            name: Set(profile_name.to_string()),
            description: Set(Some(format!("Auto-created from n8n import: {}", profile_name))),
            category: Set("general".to_string()),
            system_prompt: Set(expert_prompt.to_string()),
            color: Set(None),
            source_dir: Set("n8n-import".to_string()),
            is_enabled: Set(1),
            imported_at: Set(now),
            recommended_workflows: Set(None),
            recommended_tools: Set(None),
        };
        agency_experts::Entity::insert(expert_am)
            .exec(db)
            .await
            .map_err(|e| format!("Failed to create Expert {}: {}", expert_id, e))?;
    }

    // 2. 确保 AgentRole（岗位）存在
    ensure_agent_role(db, agent_role).await?;

    // 3. 确保 AgentProfile（组装体）存在并绑定 Expert
    let profile_exists = agent_profiles::Entity::find_by_id(profile_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if !profile_exists {
        let profile_am = agent_profiles::ActiveModel {
            id: Set(profile_id.to_string()),
            name: Set(profile_name.to_string()),
            description: Set(Some(format!("{} + {}", profile_name, agent_role))),
            category: Set("general".to_string()),
            icon: Set("🤖".to_string()),
            agent_role: Set(Some(agent_role.to_string())),
            source: Set("imported".to_string()),
            sort_order: Set(0),
            is_enabled: Set(1),
            expert_id: Set(Some(expert_id.to_string())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        agent_profiles::Entity::insert(profile_am)
            .exec(db)
            .await
            .map_err(|e| format!("Failed to create AgentProfile {}: {}", profile_id, e))?;
    }
    Ok(())
}

/// 相似工作流信息
#[derive(Debug, serde::Serialize)]
pub struct SimilarWorkflow {
    pub workflow_id: String,
    pub name: String,
    pub similarity: f64,
    pub overlapping_nodes: usize,
    pub total_nodes: usize,
}

/// 基于节点类型组成查找相似工作流（Jaccard 相似度 ≥ 0.6 视为相似）。
/// 用于创建、导入工作流时检测是否与已有模板高度重合。
pub async fn find_similar_workflows(
    db: &DatabaseConnection,
    nodes: &[WorkflowNode],
) -> Result<Vec<SimilarWorkflow>, String> {
    let input_types: std::collections::HashSet<String> = nodes
        .iter()
        .map(|n| node_type_name(n).to_string())
        .collect();

    if input_types.is_empty() {
        return Ok(Vec::new());
    }

    let all = axagent_core::entity::workflow_template::Entity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for tmpl in &all {
        let existing_nodes: Vec<WorkflowNode> =
            serde_json::from_str(&tmpl.nodes).unwrap_or_default();
        let existing_types: std::collections::HashSet<String> = existing_nodes
            .iter()
            .map(|n| node_type_name(n).to_string())
            .collect();

        let intersection = input_types.intersection(&existing_types).count();
        let union = input_types.union(&existing_types).count();
        let similarity = if union > 0 {
            intersection as f64 / union as f64
        } else {
            0.0
        };

        if similarity >= 0.6 {
            results.push(SimilarWorkflow {
                workflow_id: tmpl.id.clone(),
                name: tmpl.name.clone(),
                similarity,
                overlapping_nodes: intersection,
                total_nodes: input_types.len(),
            });
        }
    }

    Ok(results)
}

/// 语义重复检查：Jaccard 相似度 ≥ 0.6 视为重复
/// 注意：此函数当前全表扫描已导入模板进行字符级相似度比较。
/// 本地客户端模板数量有限（通常 < 1000），性能影响可接受。
/// 若未来支持云端同步或大规模模板库，应改为数据库模糊查询或向量索引。
async fn check_workflow_duplicate(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<String>, String> {
    use axagent_core::entity::workflow_template;

    let input_tokens: std::collections::HashSet<String> = name
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 1)
        .map(|s| s.to_string())
        .collect();

    if input_tokens.is_empty() {
        return Ok(None);
    }

    let all = workflow_template::Entity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    for tmpl in &all {
        let existing_tokens: std::collections::HashSet<String> = tmpl
            .name
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 1)
            .map(|s| s.to_string())
            .collect();

        let intersection = input_tokens.intersection(&existing_tokens).count();
        let union = input_tokens.union(&existing_tokens).count();
        let similarity = if union > 0 {
            (intersection as f64) / (union as f64)
        } else {
            0.0
        };

        if similarity >= 0.6 {
            return Ok(Some(tmpl.name.clone()));
        }
    }
    Ok(None)
}

/// 从 n8n 节点参数提取有意义的 goal 描述
fn extract_goal_from_n8n(node: &serde_json::Value) -> String {
    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let params = node.get("parameters");

    if let Some(p) = params {
        if node_type.contains("http") || node_type.contains("api") {
            let method = p.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
            let url = p.get("url").and_then(|v| v.as_str()).unwrap_or("(no URL)");
            return format!("HTTP {} {}", method, url);
        }
        if node_type.contains("database") || node_type.contains("sql") {
            let op = p
                .get("operation")
                .and_then(|v| v.as_str())
                .unwrap_or("query");
            let table = p.get("table").and_then(|v| v.as_str()).unwrap_or("");
            return format!("SQL {} {}", op, table);
        }
        if node_type.contains("email") {
            let subj = p.get("subject").and_then(|v| v.as_str()).unwrap_or("");
            return format!("Send email: {}", subj);
        }
        if node_type.contains("code") || node_type.contains("function") {
            let lang = node_type.rsplit('.').next().unwrap_or("code");
            return format!("Execute {} function", lang);
        }
    }
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unnamed");
    format!("{} ({})", name, node_type.rsplit('.').next().unwrap_or(node_type))
}

/// 从 n8n 节点参数提取 AxAgent AgentNodeConfig 配置
fn extract_config_from_n8n(n8n_node: &serde_json::Value, node_id: &str) -> AgentNodeConfig {
    let node_type = n8n_node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let node_name = n8n_node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unnamed");
    let params = n8n_node.get("parameters");
    let goal = extract_goal_from_n8n(n8n_node);

    // ── 构建 system_prompt：n8n 节点参数 → 自然语言任务描述 ──
    let mut prompt_parts: Vec<String> = Vec::new();
    prompt_parts.push(format!("任务目标：{goal}"));

    if let Some(p) = params {
        // HTTP / API 节点
        if node_type.contains("http") || node_type.contains("api") || node_type.contains("webhook")
        {
            let method = p.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
            let url = p.get("url").and_then(|v| v.as_str()).unwrap_or("");
            if !url.is_empty() {
                prompt_parts.push(format!("调用方式：{method} {url}"));
            }
            if let Some(body) = p.get("body") {
                if let Some(s) = body.as_str() {
                    prompt_parts.push(format!("请求体参数：{s}"));
                }
            }
            if let Some(headers) = p.get("headers") {
                prompt_parts.push(format!("请求头：{headers}"));
            }
            if let Some(auth) = p.get("authentication") {
                prompt_parts.push(format!("认证方式：{auth}"));
            }
        }
        // 数据库节点
        else if node_type.contains("database")
            || node_type.contains("sql")
            || node_type.contains("postgres")
        {
            let op = p
                .get("operation")
                .and_then(|v| v.as_str())
                .unwrap_or("query");
            prompt_parts.push(format!("操作类型：{op}"));
            if let Some(query) = p.get("query").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("SQL 语句：{query}"));
            }
            if let Some(table) = p.get("table").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("目标表：{table}"));
            }
        }
        // 邮件节点
        else if node_type.contains("email") {
            if let Some(subj) = p.get("subject").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("邮件主题：{subj}"));
            }
            if let Some(to) = p.get("toEmail").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("收件人：{to}"));
            }
            if let Some(text) = p.get("text").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("邮件内容：{text}"));
            }
        }
        // 代码/函数节点
        else if node_type.contains("code") || node_type.contains("function") {
            let lang = node_type.rsplit('.').next().unwrap_or("javascript");
            prompt_parts.push(format!("执行语言：{lang}"));
            if let Some(code) = p.get("jsCode").or(p.get("code")).and_then(|v| v.as_str()) {
                let code_preview = if code.len() > 500 {
                    format!("{}…(截断)", &code[..500])
                } else {
                    code.to_string()
                };
                prompt_parts.push(format!("代码片段：\n```{lang}\n{code_preview}\n```"));
            }
        }
        // AI / LLM 节点
        else if node_type.contains("ai")
            || node_type.contains("llm")
            || node_type.contains("openai")
            || node_type.contains("openAi")
        {
            if let Some(prompt) = p.get("prompt").or(p.get("text")).and_then(|v| v.as_str()) {
                prompt_parts.push(format!("AI 提示词：{prompt}"));
            }
            if let Some(model) = p.get("model").and_then(|v| v.as_str()) {
                prompt_parts.push(format!("使用模型：{model}"));
            }
        }
        // 通用节点：提取所有参数
        else {
            prompt_parts
                .push(format!("节点类型：{}", node_type.rsplit('.').next().unwrap_or(node_type)));
            if let Some(obj) = p.as_object() {
                let params_desc: Vec<String> = obj
                    .iter()
                    .filter_map(|(k, v)| {
                        if k == "options" || k == "additionalFields" {
                            None
                        } else if let Some(s) = v.as_str() {
                            Some(format!("  {k}: {s}"))
                        } else {
                            Some(format!("  {k}: {v}"))
                        }
                    })
                    .collect();
                if !params_desc.is_empty() {
                    prompt_parts.push(format!("参数配置：\n{}", params_desc.join("\n")));
                }
            }
        }
    }

    // 追加节点描述（n8n 节点的 notes）
    if let Some(notes) = n8n_node.get("notes").and_then(|v| v.as_str()) {
        if !notes.is_empty() {
            prompt_parts.push(format!("备注说明：{notes}"));
        }
    }

    let system_prompt = prompt_parts.join("\n\n");

    // ── 构建 tools：根据 n8n 节点类型生成 ToolDef ──
    let mut tools: Vec<ToolDef> = Vec::new();

    let (tool_name, tool_desc) =
        if node_type.contains("http") || node_type.contains("api") || node_type.contains("webhook")
        {
            ("http_request", "发送 HTTP 请求并获取响应数据".to_string())
        } else if node_type.contains("database")
            || node_type.contains("sql")
            || node_type.contains("postgres")
        {
            ("database_query", "执行数据库查询或操作".to_string())
        } else if node_type.contains("email") {
            ("send_email", "发送电子邮件".to_string())
        } else if node_type.contains("code") || node_type.contains("function") {
            let lang = node_type.rsplit('.').next().unwrap_or("javascript");
            ("execute_code", format!("执行 {lang} 代码"))
        } else if node_type.contains("file")
            || node_type.contains("spreadsheet")
            || node_type.contains("csv")
        {
            ("file_operation", "读写文件或电子表格".to_string())
        } else {
            ("process_data", "处理数据或执行业务逻辑".to_string())
        };

    tools.push(ToolDef {
        name: format!("{tool_name}_{node_id}"),
        description: Some(format!(
            "{tool_desc}。原始节点: {node_name} ({n8n_type})",
            tool_desc = tool_desc,
            node_name = node_name,
            n8n_type = node_type.rsplit('.').next().unwrap_or(node_type)
        )),
        parameters: None,
    });

    // ── 提取模型设置（如果 n8n AI 节点有） ──
    let (model, temperature, max_tokens) = if let Some(p) = params {
        let model = if node_type.contains("ai")
            || node_type.contains("openai")
            || node_type.contains("openAi")
        {
            p.get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };
        let temperature = p
            .get("temperature")
            .and_then(|v| v.as_f64())
            .map(|t| t as f32);
        let max_tokens = p
            .get("maxTokens")
            .and_then(|v| v.as_u64())
            .map(|t| t as u32);
        (model, temperature, max_tokens)
    } else {
        (None, None, None)
    };

    AgentNodeConfig {
        system_prompt,
        context_sources: Vec::new(),
        output_var: format!("{}_output", node_id),
        model,
        temperature,
        max_tokens,
        tools,
        exposed_tools: vec![],
        output_mode: OutputMode::Text,
        agent_profile_id: None,
        max_tool_rounds: None,
        execution_mode: None,
        rag_source_ids: vec![],
    }
}

/// 将 n8n JSON 转换为 AxAgent Workflow — 两阶段：先 DB 准备，再组装
async fn convert_n8n_to_axagent(
    db: &DatabaseConnection,
    json: &serde_json::Value,
) -> Result<axagent_core::workflow_types::WorkflowTemplateData, String> {
    use axagent_core::workflow_types::*;

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported n8n Workflow")
        .to_string();

    let n8n_nodes = json
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing 'nodes' array".to_string())?;

    let n8n_connections = json.get("connections").cloned();

    let mut ax_nodes: Vec<WorkflowNode> = Vec::new();
    let mut ax_edges: Vec<WorkflowEdge> = Vec::new();
    let mut edge_id_counter = 0u32;
    let mut name_to_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    let trigger_node = WorkflowNode::Trigger(TriggerNode {
        base: WorkflowNodeBase {
            id: "trigger_imported".to_string(),
            title: "Trigger".to_string(),
            description: Some("Auto-created trigger from n8n import".to_string()),
            position: Position { x: 0.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
                parent_id: None,
        },
        config: TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::Value::Null,
        },
    });
    ax_nodes.push(trigger_node);

    for n8n_node in n8n_nodes {
        let node_id = n8n_node
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let node_name = n8n_node
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed")
            .to_string();

        let n8n_type = n8n_node
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        name_to_id.insert(node_name.clone(), node_id.clone());

        let n8n_type_lower = n8n_type.to_lowercase();

        let position = n8n_node
            .get("position")
            .map(|p| Position {
                x: p.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
                y: p.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
            })
            .unwrap_or(Position { x: 0.0, y: 0.0 });

        let base = WorkflowNodeBase {
            id: node_id.clone(),
            title: node_name.clone(),
            description: None,
            position: position.clone(),
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
                parent_id: None,
        };

        if n8n_type_lower.contains("if") || n8n_type_lower.contains("switch") {
            let condition_node = WorkflowNode::Condition(ConditionNode {
                base,
                config: ConditionNodeConfig {
                    conditions: Vec::new(),
                    logical_op: LogicalOperator::And,
                    judge_by_llm: None,
                    routing_prompt: None,
                    routing_model: None,
                },
            });
            ax_nodes.push(condition_node);
            continue;
        }

        if n8n_type_lower.contains("merge") {
            let merge_node = WorkflowNode::Merge(MergeNode {
                base,
                config: MergeNodeConfig {
                    merge_type: MergeStrategy::All,
                    inputs: Vec::new(),
                    auto_inputs_from_branches: false,
                },
            });
            ax_nodes.push(merge_node);
            continue;
        }

        if n8n_type_lower.contains("wait") {
            let delay_node = WorkflowNode::Delay(DelayNode {
                base,
                config: DelayNodeConfig {
                    delay_type: "seconds".to_string(),
                    seconds: 5,
                    until: None,
                },
            });
            ax_nodes.push(delay_node);
            continue;
        }

        let (agent_profile_id, agent_role, expert_id, expert_prompt) =
            infer_agent_from_n8n(&n8n_type, &node_name);

        ensure_agent_role(db, agent_role).await?;

        ensure_agent_profile(
            db,
            agent_profile_id,
            &format!("n8n: {}", &node_name),
            agent_role,
            expert_id,
            expert_prompt,
        )
        .await?;

        let goal = extract_goal_from_n8n(n8n_node);

        // 从 n8n 节点提取配置（system_prompt、tools、model 等）
        let mut agent_config = extract_config_from_n8n(n8n_node, &node_id);
        agent_config.agent_profile_id = Some(agent_profile_id.to_string());
        agent_config.context_sources = Vec::new(); // 暂不自动关联上游节点

        let base = WorkflowNodeBase {
            id: node_id.clone(),
            title: node_name,
            description: Some(goal),
            position,
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
                parent_id: None,
        };

        let agent_node = WorkflowNode::Agent(AgentNode {
            base,
            config: agent_config,
        });

        ax_nodes.push(agent_node);
    }

    let last_position = ax_nodes
        .iter()
        .map(|n| match n {
            WorkflowNode::Trigger(t) => t.base.position.clone(),
            WorkflowNode::Agent(t) => t.base.position.clone(),
            WorkflowNode::Llm(t) => t.base.position.clone(),
            WorkflowNode::Condition(t) => t.base.position.clone(),
            WorkflowNode::Parallel(t) => t.base.position.clone(),
            WorkflowNode::Loop(t) => t.base.position.clone(),
            WorkflowNode::Merge(t) => t.base.position.clone(),
            WorkflowNode::Delay(t) => t.base.position.clone(),
            WorkflowNode::Validation(t) => t.base.position.clone(),
            WorkflowNode::Tool(t) => t.base.position.clone(),
            WorkflowNode::Code(t) => t.base.position.clone(),
            WorkflowNode::SubWorkflow(t) => t.base.position.clone(),
            WorkflowNode::DocumentParser(t) => t.base.position.clone(),
            WorkflowNode::VectorRetrieve(t) => t.base.position.clone(),
            WorkflowNode::End(t) => t.base.position.clone(),
        })
        .next_back()
        .unwrap_or(Position { x: 250.0, y: 0.0 });

    let end_node = WorkflowNode::End(EndNode {
        base: WorkflowNodeBase {
            id: "end_imported".to_string(),
            title: "End".to_string(),
            description: Some("Auto-created end node from n8n import".to_string()),
            position: Position {
                x: last_position.x + 250.0,
                y: last_position.y,
            },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
                parent_id: None,
        },
        config: EndNodeConfig {
            output_var: Some("final_output".to_string()),
        },
    });
    ax_nodes.push(end_node);

    // Convert n8n connections → edges
    if let Some(connections) = n8n_connections {
        if let Some(conn_map) = connections.as_object() {
            for (source_name, conn_val) in conn_map {
                let source_id = match name_to_id.get(source_name) {
                    Some(id) => id.clone(),
                    None => continue,
                };
                if let Some(main_arr) = conn_val.get("main").and_then(|v| v.as_array()) {
                    for main_group in main_arr {
                        if let Some(entries) = main_group.as_array() {
                            for entry in entries {
                                let target_name = entry.get("node").and_then(|v| v.as_str());
                                let target_id = match target_name.and_then(|n| name_to_id.get(n)) {
                                    Some(id) => id.clone(),
                                    None => continue,
                                };
                                ax_edges.push(WorkflowEdge {
                                    id: format!("edge_{}", edge_id_counter),
                                    source: source_id.clone(),
                                    source_handle: None,
                                    target: target_id,
                                    target_handle: None,
                                    edge_type: EdgeType::Direct,
                                    label: None,
                                });
                                edge_id_counter += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // If no edges, create sequential flow
    if ax_edges.is_empty() && ax_nodes.len() > 1 {
        for i in 1..ax_nodes.len() {
            ax_edges.push(WorkflowEdge {
                id: format!("edge_{}", edge_id_counter),
                source: ax_nodes[i - 1].base_id().to_string(),
                source_handle: None,
                target: ax_nodes[i].base_id().to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            });
            edge_id_counter += 1;
        }
    } else if !ax_edges.is_empty() {
        let targets_with_incoming: std::collections::HashSet<String> =
            ax_edges.iter().map(|e| e.target.clone()).collect();
        for node in &ax_nodes {
            let nid = node.base_id();
            if nid != "trigger_imported"
                && nid != "end_imported"
                && !targets_with_incoming.contains(nid)
            {
                ax_edges.push(WorkflowEdge {
                    id: format!("edge_{}", edge_id_counter),
                    source: "trigger_imported".to_string(),
                    source_handle: None,
                    target: nid.to_string(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
                edge_id_counter += 1;
            }
        }
        let sources_with_outgoing: std::collections::HashSet<String> =
            ax_edges.iter().map(|e| e.source.clone()).collect();
        for node in &ax_nodes {
            let nid = node.base_id();
            if nid != "trigger_imported"
                && nid != "end_imported"
                && !sources_with_outgoing.contains(nid)
            {
                ax_edges.push(WorkflowEdge {
                    id: format!("edge_{}", edge_id_counter),
                    source: nid.to_string(),
                    source_handle: None,
                    target: "end_imported".to_string(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
                edge_id_counter += 1;
            }
        }
    }

    let now = chrono::Utc::now().timestamp_millis();
    Ok(WorkflowTemplateData {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        description: Some("Imported from n8n workflow".to_string()),
        icon: "🔧".to_string(),
        tags: vec!["n8n".to_string(), "imported".to_string()],
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        trigger_config: None,
        nodes: ax_nodes,
        edges: ax_edges,
        input_schema: None,
        output_schema: None,
        variables: Vec::new(),
        error_config: None,
        tool_defs: vec![],
        created_at: now,
        updated_at: now,
    })
}

async fn do_import_workflow(
    db: &DatabaseConnection,
    json_data: String,
) -> Result<serde_json::Value, String> {
    let raw_json: serde_json::Value =
        serde_json::from_str(&json_data).map_err(|e| format!("Invalid JSON: {}", e))?;

    let workflow_name = raw_json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Workflow")
        .to_string();

    let mut new_template = if is_n8n_format(&raw_json) {
        convert_n8n_to_axagent(db, &raw_json).await?
    } else {
        let template: WorkflowTemplateResponse = serde_json::from_value(raw_json)
            .map_err(|e| format!("Invalid AxAgent format: {}", e))?;

        let nodes = template.nodes.clone();

        let now = chrono::Utc::now().timestamp_millis();
        WorkflowTemplateData {
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
            nodes,
            edges: template.edges,
            input_schema: template.input_schema,
            output_schema: template.output_schema,
            variables: template.variables,
            error_config: template.error_config,
            tool_defs: vec![],
            created_at: now,
            updated_at: now,
        }
    };

    let mut warnings: Vec<String> = Vec::new();

    // 名称相似性检查
    if let Some(_existing) = check_workflow_duplicate(db, &workflow_name).await? {
        let new_name = format!("{} (Imported)", workflow_name);
        warnings.push(format!(
            "Workflow renamed from '{}' to '{}' due to name similarity with existing workflow",
            workflow_name, new_name
        ));
        new_template.name = new_name;
    }

    // 节点组成相似性检查
    let node_similar = find_similar_workflows(db, &new_template.nodes).await?;
    if !node_similar.is_empty() {
        warnings.push(format!(
            "Node composition {}% similar to existing workflow(s): {}",
            (node_similar[0].similarity * 100.0) as u32,
            node_similar
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let active_model = model_to_active_model(&new_template);
    db_repo::insert_workflow_template(db, active_model)
        .await
        .map_err(|e| e.to_string())?;

    let mut errors: Vec<String> = Vec::new();

    if new_template.nodes.is_empty() {
        errors.push("Workflow has no nodes".to_string());
    }

    let has_trigger = new_template
        .nodes
        .iter()
        .any(|n| matches!(n, WorkflowNode::Trigger(_)));
    if !has_trigger {
        warnings.push("Workflow has no trigger node".to_string());
    }

    let node_ids: std::collections::HashSet<String> = new_template
        .nodes
        .iter()
        .map(|n| n.base_id().to_string())
        .collect();
    for edge in &new_template.edges {
        if !node_ids.contains(&edge.source) {
            errors.push(format!(
                "Edge '{}' references non-existent source node '{}'",
                edge.id, edge.source
            ));
        }
        if !node_ids.contains(&edge.target) {
            errors.push(format!(
                "Edge '{}' references non-existent target node '{}'",
                edge.id, edge.target
            ));
        }
    }

    if !warnings.is_empty() {
        tracing::warn!("Import validation warnings for {}: {:?}", new_template.id, warnings);
    }
    if !errors.is_empty() {
        tracing::warn!("Import validation errors for {}: {:?}", new_template.id, errors);
    }

    Ok(serde_json::json!({
        "id": new_template.id,
        "warnings": warnings,
        "errors": errors,
    }))
}

#[tauri::command]
pub async fn import_workflow_template(
    state: State<'_, AppState>,
    json_data: String,
) -> Result<serde_json::Value, String> {
    do_import_workflow(&state.sea_db, json_data).await
}

/// 批量导入 n8n 目录中的所有工作流 JSON 文件
fn collect_json_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_json_files(&path, files);
            } else if path.extension().is_some_and(|e| e == "json") {
                files.push(path);
            }
        }
    }
}

#[tauri::command]
pub async fn import_n8n_directory(
    state: State<'_, AppState>,
    path: String,
) -> Result<serde_json::Value, String> {
    use std::fs;
    use std::path::Path;

    let db = &state.sea_db;
    let dir = Path::new(&path);
    if !dir.is_dir() {
        return Err(ErrorResponse::new(workflow_err::NOT_FOUND)
            .with_detail(format!("Path does not exist or is not a directory: {}", path))
            .into());
    }

    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    let mut errors = Vec::new();

    let mut json_files: Vec<std::path::PathBuf> = Vec::new();
    collect_json_files(dir, &mut json_files);

    for file_path in json_files {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("{}: {}", file_path.display(), e))?;
        let raw_json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("{}: JSON parse error: {}", file_path.display(), e));
                continue;
            },
        };

        if !is_n8n_format(&raw_json) {
            skipped.push(file_path.display().to_string());
            continue;
        }

        let workflow_name = raw_json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Imported n8n Workflow")
            .to_string();

        if let Ok(Some(existing)) = check_workflow_duplicate(db, &workflow_name).await {
            skipped.push(format!(
                "{} (semantically similar to '{}')",
                file_path.display(),
                existing
            ));
            continue;
        }

        match convert_n8n_to_axagent(db, &raw_json).await {
            Ok(template) => {
                let am = model_to_active_model(&template);
                if let Err(e) = db_repo::insert_workflow_template(db, am).await {
                    errors.push(format!("{}: save error: {}", file_path.display(), e));
                } else {
                    imported.push(template.name);
                }
            },
            Err(e) => errors.push(format!("{}: conversion error: {}", file_path.display(), e)),
        }
    }

    Ok(serde_json::json!({
        "imported": imported.len(),
        "imported_names": imported,
        "skipped": skipped.len(),
        "skipped_reasons": skipped,
        "errors": errors.len(),
        "error_details": errors,
    }))
}

/// 批量导入目录下所有 JSON 工作流模板文件
#[tauri::command]
pub async fn import_workflow_directory(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<serde_json::Value, String> {
    use std::fs;
    use std::path::Path;

    let db = &state.sea_db;
    let dir = Path::new(&path);
    if !dir.is_dir() {
        return Err(ErrorResponse::new(workflow_err::NOT_FOUND)
            .with_detail(format!("Path does not exist or is not a directory: {}", path))
            .into());
    }

    let mut imported = Vec::new();
    let mut errors = Vec::new();

    let mut json_files: Vec<std::path::PathBuf> = Vec::new();
    collect_json_files(dir, &mut json_files);

    for file_path in json_files {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("{}: {}", file_path.display(), e))?;
        if serde_json::from_str::<serde_json::Value>(&content).is_err() {
            errors.push(format!("{}: Invalid JSON format", file_path.display()));
            continue;
        }

        match do_import_workflow(db, content).await {
            Ok(val) => {
                if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                    imported.push(id.to_string());
                }
            },
            Err(e) => {
                errors.push(format!("{}: {}", file_path.display(), e));
            },
        }
    }

    Ok(serde_json::json!({
        "imported": imported.len(),
        "errors": errors.len(),
        "error_details": errors,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── is_n8n_format ──────────────────────────────────────

    #[test]
    fn test_is_n8n_format_true() {
        let json = json!({
            "nodes": [
                { "type": "n8n-nodes-base.httpRequest" },
                { "type": "n8n-nodes-base.code" }
            ]
        });
        assert!(is_n8n_format(&json));
    }

    #[test]
    fn test_is_n8n_format_false_for_axagent() {
        let json = json!({
            "nodes": [
                { "type": "Agent", "id": "1" },
                { "type": "Code", "id": "2" }
            ]
        });
        assert!(!is_n8n_format(&json));
    }

    #[test]
    fn test_is_n8n_format_empty_nodes() {
        let json = json!({ "nodes": [] });
        assert!(!is_n8n_format(&json));
    }

    #[test]
    fn test_is_n8n_format_no_nodes_key() {
        let json = json!({ "other": "value" });
        assert!(!is_n8n_format(&json));
    }

    // ── infer_agent_from_n8n ────────────────────────────────

    #[test]
    fn test_infer_by_node_name_review() {
        let (profile, role, expert, prompt) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Review PR changes");
        assert_eq!(profile, "code-reviewer");
        assert_eq!(role, "reviewer");
        assert_eq!(expert, "code-reviewer");
        assert!(prompt.contains("Review"));
    }

    #[test]
    fn test_infer_by_node_name_debug() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Debug login error");
        assert_eq!(profile, "debug-expert");
        assert_eq!(role, "developer");
    }

    #[test]
    fn test_infer_by_node_name_test() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Test API endpoints");
        assert_eq!(profile, "debug-expert");
        assert_eq!(role, "reviewer");
    }

    #[test]
    fn test_infer_by_node_name_doc() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Write documentation");
        assert_eq!(profile, "tech-writer");
        assert_eq!(role, "synthesizer");
    }

    #[test]
    fn test_infer_by_node_name_plan() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Plan architecture");
        assert_eq!(profile, "architect");
        assert_eq!(role, "planner");
    }

    #[test]
    fn test_infer_by_node_name_monitor() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Monitor server health");
        assert_eq!(profile, "devops-engineer");
        assert_eq!(role, "executor");
    }

    #[test]
    fn test_infer_by_node_name_analyze() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.noOp", "Analyze user data");
        assert_eq!(profile, "data-analyst");
        assert_eq!(role, "researcher");
    }

    #[test]
    fn test_infer_by_node_type_http() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.httpRequest", "do something generic");
        assert_eq!(profile, "devops-engineer");
        assert_eq!(role, "executor");
    }

    #[test]
    fn test_infer_by_node_type_database() {
        let (profile, role, _, _) = infer_agent_from_n8n("n8n-nodes-base.postgres", "generic node");
        assert_eq!(profile, "sql-expert");
        assert_eq!(role, "researcher");
    }

    #[test]
    fn test_infer_by_node_type_code() {
        let (profile, role, _, _) = infer_agent_from_n8n("n8n-nodes-base.code", "generic node");
        assert_eq!(profile, "senior-developer");
        assert_eq!(role, "developer");
    }

    #[test]
    fn test_infer_by_node_type_ai() {
        let (profile, role, _, _) = infer_agent_from_n8n("n8n-nodes-base.openAi", "generic node");
        assert_eq!(profile, "general-assistant");
        assert_eq!(role, "coordinator");
    }

    #[test]
    fn test_infer_by_node_type_email() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.emailSend", "generic node");
        assert_eq!(profile, "product-manager");
        assert_eq!(role, "coordinator");
    }

    #[test]
    fn test_infer_by_node_type_file() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.spreadsheetFile", "generic node");
        assert_eq!(profile, "data-analyst");
        assert_eq!(role, "researcher");
    }

    #[test]
    fn test_infer_by_node_type_security() {
        let (profile, role, _, _) = infer_agent_from_n8n("n8n-nodes-base.oauth2", "generic node");
        assert_eq!(profile, "security-auditor");
        assert_eq!(role, "reviewer");
    }

    #[test]
    fn test_infer_by_node_type_transform() {
        let (profile, role, _, _) = infer_agent_from_n8n("n8n-nodes-base.merge", "generic node");
        assert_eq!(profile, "tech-writer");
        assert_eq!(role, "synthesizer");
    }

    #[test]
    fn test_infer_fallback_to_debug_expert() {
        let (profile, role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.somethingUnknown", "unknown node");
        assert_eq!(profile, "debug-expert");
        assert_eq!(role, "executor");
    }

    #[test]
    fn test_infer_name_has_priority_over_type() {
        // node name "review" should match before node type "http"
        let (profile, _role, _, _) =
            infer_agent_from_n8n("n8n-nodes-base.httpRequest", "review API response");
        assert_eq!(profile, "code-reviewer");
        // 确认名称关键词 "review" 优先级高于节点类型 "http" — 映射到 code-reviewer 而非 devops-engineer
    }

    // ── extract_goal_from_n8n ───────────────────────────────

    #[test]
    fn test_extract_goal_http_node() {
        let node = json!({
            "type": "n8n-nodes-base.httpRequest",
            "parameters": {
                "method": "GET",
                "url": "https://api.example.com/users"
            }
        });
        let goal = extract_goal_from_n8n(&node);
        assert!(goal.contains("GET"));
        assert!(goal.contains("api.example.com"));
    }

    #[test]
    fn test_extract_goal_database_node() {
        let node = json!({
            "type": "n8n-nodes-base.sqlite",
            "parameters": {
                "operation": "SELECT",
                "table": "orders"
            }
        });
        let goal = extract_goal_from_n8n(&node);
        assert!(goal.contains("SELECT"));
        assert!(goal.contains("orders"));
    }

    #[test]
    fn test_extract_goal_email_node() {
        let node = json!({
            "type": "n8n-nodes-base.emailSend",
            "parameters": {
                "subject": "Weekly Report"
            }
        });
        let goal = extract_goal_from_n8n(&node);
        assert!(goal.contains("Weekly Report"));
    }

    #[test]
    fn test_extract_goal_empty_node() {
        let node = json!({});
        let goal = extract_goal_from_n8n(&node);
        // 无任何字段时返回 "Unnamed ()"
        assert!(!goal.is_empty());
        assert!(goal.starts_with("Unnamed"));
    }
}
