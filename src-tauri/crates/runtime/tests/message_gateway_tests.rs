use axagent_runtime::message_gateway::platform_config::PlatformConfig;

#[test]
fn test_default_platform_config() {
    let config = PlatformConfig::default();
    assert!(!config.telegram_enabled);
    assert!(!config.discord_enabled);
    assert!(!config.api_server_enabled);
    assert!(config.telegram_bot_token.is_none());
    assert!(config.discord_bot_token.is_none());
    assert_eq!(config.max_history_per_session, 100);
}

#[test]
fn test_validate_telegram_requires_token() {
    let mut config = PlatformConfig::default();
    config.telegram_enabled = true;
    let result = config.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Telegram bot token"));
}

#[test]
fn test_validate_telegram_with_token() {
    let mut config = PlatformConfig::default();
    config.telegram_enabled = true;
    config.telegram_bot_token = Some("test_token".to_string());
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_validate_discord_requires_token() {
    let mut config = PlatformConfig::default();
    config.discord_enabled = true;
    let result = config.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Discord bot token"));
}

#[test]
fn test_validate_discord_with_token() {
    let mut config = PlatformConfig::default();
    config.discord_enabled = true;
    config.discord_bot_token = Some("dc_token".to_string());
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_validate_api_server_port_zero() {
    let mut config = PlatformConfig::default();
    config.api_server_enabled = true;
    config.api_server_port = Some(0);
    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("non-zero"));
}

#[test]
fn test_validate_api_server_valid_port() {
    let mut config = PlatformConfig::default();
    config.api_server_enabled = true;
    config.api_server_port = Some(8080);
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_serialize_deserialize_config() {
    let config = PlatformConfig {
        telegram_enabled: true,
        telegram_bot_token: Some("test_bot_token".to_string()),
        telegram_webhook_url: Some("https://example.com/webhook".to_string()),
        telegram_webhook_secret: Some("secret123".to_string()),
        telegram_allowed_users: Some(vec![123456, 789012]),
        discord_enabled: false,
        discord_bot_token: None,
        discord_webhook_url: None,
        discord_allowed_channels: None,
        slack_enabled: false,
        slack_bot_token: None,
        slack_signing_secret: None,
        slack_workspace_id: None,
        slack_app_token: None,
        whatsapp_enabled: false,
        whatsapp_phone_number_id: None,
        whatsapp_access_token: None,
        whatsapp_business_account_id: None,
        api_server_enabled: true,
        api_server_port: Some(9090),
        auto_sync_messages: false,
        max_history_per_session: 50,
        wechat_enabled: false,
        wechat_app_id: None,
        wechat_app_secret: None,
        wechat_token: None,
        wechat_encoding_aes_key: None,
        wechat_original_id: None,
        feishu_enabled: false,
        feishu_app_id: None,
        feishu_app_secret: None,
        feishu_verification_token: None,
        feishu_encrypt_key: None,
        qq_enabled: false,
        qq_bot_app_id: None,
        qq_bot_token: None,
        qq_bot_secret: None,
        dingtalk_enabled: false,
        dingtalk_app_key: None,
        dingtalk_app_secret: None,
        dingtalk_robot_code: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: PlatformConfig = serde_json::from_str(&json).unwrap();

    assert!(deserialized.telegram_enabled);
    assert_eq!(
        deserialized.telegram_bot_token,
        Some("test_bot_token".to_string())
    );
    assert!(deserialized.api_server_enabled);
    assert_eq!(deserialized.api_server_port, Some(9090));
    assert_eq!(deserialized.max_history_per_session, 50);
}

#[test]
fn test_allowed_users_filtering() {
    let config = PlatformConfig {
        telegram_allowed_users: Some(vec![111, 222]),
        ..Default::default()
    };

    assert!(config
        .telegram_allowed_users
        .as_ref()
        .unwrap()
        .contains(&111));
    assert!(config
        .telegram_allowed_users
        .as_ref()
        .unwrap()
        .contains(&222));
    assert!(!config
        .telegram_allowed_users
        .as_ref()
        .unwrap()
        .contains(&333));
}

#[test]
fn test_validate_slack_requires_app_token() {
    let mut config = PlatformConfig::default();
    config.slack_enabled = true;
    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Slack app token"));
}

#[test]
fn test_validate_slack_with_app_token() {
    let mut config = PlatformConfig::default();
    config.slack_enabled = true;
    config.slack_app_token = Some("xapp-test".to_string());
    let result = config.validate();
    assert!(result.is_ok());
}
