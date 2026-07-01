// SPDX-License-Identifier: AGPL-3.0-only

//! Credential type system
//!
//! Core data structures for credential management: API keys, basic auth,
//! bearer tokens, OAuth2, database connections, and SMTP configuration.

use serde::{Deserialize, Serialize};

pub mod manager;
pub mod store;
pub use manager::CredentialManager;

/// Supported credential types for various integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialType {
    /// API key with custom header name (e.g. X-API-Key)
    ApiKey { key: String, header_name: String },
    /// HTTP Basic Authentication
    BasicAuth { username: String, password: String },
    /// Bearer token (JWT, opaque token, etc.)
    BearerToken { token: String },
    /// OAuth2 client credentials
    OAuth2 {
        client_id: String,
        client_secret: String,
        token_url: String,
        scopes: Vec<String>,
    },
    /// Database connection string
    DatabaseConnection { connection_string: String },
    /// SMTP mail server configuration
    Smtp {
        host: String,
        port: u16,
        user: String,
        pass: String,
        tls: bool,
    },
}

/// A stored credential with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub id: String,
    pub name: String,
    pub credential_type: CredentialType,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Lightweight reference to a credential (used in node configs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialRef {
    pub credential_id: String,
}

impl Credential {
    pub fn new(id: String, name: String, credential_type: CredentialType) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id,
            name,
            credential_type,
            created_at: now,
            updated_at: now,
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
