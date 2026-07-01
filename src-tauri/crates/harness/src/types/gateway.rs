// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayCertResult {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStatus {
    pub is_running: bool,
    pub listen_address: String,
    pub port: u16,
    pub ssl_enabled: bool,
    pub started_at: Option<i64>,
    /// HTTPS listener port; `None` when SSL is disabled or not yet started.
    pub https_port: Option<u16>,
    /// When `true` the gateway redirects all HTTP traffic to HTTPS.
    pub force_ssl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub enabled: bool,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub has_encrypted_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGatewayKeyResult {
    pub gateway_key: GatewayKey,
    pub plain_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMetrics {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_request_tokens: u64,
    pub total_response_tokens: u64,
    pub active_connections: u32,
    pub today_requests: u64,
    pub today_tokens: u64,
    pub today_request_tokens: u64,
    pub today_response_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageByKey {
    pub key_id: String,
    pub key_name: String,
    pub request_count: u64,
    pub token_count: u64,
    pub request_tokens: u64,
    pub response_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageByProvider {
    pub provider_id: String,
    pub provider_name: String,
    pub request_count: u64,
    pub token_count: u64,
    pub request_tokens: u64,
    pub response_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageByDay {
    pub date: String,
    pub request_count: u64,
    pub token_count: u64,
    pub request_tokens: u64,
    pub response_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedProgram {
    pub key_id: String,
    pub key_name: String,
    pub key_prefix: String,
    pub today_requests: u64,
    pub today_tokens: u64,
    pub today_request_tokens: u64,
    pub today_response_tokens: u64,
    pub last_active_at: Option<i64>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStats {
    pub total_requests: u64,
    pub active_connections: u32,
    pub uptime_seconds: u64,
    pub requests_per_minute: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySettings {
    pub listen_address: String,
    pub port: u16,
    pub load_balance_strategy: LoadBalanceStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalanceStrategy {
    RoundRobin,
}

// === Gateway Link (Client-side Gateway Connection) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayLink {
    pub id: String,
    pub name: String,
    pub link_type: String,
    pub endpoint: String,
    pub api_key_id: Option<String>,
    pub enabled: bool,
    pub status: String,
    pub error_message: Option<String>,
    pub auto_sync_models: bool,
    pub auto_sync_skills: bool,
    pub last_sync_at: Option<i64>,
    pub latency_ms: Option<i64>,
    pub version: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGatewayLinkInput {
    pub name: String,
    pub link_type: String,
    pub endpoint: String,
    pub api_key_id: Option<String>,
    /// Optional plain-text API key. When provided, a gateway key will be
    /// created automatically and its id stored in api_key_id.
    pub api_key: Option<String>,
    pub auto_sync_models: Option<bool>,
    pub auto_sync_skills: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayLinkModelSync {
    pub model_id: String,
    pub provider_name: String,
    pub sync_status: String,
    pub last_sync_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayLinkSkillSync {
    pub skill_name: String,
    pub skill_version: Option<String>,
    pub sync_status: String,
    pub last_sync_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayLinkPolicy {
    pub id: String,
    pub link_id: String,
    pub route_strategy: String,
    pub model_fallback_enabled: bool,
    pub global_rpm: Option<i64>,
    pub per_model_rpm: Option<i64>,
    pub token_limit_per_minute: Option<i64>,
    pub key_rotation_strategy: String,
    pub key_failover_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGatewayLinkPolicyInput {
    pub route_strategy: Option<String>,
    pub model_fallback_enabled: Option<bool>,
    pub global_rpm: Option<Option<i64>>,
    pub per_model_rpm: Option<Option<i64>>,
    pub token_limit_per_minute: Option<Option<i64>>,
    pub key_rotation_strategy: Option<String>,
    pub key_failover_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayLinkActivity {
    pub id: String,
    pub link_id: String,
    pub activity_type: String,
    pub description: Option<String>,
    pub created_at: i64,
}

// === Settings ===
