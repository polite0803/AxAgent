// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 能力清单快照表实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_capability")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub source_type: String,
    pub source_id: String,
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub capability_type: String,
    #[sea_orm(column_type = "Text")]
    pub applicable_scenarios_json: String,
    #[sea_orm(column_type = "Text")]
    pub example_deliverables_json: String,
    #[sea_orm(column_type = "Text")]
    pub metadata_json: String,
    pub is_active: i32,
    pub scanned_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
