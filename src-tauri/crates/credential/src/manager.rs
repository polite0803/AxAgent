// SPDX-License-Identifier: AGPL-3.0-only

//! Runtime credential manager with in-memory caching.

use std::collections::HashMap;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

use crate::error::{CredentialError, Result};
use crate::store::CredentialStore;
use crate::types::{Credential, CredentialType, SmtpConfig};

/// Runtime credential manager with lazy-loading and caching.
#[derive(Debug)]
pub struct CredentialManager {
    store: CredentialStore,
    cache: tokio::sync::Mutex<HashMap<String, Credential>>,
}

impl CredentialManager {
    pub fn new(store: CredentialStore) -> Self {
        Self { store, cache: tokio::sync::Mutex::new(HashMap::new()) }
    }

    /// Get a credential by ID, loading from disk and caching on first access.
    pub async fn get_credential(&self, id: &str) -> Result<Credential> {
        {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.get(id) {
                return Ok(cached.clone());
            }
        }
        let cred = self.store.load_credential(id)?;
        {
            let mut cache = self.cache.lock().await;
            cache.insert(id.to_string(), cred.clone());
        }
        Ok(cred)
    }

    /// Clear the in-memory cache.
    pub async fn invalidate(&self, id: &str) {
        self.cache.lock().await.remove(id);
    }

    /// Clear all cached credentials.
    pub async fn invalidate_all(&self) {
        self.cache.lock().await.clear();
    }

    /// List all stored credentials (metadata only).
    pub fn list_credentials(&self) -> Result<Vec<crate::store::CredentialMeta>> {
        self.store.list_credentials()
    }

    /// Save a new or updated credential to the store.
    pub async fn save_credential(&self, credential: &Credential) -> Result<()> {
        self.store.save_credential(credential)?;
        self.invalidate(&credential.id).await;
        Ok(())
    }

    /// Delete a credential from the store.
    pub async fn delete_credential(&self, id: &str) -> Result<()> {
        self.store.delete_credential(id)?;
        self.invalidate(id).await;
        Ok(())
    }

    /// Inject credential-based authentication into an HTTP request builder.
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
            CredentialType::OAuth2 { .. } => Err(CredentialError::Internal(
                "OAuth2 credential injection not yet implemented".to_string(),
            )),
            _ => Ok(request),
        }
    }

    /// Get auth headers for manual request building.
    pub fn get_auth_headers(&self, credential: &Credential) -> Result<Vec<(String, String)>> {
        match &credential.credential_type {
            CredentialType::ApiKey { key, header_name } => {
                Ok(vec![(header_name.clone(), key.clone())])
            },
            CredentialType::BearerToken { token } => {
                Ok(vec![("Authorization".to_string(), format!("Bearer {token}"))])
            },
            CredentialType::BasicAuth { username, password } => {
                let encoded = BASE64.encode(format!("{username}:{password}"));
                Ok(vec![("Authorization".to_string(), format!("Basic {encoded}"))])
            },
            _ => Ok(vec![]),
        }
    }

    /// Extract a database connection string from a credential.
    pub async fn get_database_connection_string(&self, credential_id: &str) -> Result<String> {
        let cred = self.get_credential(credential_id).await?;
        match &cred.credential_type {
            CredentialType::DatabaseConnection { connection_string } => {
                Ok(connection_string.clone())
            },
            other => Err(CredentialError::Validation(format!(
                "credential {credential_id} is {other}, not DatabaseConnection"
            ))),
        }
    }

    /// Extract SMTP configuration from a credential.
    pub async fn get_smtp_config(&self, credential_id: &str) -> Result<SmtpConfig> {
        let cred = self.get_credential(credential_id).await?;
        match &cred.credential_type {
            CredentialType::Smtp { host, port, user, pass, tls } => Ok(SmtpConfig {
                host: host.clone(),
                port: *port,
                user: user.clone(),
                pass: pass.clone(),
                tls: *tls,
            }),
            other => Err(CredentialError::Validation(format!(
                "credential {credential_id} is {other}, not Smtp"
            ))),
        }
    }
}

// ── Harness CredentialService trait implementation ──

use axagent_harness::credential_service::{CredentialService, SmtpServiceConfig};

#[async_trait::async_trait]
impl CredentialService for CredentialManager {
    async fn get_database_connection_string(
        &self,
        credential_id: &str,
    ) -> std::result::Result<String, String> {
        CredentialManager::get_database_connection_string(self, credential_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_smtp_config(
        &self,
        credential_id: &str,
    ) -> std::result::Result<SmtpServiceConfig, String> {
        let sc = CredentialManager::get_smtp_config(self, credential_id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(SmtpServiceConfig {
            host: sc.host,
            port: sc.port,
            user: sc.user,
            pass: sc.pass,
            tls: sc.tls,
        })
    }

    async fn get_auth_headers(
        &self,
        credential_id: &str,
    ) -> std::result::Result<Vec<(String, String)>, String> {
        let cred = CredentialManager::get_credential(self, credential_id)
            .await
            .map_err(|e| e.to_string())?;
        CredentialManager::get_auth_headers(self, &cred)
            .map_err(|e| e.to_string())
    }
}
