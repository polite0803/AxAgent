//! RemoteAgent 任务 — 远程 agent 执行
//! Feature flag: REMOTE_AGENT
//!
//! 支持通过 WebSocket/HTTP/SSE 连接到远程 agent 端点，
//! 发送任务并接收执行结果。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 远程传输协议类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoteTransport {
    WebSocket { url: String },
    Http { url: String },
    Sse { url: String },
}

/// 重连策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectPolicy {
    /// 最大重连次数
    pub max_attempts: u32,
    /// 重连间隔（毫秒）
    pub interval_ms: u64,
    /// 是否启用指数退避
    pub exponential_backoff: bool,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            interval_ms: 2000,
            exponential_backoff: true,
        }
    }
}

/// 远程 Agent 任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteAgentTask {
    pub id: String,
    pub remote_url: String,
    pub transport: RemoteTransport,
    pub auth_token: Option<String>,
    pub heartbeat_interval: Duration,
    pub reconnect_policy: ReconnectPolicy,
    pub status: RemoteAgentStatus,
    pub created_at: DateTime<Utc>,
    pub last_heartbeat: Option<DateTime<Utc>>,
}

/// 远程 agent 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteAgentStatus {
    Connecting,
    Connected,
    Disconnected,
    Reconnecting,
    Failed,
    Completed,
}

/// 远程任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTaskRequest {
    pub task_id: String,
    pub prompt: String,
    pub work_dir: Option<String>,
    pub max_turns: Option<u32>,
    pub timeout_secs: Option<u64>,
}

/// 远程任务响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTaskResponse {
    pub task_id: String,
    pub status: String,
    pub content: Option<String>,
    pub tool_calls_count: usize,
    pub error: Option<String>,
}

impl RemoteAgentTask {
    /// 创建新的远程 agent 任务
    pub fn new(url: &str, transport: RemoteTransport) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            remote_url: url.to_string(),
            transport,
            auth_token: None,
            heartbeat_interval: Duration::from_secs(15),
            reconnect_policy: ReconnectPolicy::default(),
            status: RemoteAgentStatus::Connecting,
            created_at: Utc::now(),
            last_heartbeat: None,
        }
    }

    /// 设置认证令牌
    pub fn with_auth(mut self, token: &str) -> Self {
        self.auth_token = Some(token.to_string());
        self
    }

    /// 构建 HTTP 请求头
    pub fn build_headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Agent-Id".to_string(), self.id.clone()),
        ];
        if let Some(ref token) = self.auth_token {
            headers.push(("Authorization".to_string(), format!("Bearer {}", token)));
        }
        headers
    }

    /// 是否启用远程 agent（检查 feature flag）
    pub fn is_enabled() -> bool {
        crate::feature_flags::global_feature_flags().remote_agent()
    }

    /// 获取 URL（提取自 transport）
    pub fn url(&self) -> &str {
        match &self.transport {
            RemoteTransport::WebSocket { url }
            | RemoteTransport::Http { url }
            | RemoteTransport::Sse { url } => url,
        }
    }

    /// 检查心跳是否超时
    pub fn is_heartbeat_stale(&self) -> bool {
        match self.last_heartbeat {
            Some(last) => {
                let elapsed = Utc::now() - last;
                chrono::Duration::from_std(self.heartbeat_interval * 2)
                    .map(|d| elapsed > d)
                    .unwrap_or(true)
            },
            None => true,
        }
    }
}

/// 远程 Agent 客户端（基于 HTTP）
pub struct RemoteAgentClient {
    http_client: reqwest::Client,
}

impl RemoteAgentClient {
    /// 创建新的 HTTP 客户端
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }

    /// 向远程 agent 发送任务
    ///
    /// 若 REMOTE_AGENT feature flag 未启用，直接返回错误。
    pub async fn send_task(
        &self,
        task: &RemoteAgentTask,
        request: &RemoteTaskRequest,
    ) -> Result<RemoteTaskResponse, String> {
        // 检查 REMOTE_AGENT feature flag
        if !RemoteAgentTask::is_enabled() {
            return Err(
                "Remote Agent 未启用（设置 AXAGENT_FF_REMOTE_AGENT=1 或 features.RemoteAgent=true）"
                    .to_string(),
            );
        }

        let url = format!("{}/api/tasks", task.url());
        let headers = task.build_headers();

        let mut req = self.http_client
            .post(&url)
            .json(request)
            .timeout(Duration::from_secs(
                request.timeout_secs.unwrap_or(300),
            ));

        for (key, value) in &headers {
            req = req.header(key.as_str(), value.as_str());
        }

        let response = req.send().await.map_err(|e| format!("请求失败: {}", e))?;
        let body = response
            .json::<RemoteTaskResponse>()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        Ok(body)
    }

    /// 查询远程 agent 状态
    pub async fn get_status(
        &self,
        task: &RemoteAgentTask,
    ) -> Result<RemoteAgentStatus, String> {
        let url = format!("{}/api/status", task.url());
        let headers = task.build_headers();

        let mut req = self.http_client.get(&url);
        for (key, value) in &headers {
            req = req.header(key.as_str(), value.as_str());
        }

        let response = req.send().await.map_err(|e| format!("状态查询失败: {}", e))?;

        match response.status().as_u16() {
            200 => Ok(RemoteAgentStatus::Connected),
            503 => Ok(RemoteAgentStatus::Disconnected),
            _ => Ok(RemoteAgentStatus::Failed),
        }
    }
}

impl Default for RemoteAgentClient {
    fn default() -> Self {
        Self::new()
    }
}
