// SPDX-License-Identifier: AGPL-3.0-only

//! G18 Sessions Export 多格式 — Tauri 命令层
//!
//! 支持把会话（Conversation + Messages）导出为 4 种格式：
//!
//! 1. `messages.jsonl` — 每行一个 Message JSON（OpenAI ChatCompletion 兼容）
//! 2. `openai_dataset_jsonl` — OpenAI 微调数据集格式（每行一个 `{messages: [...]}`）
//! 3. `markdown` — 完整对话 markdown（含元数据头）
//! 4. `manifest_json` — 仅元数据 manifest（不含消息正文）
//!
//! ## 设计
//!
//! - 全部走 `tempfile` 写入，返回文件路径；前端通过 `download_file` 命令下载
//! - 支持单会话 / 多会话批量导出
//! - 不修改任何数据库状态，纯只读

use std::path::PathBuf;

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_harness::types::{Conversation, Message, MessageRole};
use serde::{Deserialize, Serialize};
use tauri::State;

// ── 导出格式枚举 ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// 每行一个 Message JSON
    MessagesJsonl,
    /// OpenAI 微调数据集格式
    OpenaiDatasetJsonl,
    /// Markdown 对话记录
    Markdown,
    /// 仅元数据 manifest
    ManifestJson,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "messages_jsonl" => Ok(Self::MessagesJsonl),
            "openai_dataset_jsonl" => Ok(Self::OpenaiDatasetJsonl),
            "markdown" => Ok(Self::Markdown),
            "manifest_json" => Ok(Self::ManifestJson),
            _ => Err(format!("未知导出格式: {s}")),
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::MessagesJsonl => "messages.jsonl",
            Self::OpenaiDatasetJsonl => "openai.jsonl",
            Self::Markdown => "md",
            Self::ManifestJson => "manifest.json",
        }
    }
}

// ── 导出结果 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    /// 导出文件路径
    pub file_path: String,
    /// 导出格式
    pub format: String,
    /// 导出的会话数量
    pub conversation_count: usize,
    /// 导出的消息总数
    pub message_count: usize,
    /// 文件大小（字节）
    pub file_size: u64,
}

// ── Manifest 结构 ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionManifest {
    pub exported_at: i64,
    pub app_version: String,
    pub conversations: Vec<ConversationManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationManifestEntry {
    pub id: String,
    pub title: String,
    pub model_id: String,
    pub provider_id: String,
    pub message_count: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub session_type: String,
}

// ── Tauri 命令 ───────────────────────────────────────────────────────────

/// 导出单个会话为指定格式
#[agent_command(domain = "general", safety = Caution, call_mode = StateOnly, description =  "导出单个会话")]
#[tauri::command]
pub async fn export_session(
    state: State<'_, AppState>,
    conversation_id: String,
    format: String,
    output_dir: Option<String>,
) -> Result<ExportResult, String> {
    let fmt = ExportFormat::from_str(&format)?;

    // 拉取会话和消息
    let db = state.harness.db();
    let conversation = axagent_dao::repo::conversation::get_conversation(db, &conversation_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    let messages =
        axagent_dao::repo::message::list_messages(db, &conversation_id).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    // 决定输出目录
    let out_dir = output_dir.map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    if !out_dir.exists() {
        std::fs::create_dir_all(&out_dir).map_err(|e| format!("创建输出目录失败: {e}"))?;
    }

    // 构造文件名
    let safe_title = sanitize_filename(&conversation.title);
    let filename = format!("{}_{}_{}", safe_title, &conversation_id[..8], fmt.extension());
    let file_path = out_dir.join(filename);

    // 按格式写入
    let message_count = messages.len();
    let content = match fmt {
        ExportFormat::MessagesJsonl => render_messages_jsonl(&messages),
        ExportFormat::OpenaiDatasetJsonl => render_openai_dataset_jsonl(&conversation, &messages),
        ExportFormat::Markdown => render_markdown(&conversation, &messages),
        ExportFormat::ManifestJson => {
            let manifest = build_manifest(vec![(conversation.clone(), messages.len() as u32)]);
            serde_json::to_string_pretty(&manifest).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?
        },
    };

    std::fs::write(&file_path, content).map_err(|e| format!("写入文件失败: {e}"))?;

    let file_size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

    Ok(ExportResult {
        file_path: file_path.to_string_lossy().to_string(),
        format: format.clone(),
        conversation_count: 1,
        message_count,
        file_size,
    })
}

/// 批量导出多个会话（每会话一个文件，返回 zip 路径或目录路径）
#[agent_command(domain = "general", safety = Caution, call_mode = StateOnly, description =  "批量导出多个会话")]
#[tauri::command]
pub async fn export_sessions_batch(
    state: State<'_, AppState>,
    conversation_ids: Vec<String>,
    format: String,
    output_dir: Option<String>,
) -> Result<Vec<ExportResult>, String> {
    let fmt = ExportFormat::from_str(&format)?;
    let out_dir = output_dir.map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    if !out_dir.exists() {
        std::fs::create_dir_all(&out_dir).map_err(|e| format!("创建输出目录失败: {e}"))?;
    }

    let mut results = Vec::with_capacity(conversation_ids.len());
    for cid in conversation_ids {
        let result = export_session(
            state.clone(),
            cid.clone(),
            fmt.extension().to_string(),
            Some(out_dir.to_string_lossy().to_string()),
        )
        .await;
        match result {
            Ok(r) => results.push(r),
            Err(e) => tracing::warn!("[SessionsExport] 跳过会话 {cid}: {e}"),
        }
    }

    Ok(results)
}

/// 仅导出 manifest（不包含消息正文，体积小）
#[agent_command(domain = "general", safety = Caution, call_mode = StateOnly, description =  "导出会话清单")]
#[tauri::command]
pub async fn export_sessions_manifest(
    state: State<'_, AppState>,
    conversation_ids: Option<Vec<String>>,
    output_path: Option<String>,
) -> Result<ExportResult, String> {
    let db = state.harness.db();

    // 拉取会话列表
    let conversations = if let Some(ids) = conversation_ids {
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            match axagent_dao::repo::conversation::get_conversation(db, &id).await {
                Ok(c) => out.push(c),
                Err(e) => tracing::warn!("[SessionsExport] 跳过会话 {id}: {e}"),
            }
        }
        out
    } else {
        axagent_dao::repo::conversation::list_conversations(db).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
    };

    // 构造 manifest
    let manifest =
        build_manifest(conversations.iter().map(|c| (c.clone(), c.message_count)).collect());

    let path = output_path.map(PathBuf::from).unwrap_or_else(|| {
        std::env::temp_dir()
            .join(format!("sessions_manifest_{}.json", chrono::Utc::now().timestamp()))
    });

    let content = serde_json::to_string_pretty(&manifest).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    std::fs::write(&path, content).map_err(|e| format!("写入 manifest 失败: {e}"))?;

    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    Ok(ExportResult {
        file_path: path.to_string_lossy().to_string(),
        format: "manifest_json".to_string(),
        conversation_count: manifest.conversations.len(),
        message_count: 0,
        file_size,
    })
}

// ── 渲染函数 ──────────────────────────────────────────────────────────────

/// 格式 1：messages.jsonl — 每行一个 Message JSON
fn render_messages_jsonl(messages: &[Message]) -> String {
    let mut out = String::new();
    for msg in messages {
        let line = serde_json::to_string(msg).unwrap_or_else(|_| "{}".to_string());
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// 格式 2：openai_dataset_jsonl — OpenAI 微调数据集格式
///
/// 每行一个 `{messages: [{role, content}, ...]}` 对象，仅包含 user/assistant 消息。
/// 多轮对话被合并为单条记录（按时间顺序）。
fn render_openai_dataset_jsonl(_conversation: &Conversation, messages: &[Message]) -> String {
    use serde_json::json;

    // 仅保留 user / assistant 消息，按时间排序
    let filtered: Vec<&Message> = messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::User | MessageRole::Assistant))
        .collect();

    if filtered.is_empty() {
        return String::new();
    }

    let messages_array: Vec<serde_json::Value> = filtered
        .iter()
        .map(|m| {
            let role_str = match m.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => "system",
                MessageRole::Tool => "tool",
            };
            json!({
                "role": role_str,
                "content": m.content,
            })
        })
        .collect();

    let record = json!({
        "messages": messages_array
    });

    serde_json::to_string(&record).unwrap_or_default() + "\n"
}

/// 格式 3：markdown — 完整对话 markdown
fn render_markdown(conversation: &Conversation, messages: &[Message]) -> String {
    let mut out = String::new();

    // YAML front matter
    out.push_str("---\n");
    out.push_str(&format!("title: {}\n", escape_yaml(&conversation.title)));
    out.push_str(&format!("conversation_id: {}\n", conversation.id));
    out.push_str(&format!("model: {}\n", conversation.model_id));
    out.push_str(&format!("provider: {}\n", conversation.provider_id));
    out.push_str(&format!("session_type: {}\n", conversation.session_type));
    out.push_str(&format!("message_count: {}\n", messages.len()));
    out.push_str(&format!(
        "created_at: {}\n",
        chrono::DateTime::from_timestamp(conversation.created_at / 1000, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| conversation.created_at.to_string())
    ));
    out.push_str(&format!("exported_at: {}\n", chrono::Utc::now().to_rfc3339()));
    out.push_str("---\n\n");

    out.push_str(&format!("# {}\n\n", conversation.title));

    for msg in messages {
        let role_label = match msg.role {
            MessageRole::User => "🧑 User",
            MessageRole::Assistant => "🤖 Assistant",
            MessageRole::System => "⚙️ System",
            MessageRole::Tool => "🔧 Tool",
        };

        out.push_str(&format!("## {} — ", role_label));
        out.push_str(&format!(
            "{}\n\n",
            chrono::DateTime::from_timestamp(msg.created_at / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| msg.created_at.to_string())
        ));

        out.push_str(&msg.content);
        out.push_str("\n\n");

        // 元数据
        if let Some(tokens) = msg.token_count {
            out.push_str(&format!("*Tokens: {}*\n\n", tokens));
        }
        if let Some(model) = &msg.model_id {
            out.push_str(&format!("*Model: {}*\n\n", model));
        }
        if let Some(tool_calls) = &msg.tool_calls_json {
            if !tool_calls.is_empty() {
                out.push_str("**Tool Calls:**\n```json\n");
                out.push_str(tool_calls);
                out.push_str("\n```\n\n");
            }
        }
    }

    out
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────

fn build_manifest(conversations: Vec<(Conversation, u32)>) -> SessionManifest {
    SessionManifest {
        exported_at: chrono::Utc::now().timestamp_millis(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        conversations: conversations
            .into_iter()
            .map(|(c, msg_count)| ConversationManifestEntry {
                id: c.id,
                title: c.title,
                model_id: c.model_id,
                provider_id: c.provider_id,
                message_count: msg_count,
                created_at: c.created_at,
                updated_at: c.updated_at,
                session_type: c.session_type,
            })
            .collect(),
    }
}

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn escape_yaml(s: &str) -> String {
    s.replace('"', "\\\"").replace('\n', " ")
}
