// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core_error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[async_trait]
pub trait DatabaseQueryService: Send + Sync {
    async fn execute_query(&self, conn_str: &str, query: &str) -> Result<DatabaseQueryResult>;
}
