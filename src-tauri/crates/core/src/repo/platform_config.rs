use sea_orm::DatabaseConnection;

use crate::platform_config::PlatformConfig;
use crate::repo::settings;

pub async fn get_platform_config(db: &DatabaseConnection) -> PlatformConfig {
    settings::get_setting(db, "platform_config")
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}

pub async fn save_platform_config(
    db: &DatabaseConnection,
    config: &PlatformConfig,
) -> crate::error::Result<()> {
    let json = serde_json::to_string(config)
        .map_err(|e| crate::error::AxAgentError::Internal(e.to_string()))?;
    settings::set_setting(db, "platform_config", &json).await
}

// ── 消息去重游标持久化 ──

/// 保存平台的消息去重游标 (e.g., Telegram last_update_id, Discord sequence)
pub async fn save_platform_cursor(
    db: &DatabaseConnection,
    platform: &str,
    cursor: i64,
) -> crate::error::Result<()> {
    let key = format!("platform_cursor_{}", platform);
    settings::set_setting(db, &key, &cursor.to_string()).await
}

/// 获取平台的消息去重游标
pub async fn get_platform_cursor(
    db: &DatabaseConnection,
    platform: &str,
) -> Option<i64> {
    let key = format!("platform_cursor_{}", platform);
    settings::get_setting(db, &key)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
}

// ── 会话路由持久化 ──

/// 保存会话路由映射表 (platform_userkey -> agent_session_id)
pub async fn save_session_routes(
    db: &DatabaseConnection,
    routes: &std::collections::HashMap<String, String>,
) -> crate::error::Result<()> {
    let json = serde_json::to_string(routes)
        .map_err(|e| crate::error::AxAgentError::Internal(e.to_string()))?;
    settings::set_setting(db, "platform_session_routes", &json).await
}

/// 加载会话路由映射表
pub async fn load_session_routes(
    db: &DatabaseConnection,
) -> std::collections::HashMap<String, String> {
    settings::get_setting(db, "platform_session_routes")
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}
