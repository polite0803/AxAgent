//! MCP OAuth 令牌管理器
//!
//! 管理 MCP 服务器的 OAuth 凭据：持久化到磁盘、加载、刷新。
//! 用于 `mcp_client.rs` 中在发起 HTTP/SSE 请求前注入 Authorization 头。

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::info;

/// 持久化的 MCP OAuth 凭据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpOAuthCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub scopes: Vec<String>,
    /// OAuth 授权服务器的 token endpoint
    pub token_endpoint: Option<String>,
    /// 客户端 ID
    pub client_id: Option<String>,
}

impl McpOAuthCredentials {
    /// 检查 access token 是否已过期
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now >= exp.saturating_sub(60) // 提前 60 秒刷新
        })
    }

    /// 构建 Authorization header 值
    #[must_use]
    pub fn authorization_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

/// MCP 服务器凭据存储
///
/// 将 OAuth 凭据持久化到 `~/.axagent/mcp_oauth_credentials.json`，
/// 按 server_id 索引。
#[derive(Default)]
pub struct McpOAuthStore {
    credentials: RwLock<HashMap<String, McpOAuthCredentials>>,
    store_path: PathBuf,
}

impl McpOAuthStore {
    /// 创建新的凭据存储，从磁盘加载已有凭据
    #[must_use]
    pub fn new() -> Self {
        let store_path = Self::default_store_path();
        let credentials = Self::load_from_disk(&store_path);
        Self {
            credentials: RwLock::new(credentials),
            store_path,
        }
    }

    #[must_use]
    pub fn with_path(store_path: PathBuf) -> Self {
        let credentials = Self::load_from_disk(&store_path);
        Self {
            credentials: RwLock::new(credentials),
            store_path,
        }
    }

    /// 全局单例
    #[must_use]
    pub fn global() -> Arc<McpOAuthStore> {
        use std::sync::OnceLock;
        static STORE: OnceLock<Arc<McpOAuthStore>> = OnceLock::new();
        STORE.get_or_init(|| Arc::new(McpOAuthStore::new())).clone()
    }

    fn default_store_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(".axagent").join("mcp_oauth_credentials.json")
    }

    fn load_from_disk(path: &PathBuf) -> HashMap<String, McpOAuthCredentials> {
        fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    async fn persist(&self) {
        let creds = self.credentials.read().await;
        if let Some(parent) = self.store_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&*creds) {
            let _ = fs::write(&self.store_path, json);
        }
    }

    /// 获取指定服务器的凭据
    pub async fn get(&self, server_id: &str) -> Option<McpOAuthCredentials> {
        let creds = self.credentials.read().await;
        creds.get(server_id).cloned()
    }

    /// 存储指定服务器的凭据
    pub async fn store(&self, server_id: &str, credentials: McpOAuthCredentials) {
        info!("[McpOAuth] 存储 OAuth 凭据: {server_id}");
        {
            let mut creds = self.credentials.write().await;
            creds.insert(server_id.to_string(), credentials);
        }
        self.persist().await;
    }

    /// 删除指定服务器的凭据
    pub async fn remove(&self, server_id: &str) {
        info!("[McpOAuth] 删除 OAuth 凭据: {server_id}");
        {
            let mut creds = self.credentials.write().await;
            creds.remove(server_id);
        }
        self.persist().await;
    }

    /// 获取服务器的 Authorization header（如果可用且未过期）
    pub async fn get_auth_header(&self, server_id: &str) -> Option<String> {
        let creds = self.get(server_id).await?;
        if creds.is_expired() {
            info!("[McpOAuth] Token 已过期: {server_id}");
            // 如果有 refresh_token 和 token_endpoint，尝试刷新
            if let (Some(refresh_token), Some(token_endpoint)) =
                (&creds.refresh_token, &creds.token_endpoint)
                && let Ok(new_creds) =
                    Self::refresh_token(token_endpoint, &creds.client_id, refresh_token).await
            {
                self.store(server_id, new_creds.clone()).await;
                return Some(new_creds.authorization_header());
            }
            // 无法刷新，返回 None（触发重新授权）
            return None;
        }
        Some(creds.authorization_header())
    }

    /// 刷新过期的 token
    async fn refresh_token(
        token_endpoint: &str,
        client_id: &Option<String>,
        refresh_token: &str,
    ) -> Result<McpOAuthCredentials, String> {
        let client = reqwest::Client::new();
        let mut params = HashMap::from([
            ("grant_type".to_string(), "refresh_token".to_string()),
            ("refresh_token".to_string(), refresh_token.to_string()),
        ]);
        if let Some(cid) = client_id {
            params.insert("client_id".to_string(), cid.clone());
        }

        let response = client
            .post(token_endpoint)
            .json(&params)
            .send()
            .await
            .map_err(|e| format!("Token 刷新请求失败: {e}"))?;

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Token 刷新响应解析失败: {e}"))?;

        let access_token = body["access_token"]
            .as_str()
            .ok_or_else(|| "响应中缺少 access_token".to_string())?
            .to_string();
        let refresh_token_new = body["refresh_token"].as_str().map(String::from);
        let expires_in: Option<u64> = body["expires_in"].as_u64();
        let expires_at = expires_in.map(|secs| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + secs
        });

        Ok(McpOAuthCredentials {
            access_token,
            refresh_token: refresh_token_new,
            expires_at,
            scopes: Vec::new(),
            token_endpoint: Some(token_endpoint.to_string()),
            client_id: client_id.clone(),
        })
    }
}

/// 交换授权码为 token
pub async fn exchange_code_for_token(
    token_endpoint: &str,
    client_id: &str,
    code_verifier: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<McpOAuthCredentials, String> {
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("code_verifier", code_verifier),
    ];

    let response = client
        .post(token_endpoint)
        .json(&params)
        .send()
        .await
        .map_err(|e| format!("Token 交换请求失败: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Token 交换失败 ({}): {}", status, body));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Token 响应解析失败: {e}"))?;

    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| "响应中缺少 access_token".to_string())?
        .to_string();
    let refresh_token = body["refresh_token"].as_str().map(String::from);
    let expires_in: Option<u64> = body["expires_in"].as_u64();
    let expires_at = expires_in.map(|secs| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + secs
    });

    Ok(McpOAuthCredentials {
        access_token,
        refresh_token,
        expires_at,
        scopes: Vec::new(),
        token_endpoint: Some(token_endpoint.to_string()),
        client_id: Some(client_id.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_not_expired_when_far_in_future() {
        let creds = McpOAuthCredentials {
            access_token: "tok".into(),
            refresh_token: None,
            expires_at: Some(u64::MAX),
            scopes: vec![],
            token_endpoint: None,
            client_id: None,
        };
        assert!(!creds.is_expired());
    }

    #[test]
    fn credentials_expired_when_in_past() {
        let creds = McpOAuthCredentials {
            access_token: "tok".into(),
            refresh_token: None,
            expires_at: Some(0),
            scopes: vec![],
            token_endpoint: None,
            client_id: None,
        };
        assert!(creds.is_expired());
    }

    #[test]
    fn authorization_header_has_bearer_prefix() {
        let creds = McpOAuthCredentials {
            access_token: "my-token".into(),
            refresh_token: None,
            expires_at: None,
            scopes: vec![],
            token_endpoint: None,
            client_id: None,
        };
        assert_eq!(creds.authorization_header(), "Bearer my-token");
    }
}
