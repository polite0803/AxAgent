// SPDX-License-Identifier: AGPL-3.0-only

//! Runtime credential manager with in-memory caching.
//!
//! Provides on-demand credential loading with a transparent cache to avoid
//! repeated disk reads and decrypt operations during workflow execution.

use std::collections::HashMap;

use crate::core_error::{AxAgentError, Result};

use super::store::CredentialStore;
use super::{Credential, CredentialType};

/// SMTP configuration extracted from a Smtp credential.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub tls: bool,
}

/// Runtime credential manager with lazy-loading and caching.
///
/// Thread-safe: wraps interior state behind a `std::sync::Mutex` so it can be
/// shared via `Arc<CredentialManager>` across executor threads.
pub struct CredentialManager {
    store: CredentialStore,
    cache: std::sync::Mutex<HashMap<String, Credential>>,
}

impl CredentialManager {
    /// Create a new credential manager backed by the given store.
    pub fn new(store: CredentialStore) -> Self {
        Self { store, cache: std::sync::Mutex::new(HashMap::new()) }
    }

    /// Get a credential by ID, loading from disk and caching on first access.
    pub fn get_credential(&self, id: &str) -> Result<Credential> {
        {
            let cache = self
                .cache
                .lock()
                .map_err(|e| AxAgentError::Internal(format!("credential cache lock: {e}")))?;
            if let Some(cached) = cache.get(id) {
                return Ok(cached.clone());
            }
        }
        let cred = self.store.load_credential(id)?;
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|e| AxAgentError::Internal(format!("credential cache lock: {e}")))?;
            cache.insert(id.to_string(), cred.clone());
        }
        Ok(cred)
    }

    /// Clear the in-memory cache (useful after credential updates).
    pub fn invalidate(&self, id: &str) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.remove(id);
        }
    }

    /// Clear all cached credentials.
    pub fn invalidate_all(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }

    /// List all stored credentials (metadata only).
    pub fn list_credentials(&self) -> Result<Vec<super::store::CredentialMeta>> {
        self.store.list_credentials()
    }

    /// Save a new or updated credential to the store.
    pub fn save_credential(&self, credential: &Credential) -> Result<()> {
        self.store.save_credential(credential)?;
        self.invalidate(&credential.id);
        Ok(())
    }

    /// Delete a credential from the store.
    pub fn delete_credential(&self, id: &str) -> Result<()> {
        self.store.delete_credential(id)?;
        self.invalidate(id);
        Ok(())
    }

    /// Inject credential-based authentication into an HTTP request builder.
    ///
    /// Modifies headers or auth based on the credential type.
    pub fn inject_into_http_request(
        &self,
        credential: &Credential,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder> {
        match &credential.credential_type {
            CredentialType::ApiKey { key, header_name } => Ok(request.header(header_name, key)),
            CredentialType::BasicAuth { username, password } => {
                Ok(request.basic_auth(username, Some(password)))
            },
            CredentialType::BearerToken { token } => Ok(request.bearer_auth(token)),
            CredentialType::OAuth2 { client_id: _, client_secret: _, token_url: _, scopes: _ } => {
                Err(AxAgentError::Internal(
                    "OAuth2 credential injection not yet implemented".to_string(),
                ))
            },
            _ => Ok(request),
        }
    }

    /// Inject credential-based headers into a `reqwest::Client`'s request.
    /// Returns a map of headers to add when building the request manually.
    pub fn get_auth_headers(&self, credential: &Credential) -> Result<Vec<(String, String)>> {
        match &credential.credential_type {
            CredentialType::ApiKey { key, header_name } => {
                Ok(vec![(header_name.clone(), key.clone())])
            },
            CredentialType::BearerToken { token } => {
                Ok(vec![("Authorization".to_string(), format!("Bearer {token}"))])
            },
            CredentialType::BasicAuth { username, password } => {
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    format!("{username}:{password}"),
                );
                Ok(vec![("Authorization".to_string(), format!("Basic {encoded}"))])
            },
            _ => Ok(vec![]),
        }
    }

    /// Extract a database connection string from a credential.
    pub fn get_database_connection_string(&self, credential_id: &str) -> Result<String> {
        let cred = self.get_credential(credential_id)?;
        match &cred.credential_type {
            CredentialType::DatabaseConnection { connection_string } => {
                Ok(connection_string.clone())
            },
            other => Err(AxAgentError::Validation(format!(
                "credential {credential_id} is {other}, not DatabaseConnection"
            ))),
        }
    }

    /// Extract SMTP configuration from a credential.
    pub fn get_smtp_config(&self, credential_id: &str) -> Result<SmtpConfig> {
        let cred = self.get_credential(credential_id)?;
        match &cred.credential_type {
            CredentialType::Smtp { host, port, user, pass, tls } => Ok(SmtpConfig {
                host: host.clone(),
                port: *port,
                user: user.clone(),
                pass: pass.clone(),
                tls: *tls,
            }),
            other => Err(AxAgentError::Validation(format!(
                "credential {credential_id} is {other}, not Smtp"
            ))),
        }
    }
}
