// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 行业注册表实体（Industry Pack 扫描/启用/禁用/版本追踪）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_industries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub icon: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub version: i32,
    pub enabled: i32,
    #[sea_orm(column_type = "Text")]
    pub pack_path: String,
    pub installed_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
