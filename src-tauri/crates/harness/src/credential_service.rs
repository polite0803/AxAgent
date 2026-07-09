// SPDX-License-Identifier: AGPL-3.0-only

//! Credential service trait — abstract credential management contract.
//!
//! Consumers depend on this trait rather than on the concrete `CredentialManager`,
//! enabling test stubs and DI injection.

/// Simplified SMTP configuration used by the trait interface.
#[derive(Debug, Clone)]
pub struct SmtpServiceConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub tls: bool,
}

/// Abstract credential management service.
///
/// Implementations handle encrypted storage, caching, and credential type
/// dispatch (API key / Basic / Bearer / DB connection / SMTP).
#[async_trait::async_trait]
pub trait CredentialService: Send + Sync + std::fmt::Debug {
    /// Resolve a database connection string from a credential.
    async fn get_database_connection_string(&self, credential_id: &str)
        -> Result<String, String>;

    /// Resolve SMTP configuration from a credential.
    async fn get_smtp_config(&self, credential_id: &str)
        -> Result<SmtpServiceConfig, String>;

    /// Get authentication headers for a credential (suitable for HTTP requests).
    async fn get_auth_headers(&self, credential_id: &str)
        -> Result<Vec<(String, String)>, String>;
}

/// Shared credential service reference (type-erased).
pub type SharedCredentialService = std::sync::Arc<dyn CredentialService>;
