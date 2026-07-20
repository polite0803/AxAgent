// SPDX-License-Identifier: AGPL-3.0-only

//! `axagent_harness::platform_adapter::*` trait 的默认实现。
//!
//! 4 个 repo trait（ProviderRepository / SettingsRepository /
//! GatewayKeyRepository / GatewayRequestLogRepository）的 default impl。
//! CryptoService 在 axagent-crypto crate 里实现。

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use axagent_harness::core_error::Result;
use axagent_harness::mcp_service::{McpServerConfig, McpServerStore};
use axagent_harness::platform_adapter::{
    GatewayKeyRepository, GatewayRequestLogRepository, ProviderRepository, SettingsRepository,
};
use axagent_harness::types::{AppSettings, GatewayKey, ProviderConfig, ProviderKey};

use crate::repo;

// ── ProviderRepository ──

pub struct DefaultProviderRepository {
    pub db: DatabaseConnection,
}

#[async_trait]
impl ProviderRepository for DefaultProviderRepository {
    async fn list_providers(&self) -> Result<Vec<ProviderConfig>> {
        repo::provider::list_providers(&self.db).await
    }

    async fn get_active_key(&self, provider_id: &str) -> Result<ProviderKey> {
        repo::provider::get_active_key(&self.db, provider_id).await
    }

    async fn report_key_failure(&self, key_id: &str, error_msg: &str) -> Result<()> {
        repo::provider::report_key_failure(&self.db, key_id, error_msg).await
    }
}

// ── SettingsRepository ──

pub struct DefaultSettingsRepository {
    pub db: DatabaseConnection,
}

#[async_trait]
impl SettingsRepository for DefaultSettingsRepository {
    async fn get_settings(&self) -> Result<AppSettings> {
        repo::settings::get_settings(&self.db).await
    }
}

// ── GatewayKeyRepository ──

pub struct DefaultGatewayKeyRepository {
    pub db: DatabaseConnection,
    pub master_key: [u8; 32],
    pub crypto: std::sync::Arc<dyn axagent_harness::platform_adapter::CryptoService>,
}

#[async_trait]
impl GatewayKeyRepository for DefaultGatewayKeyRepository {
    async fn list_gateway_keys(&self) -> Result<Vec<GatewayKey>> {
        repo::gateway::list_gateway_keys(&self.db).await
    }

    async fn verify_key(&self, token: &str) -> Result<Option<GatewayKey>> {
        // 原 free fn 返回 Result<GatewayKey>（用 Err 表示"找不到"），这里转成 Option。
        match repo::gateway::verify_key(&self.db, token, self.crypto.as_ref(), &self.master_key)
            .await
        {
            Ok(k) => Ok(Some(k)),
            Err(_) => Ok(None), // 调用方用 .ok_or_else() 转成 auth error
        }
    }

    async fn get_by_id(&self, key_id: &str) -> Result<Option<GatewayKey>> {
        match repo::gateway::get_by_id(&self.db, key_id).await {
            Ok(k) => Ok(Some(k)),
            Err(_) => Ok(None),
        }
    }

    async fn update_last_used(&self, key_id: &str) -> Result<()> {
        repo::gateway::update_last_used(&self.db, key_id).await
    }

    async fn record_usage(
        &self,
        key_id: &str,
        provider_id: &str,
        model_id: Option<&str>,
        request_tokens: u64,
        response_tokens: u64,
        cached_input_tokens: u64,
    ) -> Result<()> {
        repo::gateway::record_usage(
            &self.db,
            key_id,
            provider_id,
            model_id,
            request_tokens,
            response_tokens,
            cached_input_tokens,
        )
        .await
    }
}

// ── GatewayRequestLogRepository ──

pub struct DefaultGatewayRequestLogRepository {
    pub db: DatabaseConnection,
}

#[async_trait]
impl GatewayRequestLogRepository for DefaultGatewayRequestLogRepository {
    async fn record_request_log(
        &self,
        key_id: &str,
        key_name: &str,
        method: &str,
        path: &str,
        model_id: Option<&str>,
        provider_id: Option<&str>,
        status_code: i32,
        duration_ms: i64,
        request_tokens: i64,
        response_tokens: i64,
        error_message: Option<&str>,
    ) -> Result<()> {
        repo::gateway_request_log::record_request_log(
            &self.db,
            key_id,
            key_name,
            method,
            path,
            model_id,
            provider_id,
            status_code,
            duration_ms,
            request_tokens,
            response_tokens,
            error_message,
        )
        .await
    }
}

// ── PlatformAdapter facade 装配（wiring 层用） ──

/// 把 5 个子 trait 装成一个 PlatformAdapter。
/// CryptoService 由 wiring 层另外构造（来自 axagent-crypto）后传入。
pub fn build_platform_adapter(
    db: DatabaseConnection,
    master_key: [u8; 32],
    crypto: std::sync::Arc<dyn axagent_harness::platform_adapter::CryptoService>,
) -> std::sync::Arc<dyn axagent_harness::platform_adapter::PlatformAdapter> {
    use axagent_harness::platform_adapter::PlatformAdapter;

    struct PlatformAdapterImpl {
        providers: std::sync::Arc<dyn ProviderRepository>,
        settings: std::sync::Arc<dyn SettingsRepository>,
        gateway_keys: std::sync::Arc<dyn GatewayKeyRepository>,
        request_log: std::sync::Arc<dyn GatewayRequestLogRepository>,
        crypto: std::sync::Arc<dyn axagent_harness::platform_adapter::CryptoService>,
    }
    impl PlatformAdapter for PlatformAdapterImpl {
        fn providers(&self) -> &dyn ProviderRepository {
            self.providers.as_ref()
        }
        fn settings(&self) -> &dyn SettingsRepository {
            self.settings.as_ref()
        }
        fn gateway_keys(&self) -> &dyn GatewayKeyRepository {
            self.gateway_keys.as_ref()
        }
        fn request_log(&self) -> &dyn GatewayRequestLogRepository {
            self.request_log.as_ref()
        }
        fn crypto(&self) -> &dyn axagent_harness::platform_adapter::CryptoService {
            self.crypto.as_ref()
        }
    }

    std::sync::Arc::new(PlatformAdapterImpl {
        providers: std::sync::Arc::new(DefaultProviderRepository { db: db.clone() }),
        settings: std::sync::Arc::new(DefaultSettingsRepository { db: db.clone() }),
        gateway_keys: std::sync::Arc::new(DefaultGatewayKeyRepository {
            db: db.clone(),
            master_key,
            crypto: crypto.clone(),
        }),
        request_log: std::sync::Arc::new(DefaultGatewayRequestLogRepository { db }),
        crypto,
    })
}

// ── McpServerStore ──

/// `McpServerStore` 的 DAO 实现：从 `mcp_servers` 表读取启用的 MCP server。
/// 让 gateway 不再直接依赖 `axagent-entities` 与 SeaORM。
pub struct DefaultMcpServerStore {
    pub db: DatabaseConnection,
}

/// 把 SeaORM `mcp_servers::Model` 映射为 harness `McpServerConfig`。
/// args_json 解析为字符串数组、env_json 解析为字符串映射；解析失败时降级为 None。
fn model_to_config(m: axagent_entities::mcp_servers::Model) -> McpServerConfig {
    let args: Option<Vec<String>> = m.args_json.as_ref().and_then(|j| serde_json::from_str(j).ok());
    let env: Option<std::collections::HashMap<String, String>> =
        m.env_json.as_ref().and_then(|j| serde_json::from_str(j).ok());
    McpServerConfig {
        id: m.id,
        name: m.name,
        transport: m.transport,
        command: m.command,
        args,
        env,
        endpoint: m.endpoint,
    }
}

#[async_trait]
impl McpServerStore for DefaultMcpServerStore {
    async fn list_enabled(&self) -> std::result::Result<Vec<McpServerConfig>, String> {
        use axagent_entities::mcp_servers;
        let rows = mcp_servers::Entity::find()
            .filter(mcp_servers::Column::Enabled.eq(1))
            .order_by_asc(mcp_servers::Column::Name)
            .all(&self.db)
            .await
            .map_err(|e| format!("Failed to query MCP servers: {e}"))?;
        Ok(rows.into_iter().map(model_to_config).collect())
    }

    async fn get_by_id(&self, id: &str) -> std::result::Result<Option<McpServerConfig>, String> {
        use axagent_entities::mcp_servers;
        let row = mcp_servers::Entity::find_by_id(id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| format!("Failed to query MCP server: {e}"))?;
        Ok(row.map(model_to_config))
    }
}

/// 构造 `McpServerStore`，供 gateway wiring 层注入。
pub fn build_mcp_server_store(db: DatabaseConnection) -> std::sync::Arc<dyn McpServerStore> {
    std::sync::Arc::new(DefaultMcpServerStore { db })
}
