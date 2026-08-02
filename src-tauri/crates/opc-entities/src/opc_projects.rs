// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 项目表实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_projects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub customer_id: Option<String>,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub status: String,
    #[sea_orm(column_type = "Text")]
    pub milestones_json: String,
    pub budget: Option<f64>,
    pub currency: String,
    pub started_at: Option<i64>,
    pub deadline: Option<i64>,
    pub completed_at: Option<i64>,
    #[sea_orm(column_type = "Text")]
    pub notes: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
