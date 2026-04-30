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
