// SPDX-License-Identifier: AGPL-3.0-only

//! DatabaseQuery executor — executes parameterized SQL against PostgreSQL,
//! MySQL, or SQLite using `sqlx` with the Any driver.
//!
//! Connection details are resolved from the `credential_id` stored on the
//! node config (via `CredentialManager::get_database_connection_string`).

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;
use sqlx::Row;
use sqlx::any::AnyPoolOptions;

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

/// P1-15: 检测 SQL 字符串中是否含危险字符（用于拒绝未参数化的字符串拼接）。
/// 允许的参数占位符：?、$1..$N、:name。
fn contains_unsafe_concat(sql: &str) -> bool {
    // 简单启发式：查找单引号内嵌的"+"号、或 SQL 字符串中有 " ' " + " 模式。
    // 这里不期望完美 —— 真正的防御是强制占位符 + AST 解析。
    let mut in_quote = false;
    let mut prev = '\0';
    for c in sql.chars() {
        if c == '\'' {
            in_quote = !in_quote;
        } else if in_quote && c == '+' && prev == '\'' {
            return true;
        } else if !in_quote && c == '"' {
            // 字符串字面量外的双引号：可能是 SQL 注入载体
            return true;
        }
        prev = c;
    }
    false
}

/// P1-15: DDL/DML 关键字检查（只允许 SELECT 等只读查询；INSERT/UPDATE/DELETE
/// 必须显式开启 `allow_writes` 配置）。
const DDL_DML_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "DROP", "TRUNCATE", "ALTER", "CREATE", "REPLACE", "GRANT",
    "REVOKE",
];

/// 去掉 SQL 注释和多余空白，便于做关键字匹配。
fn normalize_sql(sql: &str) -> String {
    let mut s = sql.to_string();
    // 去掉行注释
    if let Some(idx) = s.find("--") {
        s.truncate(idx);
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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

        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(&conn_str)
            .await
            .map_err(|e| NodeError::exec_failed("DATABASE_CONNECT_FAILED", e.to_string()))?;

        // P1-15: SQL 注入防御 —— 拒绝未参数化的字符串拼接
        if contains_unsafe_concat(&c.query) {
            let _ = pool.close().await;
            return Err(NodeError::exec_failed(
                "DATABASE_UNSAFE_QUERY",
                "Query contains unsafe string concatenation (e.g. 'foo' + bar). \
                 Use parameterized placeholders (? / $1 / :name) instead."
                    .to_string(),
            ));
        }

        // P1-15: DDL/DML 权限校验 —— 只允许 SELECT 除非显式 allow_writes
        // 注：当前 DatabaseQueryNode 配置未必有 allow_writes 字段，按 SELECT-only 默认
        let normalized = normalize_sql(&c.query);
        let upper_first = normalized.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
        if DDL_DML_KEYWORDS.iter().any(|kw| upper_first == *kw) {
            let _ = pool.close().await;
            return Err(NodeError::exec_failed(
                "DATABASE_WRITE_NOT_ALLOWED",
                format!(
                    "DDL/DML keyword '{upper_first}' is not allowed in databaseQuery node. \
                     Use a dedicated write node if needed."
                ),
            ));
        }

        let rows = sqlx::raw_sql(sqlx::AssertSqlSafe(c.query.clone()))
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
