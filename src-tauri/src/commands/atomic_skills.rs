use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::AppState;
use axagent_core::crypto::decrypt_key;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest, ProviderType};

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicSkillResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub entry_type: String,
    pub entry_ref: String,
    pub category: String,
    pub tags: Vec<String>,
    pub version: String,
    pub enabled: bool,
    pub source: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<axagent_core::entity::atomic_skills::Model> for AtomicSkillResponse {
    fn from(m: axagent_core::entity::atomic_skills::Model) -> Self {
        let tags: Vec<String> = m
            .tags
            .as_ref()
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or_default();
        let input_schema: Option<serde_json::Value> = m
            .input_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        let output_schema: Option<serde_json::Value> = m
            .output_schema
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());

        Self {
            id: m.id,
            name: m.name,
            description: m.description,
            input_schema,
            output_schema,
            entry_type: m.entry_type,
            entry_ref: m.entry_ref,
            category: m.category,
            tags,
            version: m.version,
            enabled: m.enabled,
            source: m.source,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAtomicSkillRequest {
    pub name: String,
    pub description: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub entry_type: String,
    pub entry_ref: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAtomicSkillRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub entry_type: Option<String>,
    pub entry_ref: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicSkillFilterRequest {
    pub category: Option<String>,
    pub source: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillReferenceResponse {
    pub id: String,
    pub skill_id: String,
    pub workflow_id: String,
    pub node_id: String,
    pub created_at: i64,
}

impl From<axagent_core::entity::skill_references::Model> for SkillReferenceResponse {
    fn from(m: axagent_core::entity::skill_references::Model) -> Self {
        Self {
            id: m.id,
            skill_id: m.skill_id,
            workflow_id: m.workflow_id,
            node_id: m.node_id,
            created_at: m.created_at,
        }
    }
}

// ── Commands ──

#[tauri::command]
pub async fn list_atomic_skills(
    state: State<'_, AppState>,
    filter: Option<AtomicSkillFilterRequest>,
) -> Result<Vec<AtomicSkillResponse>, String> {
    let f = filter.unwrap_or(AtomicSkillFilterRequest {
        category: None,
        source: None,
        enabled: None,
    });

    let skills = axagent_core::repo::atomic_skill::list_atomic_skills(
        &state.sea_db,
        f.category.as_deref(),
        f.source.as_deref(),
        f.enabled,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(skills.into_iter().map(AtomicSkillResponse::from).collect())
}

#[tauri::command]
pub async fn get_atomic_skill(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<AtomicSkillResponse>, String> {
    let skill = axagent_core::repo::atomic_skill::get_atomic_skill(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(skill.map(AtomicSkillResponse::from))
}

#[tauri::command]
pub async fn create_atomic_skill(
    state: State<'_, AppState>,
    params: CreateAtomicSkillRequest,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let input_schema = params
        .input_schema
        .as_ref()
        .and_then(|s| serde_json::to_string(s).ok());
    let output_schema = params
        .output_schema
        .as_ref()
        .and_then(|s| serde_json::to_string(s).ok());
    let tags = params
        .tags
        .as_ref()
        .and_then(|t| serde_json::to_string(t).ok());

    // Check name uniqueness
    let existing =
        axagent_core::repo::atomic_skill::check_name_uniqueness(&state.sea_db, &params.name)
            .await
            .map_err(|e| e.to_string())?;

    if existing.is_some() {
        return Err(format!(
            "Atomic skill with name '{}' already exists",
            params.name
        ));
    }

    // Check semantic uniqueness
    let semantic = axagent_core::repo::atomic_skill::check_semantic_uniqueness(
        &state.sea_db,
        &params.entry_type,
        &params.entry_ref,
        input_schema.as_deref(),
        output_schema.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    if let Some(existing) = semantic {
        return Err(format!(
            "Semantically identical atomic skill already exists: '{}' (id: {})",
            existing.name, existing.id
        ));
    }

    axagent_core::repo::atomic_skill::create_atomic_skill(
        &state.sea_db,
        &id,
        &params.name,
        &params.description,
        input_schema.as_deref(),
        output_schema.as_deref(),
        &params.entry_type,
        &params.entry_ref,
        params.category.as_deref().unwrap_or("general"),
        tags.as_deref(),
        params.version.as_deref().unwrap_or("1.0.0"),
        params.enabled.unwrap_or(true),
        params.source.as_deref().unwrap_or("atomic"),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
pub async fn update_atomic_skill(
    state: State<'_, AppState>,
    id: String,
    params: UpdateAtomicSkillRequest,
) -> Result<bool, String> {
    let input_schema = params
        .input_schema
        .as_ref()
        .map(|s| serde_json::to_string(s).ok());
    let output_schema = params
        .output_schema
        .as_ref()
        .map(|s| serde_json::to_string(s).ok());
    let tags = params.tags.as_ref().map(|t| serde_json::to_string(t).ok());

    axagent_core::repo::atomic_skill::update_atomic_skill(
        &state.sea_db,
        &id,
        params.name,
        params.description,
        input_schema,
        output_schema,
        params.entry_type,
        params.entry_ref,
        params.category,
        tags,
        params.version,
        params.enabled,
        params.source,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_atomic_skill(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    // Check reference count
    let count = axagent_core::repo::skill_reference::count_references(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())?;

    if count > 0 {
        return Err(format!(
            "Cannot delete atomic skill: still referenced by {} workflow(s)",
            count
        ));
    }

    axagent_core::repo::atomic_skill::delete_atomic_skill(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_atomic_skill(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<bool, String> {
    axagent_core::repo::atomic_skill::toggle_atomic_skill(&state.sea_db, &id, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_semantic_uniqueness(
    state: State<'_, AppState>,
    entry_type: String,
    entry_ref: String,
    input_schema: Option<serde_json::Value>,
    output_schema: Option<serde_json::Value>,
) -> Result<Option<AtomicSkillResponse>, String> {
    let input_str = input_schema
        .as_ref()
        .and_then(|s| serde_json::to_string(s).ok());
    let output_str = output_schema
        .as_ref()
        .and_then(|s| serde_json::to_string(s).ok());

    let existing = axagent_core::repo::atomic_skill::check_semantic_uniqueness(
        &state.sea_db,
        &entry_type,
        &entry_ref,
        input_str.as_deref(),
        output_str.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(existing.map(AtomicSkillResponse::from))
}

#[tauri::command]
pub async fn get_skill_references(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<Vec<SkillReferenceResponse>, String> {
    let refs =
        axagent_core::repo::skill_reference::get_references_by_skill(&state.sea_db, &skill_id)
            .await
            .map_err(|e| e.to_string())?;

    Ok(refs.into_iter().map(SkillReferenceResponse::from).collect())
}

#[tauri::command]
pub async fn execute_atomic_skill(
    state: State<'_, AppState>,
    id: String,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let skill_model = axagent_core::repo::atomic_skill::get_atomic_skill(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Atomic skill not found: {}", id))?;

    // Convert to business model
    let entry_type = match skill_model.entry_type.as_str() {
        "builtin" => axagent_trajectory::EntryType::Builtin,
        "mcp" => axagent_trajectory::EntryType::Mcp,
        "local" => axagent_trajectory::EntryType::Local,
        "plugin" => axagent_trajectory::EntryType::Plugin,
        _ => return Err(format!("Unknown entry type: {}", skill_model.entry_type)),
    };

    let skill = axagent_trajectory::AtomicSkill {
        id: skill_model.id,
        name: skill_model.name,
        description: skill_model.description,
        input_schema: skill_model
            .input_schema
            .and_then(|s| serde_json::from_str(&s).ok()),
        output_schema: skill_model
            .output_schema
            .and_then(|s| serde_json::from_str(&s).ok()),
        entry_type,
        entry_ref: skill_model.entry_ref,
        category: skill_model.category,
        tags: skill_model
            .tags
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default(),
        version: skill_model.version,
        enabled: skill_model.enabled,
        source: skill_model.source,
        created_at: skill_model.created_at,
        updated_at: skill_model.updated_at,
    };

    let result = match skill.entry_type {
        axagent_trajectory::EntryType::Builtin => {
            axagent_trajectory::AtomicSkillExecutor::execute_builtin(&skill.entry_ref, input).await
        }
        _ => Err(axagent_trajectory::AtomicSkillError {
            error_type: "not_implemented".to_string(),
            message: format!(
                "Execution of {} skills not yet implemented in this context",
                skill.entry_type
            ),
        }),
    };

    serde_json::to_value(&result).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMatchResponse {
    pub existing_skill: AtomicSkillResponse,
    pub similarity_score: f32,
    pub match_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSemanticCheckRequest {
    pub skills: Vec<SkillToCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillToCheck {
    pub name: String,
    pub description: String,
    pub entry_type: String,
    pub entry_ref: String,
    pub category: String,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSemanticCheckResponse {
    pub matches: Vec<NodeSkillMatches>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSkillMatches {
    pub node_id: Option<String>,
    pub skill_name: String,
    pub matches: Vec<SkillMatchResponse>,
}

#[tauri::command]
pub async fn check_skill_semantic_matches(
    state: State<'_, AppState>,
    request: SkillSemanticCheckRequest,
    min_similarity: Option<f32>,
) -> Result<SkillSemanticCheckResponse, String> {
    let min_sim = min_similarity.unwrap_or(0.6);

    let mut all_matches: Vec<NodeSkillMatches> = Vec::new();

    for skill_to_check in request.skills {
        let matches = axagent_core::repo::atomic_skill::find_similar_skills(
            &state.sea_db,
            &skill_to_check.name,
            &skill_to_check.description,
            &skill_to_check.entry_type,
            &skill_to_check.entry_ref,
            &skill_to_check.category,
            min_sim,
        )
        .await
        .map_err(|e| e.to_string())?;

        if !matches.is_empty() {
            all_matches.push(NodeSkillMatches {
                node_id: skill_to_check.node_id,
                skill_name: skill_to_check.name,
                matches: matches
                    .into_iter()
                    .map(|m| SkillMatchResponse {
                        existing_skill: AtomicSkillResponse::from(m.existing_skill),
                        similarity_score: m.similarity_score,
                        match_reasons: m.match_reasons,
                    })
                    .collect(),
            });
        }
    }

    Ok(SkillSemanticCheckResponse {
        matches: all_matches,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpgradeRequest {
    pub existing_skill_id: String,
    pub generated_name: String,
    pub generated_description: String,
    pub generated_input_schema: Option<serde_json::Value>,
    pub generated_output_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpgradeSuggestion {
    pub name: String,
    pub description: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpgradeResponse {
    pub suggestion: SkillUpgradeSuggestion,
}

#[tauri::command]
pub async fn upgrade_skill_with_llm(
    state: State<'_, AppState>,
    request: SkillUpgradeRequest,
) -> Result<SkillUpgradeResponse, String> {
    let existing_skill = axagent_core::repo::atomic_skill::get_atomic_skill(
        &state.sea_db,
        &request.existing_skill_id,
    )
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Skill not found".to_string())?;

    let settings = axagent_core::repo::settings::get_settings(&state.sea_db)
        .await
        .map_err(|e| e.to_string())?;

    let provider_id = settings
        .default_provider_id
        .as_ref()
        .ok_or_else(|| "No default provider configured".to_string())?;
    let model_id = settings
        .default_model_id
        .as_ref()
        .ok_or_else(|| "No default model configured".to_string())?;

    let provider = axagent_core::repo::provider::get_provider(&state.sea_db, provider_id)
        .await
        .map_err(|e| e.to_string())?;
    let key_row = axagent_core::repo::provider::get_active_key(&state.sea_db, &provider.id)
        .await
        .map_err(|e| e.to_string())?;
    let decrypted_key =
        decrypt_key(&key_row.key_encrypted, &state.master_key).map_err(|e| e.to_string())?;

    let registry = axagent_providers::registry::ProviderRegistry::create_default();
    let registry_key = match provider.provider_type {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    };
    let adapter = registry
        .get(registry_key)
        .ok_or_else(|| format!("Provider adapter not found for {}", registry_key))?;

    let existing_input = existing_skill
        .input_schema
        .as_ref()
        .and_then(|s: &String| serde_json::from_str::<serde_json::Value>(s).ok())
        .map(|j: serde_json::Value| serde_json::to_string_pretty(&j).unwrap_or_default())
        .unwrap_or_else(|| "null".to_string());
    let existing_output = existing_skill
        .output_schema
        .as_ref()
        .and_then(|s: &String| serde_json::from_str::<serde_json::Value>(s).ok())
        .map(|j: serde_json::Value| serde_json::to_string_pretty(&j).unwrap_or_default())
        .unwrap_or_else(|| "null".to_string());
    let generated_input = request
        .generated_input_schema
        .as_ref()
        .map(|j: &serde_json::Value| serde_json::to_string_pretty(j).unwrap_or_default())
        .unwrap_or_else(|| "null".to_string());
    let generated_output = request
        .generated_output_schema
        .as_ref()
        .map(|j: &serde_json::Value| serde_json::to_string_pretty(j).unwrap_or_default())
        .unwrap_or_else(|| "null".to_string());

    let prompt = format!(
        r#"You are an AI skill upgrade advisor. Your task is to analyze an existing atomic skill and a newly generated skill specification, then produce an improved version.

## Existing Skill (MUST BE PRESERVED - used by existing workflows)
- Name: {}
- Description: {}
- Entry Type: {} (MUST NOT CHANGE - determines how skill is executed)
- Entry Ref: {} (MUST NOT CHANGE - the actual executable/function reference)
- Category: {} (MUST NOT CHANGE - for skill organization)
- Input Schema:
{}
- Output Schema:
{}

## Generated Skill (from LLM decomposition)
- Name: {}
- Description: {}
- Input Schema:
{}
- Output Schema:
{}

## CRITICAL COMPATIBILITY REQUIREMENTS
1. The entry_ref, entry_type, and category of the existing skill MUST remain UNCHANGED
2. The input_schema and output_schema MUST be BACKWARD COMPATIBLE with the existing skill
3. Only description, name can be improved; schemas must maintain the same contract
4. If schema changes are needed, they must be additive (optional fields) or backward-compatible

## Instructions
Analyze both skills and create an upgraded version that:
1. MAINTAINS FULL BACKWARD COMPATIBILITY with existing workflows
2. Keeps the same entry_ref, entry_type, and category
3. Has a clearer, more comprehensive description
4. Has an improved description while preserving the exact same input/output interface
5. The reasoning must explicitly confirm compatibility preservation

Output your result in JSON format:
{{
  "name": "upgraded skill name",
  "description": "comprehensive description",
  "input_schema": {{...}},  // MUST be backward compatible
  "output_schema": {{...}},  // MUST be backward compatible
  "reasoning": "explanation confirming backward compatibility is preserved"
}}

Only output the JSON, no other text."#,
        existing_skill.name,
        existing_skill.description,
        existing_skill.entry_type,
        existing_skill.entry_ref,
        existing_skill.category,
        existing_input,
        existing_output,
        request.generated_name,
        request.generated_description,
        generated_input,
        generated_output
    );

    let base_url =
        axagent_providers::resolve_base_url_for_type(&provider.api_host, &provider.provider_type);
    let ctx = axagent_providers::ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id,
        provider_id: provider.id.clone(),
        base_url: Some(base_url),
        api_path: provider.api_path.clone(),
        proxy_config: provider.proxy_config.clone(),
        custom_headers: None,
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let llm_request = ChatRequest {
        model: model_id.clone(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(prompt),
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: Some(0.7),
        top_p: None,
        max_tokens: Some(4096),
        stream: false,
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
    };

    let response = adapter
        .chat(&ctx, llm_request)
        .await
        .map_err(|e| format!("LLM call failed: {}", e))?;

    let content = response.content.trim();

    let json_start = content.find('{');
    let json_end = content.rfind('}').map(|i| i + 1);
    let json_str = match (json_start, json_end) {
        (Some(start), Some(end)) => &content[start..end],
        _ => content,
    };

    let suggestion: SkillUpgradeSuggestion = serde_json::from_str(json_str).map_err(|e| {
        format!(
            "Failed to parse LLM response as JSON: {}. Content: {}",
            e, content
        )
    })?;

    Ok(SkillUpgradeResponse { suggestion })
}
