use axagent_core::prompts::{PromptLang, PromptRegistry};
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest, Message, MessageRole};
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use serde::{Deserialize, Serialize};

/// 获取当前语言的知识提取系统提示
/// TODO: 从 AppState 或参数获取用户语言偏好
fn extraction_system_prompt(lang: PromptLang) -> &'static str {
    PromptRegistry::get("extraction.system_prompt", lang)
}

/// 获取当前语言的记忆合并系统提示
fn consolidation_system_prompt(lang: PromptLang) -> &'static str {
    PromptRegistry::get("consolidation.system_prompt", lang)
}

/// 获取当前语言的实体提取系统提示
fn entity_extraction_system_prompt(lang: PromptLang) -> &'static str {
    PromptRegistry::get("entity_extraction.system_prompt", lang)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub title: String,
    pub content: String,
    pub category: ExtractedCategory,
    pub importance: f64,
    pub nature: ExtractedNature,
    pub tags: Vec<String>,
}

impl Default for ExtractedMemory {
    fn default() -> Self {
        Self {
            title: String::new(),
            content: String::new(),
            category: ExtractedCategory::Fact,
            importance: 0.5,
            nature: ExtractedNature::Semantic,
            tags: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractedCategory {
    Fact,
    Preference,
    Procedure,
    Context,
}

impl ExtractedCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtractedCategory::Fact => "fact",
            ExtractedCategory::Preference => "preference",
            ExtractedCategory::Procedure => "procedure",
            ExtractedCategory::Context => "context",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractedNature {
    Episodic,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub items: Vec<ExtractedMemory>,
    pub conversation_id: String,
}

pub async fn extract_memories_from_messages(
    messages: &[Message],
    conversation_id: &str,
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: &str,
) -> Result<ExtractionResult, String> {
    let recent: Vec<_> = messages
        .iter()
        .rev()
        .filter(|m| !matches!(m.role, MessageRole::System))
        .take(20)
        .collect();

    if recent.len() < 3 {
        return Ok(ExtractionResult {
            items: vec![],
            conversation_id: conversation_id.to_string(),
        });
    }

    let mut transcript = String::new();
    for msg in recent.iter().rev() {
        let role_str = match msg.role {
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
            MessageRole::Tool => "Tool",
            MessageRole::System => "System",
        };
        transcript.push_str(&format!("{}: {}\n\n", role_str, msg.content));
    }

    if transcript.len() > 12000 {
        transcript = transcript[..12000].to_string();
    }

    let prompt = format!("Extract reusable knowledge from this conversation:\n\n{}", transcript);

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(extraction_system_prompt(PromptLang::ZhCN).to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        stream: false,
        temperature: Some(0.3),
        max_tokens: Some(2048),
        top_p: None,
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
        .chat(ctx, request)
        .await
        .map_err(|e| format!("LLM call failed: {}", e))?;

    let items: Vec<ExtractedMemory> = match serde_json::from_str(&response.content) {
        Ok(parsed) => parsed,
        Err(_) => {
            let content = response.content.trim();
            let json_str = if content.starts_with("```") {
                content
                    .lines()
                    .skip(1)
                    .take_while(|line| !line.starts_with("```"))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                content.to_string()
            };
            serde_json::from_str(&json_str).unwrap_or_default()
        },
    };

    let items: Vec<ExtractedMemory> = items
        .into_iter()
        .map(|mut item| {
            item.importance = item.importance.clamp(0.1, 1.0);
            if item.tags.is_empty() {
                item.tags = vec![item.category.as_str().to_string()];
            }
            item
        })
        .collect();

    Ok(ExtractionResult {
        items,
        conversation_id: conversation_id.to_string(),
    })
}

// ── Memory Consolidation ──────────────────────────────────────────────────────

pub async fn consolidate_memories(
    contents: &[String],
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: &str,
) -> Result<ConsolidatedMemory, String> {
    if contents.len() < 2 {
        return Err("Need at least 2 memories to consolidate".to_string());
    }

    let combined = contents
        .iter()
        .enumerate()
        .map(|(i, c)| format!("Memory {}: {}", i + 1, c))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Consolidate these {} similar memories into one:\n\n{}",
        contents.len(),
        combined
    );

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(
                    consolidation_system_prompt(PromptLang::ZhCN).to_string(),
                ),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        stream: false,
        temperature: Some(0.3),
        max_tokens: Some(512),
        top_p: None,
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
        .chat(ctx, request)
        .await
        .map_err(|e| format!("Consolidation LLM call failed: {}", e))?;

    let result: ConsolidatedMemory = match serde_json::from_str(&response.content) {
        Ok(parsed) => parsed,
        Err(_) => {
            let content = response.content.trim();
            let json_str = if content.starts_with("```") {
                content
                    .lines()
                    .skip(1)
                    .take_while(|line| !line.starts_with("```"))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                content.to_string()
            };
            serde_json::from_str(&json_str).unwrap_or(ConsolidatedMemory {
                content: contents.join("; "),
                importance: 0.5,
                tags: vec!["consolidated".to_string()],
            })
        },
    };

    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedMemory {
    pub content: String,
    pub importance: f64,
    pub tags: Vec<String>,
}

// ── Entity Extraction ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    pub aliases: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelation {
    pub source_name: String,
    pub target_name: String,
    pub relation_type: String,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityExtractionResult {
    pub entities: Vec<ExtractedEntity>,
    pub relations: Vec<ExtractedRelation>,
}

pub async fn extract_entities_from_messages(
    messages: &[Message],
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: &str,
) -> Result<EntityExtractionResult, String> {
    let recent: Vec<_> = messages
        .iter()
        .rev()
        .filter(|m| !matches!(m.role, MessageRole::System))
        .take(15)
        .collect();

    if recent.len() < 2 {
        return Ok(EntityExtractionResult {
            entities: vec![],
            relations: vec![],
        });
    }

    let mut transcript = String::new();
    for msg in recent.iter().rev() {
        let role_str = match msg.role {
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
            MessageRole::Tool => "Tool",
            MessageRole::System => "System",
        };
        transcript.push_str(&format!("{}: {}\n\n", role_str, msg.content));
    }

    if transcript.len() > 8000 {
        transcript = transcript[..8000].to_string();
    }

    let prompt =
        format!("Extract entities and relationships from this conversation:\n\n{}", transcript);

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(
                    entity_extraction_system_prompt(PromptLang::ZhCN).to_string(),
                ),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        stream: false,
        temperature: Some(0.2),
        max_tokens: Some(2048),
        top_p: None,
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
        .chat(ctx, request)
        .await
        .map_err(|e| format!("Entity extraction LLM call failed: {}", e))?;

    let result: EntityExtractionResult = match serde_json::from_str(&response.content) {
        Ok(parsed) => parsed,
        Err(_) => {
            let content = response.content.trim();
            let json_str = if content.starts_with("```") {
                content
                    .lines()
                    .skip(1)
                    .take_while(|line| !line.starts_with("```"))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                content.to_string()
            };
            serde_json::from_str(&json_str).unwrap_or(EntityExtractionResult {
                entities: vec![],
                relations: vec![],
            })
        },
    };

    Ok(result)
}

pub async fn extract_incremental_memories(
    new_messages: &[Message],
    conversation_id: &str,
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: &str,
) -> Result<ExtractionResult, String> {
    let user_messages: Vec<_> = new_messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::User | MessageRole::Assistant))
        .collect();

    if user_messages.len() < 2 {
        return Ok(ExtractionResult {
            items: vec![],
            conversation_id: conversation_id.to_string(),
        });
    }

    let mut transcript = String::new();
    for msg in &user_messages {
        let role_str = match msg.role {
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
            _ => "Other",
        };
        transcript.push_str(&format!("{}: {}\n\n", role_str, msg.content));
    }

    if transcript.len() > 4000 {
        transcript = transcript[..4000].to_string();
    }

    let prompt = format!(
        "Extract NEW knowledge from this recent conversation exchange. Focus on information NOT already known from previous extractions:\n\n{}",
        transcript
    );

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(extraction_system_prompt(PromptLang::ZhCN).to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        stream: false,
        temperature: Some(0.2),
        max_tokens: Some(1024),
        top_p: None,
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
        .chat(ctx, request)
        .await
        .map_err(|e| format!("Incremental extraction LLM call failed: {}", e))?;

    let items: Vec<ExtractedMemory> = match serde_json::from_str(&response.content) {
        Ok(parsed) => parsed,
        Err(_) => {
            let content = response.content.trim();
            let json_str = if content.starts_with("```") {
                content
                    .lines()
                    .skip(1)
                    .take_while(|line| !line.starts_with("```"))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                content.to_string()
            };
            serde_json::from_str(&json_str).unwrap_or_default()
        },
    };

    let items: Vec<ExtractedMemory> = items
        .into_iter()
        .map(|mut item| {
            item.importance = item.importance.clamp(0.1, 1.0);
            if item.tags.is_empty() {
                item.tags = vec![item.category.as_str().to_string()];
            }
            item
        })
        .collect();

    Ok(ExtractionResult {
        items,
        conversation_id: conversation_id.to_string(),
    })
}
