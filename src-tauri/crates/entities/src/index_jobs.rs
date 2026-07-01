// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "index_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(column_type = "Text")]
    pub job_type: String,
    #[sea_orm(column_type = "Text")]
    pub container_type: String,
    pub container_id: String,
    pub item_id: String,
    #[sea_orm(column_type = "Text")]
    pub status: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub current_stage: Option<String>,
    pub progress: i32,
    #[sea_orm(column_type = "Text", nullable)]
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub priority: i32,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    #[sea_orm(column_type = "Text", nullable)]
    pub metadata: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
