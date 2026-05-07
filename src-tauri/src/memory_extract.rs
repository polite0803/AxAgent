use axagent_core::types::{ChatContent, ChatMessage, ChatRequest, Message, MessageRole};
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub title: String,
    pub content: String,
    pub category: ExtractedCategory,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub items: Vec<ExtractedMemory>,
    pub conversation_id: String,
}

const EXTRACTION_SYSTEM_PROMPT: &str = r#"You are a knowledge extraction assistant. Your task is to extract important, reusable knowledge from conversation transcripts.

Extract the following types of knowledge:
1. **Facts**: Important factual information the user shared (e.g., "User's project uses React 18 with TypeScript")
2. **Preferences**: User preferences and patterns (e.g., "User prefers functional components over class components")
3. **Procedures**: Step-by-step processes or solutions discussed (e.g., "To deploy: run build, then push to S3")
4. **Context**: Important context about the user's work environment (e.g., "User works on a Tauri desktop app")

Rules:
- Only extract knowledge that would be useful in FUTURE conversations
- Do NOT extract trivial or obvious information
- Do NOT extract information that is only relevant to the current conversation
- Each item should be self-contained and understandable without context
- Keep titles concise (under 50 characters)
- Keep content detailed but concise (under 200 characters)

Respond with a JSON array of extracted items. Each item should have:
- "title": a short label
- "content": the detailed knowledge
- "category": one of "fact", "preference", "procedure", "context"

If no significant knowledge is found, return an empty array: []"#;

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

    if transcript.len() > 8000 {
        transcript = transcript[..8000].to_string();
    }

    let prompt = format!("Extract reusable knowledge from this conversation:\n\n{}", transcript);

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(EXTRACTION_SYSTEM_PROMPT.to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt),
                tool_calls: None,
                tool_call_id: None,
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

    Ok(ExtractionResult {
        items,
        conversation_id: conversation_id.to_string(),
    })
}
