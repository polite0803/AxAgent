//! ACP 客户端 — 用于连接远程 ACP 服务端

use reqwest::Client;
use std::time::Duration;

use crate::protocol::{
    CreateSessionParams, CreateSessionResult, RegisterHookParams, SendPromptParams,
    SendPromptResult, StatusResult,
};
use crate::types::AcpRequest;

/// ACP HTTP 客户端
pub struct AcpClient {
    pub base_url: String,
    pub auth_token: Option<String>,
    http_client: Client,
}

impl AcpClient {
    /// 创建新的 ACP 客户端
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_token: None,
            http_client: Client::new(),
        }
    }

    /// 设置认证令牌
    pub fn with_auth(mut self, token: &str) -> Self {
        self.auth_token = Some(token.to_string());
        self
    }

    /// 构建请求（带认证头）
    fn request_builder(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut builder = self
            .http_client
            .request(method, &url)
            .timeout(Duration::from_secs(60))
            .header("Content-Type", "application/json");

        if let Some(ref token) = self.auth_token {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }

        builder
    }

    /// 创建会话
    pub async fn create_session(
        &self,
        params: &CreateSessionParams,
    ) -> Result<CreateSessionResult, String> {
        let response = self
            .request_builder(reqwest::Method::POST, "/acp/v1/sessions")
            .json(params)
            .send()
            .await
            .map_err(|e| format!("创建会话失败: {}", e))?;

        response
            .json::<CreateSessionResult>()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    }

    /// 发送 prompt
    pub async fn send_prompt(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<SendPromptResult, String> {
        let params = SendPromptParams {
            session_id: session_id.to_string(),
            prompt: prompt.to_string(),
            max_turns: None,
        };

        let response = self
            .request_builder(
                reqwest::Method::POST,
                &format!("/acp/v1/sessions/{}/prompts", session_id),
            )
            .json(&params)
            .send()
            .await
            .map_err(|e| format!("发送 prompt 失败: {}", e))?;

        response
            .json::<SendPromptResult>()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    }

    /// 获取会话状态
    pub async fn get_status(&self, session_id: &str) -> Result<StatusResult, String> {
        let response = self
            .request_builder(
                reqwest::Method::GET,
                &format!("/acp/v1/sessions/{}", session_id),
            )
            .send()
            .await
            .map_err(|e| format!("获取状态失败: {}", e))?;

        response
            .json::<StatusResult>()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    }

    /// 关闭会话
    pub async fn close_session(&self, session_id: &str) -> Result<(), String> {
        let response = self
            .request_builder(
                reqwest::Method::POST,
                &format!("/acp/v1/sessions/{}/close", session_id),
            )
            .send()
            .await
            .map_err(|e| format!("关闭会话失败: {}", e))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("关闭会话失败: HTTP {}", response.status()))
        }
    }

    /// 中断执行
    pub async fn interrupt(&self, session_id: &str) -> Result<(), String> {
        let response = self
            .request_builder(
                reqwest::Method::POST,
                &format!("/acp/v1/sessions/{}/interrupt", session_id),
            )
            .send()
            .await
            .map_err(|e| format!("中断失败: {}", e))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("中断失败: HTTP {}", response.status()))
        }
    }

    /// 注册 hook 回调
    pub async fn register_hook(
        &self,
        session_id: &str,
        event: &str,
        callback_url: &str,
    ) -> Result<(), String> {
        let params = RegisterHookParams {
            session_id: session_id.to_string(),
            event: event.to_string(),
            callback_url: callback_url.to_string(),
        };

        let response = self
            .request_builder(reqwest::Method::POST, "/acp/v1/hooks")
            .json(&params)
            .send()
            .await
            .map_err(|e| format!("注册 hook 失败: {}", e))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("注册 hook 失败: HTTP {}", response.status()))
        }
    }

    /// 健康检查
    pub async fn health_check(&self) -> bool {
        self.get_status("health").await.is_ok()
    }
}
