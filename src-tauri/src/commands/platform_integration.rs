use std::sync::Arc;

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::platform as platform_err;
use axagent_core::platform_config::PlatformConfig;
use axagent_runtime::message_gateway::platform_manager::{
    PlatformAdapterStatus, PlatformReconcileReport,
};
use serde::Serialize;
use tauri::State;

const VALID_PLATFORMS: &[&str] = &["discord", "telegram", "slack", "webhook"];

// ── IPC 返回类型（替代旧 axagent_trajectory 中的类型） ──

#[derive(Debug, Clone, Serialize)]
pub struct OutgoingMessage {
    pub platform: String,
    pub chat_id: String,
    pub content: String,
    pub parse_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatformSession {
    pub session_id: String,
    pub platform: String,
    pub user_id: String,
    pub username: Option<String>,
    pub is_active: bool,
    pub last_activity: i64,
}

// ── 配置命令 ──

#[tauri::command]
pub async fn get_platform_config(state: State<'_, AppState>) -> Result<PlatformConfig, String> {
    Ok(axagent_core::repo::platform_config::get_platform_config(&state.sea_db).await)
}

#[tauri::command]
pub async fn update_platform_config(
    state: State<'_, AppState>,
    config: PlatformConfig,
) -> Result<PlatformReconcileReport, String> {
    axagent_core::repo::platform_config::save_platform_config(&state.sea_db, &config)
        .await
        .map_err(|e| e.to_string())?;
    state
        .platform_manager
        .reconcile(&config)
        .await
        .map_err(|e| e.to_string())
}

// ── 消息处理命令 ──

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn process_telegram_message(
    state: State<'_, AppState>,
    _message_id: i64,
    chat_id: i64,
    text: String,
    from_user_id: Option<i64>,
    username: Option<String>,
    _timestamp: i64,
) -> Result<Option<OutgoingMessage>, String> {
    let user_id = from_user_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| chat_id.to_string());
    let reply = state
        .platform_bridge
        .route_incoming_message(
            "telegram",
            &user_id,
            username.as_deref(),
            &chat_id.to_string(),
            &text,
        )
        .await
        .map_err(|e| format!("Telegram message processing failed: {}", e))?;
    Ok(reply.map(|content| OutgoingMessage {
        platform: "telegram".to_string(),
        chat_id: chat_id.to_string(),
        content,
        parse_mode: Some("Markdown".to_string()),
    }))
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn process_discord_message(
    state: State<'_, AppState>,
    _message_id: String,
    channel_id: String,
    _guild_id: Option<String>,
    content: String,
    author_id: String,
    author_username: String,
    _timestamp: String,
) -> Result<Option<OutgoingMessage>, String> {
    let reply = state
        .platform_bridge
        .route_incoming_message(
            "discord",
            &author_id,
            Some(&author_username),
            &channel_id,
            &content,
        )
        .await
        .map_err(|e| format!("Discord message processing failed: {}", e))?;
    Ok(reply.map(|text| OutgoingMessage {
        platform: "discord".to_string(),
        chat_id: channel_id,
        content: text,
        parse_mode: None,
    }))
}

#[tauri::command]
pub async fn process_platform_message(
    platform: String,
    payload: serde_json::Value,
) -> Result<Option<OutgoingMessage>, String> {
    if !VALID_PLATFORMS.contains(&platform.as_str()) {
        return Err(format!("Unsupported platform: {}", platform));
    }
    tracing::info!("process_platform_message: platform={}, payload={}", platform, payload);
    Ok(None)
}

// ── 发送命令 ──

#[tauri::command]
pub async fn send_platform_message(
    state: State<'_, AppState>,
    platform: String,
    chat_id: String,
    text: String,
    parse_mode: Option<String>,
) -> Result<(), String> {
    if !VALID_PLATFORMS.contains(&platform.as_str()) {
        return Err(format!("Unsupported platform: {}", platform));
    }
    let config = axagent_core::repo::platform_config::get_platform_config(&state.sea_db).await;

    let adapter = state
        .platform_manager
        .get_adapter(&platform)
        .await
        .ok_or_else(|| format!("Platform adapter not found: {}", platform))?;

    adapter
        .send_message(&config, &chat_id, &text, parse_mode.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_telegram_message(
    state: State<'_, AppState>,
    chat_id: i64,
    text: String,
) -> Result<(), String> {
    let config = axagent_core::repo::platform_config::get_platform_config(&state.sea_db).await;

    if !config.telegram_enabled {
        return Err(ErrorResponse::new(platform_err::TELEGRAM_NOT_ENABLED));
    }

    let adapter = state
        .platform_manager
        .get_adapter("telegram")
        .await
        .ok_or_else(|| "Telegram adapter not available".to_string())?;

    adapter
        .send_message(&config, &chat_id.to_string(), &text, Some("Markdown"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_discord_message(
    state: State<'_, AppState>,
    content: String,
) -> Result<(), String> {
    let config = axagent_core::repo::platform_config::get_platform_config(&state.sea_db).await;

    if !config.discord_enabled {
        return Err(ErrorResponse::new(platform_err::DISCORD_NOT_ENABLED));
    }

    let webhook_url = config
        .discord_webhook_url
        .ok_or_else(|| "Discord webhook URL not configured".to_string())?;

    let client = reqwest::Client::new();
    let body = serde_json::json!({ "content": content });
    client
        .post(&webhook_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ── 会话管理命令 ──

#[tauri::command]
pub async fn create_platform_session(
    state: State<'_, AppState>,
    platform: String,
    user_id: String,
    username: Option<String>,
) -> Result<PlatformSession, String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();

    // 持久化会话路由
    let mut routes = axagent_core::repo::platform_config::load_session_routes(&state.sea_db).await;
    let key = format!("{}_{}", platform, user_id);
    routes.insert(key.clone(), session_id.clone());
    axagent_core::repo::platform_config::save_session_routes(&state.sea_db, &routes)
        .await
        .map_err(|e| e.to_string())?;

    Ok(PlatformSession {
        session_id,
        platform,
        user_id,
        username,
        is_active: true,
        last_activity: now,
    })
}

#[tauri::command]
pub async fn get_active_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<PlatformSession>, String> {
    let routes = axagent_core::repo::platform_config::load_session_routes(&state.sea_db).await;
    let now = chrono::Utc::now().timestamp_millis();

    let sessions: Vec<PlatformSession> = routes
        .into_iter()
        .map(|(key, session_id)| {
            let mut parts = key.splitn(2, '_');
            let platform = parts.next().unwrap_or("").to_string();
            let user_id = parts.next().unwrap_or("").to_string();
            PlatformSession {
                session_id,
                platform,
                user_id,
                username: None,
                is_active: true,
                last_activity: now,
            }
        })
        .collect();

    Ok(sessions)
}

#[tauri::command]
pub async fn deactivate_platform_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    // 从路由表中移除会话
    let mut routes = axagent_core::repo::platform_config::load_session_routes(&state.sea_db).await;
    routes.retain(|_, v| v != &session_id);
    axagent_core::repo::platform_config::save_session_routes(&state.sea_db, &routes)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── 状态与协调命令 ──

#[tauri::command]
pub async fn get_platform_statuses(
    state: State<'_, AppState>,
) -> Result<Vec<PlatformAdapterStatus>, String> {
    let config = axagent_core::repo::platform_config::get_platform_config(&state.sea_db).await;
    Ok(state.platform_manager.get_statuses(&config).await)
}

#[tauri::command]
pub async fn reconcile_platforms(
    state: State<'_, AppState>,
) -> Result<PlatformReconcileReport, String> {
    let config = axagent_core::repo::platform_config::get_platform_config(&state.sea_db).await;
    state
        .platform_manager
        .reconcile(&config)
        .await
        .map_err(|e| e.to_string())
}

// ── API Server 命令 ──

#[tauri::command]
pub async fn start_api_server(state: State<'_, AppState>) -> Result<(), String> {
    let config = axagent_core::repo::platform_config::get_platform_config(&state.sea_db).await;

    if !config.api_server_enabled {
        return Err(ErrorResponse::new(platform_err::API_SERVER_NOT_ENABLED));
    }

    let port = config.api_server_port.unwrap_or(8080);
    let config_arc = Arc::new(tokio::sync::RwLock::new(config));
    let pm = state.platform_manager.clone();

    let server = axagent_runtime::api_server::ApiServer::new(config_arc, pm);

    // 停止已有的 server（如果存在）
    {
        let mut handle_guard = state.api_server_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
    }

    // 在后台启动 server
    let join_handle = tokio::spawn(async move {
        if let Err(e) = server.start(port).await {
            tracing::error!("API Server error: {}", e);
        }
    });

    {
        let mut handle_guard = state.api_server_handle.lock().await;
        *handle_guard = Some(join_handle);
    }

    tracing::info!("API Server started on port {}", port);
    Ok(())
}

#[tauri::command]
pub async fn stop_api_server(state: State<'_, AppState>) -> Result<(), String> {
    let mut handle_guard = state.api_server_handle.lock().await;
    if let Some(handle) = handle_guard.take() {
        handle.abort();
        tracing::info!("API Server stopped");
        Ok(())
    } else {
        Err("API Server is not running".to_string())
    }
}
