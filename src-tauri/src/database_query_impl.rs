// SPDX-License-Identifier: AGPL-3.0-only

//! DatabaseQueryService 的 sqlx 实现。连接外部数据库并执行通用 SQL 查询。

use async_trait::async_trait;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::{DatabaseQueryResult, DatabaseQueryService};
use sqlx::Column;
use sqlx::Row;
use sqlx::any::AnyPoolOptions;

pub struct SqlxDatabaseQueryService;

impl SqlxDatabaseQueryService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SqlxDatabaseQueryService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseQueryService for SqlxDatabaseQueryService {
    async fn execute_query(&self, conn_str: &str, query: &str) -> Result<DatabaseQueryResult> {
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(conn_str)
            .await
            .map_err(|e| AxAgentError::internal(e.to_string()))?;

        // sqlx::query 需要 &'static str，使用 leak 延长生命周期
        let query_static: &'static str = query.to_string().leak();
        let rows = sqlx::query(query_static)
            .fetch_all(&pool)
            .await
            .map_err(|e| AxAgentError::internal(format!("Database query failed: {}", e)))?;

        let mut columns = Vec::new();
        if let Some(first_row) = rows.first() {
            for col in first_row.columns() {
                columns.push(col.name().to_string());
            }
        }

        let mut result_rows: Vec<Vec<serde_json::Value>> = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut vals: Vec<serde_json::Value> = Vec::with_capacity(columns.len());
            for i in 0..columns.len() {
                let val = if let Ok(v) = row.try_get::<i64, _>(i) {
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
                vals.push(val);
            }
            result_rows.push(vals);
        }

        Ok(DatabaseQueryResult { columns, rows: result_rows })
    }
}
