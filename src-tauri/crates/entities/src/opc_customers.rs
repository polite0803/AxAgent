// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 客户表实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_customers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub source: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub tags_json: String,
    #[sea_orm(column_type = "Text")]
    pub notes: String,
    pub total_revenue: f64,
    pub invoice_count: u32,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
