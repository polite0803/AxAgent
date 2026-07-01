// SPDX-License-Identifier: AGPL-3.0-only

//! DatabaseQuery executor — executes parameterized SQL against PostgreSQL,
//! MySQL, or SQLite using `sqlx` with the Any driver.
//!
//! Connection details are resolved from the `credential_id` stored on the
//! node config (via `CredentialManager::get_database_connection_string`).

use async_trait::async_trait;
use axagent_core::workflow_types::WorkflowNode;
use sqlx::any::AnyPoolOptions;
use sqlx::Row;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{NodeError, NodeExecutorTrait, NodeOutput};

pub struct DatabaseQueryExecutor;

impl DatabaseQueryExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DatabaseQueryExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the connection string, preferring credential manager when
/// `credential_id` is set, otherwise falling back to `connection_name`
/// (which may reference an environment variable).
fn resolve_connection_string(
    ctx: &ExecutionState,
    credential_id: Option<&str>,
    connection_name: Option<&str>,
) -> Result<String, NodeError> {
    if let Some(cid) = credential_id {
        let cm = ctx.credential_manager.as_ref().ok_or_else(|| {
            NodeError::exec_failed(
                "DATABASE_CREDENTIAL_UNAVAILABLE",
                "credential_manager not injected into ExecutionState",
            )
        })?;
        cm.get_database_connection_string(cid)
            .map_err(|e| NodeError::exec_failed("DATABASE_CREDENTIAL_FAILED", e.to_string()))
    } else if let Some(name) = connection_name {
        // Interpret as env var name (e.g. DATABASE_URL)
        std::env::var(name).map_err(|_| {
            NodeError::exec_failed(
                "DATABASE_CREDENTIAL_UNAVAILABLE",
                format!("environment variable {name} not set"),
            )
        })
    } else {
        Err(NodeError::exec_failed(
            "DATABASE_CREDENTIAL_UNAVAILABLE",
            "neither credential_id nor connection_name configured",
        ))
    }
}

#[async_trait]
impl NodeExecutorTrait for DatabaseQueryExecutor {
    fn node_type(&self) -> &'static str {
        "databaseQuery"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::DatabaseQuery(n) = node else {
            return Err(NodeError::type_mismatch("databaseQuery", self.node_type()));
        };
        let c = &n.config;

        let conn_str = resolve_connection_string(
            ctx,
            c.credential_id.as_deref(),
            c.connection_name.as_deref(),
        )?;

        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(&conn_str)
            .await
            .map_err(|e| NodeError::exec_failed("DATABASE_CONNECT_FAILED", e.to_string()))?;

        let rows = sqlx::query(&c.query)
            .fetch_all(&pool)
            .await
            .map_err(|e| NodeError::exec_failed("DATABASE_QUERY_FAILED", e.to_string()));

        // Drop pool connection explicitly
        drop(pool);

        let rows = rows?;

        let mut results: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut map = serde_json::Map::new();
            for (i, col) in row.columns().iter().enumerate() {
                let val: serde_json::Value = if let Ok(v) = row.try_get::<i64, _>(i) {
                    serde_json::json!(v)
                } else if let Ok(v) = row.try_get::<f64, _>(i) {
                    serde_json::json!(v)
                } else if let Ok(v) = row.try_get::<String, _>(i) {
                    serde_json::json!(v)
                } else if let Ok(v) = row.try_get::<bool, _>(i) {
                    serde_json::json!(v)
                } else {
                    serde_json::Value::Null
                };
                map.insert(col.name.to_string(), val);
            }
            results.push(serde_json::Value::Object(map));
        }

        let output = serde_json::json!({
            "rows": results,
            "row_count": results.len(),
            "query": c.query,
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
