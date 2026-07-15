// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

use crate::constants;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub language: String,
    pub theme_mode: String,
    /// Visual theme preset key (e.g. "deep-dusk", "oceanic-dark"). Persisted in
    /// the JS store and must round-trip through the DB, so it lives on the
    /// Rust struct as well — otherwise serde silently drops it on save.
    pub theme_preset: String,
    pub primary_color: String,
    pub border_radius: u8,
    pub auto_start: bool,
    pub show_on_start: bool,
    pub minimize_to_tray: bool,
    pub font_size: u8,
    pub font_weight: u16,
    pub font_family: String,
    pub code_font_family: String,
    pub bubble_style: String,
    pub code_theme: String,
    pub code_theme_light: String,
    pub default_provider_id: Option<String>,
    pub default_model_id: Option<String>,
    pub default_temperature: Option<f32>,
    pub default_max_tokens: Option<u32>,
    pub default_top_p: Option<f32>,
    pub default_frequency_penalty: Option<f32>,
    pub default_context_count: Option<u32>,
    pub title_summary_provider_id: Option<String>,
    pub title_summary_model_id: Option<String>,
    pub title_summary_temperature: Option<f32>,
    pub title_summary_max_tokens: Option<u32>,
    pub title_summary_top_p: Option<f32>,
    pub title_summary_frequency_penalty: Option<f32>,
    pub title_summary_context_count: Option<u32>,
    pub title_summary_prompt: Option<String>,
    pub compression_provider_id: Option<String>,
    pub compression_model_id: Option<String>,
    pub compression_temperature: Option<f32>,
    pub compression_max_tokens: Option<u32>,
    pub compression_top_p: Option<f32>,
    pub compression_frequency_penalty: Option<f32>,
    pub compression_prompt: Option<String>,
    pub proxy_type: Option<String>,
    pub proxy_address: Option<String>,
    pub proxy_port: Option<u16>,
    pub global_shortcut: String,
    pub shortcut_toggle_current_window: String,
    pub shortcut_toggle_all_windows: String,
    pub shortcut_close_window: String,
    pub shortcut_new_conversation: String,
    pub shortcut_open_settings: String,
    pub shortcut_toggle_model_selector: String,
    pub shortcut_fill_last_message: String,
    pub shortcut_clear_context: String,
    pub shortcut_clear_conversation_messages: String,
    pub shortcut_toggle_gateway: String,
    pub shortcut_toggle_mode: String,
    pub shortcut_show_quick_bar: String,
    pub gateway_auto_start: bool,
    pub gateway_listen_address: String,
    pub gateway_port: u16,
    pub gateway_ssl_enabled: bool,
    pub gateway_ssl_mode: String,
    pub gateway_ssl_cert_path: Option<String>,
    pub gateway_ssl_key_path: Option<String>,
    pub gateway_ssl_port: u16,
    pub gateway_force_ssl: bool,
    pub always_on_top: bool,
    pub tray_enabled: bool,
    pub global_shortcuts_enabled: bool,
    pub shortcut_registration_logs_enabled: bool,
    pub shortcut_trigger_toast_enabled: bool,
    pub notifications_enabled: bool,
    pub mini_window_enabled: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub notify_backup: bool,
    pub notify_import: bool,
    pub notify_errors: bool,
    // Auto-backup settings
    pub backup_dir: Option<String>,
    pub auto_backup_enabled: bool,
    pub auto_backup_interval_hours: u32,
    pub auto_backup_max_count: u32,
    // WebDAV sync settings
    pub webdav_host: Option<String>,
    pub webdav_username: Option<String>,
    pub webdav_password: Option<String>,
    pub webdav_path: Option<String>,
    pub webdav_accept_invalid_certs: bool,
    pub webdav_sync_enabled: bool,
    pub webdav_sync_interval_minutes: u32,
    pub webdav_max_remote_backups: u32,
    pub webdav_include_documents: bool,
    // S3 sync settings
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_access_key_id: Option<String>,
    pub s3_secret_access_key: Option<String>,
    pub s3_root: Option<String>,
    pub s3_provider_preset: Option<serde_json::Value>,
    pub s3_use_path_style: bool,
    pub s3_sync_enabled: bool,
    pub s3_sync_interval_minutes: u32,
    pub s3_max_remote_backups: u32,
    pub s3_include_documents: bool,
    /// Cloud sync unified switch (mobile mode)
    pub cloud_sync_enabled: bool,
    /// Cloud backend selection: "s3" | "webdav"
    pub cloud_backend: Option<String>,
    /// Profile name for multi-profile cloud sync
    pub sync_profile_name: Option<String>,
    /// Closed-loop nudge scheduler enabled.
    pub closed_loop_enabled: bool,
    /// Closed-loop nudge interval in minutes (default 5).
    pub closed_loop_interval_minutes: u32,
    pub last_selected_conversation_id: Option<String>,
    /// Custom documents root directory (overrides ~/Documents/axagent/).
    pub documents_root_override: Option<String>,
    /// Auto update check interval in minutes (default 60, min 1).
    pub update_check_interval: u32,
    /// Global system prompt fallback — used when a conversation has no custom system prompt.
    pub default_system_prompt: Option<String>,
    /// Chat minimap / navigation overlay.
    pub chat_minimap_enabled: bool,
    pub chat_minimap_style: String,
    /// Multi-model response display mode: "tabs" | "side-by-side" | "stacked".
    pub multi_model_display_mode: String,
    /// Render user messages as Markdown (like AI messages). Default: false.
    pub render_user_markdown: bool,
    /// Default workspace directory for new sessions when not manually set.
    pub default_workspace_dir: Option<String>,
    /// Enable screen perception and vision-based UI control.
    pub screen_perception_enabled: bool,
    /// Enable RL optimizer for tool selection and task strategies.
    pub rl_optimizer_enabled: bool,
    /// Enable LoRA fine-tuning for custom model adaptation.
    pub lora_finetune_enabled: bool,
    /// Enable proactive nudge suggestions based on context.
    pub proactive_nudge_enabled: bool,
    /// Enable thought chain visualization for reasoning.
    pub thought_chain_enabled: bool,
    /// Enable automatic error recovery suggestions.
    pub error_recovery_enabled: bool,
    /// Cloud workspace URI (supports s3://, webdav://, local://)
    pub workspace_uri: Option<String>,
    /// RAG 高级管线配置（查询增强、重排序、自省式质检）
    #[serde(default)]
    pub rag_pipeline_config: serde_json::Value,
    /// Show the right-side agent execution panel by default.
    pub agent_panel_enabled: bool,
    /// Use the compact (simplified) agent panel view by default.
    pub agent_panel_compact: bool,
    /// Onboarding — welcome wizard completed.
    pub onboarding_completed: bool,
    /// Onboarding — wizard dismissed by the user.
    pub onboarding_wizard_dismissed: bool,
    /// Onboarding — interactive tutorial completed.
    pub onboarding_tutorial_completed: bool,
    /// Onboarding — quick-start preset selected during the wizard.
    pub onboarding_selected_preset: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            theme_mode: constants::role::SYSTEM.to_string(),
            theme_preset: "deep-dusk".to_string(),
            primary_color: "#17A93D".to_string(),
            border_radius: 8,
            auto_start: false,
            show_on_start: true,
            minimize_to_tray: true,
            font_size: 14,
            font_weight: 400,
            font_family: String::new(),
            code_font_family: String::new(),
            bubble_style: "minimal".to_string(),
            code_theme: "poimandres".to_string(),
            code_theme_light: "github-light".to_string(),
            default_provider_id: None,
            default_model_id: None,
            default_temperature: None,
            default_max_tokens: None,
            default_top_p: None,
            default_frequency_penalty: None,
            default_context_count: None,
            title_summary_provider_id: None,
            title_summary_model_id: None,
            title_summary_temperature: None,
            title_summary_max_tokens: None,
            title_summary_top_p: None,
            title_summary_frequency_penalty: None,
            title_summary_context_count: None,
            title_summary_prompt: None,
            compression_provider_id: None,
            compression_model_id: None,
            compression_temperature: None,
            compression_max_tokens: None,
            compression_top_p: None,
            compression_frequency_penalty: None,
            compression_prompt: None,
            proxy_type: None,
            proxy_address: None,
            proxy_port: None,
            global_shortcut: "CommandOrControl+Shift+A".to_string(),
            shortcut_toggle_current_window: "CommandOrControl+Shift+A".to_string(),
            shortcut_toggle_all_windows: "CommandOrControl+Shift+Alt+A".to_string(),
            shortcut_close_window: "CommandOrControl+Shift+W".to_string(),
            shortcut_new_conversation: "CommandOrControl+N".to_string(),
            shortcut_open_settings: "CommandOrControl+Comma".to_string(),
            shortcut_toggle_model_selector: "CommandOrControl+Shift+M".to_string(),
            shortcut_fill_last_message: "CommandOrControl+Shift+ArrowUp".to_string(),
            shortcut_clear_context: "CommandOrControl+Shift+K".to_string(),
            shortcut_clear_conversation_messages: "CommandOrControl+Shift+Backspace".to_string(),
            shortcut_toggle_gateway: "CommandOrControl+Shift+G".to_string(),
            shortcut_toggle_mode: "Shift+Tab".to_string(),
            shortcut_show_quick_bar: "CommandOrControl+Shift+Space".to_string(),
            gateway_auto_start: false,
            gateway_listen_address: "127.0.0.1".to_string(),
            gateway_port: 8000,
            gateway_ssl_enabled: false,
            gateway_ssl_mode: "upload".to_string(),
            gateway_ssl_cert_path: None,
            gateway_ssl_key_path: None,
            gateway_ssl_port: 8443,
            gateway_force_ssl: false,
            always_on_top: false,
            tray_enabled: true,
            global_shortcuts_enabled: true,
            shortcut_registration_logs_enabled: false,
            shortcut_trigger_toast_enabled: false,
            notifications_enabled: true,
            mini_window_enabled: false,
            start_minimized: false,
            close_to_tray: true,
            notify_backup: true,
            notify_import: true,
            notify_errors: true,
            backup_dir: None,
            auto_backup_enabled: false,
            auto_backup_interval_hours: 24,
            auto_backup_max_count: 10,
            webdav_host: None,
            webdav_username: None,
            webdav_password: None,
            webdav_path: None,
            webdav_accept_invalid_certs: false,
            webdav_sync_enabled: false,
            webdav_sync_interval_minutes: 60,
            webdav_max_remote_backups: 10,
            webdav_include_documents: false,
            s3_endpoint: None,
            s3_region: None,
            s3_bucket: None,
            s3_access_key_id: None,
            s3_secret_access_key: None,
            s3_root: None,
            s3_provider_preset: Some(serde_json::json!("Custom")),
            s3_use_path_style: false,
            s3_sync_enabled: false,
            s3_sync_interval_minutes: 60,
            s3_max_remote_backups: 10,
            s3_include_documents: false,
            cloud_sync_enabled: false,
            cloud_backend: None,
            sync_profile_name: None,
            closed_loop_enabled: true,
            closed_loop_interval_minutes: 5,
            last_selected_conversation_id: None,
            documents_root_override: None,
            update_check_interval: 60,
            default_system_prompt: None,
            chat_minimap_enabled: false,
            chat_minimap_style: "faq".to_string(),
            multi_model_display_mode: "tabs".to_string(),
            render_user_markdown: false,
            default_workspace_dir: None,
            screen_perception_enabled: false,
            rl_optimizer_enabled: false,
            lora_finetune_enabled: false,
            proactive_nudge_enabled: true,
            thought_chain_enabled: false,
            error_recovery_enabled: true,
            workspace_uri: None,
            rag_pipeline_config: serde_json::Value::Null,
            agent_panel_enabled: true,
            agent_panel_compact: false,
            onboarding_completed: false,
            onboarding_wizard_dismissed: false,
            onboarding_tutorial_completed: false,
            onboarding_selected_preset: None,
        }
    }
}

// === Chat Streaming Types ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_max_completion_tokens: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_param_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTool {
    pub r#type: String,
    pub function: ChatToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// A single tool call requested by the AI model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Provider-assigned ID (e.g., "call_abc123")
    pub id: String,
    /// Always "function" for now
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    /// JSON-encoded arguments string
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: ChatContent,
    /// For assistant messages: tool calls the model wants to make
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// For tool-result messages: the ID of the tool call this responds to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatContent {
    Text(String),
    Multipart(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    pub r#type: String,
    pub text: Option<String>,
    pub image_url: Option<ImageUrl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub content: String,
    pub thinking: Option<String>,
    pub usage: TokenUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

// TokenUsage 权威定义在 `crate::conversation_model::TokenUsage`，此处仅 re-export
// 以保持 `axagent_harness::types::TokenUsage` 路径向后兼容。
pub use crate::conversation_model::TokenUsage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamChunk {
    pub content: Option<String>,
    pub thinking: Option<String>,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_final: Option<bool>,
    pub usage: Option<TokenUsage>,
    /// Tool calls requested by the model (populated on the final content chunk or a dedicated chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamEvent {
    pub conversation_id: String,
    pub message_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    pub chunk: ChatStreamChunk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStreamErrorEvent {
    pub conversation_id: String,
    pub message_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTitleUpdatedEvent {
    pub conversation_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTitleGeneratingEvent {
    pub conversation_id: String,
    pub generating: bool,
    /// Error message if generation failed
    pub error: Option<String>,
}

// === RAG Context Events ===
