// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 发票表实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_invoices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub customer_id: String,
    pub invoice_number: String,
    pub status: String,
    #[sea_orm(column_type = "Text")]
    pub line_items_json: String,
    pub subtotal: f64,
    pub tax_total: f64,
    pub total: f64,
    pub currency: String,
    pub issued_at: Option<i64>,
    pub due_at: Option<i64>,
    pub paid_at: Option<i64>,
    #[sea_orm(column_type = "Text")]
    pub notes: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
