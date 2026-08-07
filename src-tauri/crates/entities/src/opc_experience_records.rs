// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 经验记录实体（Self-Grown 经验归因）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_experience_records")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 角色 id（只更新拥有相关工作项的角色档案，防互相污染）
    pub role_id: String,
    pub work_item_id: String,
    /// 信号（success/failure/feedback）
    pub signal: String,
    /// 经验内容（反思/教训）
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
