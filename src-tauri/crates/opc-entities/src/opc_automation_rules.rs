// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 自动化规则表实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_automation_rules")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub trigger_type: String,
    #[sea_orm(column_type = "Text")]
    pub trigger_config: String,
    pub action_type: String,
    #[sea_orm(column_type = "Text")]
    pub action_config: String,
    pub enabled: bool,
    pub last_run_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
