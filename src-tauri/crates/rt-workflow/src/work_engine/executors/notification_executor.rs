// SPDX-License-Identifier: AGPL-3.0-only

//! Notification executor — sends notifications to Slack, WeCom (企业微信),
//! DingTalk (钉钉), and Feishu (飞书) via their incoming webhook APIs.
//!
//! Channel type and webhook URL are read from `NotificationNodeConfig`. The
//! message body is formatted according to each platform's expected schema.

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use std::time::Duration;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct NotificationExecutor;

impl NotificationExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NotificationExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a JSON payload for the target notification channel.
fn build_payload(channel: &str, message: &str) -> serde_json::Value {
    match channel.to_lowercase().as_str() {
        "slack" => serde_json::json!({
            "text": message,
        }),
        "wecom" | "wechat_work" | "企业微信" => serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "content": message,
            },
        }),
        "dingtalk" | "ding" | "钉钉" => serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "title": "Notification",
                "text": message,
            },
        }),
        "feishu" | "lark" | "飞书" => serde_json::json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": "Notification",
                    },
                },
                "elements": [
                    {
                        "tag": "markdown",
                        "content": message,
                    },
                ],
            },
        }),
        _ => serde_json::json!({"text": message}),
    }
}

#[async_trait]
impl NodeExecutorTrait for NotificationExecutor {
    fn node_type(&self) -> &'static str {
        "notification"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Notification(n) = node else {
            return Err(NodeError::type_mismatch("notification", self.node_type()));
        };
        let c = &n.config;

        if c.webhook_url.trim().is_empty() {
            return Err(NodeError::exec_failed(
                "NOTIFICATION_CONFIG_INVALID",
                "webhook_url is empty",
            ));
        }

        let payload = build_payload(&c.channel, &c.message);

        tracing::info!(
            channel = %c.channel,
            url = %c.webhook_url,
            "NotificationExecutor: sending"
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| NodeError::exec_failed("NOTIFICATION_CLIENT_FAILED", e.to_string()))?;

        let response = client
            .post(&c.webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NodeError::exec_failed("NOTIFICATION_SEND_FAILED", e.to_string()))?;

        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        let success = (200..300).contains(&status);

        if !success {
            tracing::warn!(
                status,
                body = %body,
                "NotificationExecutor: non-2xx response"
            );
        }

        Ok(NodeOutput {
            output: serde_json::json!({
                "sent": success,
                "status": status,
                "channel": c.channel,
                "response_body": body,
                "node_id": node.base_id(),
            }),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
