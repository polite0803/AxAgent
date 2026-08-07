// SPDX-License-Identifier: AGPL-3.0-only

//! OPC Playbook 实体（Self-Grown 晋升共享经验）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_playbooks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub role_id: String,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    /// 晋升来源（经验记录 id / 员工 id）
    pub promoted_from: Option<String>,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
