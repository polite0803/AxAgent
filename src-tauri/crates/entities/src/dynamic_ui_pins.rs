// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 动态页面钉入导航配置（后端持久化版，替代原 localStorage 方案）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dynamic_ui_pins")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub schema_id: String,
    pub title: String,
    pub group_name: String,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
