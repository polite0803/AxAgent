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

    /// 刷新 OAuth2 access token（P1 修复：补齐 client_credentials 刷新逻辑）。
    ///
    /// 向 `token_url` 发起 `grant_type=client_credentials` 请求，
    /// 成功后更新缓存中的 `access_token` / `expires_at`，并持久化到存储。
    pub async fn refresh_oauth2_token(&self, id: &str) -> Result<Credential> {
        let mut cred = self.get_credential(id).await?;
        let (client_id, client_secret, token_url, scopes) = match &cred.credential_type {
            CredentialType::OAuth2 { client_id, client_secret, token_url, scopes, .. } => {
                (client_id.clone(), client_secret.clone(), token_url.clone(), scopes.clone())
            },
            _ => {
                return Err(CredentialError::Validation(format!(
                    "credential {id} is not OAuth2, cannot refresh"
                )));
            },
        };

        tracing::info!("[credential] Refreshing OAuth2 token for {id} from {token_url}");

        let client = reqwest::Client::new();
        let mut form = vec![
            ("grant_type".to_string(), "client_credentials".to_string()),
            ("client_id".to_string(), client_id),
            ("client_secret".to_string(), client_secret),
        ];
        if !scopes.is_empty() {
            form.push(("scope".to_string(), scopes.join(" ")));
        }

        let resp = client.post(&token_url).form(&form).send().await.map_err(|e| {
            CredentialError::Validation(format!("OAuth2 refresh request failed: {e}"))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(CredentialError::Validation(format!(
                "OAuth2 token endpoint returned {status}: {body}"
            )));
        }

        #[derive(serde::Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: Option<u64>,
        }

        let token_data: TokenResponse = resp.json().await.map_err(|e| {
            CredentialError::Validation(format!("OAuth2 token response parse failed: {e}"))
        })?;

        let expires_at = token_data
            .expires_in
            .map(|secs| chrono::Utc::now().timestamp_millis() + (secs as i64) * 1000);

        // 更新 credential_type 中的 access_token / expires_at
        if let CredentialType::OAuth2 { access_token, expires_at: exp_slot, .. } =
            &mut cred.credential_type
        {
            *access_token = Some(token_data.access_token);
            *exp_slot = expires_at;
        }
        cred.updated_at = chrono::Utc::now().timestamp_millis();

        // 持久化更新后的凭证（含新 token）
        self.store.save_credential(&cred)?;
        {
            let mut cache = self.cache.lock().await;
            cache.insert(id.to_string(), cred.clone());
        }

        tracing::info!("[credential] OAuth2 token refreshed for {id}, expires_at={expires_at:?}");
        Ok(cred)
    }

    /// 获取凭证，如果 OAuth2 token 已过期则自动刷新（P1 修复）。
    pub async fn get_credential_with_refresh(&self, id: &str) -> Result<Credential> {
        let cred = self.get_credential(id).await?;
        if cred.credential_type.is_oauth2_expired() {
            tracing::debug!("[credential] OAuth2 token for {id} expired, auto-refreshing");
            return self.refresh_oauth2_token(id).await;
        }
        Ok(cred)
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
            CredentialType::OAuth2 { access_token, .. } => {
                // 过期检测：阻断已过期的 token，避免无效请求
                if credential.credential_type.is_oauth2_expired() {
                    return Err(CredentialError::Validation(
                        "OAuth2 access token 已过期，需要刷新".to_string(),
                    ));
                }
                let token = access_token.as_ref().ok_or_else(|| {
                    CredentialError::Validation(
                        "OAuth2 凭证尚未授权（access_token 缺失，需先刷新）".to_string(),
                    )
                })?;
                Ok(request.bearer_auth(token))
            },
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
            CredentialType::OAuth2 { access_token, .. } => {
                // 过期检测：阻断已过期的 token
                if credential.credential_type.is_oauth2_expired() {
                    return Err(CredentialError::Validation(
                        "OAuth2 access token 已过期，需要刷新".to_string(),
                    ));
                }
                let token = access_token.as_ref().ok_or_else(|| {
                    CredentialError::Validation(
                        "OAuth2 凭证尚未授权（access_token 缺失，需先刷新）".to_string(),
                    )
                })?;
                Ok(vec![("Authorization".to_string(), format!("Bearer {token}"))])
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
        // P1 修复：使用 get_credential_with_refresh 自动刷新过期的 OAuth2 token
        let cred = CredentialManager::get_credential_with_refresh(self, credential_id)
            .await
            .map_err(|e| e.to_string())?;
        CredentialManager::get_auth_headers(self, &cred).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::CredentialStore;
    use crate::types::{Credential, CredentialType};
    use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
    use rand::RngExt;

    struct TempManager {
        _dir: std::path::PathBuf,
        mgr: CredentialManager,
    }

    impl Drop for TempManager {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self._dir);
        }
    }

    fn temp_manager() -> TempManager {
        let mut suffix = [0u8; 8];
        rand::rng().fill(&mut suffix);
        let dir = std::env::temp_dir().join(format!("axagent_cred_mgr_{}", hex::encode(suffix)));
        std::fs::create_dir_all(&dir).ok();
        let store = CredentialStore::new(dir.clone(), [0x99u8; 32]);
        TempManager { _dir: dir, mgr: CredentialManager::new(store) }
    }

    #[tokio::test]
    async fn save_get_and_cache() {
        let tm = temp_manager();
        let cred = Credential::new(
            "c1".into(),
            "n".into(),
            CredentialType::ApiKey { key: "k".into(), header_name: "h".into() },
        );
        tm.mgr.save_credential(&cred).await.unwrap();
        let got = tm.mgr.get_credential("c1").await.unwrap();
        assert_eq!(serde_json::to_string(&got).unwrap(), serde_json::to_string(&cred).unwrap());
        // 第二次命中内存缓存，仍成功。
        let got2 = tm.mgr.get_credential("c1").await.unwrap();
        assert_eq!(got2.id, "c1");
        // 失效后仍能从磁盘重载。
        tm.mgr.invalidate("c1").await;
        let got3 = tm.mgr.get_credential("c1").await.unwrap();
        assert_eq!(got3.id, "c1");
    }

    #[tokio::test]
    async fn get_auth_headers_variants() {
        let tm = temp_manager();
        let api = Credential::new(
            "a".into(),
            "a".into(),
            CredentialType::ApiKey { key: "k1".into(), header_name: "X-Key".into() },
        );
        let hdrs = tm.mgr.get_auth_headers(&api).unwrap();
        assert_eq!(hdrs, vec![("X-Key".to_string(), "k1".to_string())]);

        let bearer = Credential::new(
            "b".into(),
            "b".into(),
            CredentialType::BearerToken { token: "tok".into() },
        );
        let hdrs = tm.mgr.get_auth_headers(&bearer).unwrap();
        assert_eq!(hdrs, vec![("Authorization".to_string(), "Bearer tok".to_string())]);

        let basic = Credential::new(
            "c".into(),
            "c".into(),
            CredentialType::BasicAuth { username: "u".into(), password: "p".into() },
        );
        let hdrs = tm.mgr.get_auth_headers(&basic).unwrap();
        let expected = format!("Basic {}", BASE64.encode("u:p"));
        assert_eq!(hdrs, vec![("Authorization".to_string(), expected)]);

        // OAuth2：有 access_token 且未过期 → 返回 Bearer
        let oauth_ok = Credential::new(
            "d".into(),
            "d".into(),
            CredentialType::OAuth2 {
                client_id: "ci".into(),
                client_secret: "cs".into(),
                token_url: "tu".into(),
                scopes: vec!["s1".into()],
                access_token: Some("access-tok".into()),
                expires_at: Some(chrono::Utc::now().timestamp_millis() + 3_600_000),
            },
        );
        let hdrs = tm.mgr.get_auth_headers(&oauth_ok).unwrap();
        assert_eq!(hdrs, vec![("Authorization".to_string(), "Bearer access-tok".to_string())]);

        // OAuth2：access_token 为 None → 返回 Validation 错误
        let oauth_no_token = Credential::new(
            "e".into(),
            "e".into(),
            CredentialType::OAuth2 {
                client_id: "ci".into(),
                client_secret: "cs".into(),
                token_url: "tu".into(),
                scopes: vec!["s1".into()],
                access_token: None,
                expires_at: None,
            },
        );
        assert!(tm.mgr.get_auth_headers(&oauth_no_token).is_err());

        // OAuth2：已过期 → 返回 Validation 错误
        let oauth_expired = Credential::new(
            "f".into(),
            "f".into(),
            CredentialType::OAuth2 {
                client_id: "ci".into(),
                client_secret: "cs".into(),
                token_url: "tu".into(),
                scopes: vec!["s1".into()],
                access_token: Some("access-tok".into()),
                expires_at: Some(chrono::Utc::now().timestamp_millis() - 1_000),
            },
        );
        assert!(tm.mgr.get_auth_headers(&oauth_expired).is_err());
    }

    #[tokio::test]
    async fn database_connection_string_ok_and_wrong_type() {
        let tm = temp_manager();
        let db = Credential::new(
            "d".into(),
            "d".into(),
            CredentialType::DatabaseConnection { connection_string: "postgres://x".into() },
        );
        tm.mgr.save_credential(&db).await.unwrap();
        assert_eq!(tm.mgr.get_database_connection_string("d").await.unwrap(), "postgres://x");
        // 错误类型应返回 Validation 错误。
        let api = Credential::new(
            "e".into(),
            "e".into(),
            CredentialType::ApiKey { key: "k".into(), header_name: "h".into() },
        );
        tm.mgr.save_credential(&api).await.unwrap();
        assert!(tm.mgr.get_database_connection_string("e").await.is_err());
    }

    #[tokio::test]
    async fn smtp_config_ok_and_wrong_type() {
        let tm = temp_manager();
        let smtp = Credential::new(
            "s".into(),
            "s".into(),
            CredentialType::Smtp {
                host: "mx".into(),
                port: 587,
                user: "u".into(),
                pass: "p".into(),
                tls: true,
            },
        );
        tm.mgr.save_credential(&smtp).await.unwrap();
        let cfg = tm.mgr.get_smtp_config("s").await.unwrap();
        assert_eq!(cfg.host, "mx");
        assert_eq!(cfg.port, 587);
        assert!(cfg.tls);
        // 错误类型应返回 Validation 错误。
        let api = Credential::new(
            "e".into(),
            "e".into(),
            CredentialType::ApiKey { key: "k".into(), header_name: "h".into() },
        );
        tm.mgr.save_credential(&api).await.unwrap();
        assert!(tm.mgr.get_smtp_config("e").await.is_err());
    }
}
