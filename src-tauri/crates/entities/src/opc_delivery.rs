// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 交付记录表实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_delivery")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub lead_id: Option<String>,
    pub project_id: Option<String>,
    pub customer_id: Option<String>,
    pub title: String,
    pub workflow_template_id: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub status: String,
    pub progress: f64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub result_summary: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub deliverables_json: String,
    #[sea_orm(column_type = "Text")]
    pub errors_json: String,
    #[sea_orm(column_type = "Text")]
    pub metadata_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
