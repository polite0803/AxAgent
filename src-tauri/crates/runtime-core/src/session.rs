// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::json::JsonValue;
use crate::usage::TokenUsage;

const SESSION_VERSION: u32 = 1;
const ROTATE_AFTER_BYTES: u64 = 256 * 1024;
const MAX_ROTATED_FILES: usize = 3;
static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
static LAST_TIMESTAMP_MS: AtomicU64 = AtomicU64::new(0);

/// Speaker role associated with a persisted conversation message.
///
/// 权威源:`axagent_harness::types::MessageRole`(lowercase serde 格式)。
/// 业务代码请统一 `use axagent_harness::MessageRole` 访问
/// (走契约层,符合"业务组件 → harness ← 实现"依赖方向),
/// 避免与本 crate 的 `axagent_runtime_core::MessageRole` 产生同名异类歧义。
pub use axagent_harness::types::MessageRole;

/// Structured message content stored inside a [`Session`].
///
/// 权威源: `axagent_harness::ContentBlock`
pub use axagent_harness::ContentBlock;

/// One conversation message with optional token-usage metadata.
///
/// 权威源: `axagent_harness::ConversationMessage`
pub use axagent_harness::ConversationMessage;

// ── 类型已上移至 harness ──
pub use axagent_harness::runtime_types::session::{
    Session, SessionCompaction, SessionError, SessionFork, SessionPersistence, SessionPromptEntry,
};

// ── SessionExt (I/O + JSON 方法，因 Session 类型已上移至 harness) ──

/// I/O 与 JSON 序列化扩展方法。
pub trait SessionExt: Sized {
    fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), SessionError>;
    fn push_message(&mut self, message: ConversationMessage) -> Result<(), SessionError>;
    fn push_user_text(&mut self, text: impl Into<String>) -> Result<(), SessionError>;
    fn push_prompt_entry(&mut self, text: impl Into<String>) -> Result<(), SessionError>;
    fn to_json(&self) -> Result<JsonValue, SessionError>;
}

/// 从文件加载会话。
pub fn session_load_from_path(path: impl AsRef<Path>) -> Result<Session, SessionError> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)?;
    let session = match JsonValue::parse(&contents) {
        Ok(value) if value.as_object().is_some_and(|object| object.contains_key("messages")) => {
            session_from_json(&value)?
        },
        Err(_) | Ok(_) => session_from_jsonl(&contents)?,
    };
    Ok(session.with_persistence_path(path.to_path_buf()))
}

impl SessionExt for Session {
    fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), SessionError> {
        let path = path.as_ref();
        let snapshot = render_jsonl_snapshot(self)?;
        rotate_session_file_if_needed(path)?;
        write_atomic(path, &snapshot)?;
        cleanup_rotated_logs(path)?;
        Ok(())
    }

    fn push_message(&mut self, message: ConversationMessage) -> Result<(), SessionError> {
        self.touch();
        self.messages.push(message);
        let persist_result = {
            let message_ref = self.messages.last().ok_or_else(|| {
                SessionError::Format("message was just pushed but missing".to_string())
            })?;
            append_persisted_message(self, message_ref)
        };
        if let Err(error) = persist_result {
            self.messages.pop();
            return Err(error);
        }
        Ok(())
    }

    fn push_user_text(&mut self, text: impl Into<String>) -> Result<(), SessionError> {
        let raw_text: String = text.into();

        // 提示词注入防护：通过注入的 PromptGuard（如未配置则跳过）
        let processed_text = match &self.prompt_guard {
            Some(guard) => match guard.process_user_input(&raw_text) {
                Ok(wrapped) => wrapped,
                Err(reason) => {
                    tracing::warn!("User input blocked by prompt-guard: {}", reason);
                    return Err(SessionError::ContentBlocked(reason));
                },
            },
            None => raw_text,
        };

        self.push_message(ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text { text: processed_text }],
            usage: None,
        })
    }

    fn push_prompt_entry(&mut self, text: impl Into<String>) -> Result<(), SessionError> {
        let timestamp_ms = current_time_millis();
        let entry = SessionPromptEntry { timestamp_ms, text: text.into() };
        self.prompt_history.push(entry);
        let entry_ref = self.prompt_history.last().expect("entry was just pushed");
        append_persisted_prompt_entry(self, entry_ref)
    }

    fn to_json(&self) -> Result<JsonValue, SessionError> {
        let mut object = BTreeMap::new();
        object.insert("version".to_string(), JsonValue::Number(i64::from(self.version)));
        object.insert("session_id".to_string(), JsonValue::String(self.session_id.clone()));
        object.insert(
            "created_at_ms".to_string(),
            JsonValue::Number(i64_from_u64(self.created_at_ms, "created_at_ms")?),
        );
        object.insert(
            "updated_at_ms".to_string(),
            JsonValue::Number(i64_from_u64(self.updated_at_ms, "updated_at_ms")?),
        );
        object.insert(
            "messages".to_string(),
            JsonValue::Array(self.messages.iter().map(ConversationMessageExt::to_json).collect()),
        );
        if let Some(compaction) = &self.compaction {
            object.insert("compaction".to_string(), session_compaction_to_json(compaction)?);
        }
        if let Some(fork) = &self.fork {
            object.insert("fork".to_string(), session_fork_to_json(fork));
        }
        if let Some(workspace_root) = &self.workspace_root {
            object.insert(
                "workspace_root".to_string(),
                JsonValue::String(workspace_root_to_string(workspace_root)?),
            );
        }
        if !self.prompt_history.is_empty() {
            object.insert(
                "prompt_history".to_string(),
                JsonValue::Array(
                    self.prompt_history.iter().map(session_prompt_entry_to_jsonl).collect(),
                ),
            );
        }
        Ok(JsonValue::Object(object))
    }
}

// ── JSON 反序列化（纯函数）──

/// 从 JSON 解析 Session。
pub fn session_from_json(value: &JsonValue) -> Result<Session, SessionError> {
    let object = value
        .as_object()
        .ok_or_else(|| SessionError::Format("session must be an object".to_string()))?;
    let version = object
        .get("version")
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| SessionError::Format("missing version".to_string()))?;
    let version = u32::try_from(version)
        .map_err(|_| SessionError::Format("version out of range".to_string()))?;
    let messages = object
        .get("messages")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| SessionError::Format("missing messages".to_string()))?
        .iter()
        .map(ConversationMessageExt::from_json)
        .collect::<Result<Vec<_>, _>>()?;
    let now = current_time_millis();
    let session_id = object
        .get("session_id")
        .and_then(JsonValue::as_str)
        .map_or_else(generate_session_id, ToOwned::to_owned);
    let created_at_ms = object
        .get("created_at_ms")
        .map(|value| required_u64_from_value(value, "created_at_ms"))
        .transpose()?
        .unwrap_or(now);
    let updated_at_ms = object
        .get("updated_at_ms")
        .map(|value| required_u64_from_value(value, "updated_at_ms"))
        .transpose()?
        .unwrap_or(created_at_ms);
    let compaction = object.get("compaction").map(session_compaction_from_json).transpose()?;
    let fork = object.get("fork").map(session_fork_from_json).transpose()?;
    let workspace_root =
        object.get("workspace_root").and_then(JsonValue::as_str).map(PathBuf::from);
    let prompt_history = object
        .get("prompt_history")
        .and_then(JsonValue::as_array)
        .map(|entries| entries.iter().filter_map(session_prompt_entry_from_json_opt).collect())
        .unwrap_or_default();
    let model = object.get("model").and_then(JsonValue::as_str).map(String::from);
    Ok(Session {
        version,
        session_id,
        created_at_ms,
        updated_at_ms,
        messages,
        compaction,
        fork,
        workspace_root,
        prompt_history,
        last_health_check_ms: None,
        model,
        persistence: None,
        prompt_guard: None,
    })
}

fn session_from_jsonl(contents: &str) -> Result<Session, SessionError> {
    let mut version = SESSION_VERSION;
    let mut session_id = None;
    let mut created_at_ms = None;
    let mut updated_at_ms = None;
    let mut messages = Vec::new();
    let mut compaction = None;
    let mut fork = None;
    let mut workspace_root = None;
    let mut model = None;
    let mut prompt_history = Vec::new();

    for (line_number, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let value = JsonValue::parse(line).map_err(|error| {
            SessionError::Format(format!(
                "invalid JSONL record at line {}: {}",
                line_number + 1,
                error
            ))
        })?;
        let object = value.as_object().ok_or_else(|| {
            SessionError::Format(format!(
                "JSONL record at line {} must be an object",
                line_number + 1
            ))
        })?;
        match object.get("type").and_then(JsonValue::as_str).ok_or_else(|| {
            SessionError::Format(format!("JSONL record at line {} missing type", line_number + 1))
        })? {
            "session_meta" => {
                version = required_u32(object, "version")?;
                session_id = Some(required_string(object, "session_id")?);
                created_at_ms = Some(required_u64(object, "created_at_ms")?);
                updated_at_ms = Some(required_u64(object, "updated_at_ms")?);
                fork = object.get("fork").map(session_fork_from_json).transpose()?;
                workspace_root =
                    object.get("workspace_root").and_then(JsonValue::as_str).map(PathBuf::from);
                model = object.get("model").and_then(JsonValue::as_str).map(String::from);
            },
            "message" => {
                let message_value = object.get("message").ok_or_else(|| {
                    SessionError::Format(format!(
                        "JSONL record at line {} missing message",
                        line_number + 1
                    ))
                })?;
                messages.push(ConversationMessageExt::from_json(message_value)?);
            },
            "compaction" => {
                compaction =
                    Some(session_compaction_from_json(&JsonValue::Object(object.clone()))?);
            },
            "prompt_history" => {
                if let Some(entry) =
                    session_prompt_entry_from_json_opt(&JsonValue::Object(object.clone()))
                {
                    prompt_history.push(entry);
                }
            },
            other => {
                return Err(SessionError::Format(format!(
                    "unsupported JSONL record type at line {}: {other}",
                    line_number + 1
                )));
            },
        }
    }

    let now = current_time_millis();
    Ok(Session {
        version,
        session_id: session_id.unwrap_or_else(generate_session_id),
        created_at_ms: created_at_ms.unwrap_or(now),
        updated_at_ms: updated_at_ms.unwrap_or(created_at_ms.unwrap_or(now)),
        messages,
        compaction,
        fork,
        workspace_root,
        prompt_history,
        last_health_check_ms: None,
        model,
        persistence: None,
        prompt_guard: None,
    })
}

// ── ConversationMessageExt trait (扩展方法，因类型已上移至 harness) ──

/// 构造器和自定义 JSON 序列化方法。
pub trait ConversationMessageExt: Sized {
    fn user_text(text: impl Into<String>) -> Self;
    fn assistant(blocks: Vec<ContentBlock>) -> Self;
    fn assistant_with_usage(blocks: Vec<ContentBlock>, usage: Option<TokenUsage>) -> Self;
    fn tool_result(
        tool_use_id: impl Into<String>,
        tool_name: impl Into<String>,
        output: impl Into<String>,
        is_error: bool,
    ) -> Self;
    fn to_json(&self) -> JsonValue;
    fn from_json(value: &JsonValue) -> Result<Self, SessionError>
    where
        Self: Sized;
}

impl ConversationMessageExt for ConversationMessage {
    fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text { text: text.into() }],
            usage: None,
        }
    }

    fn assistant(blocks: Vec<ContentBlock>) -> Self {
        Self { role: MessageRole::Assistant, blocks, usage: None }
    }

    fn assistant_with_usage(blocks: Vec<ContentBlock>, usage: Option<TokenUsage>) -> Self {
        Self { role: MessageRole::Assistant, blocks, usage }
    }

    fn tool_result(
        tool_use_id: impl Into<String>,
        tool_name: impl Into<String>,
        output: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: tool_use_id.into(),
                tool_name: tool_name.into(),
                output: output.into(),
                is_error,
            }],
            usage: None,
        }
    }

    fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();
        object.insert(
            "role".to_string(),
            JsonValue::String(
                match self.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                }
                .to_string(),
            ),
        );
        object.insert(
            "blocks".to_string(),
            JsonValue::Array(self.blocks.iter().map(ContentBlockExt::to_json).collect()),
        );
        if let Some(usage) = self.usage {
            object.insert("usage".to_string(), usage_to_json(usage));
        }
        JsonValue::Object(object)
    }

    fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("message must be an object".to_string()))?;
        let role = match object
            .get("role")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| SessionError::Format("missing role".to_string()))?
        {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::Tool,
            other => {
                return Err(SessionError::Format(format!("unsupported message role: {other}")));
            },
        };
        let blocks = object
            .get("blocks")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| SessionError::Format("missing blocks".to_string()))?
            .iter()
            .map(<ContentBlock as ContentBlockExt>::from_json)
            .collect::<Result<Vec<_>, _>>()?;
        let usage = object.get("usage").map(usage_from_json).transpose()?;
        Ok(Self { role, blocks, usage })
    }
}

// ── ContentBlockExt trait (扩展方法，因类型已上移至 harness) ──

/// 自定义 JSON 序列化方法。
pub trait ContentBlockExt {
    fn to_json(&self) -> JsonValue;
    fn from_json(value: &JsonValue) -> Result<ContentBlock, SessionError>
    where
        Self: Sized;
}

impl ContentBlockExt for ContentBlock {
    fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();
        match self {
            Self::Text { text } => {
                object.insert("type".to_string(), JsonValue::String("text".to_string()));
                object.insert("text".to_string(), JsonValue::String(text.clone()));
            },
            Self::ToolUse { id, name, input } => {
                object.insert("type".to_string(), JsonValue::String("tool_use".to_string()));
                object.insert("id".to_string(), JsonValue::String(id.clone()));
                object.insert("name".to_string(), JsonValue::String(name.clone()));
                object.insert("input".to_string(), JsonValue::String(input.clone()));
            },
            Self::ToolResult { tool_use_id, tool_name, output, is_error } => {
                object.insert("type".to_string(), JsonValue::String("tool_result".to_string()));
                object.insert("tool_use_id".to_string(), JsonValue::String(tool_use_id.clone()));
                object.insert("tool_name".to_string(), JsonValue::String(tool_name.clone()));
                object.insert("output".to_string(), JsonValue::String(output.clone()));
                object.insert("is_error".to_string(), JsonValue::Bool(*is_error));
            },
        }
        JsonValue::Object(object)
    }

    fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("block must be an object".to_string()))?;
        match object
            .get("type")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| SessionError::Format("missing block type".to_string()))?
        {
            "text" => Ok(Self::Text { text: required_string(object, "text")? }),
            "tool_use" => Ok(Self::ToolUse {
                id: required_string(object, "id")?,
                name: required_string(object, "name")?,
                input: required_string(object, "input")?,
            }),
            "tool_result" => Ok(Self::ToolResult {
                tool_use_id: required_string(object, "tool_use_id")?,
                tool_name: required_string(object, "tool_name")?,
                output: required_string(object, "output")?,
                is_error: object
                    .get("is_error")
                    .and_then(JsonValue::as_bool)
                    .ok_or_else(|| SessionError::Format("missing is_error".to_string()))?,
            }),
            other => Err(SessionError::Format(format!("unsupported block type: {other}"))),
        }
    }
}

fn message_record(message: &ConversationMessage) -> JsonValue {
    let mut object = BTreeMap::new();
    object.insert("type".to_string(), JsonValue::String("message".to_string()));
    object.insert("message".to_string(), message.to_json());
    JsonValue::Object(object)
}

fn usage_to_json(usage: TokenUsage) -> JsonValue {
    let mut object = BTreeMap::new();
    object.insert("input_tokens".to_string(), JsonValue::Number(i64::from(usage.input_tokens)));
    object.insert("output_tokens".to_string(), JsonValue::Number(i64::from(usage.output_tokens)));
    object.insert(
        "cache_creation_input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.cache_creation_input_tokens)),
    );
    object.insert(
        "cache_read_input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.cache_read_input_tokens)),
    );
    JsonValue::Object(object)
}

fn usage_from_json(value: &JsonValue) -> Result<TokenUsage, SessionError> {
    let object = value
        .as_object()
        .ok_or_else(|| SessionError::Format("usage must be an object".to_string()))?;
    Ok(TokenUsage {
        input_tokens: required_u32(object, "input_tokens")?,
        output_tokens: required_u32(object, "output_tokens")?,
        cache_creation_input_tokens: required_u32(object, "cache_creation_input_tokens")?,
        cache_read_input_tokens: required_u32(object, "cache_read_input_tokens")?,
        cache_miss_input_tokens: object
            .get("cache_miss_input_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as u32),
    })
}

fn required_string(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<String, SessionError> {
    object
        .get(key)
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))
}

fn required_u32(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<u32, SessionError> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    u32::try_from(value).map_err(|_| SessionError::Format(format!("{key} out of range")))
}

fn required_u64(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<u64, SessionError> {
    let value = object.get(key).ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    required_u64_from_value(value, key)
}

fn required_u64_from_value(value: &JsonValue, key: &str) -> Result<u64, SessionError> {
    let value = value.as_i64().ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    u64::try_from(value).map_err(|_| SessionError::Format(format!("{key} out of range")))
}

fn required_usize(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<usize, SessionError> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    usize::try_from(value).map_err(|_| SessionError::Format(format!("{key} out of range")))
}

fn i64_from_u64(value: u64, key: &str) -> Result<i64, SessionError> {
    i64::try_from(value)
        .map_err(|_| SessionError::Format(format!("{key} out of range for JSON number")))
}

fn i64_from_usize(value: usize, key: &str) -> Result<i64, SessionError> {
    i64::try_from(value)
        .map_err(|_| SessionError::Format(format!("{key} out of range for JSON number")))
}

fn workspace_root_to_string(path: &Path) -> Result<String, SessionError> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        SessionError::Format(format!("workspace_root is not valid UTF-8: {}", path.display()))
    })
}

fn current_time_millis() -> u64 {
    let wall_clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default();

    let mut candidate = wall_clock;
    loop {
        let previous = LAST_TIMESTAMP_MS.load(Ordering::Acquire);
        if candidate <= previous {
            candidate = previous.saturating_add(1);
        }
        match LAST_TIMESTAMP_MS.compare_exchange(
            previous,
            candidate,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => return candidate,
            Err(actual) => candidate = actual.saturating_add(1),
        }
    }
}

fn generate_session_id() -> String {
    let millis = current_time_millis();
    let counter = SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("session-{millis}-{counter}")
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), SessionError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp_path = temporary_path_for(path);
    fs::write(&temp_path, contents)?;
    fs::rename(temp_path, path)?;
    Ok(())
}

fn temporary_path_for(path: &Path) -> PathBuf {
    let file_name = path.file_name().and_then(|value| value.to_str()).unwrap_or("session");
    path.with_file_name(format!(
        "{file_name}.tmp-{}-{}",
        current_time_millis(),
        SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn rotate_session_file_if_needed(path: &Path) -> Result<(), SessionError> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };
    if metadata.len() < ROTATE_AFTER_BYTES {
        return Ok(());
    }
    let rotated_path = rotated_log_path(path);
    fs::rename(path, rotated_path)?;
    Ok(())
}

fn rotated_log_path(path: &Path) -> PathBuf {
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("session");
    path.with_file_name(format!("{stem}.rot-{}.jsonl", current_time_millis()))
}

fn cleanup_rotated_logs(path: &Path) -> Result<(), SessionError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("session");
    let prefix = format!("{stem}.rot-");
    let mut rotated_paths = fs::read_dir(parent)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|entry_path| {
            entry_path.file_name().and_then(|value| value.to_str()).is_some_and(|name| {
                name.starts_with(&prefix)
                    && Path::new(name)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
            })
        })
        .collect::<Vec<_>>();

    rotated_paths.sort_by_key(|entry_path| {
        fs::metadata(entry_path).and_then(|metadata| metadata.modified()).unwrap_or(UNIX_EPOCH)
    });

    let remove_count = rotated_paths.len().saturating_sub(MAX_ROTATED_FILES);
    for stale_path in rotated_paths.into_iter().take(remove_count) {
        fs::remove_file(stale_path)?;
    }
    Ok(())
}

// ── 私有辅助函数 ──

fn render_jsonl_snapshot(session: &Session) -> Result<String, SessionError> {
    let meta = meta_record(session)?;
    let mut lines = vec![meta.render()];
    if let Some(compaction) = &session.compaction {
        lines.push(session_compaction_to_jsonl(compaction)?.render());
    }
    lines.extend(
        session.prompt_history.iter().map(|entry| session_prompt_entry_to_jsonl(entry).render()),
    );
    lines.extend(session.messages.iter().map(|message| message_record(message).render()));
    let mut rendered = lines.join("\n");
    rendered.push('\n');
    Ok(rendered)
}

fn append_persisted_message(
    session: &Session,
    message: &ConversationMessage,
) -> Result<(), SessionError> {
    let Some(path) = session.persistence_path() else {
        return Ok(());
    };

    let needs_bootstrap = !path.exists() || fs::metadata(path)?.len() == 0;
    if needs_bootstrap {
        session.save_to_path(path)?;
        return Ok(());
    }

    let mut file = OpenOptions::new().append(true).open(path)?;
    writeln!(file, "{}", message_record(message).render())?;
    Ok(())
}

fn append_persisted_prompt_entry(
    session: &Session,
    entry: &SessionPromptEntry,
) -> Result<(), SessionError> {
    let Some(path) = session.persistence_path() else {
        return Ok(());
    };

    let needs_bootstrap = !path.exists() || fs::metadata(path)?.len() == 0;
    if needs_bootstrap {
        session.save_to_path(path)?;
        return Ok(());
    }

    let mut file = OpenOptions::new().append(true).open(path)?;
    writeln!(file, "{}", session_prompt_entry_to_jsonl(entry).render())?;
    Ok(())
}

fn meta_record(session: &Session) -> Result<JsonValue, SessionError> {
    let mut object = BTreeMap::new();
    object.insert("type".to_string(), JsonValue::String("session_meta".to_string()));
    object.insert("version".to_string(), JsonValue::Number(i64::from(session.version)));
    object.insert("session_id".to_string(), JsonValue::String(session.session_id.clone()));
    object.insert(
        "created_at_ms".to_string(),
        JsonValue::Number(i64_from_u64(session.created_at_ms, "created_at_ms")?),
    );
    object.insert(
        "updated_at_ms".to_string(),
        JsonValue::Number(i64_from_u64(session.updated_at_ms, "updated_at_ms")?),
    );
    if let Some(fork) = &session.fork {
        object.insert("fork".to_string(), session_fork_to_json(fork));
    }
    if let Some(workspace_root) = &session.workspace_root {
        object.insert(
            "workspace_root".to_string(),
            JsonValue::String(workspace_root_to_string(workspace_root)?),
        );
    }
    if let Some(model) = &session.model {
        object.insert("model".to_string(), JsonValue::String(model.clone()));
    }
    Ok(JsonValue::Object(object))
}

// ── SessionCompaction JSON 辅助函数 ──

fn session_compaction_to_json(compaction: &SessionCompaction) -> Result<JsonValue, SessionError> {
    let mut object = BTreeMap::new();
    object.insert("count".to_string(), JsonValue::Number(i64::from(compaction.count)));
    object.insert(
        "removed_message_count".to_string(),
        JsonValue::Number(i64_from_usize(
            compaction.removed_message_count,
            "removed_message_count",
        )?),
    );
    object.insert("summary".to_string(), JsonValue::String(compaction.summary.clone()));
    Ok(JsonValue::Object(object))
}

fn session_compaction_to_jsonl(compaction: &SessionCompaction) -> Result<JsonValue, SessionError> {
    let mut object = BTreeMap::new();
    object.insert("type".to_string(), JsonValue::String("compaction".to_string()));
    object.insert("count".to_string(), JsonValue::Number(i64::from(compaction.count)));
    object.insert(
        "removed_message_count".to_string(),
        JsonValue::Number(i64_from_usize(
            compaction.removed_message_count,
            "removed_message_count",
        )?),
    );
    object.insert("summary".to_string(), JsonValue::String(compaction.summary.clone()));
    Ok(JsonValue::Object(object))
}

fn session_compaction_from_json(value: &JsonValue) -> Result<SessionCompaction, SessionError> {
    let object = value
        .as_object()
        .ok_or_else(|| SessionError::Format("compaction not an object".to_string()))?;
    Ok(SessionCompaction {
        count: required_u32(object, "count")?,
        removed_message_count: required_usize(object, "removed_message_count")?,
        summary: required_string(object, "summary")?,
    })
}

// ── SessionFork JSON 辅助函数 ──

fn session_fork_to_json(fork: &SessionFork) -> JsonValue {
    let mut object = BTreeMap::new();
    object
        .insert("parent_session_id".to_string(), JsonValue::String(fork.parent_session_id.clone()));
    if let Some(branch_name) = &fork.branch_name {
        object.insert("branch_name".to_string(), JsonValue::String(branch_name.clone()));
    }
    JsonValue::Object(object)
}

fn session_fork_from_json(value: &JsonValue) -> Result<SessionFork, SessionError> {
    let object =
        value.as_object().ok_or_else(|| SessionError::Format("fork not an object".to_string()))?;
    Ok(SessionFork {
        parent_session_id: required_string(object, "parent_session_id")?,
        branch_name: object.get("branch_name").and_then(JsonValue::as_str).map(String::from),
    })
}

// ── SessionPromptEntry JSON 辅助函数 ──

fn session_prompt_entry_to_jsonl(entry: &SessionPromptEntry) -> JsonValue {
    let mut object = BTreeMap::new();
    object.insert("type".to_string(), JsonValue::String("prompt_history".to_string()));
    object.insert("timestamp_ms".to_string(), JsonValue::Number(entry.timestamp_ms as i64));
    object.insert("text".to_string(), JsonValue::String(entry.text.clone()));
    JsonValue::Object(object)
}

fn session_prompt_entry_from_json_opt(value: &JsonValue) -> Option<SessionPromptEntry> {
    let object = value.as_object()?;
    Some(SessionPromptEntry {
        timestamp_ms: object.get("timestamp_ms")?.as_i64()? as u64,
        text: object.get("text")?.as_str()?.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ContentBlock, ConversationMessage, MessageRole, Session, SessionFork, cleanup_rotated_logs,
        current_time_millis, rotate_session_file_if_needed,
    };
    use crate::json::JsonValue;
    use crate::session::{
        ContentBlockExt, ConversationMessageExt, SessionExt, session_from_json,
        session_load_from_path,
    };
    use crate::usage::TokenUsage;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn session_timestamps_are_monotonic_under_tight_loops() {
        let first = current_time_millis();
        let second = current_time_millis();
        let third = current_time_millis();

        assert!(first < second);
        assert!(second < third);
    }

    #[test]
    fn persists_and_restores_session_jsonl() {
        let mut session = Session::new();
        session.push_user_text("hello").expect("user message should append");
        session
            .push_message(ConversationMessageExt::assistant_with_usage(
                vec![
                    ContentBlock::Text { text: "thinking".to_string() },
                    ContentBlock::ToolUse {
                        id: "tool-1".to_string(),
                        name: "bash".to_string(),
                        input: "echo hi".to_string(),
                    },
                ],
                Some(TokenUsage {
                    input_tokens: 10,
                    output_tokens: 4,
                    cache_creation_input_tokens: 1,
                    cache_read_input_tokens: 2,
                    cache_miss_input_tokens: None,
                }),
            ))
            .expect("assistant message should append");
        session
            .push_message(ConversationMessageExt::tool_result("tool-1", "bash", "hi", false))
            .expect("tool result should append");

        let path = temp_session_path("jsonl");
        session.save_to_path(&path).expect("session should save");
        let restored = session_load_from_path(&path).expect("session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        assert_eq!(restored, session);
        assert_eq!(restored.messages[2].role, MessageRole::Tool);
        assert_eq!(restored.messages[1].usage.expect("usage").total_tokens(), 17);
        assert_eq!(restored.session_id, session.session_id);
    }

    #[test]
    fn loads_legacy_session_json_object() {
        let path = temp_session_path("legacy");
        let legacy = JsonValue::Object(
            [
                ("version".to_string(), JsonValue::Number(1)),
                (
                    "messages".to_string(),
                    JsonValue::Array(vec![ConversationMessage::user_text("legacy").to_json()]),
                ),
            ]
            .into_iter()
            .collect(),
        );
        fs::write(&path, legacy.render()).expect("legacy file should write");

        let restored = session_load_from_path(&path).expect("legacy session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        assert_eq!(restored.messages.len(), 1);
        assert_eq!(restored.messages[0], ConversationMessageExt::user_text("legacy"));
        assert!(!restored.session_id.is_empty());
    }

    #[test]
    fn appends_messages_to_persisted_jsonl_session() {
        use axagent_prompt_guard::{GuardConfig, PromptGuardPipeline};
        use std::sync::Arc;
        let path = temp_session_path("append");
        let mut session = Session::new()
            .with_persistence_path(path.clone())
            .with_prompt_guard(Arc::new(PromptGuardPipeline::new(GuardConfig::default())));
        session.save_to_path(&path).expect("initial save should succeed");
        session.push_user_text("hi").expect("user append should succeed");
        session
            .push_message(ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "hello".to_string(),
            }]))
            .expect("assistant append should succeed");

        let restored = session_load_from_path(&path).expect("session should replay from jsonl");
        fs::remove_file(&path).expect("temp file should be removable");

        assert_eq!(restored.messages.len(), 2);
        assert_eq!(
            restored.messages[0],
            ConversationMessage {
                role: MessageRole::User,
                blocks: vec![ContentBlock::Text {
                    text: "<user_query role=\"user\" sanitized=\"true\">\nhi\n</user_query>"
                        .to_string(),
                }],
                usage: None,
            }
        );
    }

    #[test]
    fn persists_compaction_metadata() {
        let path = temp_session_path("compaction");
        let mut session = Session::new();
        session.push_user_text("before").expect("message should append");
        session.record_compaction("summarized earlier work", 4);
        session.save_to_path(&path).expect("session should save");

        let restored = session_load_from_path(&path).expect("session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        let compaction = restored.compaction.expect("compaction metadata");
        assert_eq!(compaction.count, 1);
        assert_eq!(compaction.removed_message_count, 4);
        assert!(compaction.summary.contains("summarized"));
    }

    #[test]
    fn forks_sessions_with_branch_metadata_and_persists_it() {
        let path = temp_session_path("fork");
        let mut session = Session::new();
        session.push_user_text("before fork").expect("message should append");

        let forked =
            session.fork(Some("investigation".to_string())).with_persistence_path(path.clone());
        forked.save_to_path(&path).expect("forked session should save");

        let restored = session_load_from_path(&path).expect("forked session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        assert_ne!(restored.session_id, session.session_id);
        assert_eq!(
            restored.fork,
            Some(SessionFork {
                parent_session_id: session.session_id,
                branch_name: Some("investigation".to_string()),
            })
        );
        assert_eq!(restored.messages, forked.messages);
    }

    #[test]
    fn rotates_and_cleans_up_large_session_logs() {
        // given
        let path = temp_session_path("rotation");
        let oversized_length =
            usize::try_from(super::ROTATE_AFTER_BYTES + 10).expect("rotate threshold should fit");
        fs::write(&path, "x".repeat(oversized_length)).expect("oversized file should write");

        // when
        rotate_session_file_if_needed(&path).expect("rotation should succeed");

        // then
        assert!(!path.exists(), "original path should be rotated away before rewrite");

        for _ in 0..5 {
            let rotated = super::rotated_log_path(&path);
            fs::write(&rotated, "old").expect("rotated file should write");
        }
        cleanup_rotated_logs(&path).expect("cleanup should succeed");

        let rotated_count = rotation_files(&path).len();
        assert!(rotated_count <= super::MAX_ROTATED_FILES);
        for rotated in rotation_files(&path) {
            fs::remove_file(rotated).expect("rotated file should be removable");
        }
    }

    #[test]
    fn rejects_jsonl_record_without_type() {
        // given
        let path = write_temp_session_file(
            "missing-type",
            r#"{"message":{"role":"user","blocks":[{"type":"text","text":"hello"}]}}"#,
        );

        // when
        let error = session_load_from_path(&path)
            .expect_err("session should reject JSONL records without a type");

        // then
        assert!(error.to_string().contains("missing type"));
        fs::remove_file(path).expect("temp file should be removable");
    }

    #[test]
    fn rejects_jsonl_message_record_without_message_payload() {
        // given
        let path = write_temp_session_file("missing-message", r#"{"type":"message"}"#);

        // when
        let error = session_load_from_path(&path)
            .expect_err("session should reject JSONL message records without message payload");

        // then
        assert!(error.to_string().contains("missing message"));
        fs::remove_file(path).expect("temp file should be removable");
    }

    #[test]
    fn rejects_jsonl_record_with_unknown_type() {
        // given
        let path = write_temp_session_file("unknown-type", r#"{"type":"mystery"}"#);

        // when
        let error = session_load_from_path(&path)
            .expect_err("session should reject unknown JSONL record types");

        // then
        assert!(error.to_string().contains("unsupported JSONL record type"));
        fs::remove_file(path).expect("temp file should be removable");
    }

    #[test]
    fn rejects_legacy_session_json_without_messages() {
        // given
        let session = JsonValue::Object(
            [("version".to_string(), JsonValue::Number(1))].into_iter().collect(),
        );

        // when
        let error = session_from_json(&session)
            .expect_err("legacy session objects should require messages");

        // then
        assert!(error.to_string().contains("missing messages"));
    }

    #[test]
    fn normalizes_blank_fork_branch_name_to_none() {
        // given
        let session = Session::new();

        // when
        let forked = session.fork(Some("   ".to_string()));

        // then
        assert_eq!(forked.fork.expect("fork metadata").branch_name, None);
    }

    #[test]
    fn rejects_unknown_content_block_type() {
        // given
        let block = JsonValue::Object(
            [("type".to_string(), JsonValue::String("unknown".to_string()))].into_iter().collect(),
        );

        // when
        let error = <ContentBlock as ContentBlockExt>::from_json(&block)
            .expect_err("content blocks should reject unknown types");

        // then
        assert!(error.to_string().contains("unsupported block type"));
    }

    #[test]
    fn persists_workspace_root_round_trip_and_forks_inherit_it() {
        // given
        let path = temp_session_path("workspace-root");
        let workspace_root = PathBuf::from("/tmp/b4-phantom-diag");
        let mut session = Session::new().with_workspace_root(workspace_root.clone());
        session.push_user_text("write to the right cwd").expect("user message should append");

        // when
        session.save_to_path(&path).expect("workspace-bound session should save");
        let restored = session_load_from_path(&path).expect("session should load");
        let forked = restored.fork(Some("phantom-diag".to_string()));
        fs::remove_file(&path).expect("temp file should be removable");

        // then
        assert_eq!(restored.workspace_root(), Some(workspace_root.as_path()));
        assert_eq!(forked.workspace_root(), Some(workspace_root.as_path()));
    }

    fn temp_session_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("runtime-session-{label}-{nanos}.json"))
    }

    fn write_temp_session_file(label: &str, contents: &str) -> PathBuf {
        let path = temp_session_path(label);
        fs::write(&path, format!("{contents}\n")).expect("temp session file should write");
        path
    }

    fn rotation_files(path: &Path) -> Vec<PathBuf> {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("temp path should have file stem")
            .to_string();
        fs::read_dir(path.parent().expect("temp path should have parent"))
            .expect("temp dir should read")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|entry_path| {
                entry_path.file_name().and_then(|value| value.to_str()).is_some_and(|name| {
                    name.starts_with(&format!("{stem}.rot-"))
                        && Path::new(name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
                })
            })
            .collect()
    }
}

/// Per-worktree session isolation: returns a session directory namespaced
/// by the workspace fingerprint of the given working directory.
/// This prevents parallel `opencode serve` instances from colliding.
/// Called by external consumers (e.g. clawhip) to enumerate sessions for a CWD.
pub fn workspace_sessions_dir(cwd: &std::path::Path) -> Result<std::path::PathBuf, SessionError> {
    let store = crate::session_control::SessionStore::from_cwd(cwd)
        .map_err(|e| SessionError::Io(std::io::Error::other(e.to_string())))?;
    Ok(store.sessions_dir().to_path_buf())
}

#[cfg(test)]
mod workspace_sessions_dir_tests {
    use super::*;
    use std::fs;

    #[test]
    fn workspace_sessions_dir_returns_fingerprinted_path_for_valid_cwd() {
        let tmp = std::env::temp_dir().join("claw-session-dir-test");
        fs::create_dir_all(&tmp).expect("create temp dir");

        let result = workspace_sessions_dir(&tmp);
        assert!(
            result.is_ok(),
            "workspace_sessions_dir should succeed for a valid CWD, got: {result:?}"
        );
        let dir = result.unwrap();
        // The returned path should be non-empty and end with a hash component
        assert!(!dir.as_os_str().is_empty());
        // Two calls with the same CWD should produce identical paths (deterministic)
        let result2 = workspace_sessions_dir(&tmp).unwrap();
        assert_eq!(dir, result2, "workspace_sessions_dir must be deterministic");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn workspace_sessions_dir_differs_for_different_cwds() {
        let tmp_a = std::env::temp_dir().join("claw-session-dir-a");
        let tmp_b = std::env::temp_dir().join("claw-session-dir-b");
        fs::create_dir_all(&tmp_a).expect("create dir a");
        fs::create_dir_all(&tmp_b).expect("create dir b");

        let dir_a = workspace_sessions_dir(&tmp_a).expect("dir a");
        let dir_b = workspace_sessions_dir(&tmp_b).expect("dir b");
        assert_ne!(dir_a, dir_b, "different CWDs must produce different session dirs");

        fs::remove_dir_all(&tmp_a).ok();
        fs::remove_dir_all(&tmp_b).ok();
    }
}
