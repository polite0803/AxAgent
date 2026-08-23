// SPDX-License-Identifier: AGPL-3.0-only

//! DatabaseQuery executor — executes parameterized SQL against PostgreSQL,
//! MySQL, or SQLite using `sqlx` with the Any driver.
//!
//! Connection details are resolved from the `credential_id` stored on the
//! node config (via `CredentialManager::get_database_connection_string`).

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;

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
async fn resolve_connection_string(
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
            .await
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
        )
        .await?;

        // 注入式：通过 DatabaseQueryService trait 执行查询，连接、SQL 注入防御、权限校验均由 trait 实现方负责
        let db_service = ctx.database_query_service.as_ref().ok_or_else(|| {
            NodeError::exec_failed(
                "DATABASE_SERVICE_UNAVAILABLE",
                "database_query_service not injected into ExecutionState",
            )
        })?;

        let query_str = c.query.clone();
        let result = db_service
            .execute_query(&conn_str, &query_str)
            .await
            .map_err(|e| NodeError::exec_failed("DATABASE_QUERY_FAILED", e.to_string()))?;

        let mut results: Vec<serde_json::Value> = Vec::with_capacity(result.rows.len());
        for row in &result.rows {
            let mut map = serde_json::Map::new();
            for (i, col_name) in result.columns.iter().enumerate() {
                let val = row.get(i).cloned().unwrap_or(serde_json::Value::Null);
                map.insert(col_name.clone(), val);
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
            control: None,
        })
    }
}
