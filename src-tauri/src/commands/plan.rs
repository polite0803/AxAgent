//! Plan commands — implements the "plan first, then execute" agent work strategy.
//!
//! ## Flow
//! 1. Frontend sends user message → `plan_generate` generates a structured plan via LLM
//! 2. Plan is emitted as `plan-generated` event → frontend renders PlanCard
//! 3. User approves → `plan_execute` runs each step using the agent infrastructure
//! 4. Step updates are emitted as `plan-step-update` events
//! 5. Final result emitted as `plan-execution-complete` event

use crate::app_state::AppState;
use axagent_core::types::{
    ChatContent, ChatMessage, ChatRequest, ChatTool, ChatToolFunction, ProviderProxyConfig,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

// ── Request / Response types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PlanGenerateRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanExecuteRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "planId")]
    pub plan_id: String,
    #[serde(rename = "stepIds")]
    pub step_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct PlanCancelRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "planId")]
    pub plan_id: String,
    #[allow(dead_code)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlanGetRequest {
    #[serde(rename = "planId")]
    pub plan_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanListRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "includeCompleted")]
    pub include_completed: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PlanModifyStepRequest {
    #[serde(rename = "planId")]
    pub plan_id: String,
    #[serde(rename = "stepId")]
    pub step_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub approved: Option<bool>,
}

// ── Plan data types (mirrors frontend Plan/PlanStep) ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: PlanStepStatus,
    #[serde(rename = "estimatedTools", skip_serializing_if = "Option::is_none")]
    pub estimated_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PlanStepStatus {
    Pending,
    Approved,
    Rejected,
    Running,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "userMessageId")]
    pub user_message_id: String,
    pub title: String,
    pub steps: Vec<PlanStep>,
    pub status: PlanStatus,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(
        rename = "createdUnderStrategy",
        skip_serializing_if = "Option::is_none"
    )]
    pub created_under_strategy: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PlanStatus {
    Draft,
    Reviewing,
    Approved,
    Executing,
    Completed,
    Cancelled,
}

// ── Event payloads ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PlanGeneratedEvent {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub plan: Plan,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanStepUpdateEvent {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "planId")]
    pub plan_id: String,
    #[serde(rename = "stepId")]
    pub step_id: String,
    pub status: PlanStepStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanExecutionCompleteEvent {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "planId")]
    pub plan_id: String,
    pub status: String,
}

// ── LLM-based Plan Generation ─────────────────────────────────────────

/// The system prompt used to instruct the LLM to generate a structured execution plan.
const PLAN_SYSTEM_PROMPT: &str = r#"You are an expert task planner. Your job is to break down a user's request into a structured execution plan with clear, actionable steps.

## Output Format
You MUST respond with ONLY a valid JSON object matching this schema, no other text:

```json
{
  "title": "A concise title for the plan (max 80 chars)",
  "steps": [
    {
      "title": "Step title (brief, imperative)",
      "description": "Detailed description of what this step involves and why",
      "estimatedTools": ["ToolName1", "ToolName2"]
    }
  ]
}
```

## Guidelines
- Each step should be a discrete, independently verifiable unit of work
- 3-6 steps is ideal; never more than 10
- estimatedTools should list the tools likely needed (e.g. "Read", "Write", "Bash", "WebSearch", "Grep", "Edit")
- Steps should be ordered logically (analysis → design → implementation → verification)
- Be specific, not generic — the plan should be immediately actionable
- Prioritize safety: read-only steps first, destructive steps last
"#;

/// Generate a structured plan from user input by calling the LLM.
async fn generate_plan_via_llm(
    state: &AppState,
    conversation_id: &str,
    provider_id: &str,
    model_id: &str,
    content: &str,
    user_message_id: &str,
) -> Result<Plan, String> {
    use axagent_core::repo::provider::{self, get_active_key};
    use axagent_providers::registry::ProviderRegistry;
    use axagent_providers::resolve_base_url_for_type;

    let db = &state.sea_db;

    // Load provider config
    let provider_config = provider::get_provider(db, provider_id)
        .await
        .map_err(|e| format!("Failed to load provider: {}", e))?;

    // Resolve provider adapter
    let registry_key = format!("{:?}", provider_config.provider_type).to_lowercase();
    let registry = ProviderRegistry::create_default();
    let adapter = registry
        .get(&registry_key)
        .ok_or_else(|| format!("Provider adapter not found for: {}", registry_key))?;

    // Get active key and decrypt
    let key_row = get_active_key(db, provider_id)
        .await
        .map_err(|e| format!("No active provider key: {}", e))?;

    let api_key = axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &state.master_key)
        .map_err(|e| format!("Failed to decrypt provider key: {}", e))?;

    // Parse proxy config from the provider
    let proxy_config = provider_config.proxy_config.clone();

    let ctx = axagent_providers::ProviderRequestContext {
        api_key,
        key_id: key_row.id.clone(),
        provider_id: provider_id.to_string(),
        base_url: Some(resolve_base_url_for_type(
            &provider_config.api_host,
            &provider_config.provider_type,
        )),
        api_path: provider_config.api_path.clone(),
        proxy_config,
        custom_headers: provider_config
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    // Build LLM messages
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text(PLAN_SYSTEM_PROMPT.to_string()),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(format!(
                "Create an execution plan for this task:\n\n{}",
                content
            )),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    let request = ChatRequest {
        model: model_id.to_string(),
        messages,
        stream: false,
        temperature: Some(0.3),
        top_p: None,
        max_tokens: Some(2048),
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

    // Call LLM
    let response = adapter
        .chat(&ctx, request)
        .await
        .map_err(|e| format!("LLM call failed: {}", e))?;

    // Parse JSON from response
    let plan_json = extract_json_from_text(&response.content).map_err(|e| {
        let preview = &response.content[..200.min(response.content.len())];
        format!(
            "Failed to parse plan JSON: {}. Raw response: {}",
            e, preview
        )
    })?;

    // Validate and build Plan
    let title = plan_json["title"]
        .as_str()
        .unwrap_or("Execution Plan")
        .to_string();
    let steps_raw = plan_json["steps"]
        .as_array()
        .ok_or_else(|| "Plan response missing 'steps' array".to_string())?;

    if steps_raw.is_empty() {
        return Err("Plan must have at least one step".to_string());
    }

    let now = chrono::Utc::now().timestamp_millis();
    let steps: Vec<PlanStep> = steps_raw
        .iter()
        .map(|s| {
            let tools = s["estimatedTools"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: s["title"].as_str().unwrap_or("Unnamed Step").to_string(),
                description: s["description"].as_str().unwrap_or("").to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: tools,
                result: None,
            }
        })
        .collect();

    Ok(Plan {
        id: Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_string(),
        user_message_id: user_message_id.to_string(),
        title,
        steps,
        status: PlanStatus::Reviewing,
        is_active: true,
        created_under_strategy: Some("plan".to_string()),
        created_at: now,
        updated_at: now,
    })
}

/// Extract JSON from LLM response text, stripping markdown code fences if present.
fn extract_json_from_text(text: &str) -> Result<Value, String> {
    let text = text.trim();

    // Try to extract JSON from markdown code fences
    let json_str = if let Some(start) = text.find("```json") {
        let inner = &text[start + 7..];
        let end = inner.find("```").unwrap_or(inner.len());
        &inner[..end]
    } else if let Some(start) = text.find("```") {
        let inner = &text[start + 3..];
        let end = inner.find("```").unwrap_or(inner.len());
        &inner[..end]
    } else if let Some(start) = text.find('{') {
        // Find matching closing brace
        let rest = &text[start..];
        let end = find_matching_brace(rest)?;
        &text[start..start + end + 1]
    } else {
        text
    };

    serde_json::from_str(json_str.trim()).map_err(|e| format!("Invalid JSON: {}", e))
}

fn find_matching_brace(s: &str) -> Result<usize, String> {
    let mut depth = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            },
            _ => {},
        }
    }
    Err("No matching closing brace found".to_string())
}

// ── Agent Step Execution ────────────────────────────────────────────────

/// Holds the reusable parts of agent context (adapter + credentials).
/// The `api_client` and `tool_registry` are rebuilt per-step.
struct AgentContext {
    adapter: Arc<dyn axagent_providers::ProviderAdapter>,
    ctx: axagent_providers::ProviderRequestContext,
    provider_id: String,
    model_id: String,
    enabled_mcp_server_ids: Vec<String>,
}

/// Build the reusable `AgentContext` for a conversation.
async fn build_agent_context(
    state: &AppState,
    conversation_id: &str,
    provider_id: &str,
    model_id: &str,
) -> Result<AgentContext, String> {
    use axagent_providers::{resolve_base_url_for_type, ProviderAdapter};

    let db = &state.sea_db;

    let prov = axagent_core::repo::provider::get_provider(db, provider_id)
        .await
        .map_err(|e| format!("Failed to load provider: {}", e))?;

    let key = prov
        .keys
        .iter()
        .find(|k| k.enabled)
        .ok_or_else(|| "No active API key for provider".to_string())?;

    let api_key = axagent_core::crypto::decrypt_key(&key.key_encrypted, &state.master_key)
        .map_err(|e| e.to_string())?;

    let settings = axagent_core::repo::settings::get_settings(db)
        .await
        .unwrap_or_default();

    let ctx = axagent_providers::ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: prov.id.clone(),
        base_url: Some(resolve_base_url_for_type(
            &prov.api_host,
            &prov.provider_type,
        )),
        api_path: prov.api_path.clone(),
        proxy_config: ProviderProxyConfig::resolve(&prov.proxy_config, &settings),
        custom_headers: prov
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let adapter: Arc<dyn ProviderAdapter> = match prov.provider_type {
        axagent_core::types::ProviderType::OpenAI => {
            Arc::new(axagent_providers::openai::OpenAIAdapter::new())
        },
        axagent_core::types::ProviderType::OpenAIResponses => {
            Arc::new(axagent_providers::openai_responses::OpenAIResponsesAdapter::new())
        },
        axagent_core::types::ProviderType::Anthropic => {
            Arc::new(axagent_providers::anthropic::AnthropicAdapter::new())
        },
        axagent_core::types::ProviderType::Gemini => {
            Arc::new(axagent_providers::gemini::GeminiAdapter::new())
        },
        axagent_core::types::ProviderType::OpenClaw => {
            Arc::new(axagent_providers::openclaw::OpenClawAdapter::new())
        },
        axagent_core::types::ProviderType::Hermes => {
            Arc::new(axagent_providers::hermes::HermesAdapter::new())
        },
        axagent_core::types::ProviderType::Ollama => {
            Arc::new(axagent_providers::ollama::OllamaAdapter::new())
        },
    };

    let conversation = axagent_core::repo::conversation::get_conversation(db, conversation_id)
        .await
        .map_err(|e| format!("Failed to load conversation: {}", e))?;

    Ok(AgentContext {
        adapter,
        ctx,
        provider_id: provider_id.to_string(),
        model_id: model_id.to_string(),
        enabled_mcp_server_ids: conversation.enabled_mcp_server_ids,
    })
}

/// Build tool_registry and api_client for a step execution.
async fn build_step_tools(
    agent_ctx: &AgentContext,
    db: &sea_orm::DatabaseConnection,
) -> (axagent_agent::AxAgentApiClient, axagent_agent::ToolRegistry) {
    let mut tool_registry = axagent_agent::ToolRegistry::new();
    let mut chat_tools: Vec<ChatTool> = Vec::new();

    for server_id in &agent_ctx.enabled_mcp_server_ids {
        let server = match axagent_core::repo::mcp_server::get_mcp_server(db, server_id).await {
            Ok(s) => s,
            Err(_) => continue,
        };

        if let Ok(descriptors) =
            axagent_core::repo::mcp_server::list_tools_for_server(db, server_id).await
        {
            for td in descriptors {
                let parameters: Option<Value> = td
                    .input_schema_json
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok());

                chat_tools.push(ChatTool {
                    r#type: "function".to_string(),
                    function: ChatToolFunction {
                        name: td.name.clone(),
                        description: td.description.clone(),
                        parameters: parameters.clone(),
                    },
                });

                tool_registry = tool_registry.register_mcp_tool(
                    server.id.clone(),
                    server.name.clone(),
                    td.name,
                    td.description,
                    parameters,
                    axagent_agent::McpServerConfig {
                        server_id: server.id.clone(),
                        server_name: server.name.clone(),
                        transport: server.transport.clone(),
                        command: server.command.clone(),
                        args_json: server.args_json.clone(),
                        env_json: server.env_json.clone(),
                        endpoint: server.endpoint.clone(),
                        execute_timeout_secs: server.execute_timeout_secs,
                        connection_pool_size: None,
                        retry_attempts: None,
                        retry_delay_ms: None,
                    },
                );
            }
        }
    }

    let mut local_tools = axagent_agent::LocalToolRegistry::init_from_registry();
    local_tools.load_enabled_state(db).await;
    chat_tools.extend(local_tools.get_enabled_chat_tools());
    tool_registry = tool_registry.with_local_tools(local_tools);

    tool_registry = tool_registry.with_recorder(axagent_agent::ToolExecutionRecorder::new(
        Arc::new(db.clone()),
    ));

    let api_client = if chat_tools.is_empty() {
        axagent_agent::AxAgentApiClient::new(agent_ctx.adapter.clone(), agent_ctx.ctx.clone())
            .with_model(&agent_ctx.model_id)
    } else {
        axagent_agent::AxAgentApiClient::with_tools(
            agent_ctx.adapter.clone(),
            agent_ctx.ctx.clone(),
            chat_tools,
        )
        .with_model(&agent_ctx.model_id)
    };

    (api_client, tool_registry)
}

/// Execute a single plan step using the agent's tool-calling loop.
async fn execute_step_with_agent(
    state: &AppState,
    app: &AppHandle,
    conversation_id: &str,
    plan_id: &str,
    step: &PlanStep,
    agent_ctx: &AgentContext,
) -> Result<String, String> {
    let session_manager = &state.agent_session_manager;

    let session = session_manager
        .get_or_create_session(agent_ctx.provider_id.clone(), conversation_id.to_string())
        .await
        .map_err(|e| format!("Failed to get agent session: {}", e))?;

    let session_id = session.session().session_id.clone();

    let system_prompt = vec![format!(
        "You are executing step {} of a plan. Focus exclusively on this step.\n\n\
         Step: {}\n\n\
         Description: {}\n\n\
         Complete this step now. Use tools as needed. \
         When done, summarize the result in plain text.",
        step.title, step.title, step.description,
    )];

    // Emit running status
    let _ = app.emit(
        "plan-step-update",
        PlanStepUpdateEvent {
            conversation_id: conversation_id.to_string(),
            plan_id: plan_id.to_string(),
            step_id: step.id.clone(),
            status: PlanStepStatus::Running,
            result: None,
        },
    );

    // Build fresh api_client + tool_registry for this step
    let (api_client, tool_registry) = build_step_tools(agent_ctx, &state.sea_db).await;

    let result = session_manager
        .run_turn_with_tools(
            &session_id,
            format!(
                "Execute this plan step: {}\n\nContext: {}",
                step.title, step.description
            ),
            api_client,
            tool_registry,
            system_prompt,
            conversation_id.to_string(),
            axagent_runtime::PermissionMode::Prompt,
            state.agent_prompters.clone(),
            None,
        )
        .await;

    match result {
        Ok((summary, _session)) => {
            let last_msg = summary.assistant_messages.last();
            let result_text = match last_msg {
                Some(msg) => {
                    let text_blocks = msg
                        .blocks
                        .iter()
                        .filter_map(|b| {
                            if let axagent_runtime::ContentBlock::Text { text } = b {
                                Some(text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if text_blocks.is_empty() {
                        format!(
                            "Step '{}' completed ({} iterations)",
                            step.title, summary.iterations
                        )
                    } else {
                        text_blocks
                    }
                },
                None => format!(
                    "Step '{}' completed ({} iterations)",
                    step.title, summary.iterations
                ),
            };

            let _ = app.emit(
                "plan-step-update",
                PlanStepUpdateEvent {
                    conversation_id: conversation_id.to_string(),
                    plan_id: plan_id.to_string(),
                    step_id: step.id.clone(),
                    status: PlanStepStatus::Completed,
                    result: Some(result_text.clone()),
                },
            );

            Ok(result_text)
        },
        Err(e) => {
            let err_text = format!("Step failed: {}", e);

            let _ = app.emit(
                "plan-step-update",
                PlanStepUpdateEvent {
                    conversation_id: conversation_id.to_string(),
                    plan_id: plan_id.to_string(),
                    step_id: step.id.clone(),
                    status: PlanStepStatus::Error,
                    result: Some(err_text.clone()),
                },
            );

            Err(err_text)
        },
    }
}

// ── Tauri Commands ────────────────────────────────────────────────────

/// Generate a structured execution plan from the user's message using LLM.
/// The plan is persisted to the database and emitted as a `plan-generated` event.
#[tauri::command]
pub async fn plan_generate(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    request: PlanGenerateRequest,
) -> Result<Plan, String> {
    let db = &state.sea_db;

    // Load conversation to get provider/model info
    let conversation =
        axagent_core::repo::conversation::get_conversation(db, &request.conversation_id)
            .await
            .map_err(|e| format!("Conversation not found: {}", e))?;

    // Find the latest user message for this conversation
    let messages = axagent_core::repo::message::list_messages(db, &request.conversation_id)
        .await
        .map_err(|e| format!("Failed to get messages: {}", e))?;

    let user_message_id = messages
        .first()
        .map(|m| m.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Deactivate any existing active plans for this conversation
    let existing_active = axagent_core::entity::plans::Entity::find()
        .filter(axagent_core::entity::plans::Column::ConversationId.eq(&request.conversation_id))
        .filter(axagent_core::entity::plans::Column::IsActive.eq(1))
        .all(db)
        .await
        .unwrap_or_default();

    for plan in existing_active {
        let mut am: axagent_core::entity::plans::ActiveModel = plan.into();
        am.is_active = Set(0);
        am.update(db).await.ok();
    }

    // Generate plan via LLM
    let plan = generate_plan_via_llm(
        &state,
        &request.conversation_id,
        &conversation.provider_id,
        &conversation.model_id,
        &request.content,
        &user_message_id,
    )
    .await?;

    // Persist plan to database
    let steps_json = serde_json::to_string(&plan.steps)
        .map_err(|e| format!("Failed to serialize steps: {}", e))?;

    let plan_entity = axagent_core::entity::plans::ActiveModel {
        id: Set(plan.id.clone()),
        conversation_id: Set(plan.conversation_id.clone()),
        user_message_id: Set(plan.user_message_id.clone()),
        title: Set(plan.title.clone()),
        steps_json: Set(steps_json),
        status: Set("reviewing".to_string()),
        is_active: Set(1),
        created_under_strategy: Set(Some("plan".to_string())),
        reason: Set(None),
        created_at: Set(plan.created_at),
        updated_at: Set(plan.updated_at),
    };

    axagent_core::entity::plans::Entity::insert(plan_entity)
        .exec(db)
        .await
        .map_err(|e| format!("Failed to save plan: {}", e))?;

    // Emit plan-generated event
    let _ = app.emit(
        "plan-generated",
        PlanGeneratedEvent {
            conversation_id: request.conversation_id.clone(),
            plan: plan.clone(),
        },
    );

    Ok(plan)
}

/// Execute an approved plan — runs each step sequentially using the agent tool-calling loop.
#[tauri::command]
pub async fn plan_execute(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    request: PlanExecuteRequest,
) -> Result<(), String> {
    let db = &state.sea_db;

    // Load plan from database
    let plan_row = axagent_core::entity::plans::Entity::find_by_id(&request.plan_id)
        .one(db)
        .await
        .map_err(|e| format!("Failed to load plan: {}", e))?
        .ok_or_else(|| format!("Plan not found: {}", request.plan_id))?;

    // Update plan status to executing
    let mut am: axagent_core::entity::plans::ActiveModel = plan_row.clone().into();
    am.status = Set("executing".to_string());
    am.updated_at = Set(chrono::Utc::now().timestamp_millis());
    am.update(db)
        .await
        .map_err(|e| format!("Failed to update plan: {}", e))?;

    // Parse steps
    let steps: Vec<PlanStep> = serde_json::from_str(&plan_row.steps_json)
        .map_err(|e| format!("Failed to parse plan steps: {}", e))?;

    // Filter steps to execute
    let steps_to_run: Vec<&PlanStep> = if let Some(ref step_ids) = request.step_ids {
        steps.iter().filter(|s| step_ids.contains(&s.id)).collect()
    } else {
        steps
            .iter()
            .filter(|s| s.status == PlanStepStatus::Approved || s.status == PlanStepStatus::Pending)
            .collect()
    };

    if steps_to_run.is_empty() {
        // No steps to execute — mark as completed
        let mut am: axagent_core::entity::plans::ActiveModel = plan_row.into();
        am.status = Set("completed".to_string());
        am.updated_at = Set(chrono::Utc::now().timestamp_millis());
        am.update(db).await.ok();

        let _ = app.emit(
            "plan-execution-complete",
            PlanExecutionCompleteEvent {
                conversation_id: request.conversation_id.clone(),
                plan_id: request.plan_id.clone(),
                status: "completed".to_string(),
            },
        );
        return Ok(());
    }

    // Load conversation for provider/model info
    let conversation =
        axagent_core::repo::conversation::get_conversation(db, &request.conversation_id)
            .await
            .map_err(|e| format!("Failed to load conversation: {}", e))?;

    // Build agent context once (adapter + credentials) — reusable across steps
    let agent_ctx = build_agent_context(
        &state,
        &request.conversation_id,
        &conversation.provider_id,
        &conversation.model_id,
    )
    .await?;

    let mut step_results: Vec<(String, String)> = Vec::new(); // (step_id, result)

    // Execute each step sequentially
    for step in &steps_to_run {
        let result = execute_step_with_agent(
            &state,
            &app,
            &request.conversation_id,
            &request.plan_id,
            step,
            &agent_ctx,
        )
        .await;

        match result {
            Ok(text) => step_results.push((step.id.clone(), text)),
            Err(err) => {
                step_results.push((step.id.clone(), err));
                // Continue with remaining steps even if one fails
            },
        }
    }

    // Update plan steps to completed/error status in DB
    let mut updated_steps: Vec<PlanStep> = steps;
    for (step_id, result) in &step_results {
        if let Some(step) = updated_steps.iter_mut().find(|s| &s.id == step_id) {
            step.status = if result.starts_with("Step failed:") {
                PlanStepStatus::Error
            } else {
                PlanStepStatus::Completed
            };
            step.result = Some(result.clone());
        }
    }
    let steps_json = serde_json::to_string(&updated_steps).unwrap_or_default();

    let _has_errors = updated_steps
        .iter()
        .any(|s| s.status == PlanStepStatus::Error);
    let final_status = "completed";

    let mut am2: axagent_core::entity::plans::ActiveModel = plan_row.into();
    am2.steps_json = Set(steps_json);
    am2.status = Set(final_status.to_string());
    am2.updated_at = Set(chrono::Utc::now().timestamp_millis());
    am2.update(db).await.ok();

    // Emit completion
    let _ = app.emit(
        "plan-execution-complete",
        PlanExecutionCompleteEvent {
            conversation_id: request.conversation_id,
            plan_id: request.plan_id,
            status: final_status.to_string(),
        },
    );

    Ok(())
}

/// Cancel a plan.
#[tauri::command]
pub async fn plan_cancel(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    request: PlanCancelRequest,
) -> Result<(), String> {
    let db = &state.sea_db;

    if let Some(row) = axagent_core::entity::plans::Entity::find_by_id(&request.plan_id)
        .one(db)
        .await
        .ok()
        .flatten()
    {
        let mut am: axagent_core::entity::plans::ActiveModel = row.into();
        am.status = Set("cancelled".to_string());
        am.is_active = Set(0);
        am.updated_at = Set(chrono::Utc::now().timestamp_millis());
        am.update(db).await.ok();
    }

    let _ = app.emit(
        "plan-execution-complete",
        PlanExecutionCompleteEvent {
            conversation_id: request.conversation_id,
            plan_id: request.plan_id,
            status: "cancelled".to_string(),
        },
    );

    Ok(())
}

/// Get a plan by ID.
#[tauri::command]
pub async fn plan_get(
    state: tauri::State<'_, AppState>,
    request: PlanGetRequest,
) -> Result<Option<Plan>, String> {
    let db = &state.sea_db;

    let row = axagent_core::entity::plans::Entity::find_by_id(&request.plan_id)
        .one(db)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    match row {
        Some(row) => {
            let steps: Vec<PlanStep> = serde_json::from_str(&row.steps_json).unwrap_or_default();
            let status = match row.status.as_str() {
                "draft" => PlanStatus::Draft,
                "reviewing" => PlanStatus::Reviewing,
                "approved" => PlanStatus::Approved,
                "executing" => PlanStatus::Executing,
                "completed" => PlanStatus::Completed,
                _ => PlanStatus::Cancelled,
            };
            Ok(Some(Plan {
                id: row.id,
                conversation_id: row.conversation_id,
                user_message_id: row.user_message_id,
                title: row.title,
                steps,
                status,
                is_active: row.is_active != 0,
                created_under_strategy: row.created_under_strategy,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
        },
        None => Ok(None),
    }
}

/// List plans for a conversation.
#[tauri::command]
pub async fn plan_list(
    state: tauri::State<'_, AppState>,
    request: PlanListRequest,
) -> Result<Vec<Plan>, String> {
    let db = &state.sea_db;

    let mut query = axagent_core::entity::plans::Entity::find()
        .filter(axagent_core::entity::plans::Column::ConversationId.eq(&request.conversation_id));

    if !request.include_completed.unwrap_or(false) {
        query = query.filter(axagent_core::entity::plans::Column::IsActive.eq(1));
    }

    let rows = query
        .order_by_desc(axagent_core::entity::plans::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    let plans = rows
        .into_iter()
        .map(|row| {
            let steps: Vec<PlanStep> = serde_json::from_str(&row.steps_json).unwrap_or_default();
            let status = match row.status.as_str() {
                "draft" => PlanStatus::Draft,
                "reviewing" => PlanStatus::Reviewing,
                "approved" => PlanStatus::Approved,
                "executing" => PlanStatus::Executing,
                "completed" => PlanStatus::Completed,
                _ => PlanStatus::Cancelled,
            };
            Plan {
                id: row.id,
                conversation_id: row.conversation_id,
                user_message_id: row.user_message_id,
                title: row.title,
                steps,
                status,
                is_active: row.is_active != 0,
                created_under_strategy: row.created_under_strategy,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }
        })
        .collect();

    Ok(plans)
}

/// Modify a step in a plan.
#[tauri::command]
pub async fn plan_modify_step(
    state: tauri::State<'_, AppState>,
    request: PlanModifyStepRequest,
) -> Result<Option<Plan>, String> {
    let db = &state.sea_db;

    let row = axagent_core::entity::plans::Entity::find_by_id(&request.plan_id)
        .one(db)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Plan not found: {}", request.plan_id))?;

    let mut steps: Vec<PlanStep> = serde_json::from_str(&row.steps_json)
        .map_err(|e| format!("Failed to parse steps: {}", e))?;

    if let Some(step) = steps.iter_mut().find(|s| s.id == request.step_id) {
        if let Some(ref title) = request.title {
            step.title = title.clone();
        }
        if let Some(ref description) = request.description {
            step.description = description.clone();
        }
        if let Some(approved) = request.approved {
            step.status = if approved {
                PlanStepStatus::Approved
            } else {
                PlanStepStatus::Rejected
            };
        }
    }

    let steps_json =
        serde_json::to_string(&steps).map_err(|e| format!("Failed to serialize steps: {}", e))?;

    let now = chrono::Utc::now().timestamp_millis();
    let mut am: axagent_core::entity::plans::ActiveModel = row.clone().into();
    am.steps_json = Set(steps_json);
    am.updated_at = Set(now);
    am.update(db)
        .await
        .map_err(|e| format!("Failed to update plan: {}", e))?;

    let status = match row.status.as_str() {
        "draft" => PlanStatus::Draft,
        "reviewing" => PlanStatus::Reviewing,
        "approved" => PlanStatus::Approved,
        "executing" => PlanStatus::Executing,
        "completed" => PlanStatus::Completed,
        _ => PlanStatus::Cancelled,
    };

    Ok(Some(Plan {
        id: row.id,
        conversation_id: row.conversation_id,
        user_message_id: row.user_message_id,
        title: row.title,
        steps,
        status,
        is_active: row.is_active != 0,
        created_under_strategy: row.created_under_strategy,
        created_at: row.created_at,
        updated_at: now,
    }))
}
