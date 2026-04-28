use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnifiedConfig {
    pub agent: AgentSettings,
    pub database: DatabaseSettings,
    pub cache: CacheSettings,
    pub security: SecuritySettings,
    pub logging: LoggingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSettings {
    pub max_iterations: usize,
    pub max_concurrent_agents: usize,
    pub default_timeout_secs: u64,
    pub enable_self_verification: bool,
    pub enable_error_recovery: bool,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            max_concurrent_agents: 5,
            default_timeout_secs: 300,
            enable_self_verification: false,
            enable_error_recovery: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub max_lifetime_secs: u64,
    pub idle_timeout_secs: u64,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_secs: 30,
            max_lifetime_secs: 3600,
            idle_timeout_secs: 600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    pub vector_cache_max_entries: usize,
    pub vector_cache_ttl_secs: u64,
    pub tool_result_cache_max_entries: usize,
    pub tool_result_cache_ttl_secs: u64,
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            vector_cache_max_entries: 1000,
            vector_cache_ttl_secs: 300,
            tool_result_cache_max_entries: 500,
            tool_result_cache_ttl_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub tool_whitelist_enabled: bool,
    pub command_injection_protection: bool,
    pub strict_mode: bool,
    pub max_command_length: usize,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            tool_whitelist_enabled: true,
            command_injection_protection: true,
            strict_mode: false,
            max_command_length: 10000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    pub level: String,
    pub enable_sql_logging: bool,
    pub enable_performance_metrics: bool,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            enable_sql_logging: false,
            enable_performance_metrics: true,
        }
    }
}

pub struct ConfigManager {
    config: Arc<RwLock<UnifiedConfig>>,
    overrides: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(UnifiedConfig::default())),
            overrides: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_config(&self) -> UnifiedConfig {
        self.config.read().await.clone()
    }

    pub async fn update_config(&self, config: UnifiedConfig) {
        *self.config.write().await = config;
    }

    pub async fn get_agent_settings(&self) -> AgentSettings {
        self.config.read().await.agent.clone()
    }

    pub async fn get_database_settings(&self) -> DatabaseSettings {
        self.config.read().await.database.clone()
    }

    pub async fn get_cache_settings(&self) -> CacheSettings {
        self.config.read().await.cache.clone()
    }

    pub async fn get_security_settings(&self) -> SecuritySettings {
        self.config.read().await.security.clone()
    }

    pub async fn set_override(&self, key: String, value: serde_json::Value) {
        self.overrides.write().await.insert(key, value);
    }

    pub async fn get_override(&self, key: &str) -> Option<serde_json::Value> {
        self.overrides.read().await.get(key).cloned()
    }

    pub async fn clear_overrides(&self) {
        self.overrides.write().await.clear();
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ConfigManager {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            overrides: Arc::clone(&self.overrides),
        }
    }
}
