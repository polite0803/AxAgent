// SPDX-License-Identifier: AGPL-3.0-only

//! 网络传输模块
//!
//! 提供设备间数据传输的 HTTP/WebSocket 客户端实现，
//! 支持推送变更、拉取变更、同步状态查询等操作。

use std::time::Duration;

use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use axagent_harness::device_sync::{ChangeLogEntry, ConflictInfo, SyncResult};

/// 传输配置
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// 远程设备地址（IP:Port）
    pub remote_url: String,
    /// 会话令牌（用于认证）
    pub session_token: Option<String>,
    /// 请求超时
    pub timeout_secs: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self { remote_url: String::new(), session_token: None, timeout_secs: 30 }
    }
}

/// 网络传输客户端
pub struct SyncTransport {
    config: RwLock<TransportConfig>,
    client: Client,
}

impl SyncTransport {
    /// 创建新的传输客户端
    pub fn new(config: TransportConfig) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self { config: RwLock::new(config), client })
    }

    /// 更新配置
    pub async fn update_config(&self, config: TransportConfig) {
        *self.config.write().await = config;
    }

    /// 获取远程 URL
    pub async fn remote_url(&self) -> String {
        self.config.read().await.remote_url.clone()
    }

    /// 发送 HTTP POST 请求
    async fn post<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, String> {
        let config = self.config.read().await;
        let url = format!("{}{}", config.remote_url, path);

        let mut request = self.client.post(&url).json(body);

        if let Some(ref token) = config.session_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        drop(config);

        let response = request.send().await.map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {} error: {}", status, error_body));
        }

        response.json::<R>().await.map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// 推送变更日志到远程设备
    pub async fn push_changes(
        &self,
        changes: &[ChangeLogEntry],
    ) -> Result<Vec<ConflictInfo>, String> {
        self.post("/api/sync/push", &changes.to_vec()).await
    }

    /// 从远程设备拉取变更日志
    pub async fn pull_changes(&self, since_timestamp: u64) -> Result<Vec<ChangeLogEntry>, String> {
        self.post("/api/sync/pull", &since_timestamp).await
    }

    /// 执行全量同步
    pub async fn full_sync(&self, device_id: &str) -> Result<SyncResult, String> {
        self.post("/api/sync/full", &device_id.to_string()).await
    }

    /// 执行增量同步
    pub async fn incremental_sync(&self, device_id: &str) -> Result<SyncResult, String> {
        self.post("/api/sync/incremental", &device_id.to_string()).await
    }

    /// 解决冲突
    pub async fn resolve_conflict(&self, conflict_id: &str, strategy: &str) -> Result<(), String> {
        #[derive(Serialize)]
        struct ResolveRequest {
            conflict_id: String,
            strategy: String,
        }

        self.post(
            "/api/sync/resolve-conflict",
            &ResolveRequest {
                conflict_id: conflict_id.to_string(),
                strategy: strategy.to_string(),
            },
        )
        .await
    }

    /// 发送心跳（检查设备是否在线）
    pub async fn ping(&self) -> Result<bool, String> {
        let config = self.config.read().await;
        let url = format!("{}/api/sync/ping", config.remote_url);

        let mut request = self.client.get(&url);
        if let Some(ref token) = config.session_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        drop(config);

        let response = request.send().await.map_err(|e| format!("Ping failed: {}", e))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_transport() {
        let config = TransportConfig {
            remote_url: "http://localhost:8080".to_string(),
            session_token: Some("test-token".to_string()),
            timeout_secs: 10,
        };

        let transport = SyncTransport::new(config);
        assert!(transport.is_ok());
    }

    #[test]
    fn test_default_config() {
        let config = TransportConfig::default();
        assert!(config.remote_url.is_empty());
        assert!(config.session_token.is_none());
        assert_eq!(config.timeout_secs, 30);
    }
}
