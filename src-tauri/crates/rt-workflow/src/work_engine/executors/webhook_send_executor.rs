// SPDX-License-Identifier: AGPL-3.0-only

//! WebhookSend executor — sends HTTP requests with optional credential-based
//! authentication.
//!
//! Reuses `reqwest` (already depended upon by HttpRequestExecutor). When
//! `credential_id` is set, the executor resolves the credential via
//! `CredentialManager` and injects the appropriate auth headers.

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use std::time::Duration;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct WebhookSendExecutor;

impl WebhookSendExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebhookSendExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for WebhookSendExecutor {
    fn node_type(&self) -> &'static str {
        "webhookSend"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::WebhookSend(n) = node else {
            return Err(NodeError::type_mismatch("webhookSend", self.node_type()));
        };
        let c = &n.config;

        if c.url.trim().is_empty() {
            return Err(NodeError::exec_failed("WEBHOOK_CONFIG_INVALID", "webhook URL is empty"));
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| NodeError::exec_failed("WEBHOOK_CLIENT_FAILED", e.to_string()))?;

        let method = c.method.to_uppercase();
        let mut req = match method.as_str() {
            "POST" => client.post(&c.url),
            "PUT" => client.put(&c.url),
            "PATCH" => client.patch(&c.url),
            "GET" => client.get(&c.url),
            "DELETE" => client.delete(&c.url),
            other => {
                return Err(NodeError::exec_failed(
                    "WEBHOOK_METHOD_UNSUPPORTED",
                    format!("unsupported method: {other}"),
                ));
            },
        };

        // Inject credential-based auth headers if credential_id is set
        if let Some(cid) = c.credential_id.as_deref()
            && let Some(cm) = &ctx.credential_manager
        {
            match cm.get_credential(cid) {
                Ok(cred) => {
                    for (key, value) in cm.get_auth_headers(&cred).unwrap_or_default() {
                        req = req.header(&key, &value);
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        credential_id = cid,
                        error = %e,
                        "WebhookSend: failed to load credential, proceeding without auth"
                    );
                },
            }
        }

        // Add explicit headers from config (override credential-injected ones)
        for (key, value) in &c.headers {
            req = req.header(key, value);
        }

        // Attach body
        if let Some(ref body) = c.body
            && !body.trim().is_empty()
        {
            req = req
                .header("Content-Type", "application/json")
                .body(body.clone());
        }

        let start = std::time::Instant::now();
        let response = req
            .send()
            .await
            .map_err(|e| NodeError::exec_failed("WEBHOOK_SEND_FAILED", e.to_string()))?;

        let status = response.status().as_u16();
        let body_text = response.text().await.unwrap_or_default();
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let output = serde_json::json!({
            "url": c.url,
            "method": method,
            "status": status,
            "status_text": if (200..300).contains(&status) { "success" } else { "error" },
            "body": body_text,
            "elapsed_ms": elapsed_ms,
            "node_id": node.base_id(),
        });

        Ok(NodeOutput {
            output,
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
