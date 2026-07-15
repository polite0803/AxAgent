// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_approvals")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub execution_id: String,
    pub node_id: String,
    pub status: String,
    pub title: String,
    pub message: String,
    pub approver: Option<String>,
    pub channels: Option<String>,
    pub payload: Option<String>,
    pub decision: Option<String>,
    pub approver_actual: Option<String>,
    pub comment: Option<String>,
    pub timeout_secs: i64,
    pub expires_at: i64,
    pub created_at: i64,
    pub resolved_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
