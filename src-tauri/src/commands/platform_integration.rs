use crate::AppState;
use axagent_trajectory::{
    DiscordMessage, MessagePlatform, OutgoingMessage, PlatformConfig, PlatformMessage,
    PlatformSession, TelegramMessage,
};
use tauri::State;

#[tauri::command]
pub async fn get_platform_config(
    state: State<'_, AppState>,
) -> Result<PlatformConfig, String> {
    let service = state.platform_integration_service.read().await;
    Ok(service.get_config().await)
}

#[tauri::command]
pub async fn update_platform_config(
    state: State<'_, AppState>,
    config: PlatformConfig,
) -> Result<(), String> {
    let service = state.platform_integration_service.write().await;
    service.update_config(config).await;
    Ok(())
}

#[tauri::command]
pub async fn process_telegram_message(
    state: State<'_, AppState>,
    message_id: i64,
    chat_id: i64,
    text: String,
    from_user_id: Option<i64>,
    username: Option<String>,
    timestamp: i64,
) -> Result<Option<OutgoingMessage>, String> {
    let tg_message = TelegramMessage {
        message_id,
        chat_id,
        text,
        from_user_id,
        username,
        timestamp,
    };
    let platform_message = PlatformMessage::Telegram(tg_message);
    let service = state.platform_integration_service.read().await;
    Ok(service.process_message(platform_message).await)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn process_discord_message(
    state: State<'_, AppState>,
    message_id: String,
    channel_id: String,
    guild_id: Option<String>,
    content: String,
    author_id: String,
    author_username: String,
    timestamp: String,
) -> Result<Option<OutgoingMessage>, String> {
    let dc_message = DiscordMessage {
        message_id,
        channel_id,
        guild_id,
        content,
        author_id,
        author_username,
        timestamp,
    };
    let platform_message = PlatformMessage::Discord(dc_message);
    let service = state.platform_integration_service.read().await;
    Ok(service.process_message(platform_message).await)
}

#[tauri::command]
pub async fn create_platform_session(
    state: State<'_, AppState>,
    platform: MessagePlatform,
    user_id: String,
    username: Option<String>,
) -> Result<String, String> {
    let service = state.platform_integration_service.read().await;
    Ok(service.create_session(platform, user_id, username).await)
}

#[tauri::command]
pub async fn get_active_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<PlatformSession>, String> {
    let service = state.platform_integration_service.read().await;
    Ok(service.get_active_sessions().await)
}

#[tauri::command]
pub async fn deactivate_platform_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let service = state.platform_integration_service.write().await;
    service.deactivate_session(&session_id).await;
    Ok(())
}

#[tauri::command]
pub async fn send_telegram_message(
    state: State<'_, AppState>,
    chat_id: i64,
    text: String,
) -> Result<(), String> {
    let cfg = {
        let guard = state.platform_integration_service.read().await;
        guard.get_config().await
    };

    if !cfg.telegram_enabled {
        return Err("Telegram integration is not enabled".to_string());
    }

    let bot_token = cfg.telegram_bot_token.ok_or_else(|| "Telegram bot token not configured".to_string())?;
    let handler = axagent_trajectory::TelegramHandler::new(bot_token);
    handler.send_message(chat_id, &text).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_discord_message(
    state: State<'_, AppState>,
    content: String,
) -> Result<(), String> {
    let cfg = {
        let guard = state.platform_integration_service.read().await;
        guard.get_config().await
    };

    if !cfg.discord_enabled {
        return Err("Discord integration is not enabled".to_string());
    }

    let webhook_url = cfg.discord_webhook_url.ok_or_else(|| "Discord webhook URL not configured".to_string())?;

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
