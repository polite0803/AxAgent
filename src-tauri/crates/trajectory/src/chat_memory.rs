//! Chat memory module
//!
//! Replaces TypeScript `ChatMemory.ts` with Rust implementation.
//! Provides session management, message buffering, and entity extraction.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMemoryConfig {
    pub buffer_flush_interval_ms: u64,
    pub entity_extract_delay_ms: u64,
    pub max_buffer_size: usize,
}

impl Default for ChatMemoryConfig {
    fn default() -> Self {
        Self {
            buffer_flush_interval_ms: 5000,
            entity_extract_delay_ms: 2000,
            max_buffer_size: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCount {
    #[serde(rename = "input")]
    pub input: u32,
    #[serde(rename = "output")]
    pub output: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub title: String,
    pub platform: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    pub model: String,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: String,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
    #[serde(rename = "parentSessionId")]
    pub parent_session_id: Option<String>,
    #[serde(rename = "tokenCount")]
    pub token_count: TokenCount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub role: String,
    pub content: String,
    #[serde(rename = "toolCalls")]
    pub tool_calls: Option<String>,
    #[serde(rename = "toolResults")]
    pub tool_results: Option<String>,
    pub usage: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateSessionParams {
    pub title: Option<String>,
    pub platform: Option<String>,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub model: Option<String>,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: Option<String>,
    #[serde(rename = "parentSessionId")]
    pub parent_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMessageParams {
    pub role: String,
    pub content: String,
    #[serde(rename = "toolCalls")]
    pub tool_calls: Option<String>,
    #[serde(rename = "toolResults")]
    pub tool_results: Option<String>,
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    #[serde(rename = "inputTokens")]
    pub input_tokens: Option<u32>,
    #[serde(rename = "outputTokens")]
    pub output_tokens: Option<u32>,
}

fn generate_session_id() -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let random: String = (0..11)
        .map(|_| {
            let idx = (chrono::Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or(0)
                .unsigned_abs() as usize)
                % 36;
            let chars = b"0123456789abcdefghijklmnopqrstuvwxyz";
            chars[idx] as char
        })
        .collect();
    format!("session_{}_{}", timestamp, random)
}

fn generate_message_id() -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let random: String = (0..11)
        .map(|_| {
            let idx = (chrono::Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or(0)
                .unsigned_abs() as usize)
                % 36;
            let chars = b"0123456789abcdefghijklmnopqrstuvwxyz";
            chars[idx] as char
        })
        .collect();
    format!("msg_{}_{}", timestamp, random)
}

/// In-memory chat session and message tracking.
///
/// Not currently integrated into the conversation flow.
/// Session/message persistence is handled by SeaORM in `axagent_core::repo::conversation`.
/// Retained for potential future use in offline session caching.
#[allow(dead_code)]
pub struct ChatMemory {
    current_session_id: Option<String>,
    message_buffer: Vec<MessageRecord>,
    config: ChatMemoryConfig,
    sessions: HashMap<String, SessionRecord>,
    messages: HashMap<String, Vec<MessageRecord>>,
}

impl Default for ChatMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatMemory {
    pub fn new() -> Self {
        Self {
            current_session_id: None,
            message_buffer: Vec::new(),
            config: ChatMemoryConfig::default(),
            sessions: HashMap::new(),
            messages: HashMap::new(),
        }
    }

    pub fn with_config(config: ChatMemoryConfig) -> Self {
        Self {
            current_session_id: None,
            message_buffer: Vec::new(),
            config,
            sessions: HashMap::new(),
            messages: HashMap::new(),
        }
    }

    pub fn create_session(&mut self, params: CreateSessionParams) -> String {
        let session_id = generate_session_id();
        let now = chrono::Utc::now().timestamp_millis();

        let title = params
            .title
            .unwrap_or_else(|| format!("会话 {}", chrono::Utc::now().format("%Y-%m-%d")));

        let platform = params.platform.unwrap_or_else(|| "web".to_string());

        let user_id = params.user_id.unwrap_or_else(|| "default".to_string());

        let model = params.model.unwrap_or_else(|| "unknown".to_string());

        let system_prompt = params.system_prompt.unwrap_or_default();

        let record = SessionRecord {
            id: session_id.clone(),
            title,
            platform,
            user_id,
            model,
            system_prompt,
            created_at: now,
            updated_at: now,
            parent_session_id: params.parent_session_id,
            token_count: TokenCount {
                input: 0,
                output: 0,
            },
        };

        self.sessions.insert(session_id.clone(), record);
        self.messages.insert(session_id.clone(), Vec::new());
        self.current_session_id = Some(session_id.clone());

        session_id
    }

    pub fn get_or_create_session(&mut self) -> String {
        if let Some(ref session_id) = self.current_session_id {
            if self.sessions.contains_key(session_id) {
                return session_id.clone();
            }
        }

        self.create_session(CreateSessionParams {
            title: Some(format!("会话 {}", chrono::Utc::now().format("%Y-%m-%d"))),
            platform: None,
            user_id: None,
            model: None,
            system_prompt: None,
            parent_session_id: None,
        })
    }

    pub fn get_session(&self, session_id: &str) -> Option<&SessionRecord> {
        self.sessions.get(session_id)
    }

    pub fn add_message(&mut self, params: AddMessageParams) -> String {
        let session_id = self.get_or_create_session();
        let message_id = generate_message_id();

        let usage_json = params
            .usage
            .as_ref()
            .map(|u| serde_json::to_string(u).unwrap_or_default());

        let record = MessageRecord {
            id: message_id.clone(),
            session_id: session_id.clone(),
            role: params.role.clone(),
            content: params.content.clone(),
            tool_calls: params.tool_calls,
            tool_results: params.tool_results,
            usage: usage_json,
            created_at: chrono::Utc::now().timestamp_millis(),
        };

        // Immediately persist to in-memory store (no buffering) to prevent data loss on crash.
        // The message_buffer is kept for backward compatibility but is flushed on every add.
        self.message_buffer.push(record.clone());
        self.flush_message_buffer();

        if params.role == "user" || params.role == "assistant" {
            self.schedule_entity_extraction(&params.content);
        }

        message_id
    }

    pub fn get_message(&self, message_id: &str) -> Option<&MessageRecord> {
        for messages in self.messages.values() {
            for msg in messages {
                if msg.id == message_id {
                    return Some(msg);
                }
            }
        }
        None
    }

    pub fn get_session_messages(&self, session_id: &str) -> Vec<&MessageRecord> {
        self.messages
            .get(session_id)
            .map(|msgs| msgs.iter().collect())
            .unwrap_or_default()
    }

    pub fn flush_message_buffer(&mut self) -> usize {
        if self.message_buffer.is_empty() {
            return 0;
        }

        let messages_to_flush: Vec<MessageRecord> = self.message_buffer.drain(..).collect();
        let count = messages_to_flush.len();

        for message in messages_to_flush {
            if let Some(msgs) = self.messages.get_mut(&message.session_id) {
                msgs.push(message);
            }
        }

        count
    }

    pub fn force_flush(&mut self) -> usize {
        self.flush_message_buffer()
    }

    pub fn get_current_session_id(&self) -> Option<&String> {
        self.current_session_id.as_ref()
    }

    pub fn set_current_session(&mut self, session_id: String) -> bool {
        if self.sessions.contains_key(&session_id) {
            self.current_session_id = Some(session_id);
            true
        } else {
            false
        }
    }

    pub fn update_session_title(&mut self, session_id: &str, title: String) -> bool {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.title = title;
            session.updated_at = chrono::Utc::now().timestamp_millis();
            true
        } else {
            false
        }
    }

    pub fn update_token_count(
        &mut self,
        session_id: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> bool {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.token_count = TokenCount {
                input: input_tokens,
                output: output_tokens,
            };
            session.updated_at = chrono::Utc::now().timestamp_millis();
            true
        } else {
            false
        }
    }

    pub fn get_all_sessions(&self) -> Vec<&SessionRecord> {
        self.sessions.values().collect()
    }

    pub fn delete_session(&mut self, session_id: &str) -> bool {
        if self.sessions.remove(session_id).is_some() {
            self.messages.remove(session_id);
            if self.current_session_id.as_ref() == Some(&session_id.to_string()) {
                self.current_session_id = None;
            }
            true
        } else {
            false
        }
    }

    pub fn get_buffer_size(&self) -> usize {
        self.message_buffer.len()
    }

    fn schedule_entity_extraction(&mut self, _content: &str) {
        // Entity extraction would be handled by the knowledge graph service
        // This is a placeholder for integration with the entity extraction system
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let mut chat_memory = ChatMemory::new();

        let session_id = chat_memory.create_session(CreateSessionParams {
            title: Some("Test Session".to_string()),
            platform: Some("web".to_string()),
            user_id: Some("user123".to_string()),
            model: Some("gpt-4".to_string()),
            system_prompt: None,
            parent_session_id: None,
        });

        assert!(!session_id.is_empty());
        assert!(session_id.starts_with("session_"));

        let session = chat_memory.get_session(&session_id);
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.title, "Test Session");
        assert_eq!(session.platform, "web");
        assert_eq!(session.user_id, "user123");
        assert_eq!(session.model, "gpt-4");
    }

    #[test]
    fn test_add_message() {
        let mut chat_memory = ChatMemory::new();
        let session_id = chat_memory.get_or_create_session();

        let message_id = chat_memory.add_message(AddMessageParams {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
            tool_calls: None,
            tool_results: None,
            usage: None,
        });

        assert!(!message_id.is_empty());
        assert!(message_id.starts_with("msg_"));

        let messages = chat_memory.get_session_messages(&session_id);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello, world!");
        assert_eq!(messages[0].role, "user");
    }

    #[test]
    fn test_buffer_flush() {
        let mut config = ChatMemoryConfig::default();
        config.max_buffer_size = 5;
        let mut chat_memory = ChatMemory::with_config(config);

        let session_id = chat_memory.get_or_create_session();

        for i in 0..3 {
            chat_memory.add_message(AddMessageParams {
                role: "user".to_string(),
                content: format!("Message {}", i),
                tool_calls: None,
                tool_results: None,
                usage: None,
            });
        }

        assert_eq!(chat_memory.get_buffer_size(), 3);

        let flushed = chat_memory.flush_message_buffer();
        assert_eq!(flushed, 3);
        assert_eq!(chat_memory.get_buffer_size(), 0);

        let messages = chat_memory.get_session_messages(&session_id);
        assert_eq!(messages.len(), 3);
    }

    #[test]
    fn test_auto_flush_at_max_size() {
        let mut config = ChatMemoryConfig::default();
        config.max_buffer_size = 3;
        let mut chat_memory = ChatMemory::with_config(config);

        let session_id = chat_memory.get_or_create_session();

        for i in 0..3 {
            chat_memory.add_message(AddMessageParams {
                role: "user".to_string(),
                content: format!("Message {}", i),
                tool_calls: None,
                tool_results: None,
                usage: None,
            });
        }

        let messages = chat_memory.get_session_messages(&session_id);
        assert_eq!(messages.len(), 3);
        assert_eq!(chat_memory.get_buffer_size(), 0);
    }

    #[test]
    fn test_delete_session() {
        let mut chat_memory = ChatMemory::new();
        let session_id = chat_memory.create_session(CreateSessionParams::default());

        assert!(chat_memory.get_session(&session_id).is_some());

        let result = chat_memory.delete_session(&session_id);
        assert!(result);
        assert!(chat_memory.get_session(&session_id).is_none());
    }

    #[test]
    fn test_update_session_title() {
        let mut chat_memory = ChatMemory::new();
        let session_id = chat_memory.create_session(CreateSessionParams::default());

        let result = chat_memory.update_session_title(&session_id, "New Title".to_string());
        assert!(result);

        let session = chat_memory.get_session(&session_id).unwrap();
        assert_eq!(session.title, "New Title");
    }
}
