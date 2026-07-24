// SPDX-License-Identifier: AGPL-3.0-only

//! Core data structures for credential management.

use serde::{Deserialize, Serialize};

/// OAuth2 access token 过期时间提前量（毫秒）。
/// 在 `expires_at` 之前 60 秒即视为已过期，避免请求途中 token 失效。
const OAUTH2_EXPIRY_SKEW_MS: i64 = 60_000;

/// Supported credential types for various integrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialType {
    /// API key with custom header name (e.g. X-API-Key)
    ApiKey { key: String, header_name: String },
    /// HTTP Basic Authentication
    BasicAuth { username: String, password: String },
    /// Bearer token (JWT, opaque token, etc.)
    BearerToken { token: String },
    /// OAuth2 client credentials
    ///
    /// `access_token` / `expires_at` 用于缓存已刷新的 token：
    /// - `access_token`: 当前持有的 access token；首次构造时为 None，刷新后填入
    /// - `expires_at`:   access token 过期时间（Unix 毫秒），None 表示尚未刷新或无过期信息
    ///
    /// 完整的 token 刷新逻辑（向 `token_url` 发起 client_credentials 请求）
    /// 暂未实现——当前仅检测过期并返回错误，由调用方决定如何处理。
    ///
    /// 向后兼容：`Option<T>` 字段在反序列化缺失时默认为 `None`，
    /// 旧版本写入的 OAuth2 凭证（无 access_token/expires_at）仍可正常读取。
    OAuth2 {
        client_id: String,
        client_secret: String,
        token_url: String,
        scopes: Vec<String>,
        /// 当前 access token（None 表示尚未授权）
        access_token: Option<String>,
        /// access token 过期时间（Unix 毫秒）
        expires_at: Option<i64>,
    },
    /// Database connection string
    DatabaseConnection { connection_string: String },
    /// SMTP mail server configuration
    Smtp { host: String, port: u16, user: String, pass: String, tls: bool },
}

impl CredentialType {
    /// 判断 OAuth2 access token 是否已过期（或即将过期）。
    ///
    /// - 非 OAuth2 类型返回 `false`
    /// - `expires_at` 为 `None` 视为未过期（无过期信息时不阻断请求）
    /// - 距 `expires_at` 不足 `OAUTH2_EXPIRY_SKEW_MS` 毫秒视为已过期
    pub fn is_oauth2_expired(&self) -> bool {
        match self {
            CredentialType::OAuth2 { expires_at: Some(exp), .. } => {
                let now = chrono::Utc::now().timestamp_millis();
                now >= *exp - OAUTH2_EXPIRY_SKEW_MS
            },
            _ => false,
        }
    }
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialType::ApiKey { .. } => write!(f, "ApiKey"),
            CredentialType::BasicAuth { .. } => write!(f, "BasicAuth"),
            CredentialType::BearerToken { .. } => write!(f, "BearerToken"),
            CredentialType::OAuth2 { .. } => write!(f, "OAuth2"),
            CredentialType::DatabaseConnection { .. } => write!(f, "DatabaseConnection"),
            CredentialType::Smtp { .. } => write!(f, "Smtp"),
        }
    }
}

/// Stored credential with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub id: String,
    pub name: String,
    pub credential_type: CredentialType,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Credential {
    pub fn new(id: String, name: String, credential_type: CredentialType) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self { id, name, credential_type, created_at: now, updated_at: now }
    }
}

/// Lightweight reference to a credential (used in node configs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialRef {
    pub credential_id: String,
}

/// SMTP configuration extracted from a Smtp credential.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub tls: bool,
}
