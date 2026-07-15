// SPDX-License-Identifier: AGPL-3.0-only

//! MCP OAuth 令牌管理器
//!
//! 管理 MCP 服务器的 OAuth 凭据：持久化到磁盘、加载、刷新。
//! 用于 `mcp_client.rs` 中在发起 HTTP/SSE 请求前注入 Authorization 头。

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use base64::Engine;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{info, warn};

use axagent_harness::platform_adapter::CryptoService;
use urlencoding;

/// 限制密钥文件权限为仅当前用户可访问
fn restrict_file_permissions(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(windows)]
    {
        let username = std::env::var("USERNAME").unwrap_or_else(|_| "SYSTEM".into());
        let mut scmd = std::process::Command::new("icacls");
        scmd.arg(path.as_os_str())
            .arg("/inheritance:r")
            .arg("/grant")
            .arg(format!("{}:(R,W)", username));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            scmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let result = scmd.output();
        match result {
            Ok(output) if !output.status.success() => {
                warn!(
                    "[McpOAuth] icacls 权限设置警告: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            },
            Err(e) => warn!("[McpOAuth] icacls 执行失败: {}", e),
            _ => {},
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
    }
}

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
/// 模块级全局单例存储。
static GLOBAL_STORE: std::sync::OnceLock<Arc<McpOAuthStore>> = std::sync::OnceLock::new();

pub struct McpOAuthStore {
    credentials: RwLock<HashMap<String, McpOAuthCredentials>>,
    store_path: PathBuf,
    crypto_service: Arc<dyn CryptoService>,
    oauth_key: [u8; 32],
}

impl McpOAuthStore {
    /// 创建新的凭据存储，从磁盘加载已有凭据
    pub fn new(crypto_service: Arc<dyn CryptoService>) -> Self {
        let store_path = Self::default_store_path();
        let oauth_key = Self::get_master_key(crypto_service.as_ref());
        let credentials = Self::load_from_disk(&store_path, crypto_service.as_ref(), &oauth_key);
        Self { credentials: RwLock::new(credentials), store_path, crypto_service, oauth_key }
    }

    #[must_use]
    pub fn with_path(store_path: PathBuf, crypto_service: Arc<dyn CryptoService>) -> Self {
        let oauth_key = Self::get_master_key(crypto_service.as_ref());
        let credentials = Self::load_from_disk(&store_path, crypto_service.as_ref(), &oauth_key);
        Self { credentials: RwLock::new(credentials), store_path, crypto_service, oauth_key }
    }

    /// 全局单例
    ///
    /// 必须先通过 `Self::new()` 或 `Self::with_path()` 构造实例后调用
    /// `init_global()` 初始化；否则会 panic。
    #[must_use]
    pub fn global() -> Arc<McpOAuthStore> {
        GLOBAL_STORE.get().expect("McpOAuthStore::global() called before init_global()").clone()
    }

    /// 初始化全局单例。
    pub fn init_global(store: Arc<McpOAuthStore>) {
        let _ = GLOBAL_STORE.set(store);
    }

    /// 非 panic 版本的全局单例访问。未初始化时返回 `None`。
    #[must_use]
    pub fn try_global() -> Option<Arc<McpOAuthStore>> {
        GLOBAL_STORE.get().cloned()
    }

    fn default_store_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(".axagent").join("mcp_oauth_credentials.enc")
    }

    fn load_from_disk(
        path: &PathBuf,
        crypto: &dyn CryptoService,
        master_key: &[u8; 32],
    ) -> HashMap<String, McpOAuthCredentials> {
        let encrypted = fs::read(path).ok().unwrap_or_default();
        if encrypted.is_empty() {
            let legacy_path = {
                let home = dirs::home_dir().unwrap_or_default();
                home.join(".axagent").join("mcp_oauth_credentials.json")
            };
            if legacy_path.exists()
                && legacy_path != *path
                && let Ok(content) = fs::read_to_string(&legacy_path)
                && let Ok(creds) =
                    serde_json::from_str::<HashMap<String, McpOAuthCredentials>>(&content)
            {
                warn!("[McpOAuth] 检测到旧版明文凭据文件，将在首次持久化时自动迁移为加密格式");
                return creds;
            }
            return HashMap::new();
        }
        let encrypted_str = String::from_utf8_lossy(&encrypted);
        let decrypted = match crypto.decrypt_key_with(&encrypted_str, master_key) {
            Ok(d) => d,
            Err(e) => {
                warn!("[McpOAuth] 凭据解密失败，将使用空存储: {e}");
                return HashMap::new();
            },
        };
        serde_json::from_str(&decrypted).unwrap_or_default()
    }

    fn get_master_key(crypto: &dyn CryptoService) -> [u8; 32] {
        let home = dirs::home_dir().unwrap_or_default();
        let key_path = home.join(".axagent").join(".oauth_key");
        if key_path.exists()
            && let Ok(key_bytes) = fs::read(&key_path)
            && key_bytes.len() == 32
        {
            let mut key = [0u8; 32];
            key.copy_from_slice(&key_bytes);
            return key;
        }
        let key = crypto.generate_master_key();
        if let Some(parent) = key_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(&key_path, key).is_err() {
            warn!("[McpOAuth] OAuth 密钥文件写入失败，凭据可能无法在下次启动时恢复");
        }
        // 限制密钥文件权限为仅当前用户可访问
        restrict_file_permissions(&key_path);
        key
    }

    async fn persist(&self) {
        let creds = self.credentials.read().await;
        if let Some(parent) = self.store_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&*creds) {
            match self.crypto_service.encrypt_key_with(&json, &self.oauth_key) {
                Ok(encrypted) => {
                    let _ = fs::write(&self.store_path, encrypted.as_bytes());
                },
                Err(e) => {
                    warn!("[McpOAuth] 凭据加密持久化失败: {e}");
                },
            }
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
                && let Ok(new_creds) = Self::refresh_token(
                    token_endpoint,
                    &creds.client_id,
                    refresh_token,
                    &creds.scopes,
                )
                .await
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
        scopes: &[String],
    ) -> std::result::Result<McpOAuthCredentials, String> {
        let client = reqwest::Client::new();
        let mut body = format!(
            "grant_type=refresh_token&refresh_token={}",
            urlencoding::encode(refresh_token)
        );
        if let Some(cid) = client_id {
            body.push_str(&format!("&client_id={}", urlencoding::encode(cid)));
        }

        let response = client
            .post(token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Token 刷新请求失败: {e}"))?;

        let body: serde_json::Value =
            response.json().await.map_err(|e| format!("Token 刷新响应解析失败: {e}"))?;

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
            scopes: scopes.to_vec(),
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
    scopes: &[String],
) -> std::result::Result<McpOAuthCredentials, String> {
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("code_verifier", code_verifier),
    ];

    let body_params: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let response = client
        .post(token_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body_params)
        .send()
        .await
        .map_err(|e| format!("Token 交换请求失败: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Token 交换失败 ({}): {}", status, body));
    }

    let body: serde_json::Value =
        response.json().await.map_err(|e| format!("Token 响应解析失败: {e}"))?;

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
        scopes: scopes.to_vec(),
        token_endpoint: Some(token_endpoint.to_string()),
        client_id: Some(client_id.to_string()),
    })
}

/// 待完成的 OAuth 授权请求（保存 PKCE verifier 等中间态）。
#[derive(Debug, Clone)]
struct PendingOAuthState {
    verifier: String,
    redirect_uri: String,
    token_endpoint: String,
    client_id: Option<String>,
    scopes: Vec<String>,
}

static PENDING_OAUTH: std::sync::OnceLock<Mutex<HashMap<String, PendingOAuthState>>> =
    std::sync::OnceLock::new();

/// 生成 PKCE code_verifier 与 code_challenge（S256）。
fn generate_pkce_pair() -> (String, String) {
    let mut buf = [0u8; 64];
    rand::rng().fill(&mut buf);
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf);
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

/// 为受保护的 MCP 服务器发起 OAuth 2.1 (PKCE) 授权，
/// 返回需要在浏览器中打开的授权 URL。授权码回调后调用
/// [`complete_oauth_authorization`] 兑换并持久化 token。
pub fn begin_oauth_authorization(
    server_id: &str,
    authorization_endpoint: &str,
    token_endpoint: &str,
    client_id: &str,
    redirect_uri: &str,
    scopes: &[String],
) -> std::result::Result<String, String> {
    let (verifier, challenge) = generate_pkce_pair();
    let mut state_buf = [0u8; 16];
    rand::rng().fill(&mut state_buf);
    let state = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(state_buf);

    let pending = PendingOAuthState {
        verifier,
        redirect_uri: redirect_uri.to_string(),
        token_endpoint: token_endpoint.to_string(),
        client_id: Some(client_id.to_string()),
        scopes: scopes.to_vec(),
    };
    PENDING_OAUTH
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "OAuth pending 锁已损坏".to_string())?
        .insert(server_id.to_string(), pending);

    let scope_str = scopes.join(" ");
    let url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        authorization_endpoint,
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(&scope_str),
        urlencoding::encode(&state),
        urlencoding::encode(&challenge),
    );
    Ok(url)
}

/// 用授权码兑换 token 并持久化到 OAuth 存储，供后续请求自动注入。
pub async fn complete_oauth_authorization(
    server_id: &str,
    code: &str,
) -> std::result::Result<(), String> {
    let pending = PENDING_OAUTH
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "OAuth pending 锁已损坏".to_string())?
        .get(server_id)
        .cloned()
        .ok_or_else(|| {
            "该服务器尚未发起 OAuth 授权（请先调用 begin_oauth_authorization）".to_string()
        })?;

    let creds = exchange_code_for_token(
        &pending.token_endpoint,
        pending.client_id.as_deref().unwrap_or(""),
        &pending.verifier,
        code,
        &pending.redirect_uri,
        &pending.scopes,
    )
    .await?;

    let store =
        McpOAuthStore::try_global().ok_or_else(|| "MCP OAuth 存储尚未初始化".to_string())?;
    store.store(server_id, creds).await;

    if let Some(mut map) = PENDING_OAUTH.get().and_then(|m| m.lock().ok()) {
        map.remove(server_id);
    }

    info!("[McpOAuth] 服务器 '{server_id}' OAuth 授权完成，token 已持久化");
    Ok(())
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
