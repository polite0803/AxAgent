
use async_trait::async_trait;
use axagent_core::workflow_types::{WorkflowNode};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use std::time::Duration;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct HttpRequestExecutor {
    db: Arc<DatabaseConnection>,
    master_key: String,
}

impl HttpRequestExecutor {
    pub fn new(db: Arc<DatabaseConnection>, master_key: String) -> Self {
        Self { db, master_key }
    }
}

#[async_trait]
impl NodeExecutorTrait for HttpRequestExecutor {
    fn node_type(&self) -> &'static str {
        "httpRequest"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        _context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::HttpRequest(http_node) = node else {
            return Err(NodeError::type_mismatch(
                "httpRequest".to_string(),
                crate::work_engine::node_executor_trait::node_type_name(node).to_string(),
            ));
        };

        let config = &http_node.config;
        if config.url.trim().is_empty() {
            return Err(NodeError::exec_failed("http_error", "HTTP Request URL is empty".to_string()));
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs.max(5).min(300)))
            .build()
            .map_err(|e| NodeError::exec_failed("http_error", format!("Failed to create HTTP client: {e}")))?;

        let mut req = match config.method.to_uppercase().as_str() {
            "GET" => client.get(&config.url),
            "POST" => {
                let mut r = client.post(&config.url);
                if let Some(ref body) = config.body {
                    r = match config.body_type.as_str() {
                        "json" => r.json(&serde_json::from_str::<serde_json::Value>(body)
                            .unwrap_or(serde_json::Value::String(body.clone()))),
                        "form" => r.form(&serde_json::from_str::<std::collections::HashMap<String, String>>(body)
                            .unwrap_or_default()),
                        _ => r.body(body.clone()),
                    };
                }
                r
            },
            "PUT" => {
                let mut r = client.put(&config.url);
                if let Some(ref body) = config.body {
                    r = r.body(body.clone());
                }
                r
            },
            "PATCH" => {
                let mut r = client.patch(&config.url);
                if let Some(ref body) = config.body {
                    r = r.body(body.clone());
                }
                r
            },
            "DELETE" => client.delete(&config.url),
            "HEAD" => client.head(&config.url),
            "OPTIONS" => client.request(reqwest::Method::OPTIONS, &config.url),
            _ => return Err(NodeError::exec_failed("http_error", 
                format!("Unsupported HTTP method: {}", config.method)
            )),
        };

        // Add headers
        for (key, value) in &config.headers {
            req = req.header(key, value);
        }

        let response = req.send().await
            .map_err(|e| NodeError::exec_failed("http_error", format!("HTTP request failed: {e}")))?;

        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body_text = response.text().await
            .unwrap_or_else(|e| format!("Failed to read response body: {e}"));

        let output = serde_json::json!({
            "status": status,
            "status_text": if status >= 200 && status < 300 { "success" } else { "error" },
            "headers": headers.iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string())).collect::<std::collections::HashMap<_, _>>(),
            "body": body_text,
            "node_id": node.base_id(),
        });

        Ok(NodeOutput {
            output,
            output_var: if config.output_var.is_empty() { None } else { Some(config.output_var.clone()) },
        })
    }
}
