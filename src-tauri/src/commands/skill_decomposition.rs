use axagent_plugins::{PluginManager, PluginManagerConfig};
use axagent_trajectory::ToolResolver;
use dirs;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tauri::State;

use crate::app_state::AppState;

#[derive(Debug, Clone)]
pub struct CachedDecomposition {
    pub result: axagent_trajectory::DecompositionResult,
    pub cache_id: String,
    pub created_at: Instant,
}

pub struct DecompositionCache {
    cache: HashMap<String, CachedDecomposition>,
    ttl: Duration,
}

impl DecompositionCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: HashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub fn get(&self, key: &str) -> Option<CachedDecomposition> {
        self.cache.get(key).and_then(|cached| {
            if cached.created_at.elapsed() < self.ttl {
                Some(cached.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&mut self, key: String, value: CachedDecomposition) {
        self.cache.insert(key, value);
    }

    pub fn cleanup_expired(&mut self) {
        self.cache.retain(|_, v| v.created_at.elapsed() < self.ttl);
    }
}

lazy_static::lazy_static! {
    pub static ref DECOMPOSITION_CACHE: tokio::sync::Mutex<DecompositionCache> =
        tokio::sync::Mutex::new(DecompositionCache::new(300));
}

fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionPreviewResponse {
    pub tool_dependencies: Vec<ToolDependencyPreview>,
    pub workflow_nodes: serde_json::Value,
    pub workflow_edges: serde_json::Value,
    pub original_source: CompositeSourceInfoResponse,
    pub cache_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDependencyPreview {
    pub name: String,
    pub tool_type: String,
    pub status: String,
    pub install_instructions: Option<String>,
    pub config_requirements: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeSourceInfoResponse {
    pub market: String,
    pub repo: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMatchResponse {
    pub tool_name: String,
    pub tool_type: String,
    pub description: String,
    pub similarity_score: f32,
    pub match_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSemanticCheckRequest {
    pub tools: Vec<ToolToCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolToCheck {
    pub name: String,
    pub description: String,
    pub tool_type: String,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSemanticCheckResponse {
    pub matches: Vec<NodeToolMatches>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeToolMatches {
    pub node_id: Option<String>,
    pub tool_name: String,
    pub matches: Vec<ToolMatchResponse>,
}

fn compute_word_set(text: &str) -> std::collections::HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(|s| s.to_string())
        .collect()
}

fn jaccard_similarity(
    set1: &std::collections::HashSet<String>,
    set2: &std::collections::HashSet<String>,
) -> f32 {
    if set1.is_empty() && set2.is_empty() {
        return 1.0;
    }
    if set1.is_empty() || set2.is_empty() {
        return 0.0;
    }
    let intersection = set1.intersection(set2).count();
    let union = set1.union(set2).count();
    intersection as f32 / union as f32
}

fn find_similar_local_tools(
    target_name: &str,
    target_description: &str,
    _target_tool_type: &str,
    local_tool_defs: &std::collections::HashMap<String, axagent_agent::LocalToolDef>,
    min_similarity: f32,
) -> Vec<ToolMatchResponse> {
    let target_words_name = compute_word_set(target_name);
    let target_words_desc = compute_word_set(target_description);

    let mut matches: Vec<ToolMatchResponse> = Vec::new();

    for (tool_name, def) in local_tool_defs {
        let mut reasons = Vec::new();
        let mut score: f32 = 0.0;
        let mut weight_sum: f32 = 0.0;

        let name_words = compute_word_set(tool_name);
        let desc_words = compute_word_set(&def.description);

        let name_sim = jaccard_similarity(&target_words_name, &name_words);
        if name_sim > 0.3 {
            weight_sum += 0.5;
            score += name_sim * 0.5;
            if name_sim > 0.5 {
                reasons.push(format!("名称相似度: {:.0}%", name_sim * 100.0));
            }
        }

        let desc_sim = jaccard_similarity(&target_words_desc, &desc_words);
        if desc_sim > 0.2 {
            weight_sum += 0.5;
            score += desc_sim * 0.5;
            if desc_sim > 0.4 {
                reasons.push(format!("描述相似度: {:.0}%", desc_sim * 100.0));
            }
        }

        let final_score = if weight_sum > 0.0 {
            score / weight_sum
        } else {
            0.0
        };

        if final_score >= min_similarity {
            matches.push(ToolMatchResponse {
                tool_name: tool_name.clone(),
                tool_type: format!("local/{}", def.group_id),
                description: def.description.clone(),
                similarity_score: final_score,
                match_reasons: reasons,
            });
        }
    }

    matches.sort_by(|a, b| {
        b.similarity_score
            .partial_cmp(&a.similarity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matches
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUpgradeRequest {
    pub existing_tool_name: String,
    pub existing_tool_description: String,
    pub existing_tool_type: String,
    pub existing_input_schema: Option<serde_json::Value>,
    pub existing_output_schema: Option<serde_json::Value>,
    pub generated_name: String,
    pub generated_description: String,
    pub generated_input_schema: Option<serde_json::Value>,
    pub generated_output_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUpgradeSuggestion {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUpgradeResponse {
    pub suggestion: ToolUpgradeSuggestion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewDecompositionRequest {
    pub name: String,
    pub description: String,
    pub content: String,
    pub source: String,
    pub version: Option<String>,
    pub repo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmDecompositionRequest {
    pub preview: PreviewDecompositionRequest,
    pub cache_id: Option<String>,
    pub workflow_name: String,
    pub workflow_description: Option<String>,
    pub workflow_icon: Option<String>,
}

fn create_plugin_manager() -> Result<PluginManager, String> {
    let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    let config_home = home.join(".claw");
    let mut config = PluginManagerConfig::new(config_home);
    config.external_dirs = vec![
        home.join(".axagent").join("skills"),
        home.join(".claude").join("skills"),
        home.join(".agents").join("skills"),
    ];
    Ok(PluginManager::new(config))
}

async fn get_mcp_tool_names(db: &sea_orm::DatabaseConnection) -> Result<Vec<String>, String> {
    let servers = axagent_core::repo::mcp_server::list_mcp_servers(db)
        .await
        .map_err(|e| e.to_string())?;
    let mut tool_names = Vec::new();
    for server in servers {
        if let Ok(tools) =
            axagent_core::repo::mcp_server::list_tools_for_server(db, &server.id).await
        {
            for tool in tools {
                tool_names.push(tool.name);
            }
        }
    }
    Ok(tool_names)
}

async fn get_local_tool_names(state: &AppState) -> Vec<String> {
    let registry = state.local_tool_registry.lock().await;
    registry.enabled_tool_names()
}

fn get_plugin_tool_names() -> Result<Vec<String>, String> {
    let manager = create_plugin_manager()?;
    let tools = manager.aggregated_tools().map_err(|e| e.to_string())?;
    Ok(tools
        .into_iter()
        .map(|t| t.definition().name.clone())
        .collect())
}

// ── Commands ──

#[tauri::command]
pub async fn preview_decomposition(
    state: State<'_, AppState>,
    request: PreviewDecompositionRequest,
) -> Result<DecompositionPreviewResponse, String> {
    let content_hash = compute_content_hash(&request.content);

    {
        let mut cache = DECOMPOSITION_CACHE.lock().await;
        cache.cleanup_expired();
        if let Some(cached) = cache.get(&content_hash) {
            let dep_results = ToolResolver::check_tool_dependencies(
                &cached.result.tool_dependencies,
                &get_mcp_tool_names(&state.sea_db).await.unwrap_or_default(),
                &get_local_tool_names(&state).await,
                &get_plugin_tool_names().unwrap_or_default(),
            );

            return Ok(DecompositionPreviewResponse {
                tool_dependencies: dep_results
                    .iter()
                    .map(|d| ToolDependencyPreview {
                        name: d.dependency.name.clone(),
                        tool_type: d.dependency.tool_type.clone(),
                        status: format!("{:?}", d.dependency.status),
                        install_instructions: d.install_instructions.clone(),
                        config_requirements: d.config_requirements.clone(),
                    })
                    .collect(),
                workflow_nodes: cached.result.workflow_nodes.clone(),
                workflow_edges: cached.result.workflow_edges.clone(),
                original_source: CompositeSourceInfoResponse {
                    market: cached.result.original_source.market.clone(),
                    repo: cached.result.original_source.repo.clone(),
                    version: cached.result.original_source.version.clone(),
                },
                cache_id: cached.cache_id.clone(),
            });
        }
    }

    let composite = axagent_trajectory::CompositeSkillData {
        name: request.name.clone(),
        description: request.description.clone(),
        content: request.content.clone(),
        source: request.source.clone(),
        version: request.version.clone(),
        repo: request.repo.clone(),
    };

    let parsed = axagent_trajectory::SkillDecomposer::parse(&composite).map_err(|e| e.message)?;

    let result = axagent_trajectory::SkillDecomposer::decompose(&parsed)
        .map_err(|e| e.message)?;

    let mcp_tools = get_mcp_tool_names(&state.sea_db).await.unwrap_or_default();
    let local_tools = get_local_tool_names(&state).await;
    let plugin_tools = get_plugin_tool_names().unwrap_or_default();

    let dep_results = ToolResolver::check_tool_dependencies(
        &result.tool_dependencies,
        &mcp_tools,
        &local_tools,
        &plugin_tools,
    );

    let cache_id = uuid::Uuid::new_v4().to_string();

    {
        let mut cache = DECOMPOSITION_CACHE.lock().await;
        cache.set(
            content_hash,
            CachedDecomposition {
                result: result.clone(),
                cache_id: cache_id.clone(),
                created_at: Instant::now(),
            },
        );
    }

    Ok(DecompositionPreviewResponse {
        tool_dependencies: dep_results
            .iter()
            .map(|d| ToolDependencyPreview {
                name: d.dependency.name.clone(),
                tool_type: d.dependency.tool_type.clone(),
                status: format!("{:?}", d.dependency.status),
                install_instructions: d.install_instructions.clone(),
                config_requirements: d.config_requirements.clone(),
            })
            .collect(),
        workflow_nodes: result.workflow_nodes,
        workflow_edges: result.workflow_edges,
        original_source: CompositeSourceInfoResponse {
            market: result.original_source.market,
            repo: result.original_source.repo,
            version: result.original_source.version,
        },
        cache_id,
    })
}

#[tauri::command]
pub async fn confirm_decomposition(
    state: State<'_, AppState>,
    request: ConfirmDecompositionRequest,
) -> Result<serde_json::Value, String> {
    let composite = axagent_trajectory::CompositeSkillData {
        name: request.preview.name.clone(),
        description: request.preview.description.clone(),
        content: request.preview.content.clone(),
        source: request.preview.source.clone(),
        version: request.preview.version.clone(),
        repo: request.preview.repo.clone(),
    };

    let parsed = axagent_trajectory::SkillDecomposer::parse(&composite).map_err(|e| e.message)?;

    let result = axagent_trajectory::SkillDecomposer::decompose(&parsed)
        .map_err(|e| e.message)?;

    let workflow_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let composite_source = serde_json::to_string(&result.original_source).ok();

    let template = axagent_core::entity::workflow_template::ActiveModel {
        id: Set(workflow_id.clone()),
        name: Set(request.workflow_name),
        description: Set(request.workflow_description),
        icon: Set(request.workflow_icon.unwrap_or_else(|| "🔧".to_string())),
        tags: Set(Some("[]".to_string())),
        version: Set(1),
        is_preset: Set(false),
        is_editable: Set(true),
        is_public: Set(false),
        trigger_config: Set(None),
        nodes: Set(serde_json::to_string(&result.workflow_nodes).unwrap_or_default()),
        edges: Set(serde_json::to_string(&result.workflow_edges).unwrap_or_default()),
        input_schema: Set(None),
        output_schema: Set(None),
        variables: Set(Some("[]".to_string())),
        error_config: Set(None),
        composite_source: Set(composite_source),
        created_at: Set(now),
        updated_at: Set(now),
    };

    axagent_core::repo::workflow_template::insert_workflow_template(&state.sea_db, template)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "workflow_id": workflow_id,
    }))
}

#[tauri::command]
pub async fn generate_missing_tool(
    state: State<'_, AppState>,
    name: String,
    description: String,
    input_schema: serde_json::Value,
    output_schema: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let input = axagent_runtime::tool_generator::ToolGenerationInput {
        name,
        description,
        input_schema,
        output_schema,
    };

    // Build the generation prompt
    let _prompt = axagent_runtime::tool_generator::ToolGenerator::build_generation_prompt(&input);

    // TODO: Call Developer Agent to generate the Prompt template
    // For now, create a simple template
    let template = "You are a tool that processes the following input according to its description.\n\nInput: {{{{input}}}}\n\nProcess the input and return a result that matches the expected output format.".to_string();

    let tool = axagent_runtime::tool_generator::ToolGenerator::parse_agent_response(
        &template, &input, None,
    )
    .map_err(|e| e.to_string())?;

    // Persist to database
    axagent_runtime::tool_generator::persist_to_db(&tool, &state.sea_db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "tool_name": tool.tool_name,
        "success": true,
    }))
}

#[tauri::command]
pub async fn check_tool_semantic_matches(
    state: State<'_, AppState>,
    request: ToolSemanticCheckRequest,
    min_similarity: Option<f32>,
) -> Result<ToolSemanticCheckResponse, String> {
    let min_sim = min_similarity.unwrap_or(0.6);

    let registry = state.local_tool_registry.lock().await;
    let local_tool_defs = registry.all_tool_defs();

    let mut all_matches: Vec<NodeToolMatches> = Vec::new();

    for tool_to_check in request.tools {
        let matches = find_similar_local_tools(
            &tool_to_check.name,
            &tool_to_check.description,
            &tool_to_check.tool_type,
            local_tool_defs,
            min_sim,
        );

        if !matches.is_empty() {
            all_matches.push(NodeToolMatches {
                node_id: tool_to_check.node_id,
                tool_name: tool_to_check.name,
                matches,
            });
        }
    }

    Ok(ToolSemanticCheckResponse {
        matches: all_matches,
    })
}

#[tauri::command]
pub async fn upgrade_tool_with_llm(
    state: State<'_, AppState>,
    request: ToolUpgradeRequest,
) -> Result<ToolUpgradeResponse, String> {
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
        axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &state.master_key)
            .map_err(|e| e.to_string())?;

    let registry = axagent_providers::registry::ProviderRegistry::create_default();
    let registry_key = match provider.provider_type {
        axagent_core::types::ProviderType::OpenAI => "openai",
        axagent_core::types::ProviderType::OpenAIResponses => "openai_responses",
        axagent_core::types::ProviderType::Anthropic => "anthropic",
        axagent_core::types::ProviderType::Gemini => "gemini",
        axagent_core::types::ProviderType::OpenClaw => "openclaw",
        axagent_core::types::ProviderType::Hermes => "hermes",
        axagent_core::types::ProviderType::Ollama => "ollama",
    };

    let adapter = registry
        .get(registry_key)
        .ok_or_else(|| format!("Provider adapter not found for {}", registry_key))?;

    let existing_input = request
        .existing_input_schema
        .as_ref()
        .map(|j| serde_json::to_string_pretty(j).unwrap_or_default())
        .unwrap_or_else(|| "null".to_string());
    let existing_output = request
        .existing_output_schema
        .as_ref()
        .map(|j| serde_json::to_string_pretty(j).unwrap_or_default())
        .unwrap_or_else(|| "null".to_string());
    let generated_input = request
        .generated_input_schema
        .as_ref()
        .map(|j| serde_json::to_string_pretty(j).unwrap_or_default())
        .unwrap_or_else(|| "null".to_string());
    let generated_output = request
        .generated_output_schema
        .as_ref()
        .map(|j| serde_json::to_string_pretty(j).unwrap_or_default())
        .unwrap_or_else(|| "null".to_string());

    let prompt = format!(
        r#"You are an AI tool upgrade advisor. Your task is to analyze an existing tool and a newly generated tool specification, then produce an improved version.

## Existing Tool (MUST BE PRESERVED - used by existing workflows)
- Name: {}
- Description: {}
- Tool Type: {} (determines how tool is executed)
- Input Schema:
{}
- Output Schema:
{}

## Generated Tool (from LLM decomposition)
- Name: {}
- Description: {}
- Input Schema:
{}
- Output Schema:
{}

## CRITICAL COMPATIBILITY REQUIREMENTS
1. The tool type and execution method MUST remain UNCHANGED
2. The input_schema and output_schema MUST be BACKWARD COMPATIBLE
3. Only description can be improved; schemas must maintain the same contract
4. If schema changes are needed, they must be additive (optional fields) or backward-compatible

## Instructions
Analyze both tools and create an upgraded version that:
1. MAINTAINS FULL BACKWARD COMPATIBILITY with existing workflows
2. Keeps the same tool type and execution method
3. Has a clearer, more comprehensive description
4. Has an improved description while preserving the exact same input/output interface
5. The reasoning must explicitly confirm compatibility preservation

Output your result in JSON format:
{{
  "name": "upgraded tool name",
  "description": "comprehensive description",
  "input_schema": {{...}},
  "output_schema": {{...}},
  "reasoning": "explanation confirming backward compatibility is preserved"
}}

Only output the JSON, no other text."#,
        request.existing_tool_name,
        request.existing_tool_description,
        request.existing_tool_type,
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

    use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
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

    let suggestion: ToolUpgradeSuggestion = serde_json::from_str(json_str).map_err(|e| {
        format!(
            "Failed to parse LLM response as JSON: {}. Content: {}",
            e, content
        )
    })?;

    Ok(ToolUpgradeResponse { suggestion })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MarketplaceSkillContent {
    pub content: String,
    pub file_name: String,
    pub found: bool,
    pub error: Option<String>,
}

const SKILL_FILENAMES: [&str; 4] = ["SKILL.md", "skill.md", "README.md", "readme.md"];

#[tauri::command]
pub async fn get_marketplace_skill_content(
    repo: String,
) -> Result<MarketplaceSkillContent, String> {
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid repo format: '{}'. Expected 'owner/repo'",
            repo
        ));
    }
    let owner = parts[0];
    let repo_name = parts[1];

    let client = reqwest::Client::new();

    let default_branch = {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo_name);
        let response = client
            .get(&url)
            .header("User-Agent", "AxAgent")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch repo info: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "GitHub API returned {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse repo info: {}", e))?;
        body["default_branch"]
            .as_str()
            .unwrap_or("main")
            .to_string()
    };

    let contents_url = format!(
        "https://api.github.com/repos/{}/{}/contents?ref={}",
        owner, repo_name, default_branch
    );
    let response = client
        .get(&contents_url)
        .header("User-Agent", "AxAgent")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch contents: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "GitHub Contents API returned {}",
            response.status()
        ));
    }

    let contents: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse contents: {}", e))?;

    let md_files: Vec<&serde_json::Value> = contents
        .iter()
        .filter(|item| {
            item["type"].as_str() == Some("file")
                && item["name"]
                    .as_str()
                    .map(|n| n.to_uppercase().ends_with(".MD"))
                    .unwrap_or(false)
        })
        .collect();

    for filename in &SKILL_FILENAMES {
        if let Some(file) = md_files.iter().find(|f| {
            f["name"]
                .as_str()
                .map(|n| n.eq_ignore_ascii_case(filename))
                .unwrap_or(false)
        }) {
            let download_url = file["download_url"].as_str().unwrap_or("");
            if download_url.is_empty() {
                continue;
            }
            let content_response = client
                .get(download_url)
                .header("User-Agent", "AxAgent")
                .send()
                .await
                .map_err(|e| format!("Failed to download file: {}", e))?;

            if content_response.status().is_success() {
                let content = content_response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read file content: {}", e))?;
                return Ok(MarketplaceSkillContent {
                    content,
                    file_name: file["name"].as_str().unwrap_or(filename).to_string(),
                    found: true,
                    error: None,
                });
            }
        }
    }

    Ok(MarketplaceSkillContent {
        content: String::new(),
        file_name: String::new(),
        found: false,
        error: Some(format!(
            "No skill definition file found. Searched for: {}",
            SKILL_FILENAMES.join(", ")
        )),
    })
}
