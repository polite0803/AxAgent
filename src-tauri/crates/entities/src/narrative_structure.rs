// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 叙事结构持久化实体
///
/// 用于跨会话保存和恢复叙事结构设计，支持模板复用和版本管理。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "narrative_structures")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub genre: String,
    #[sea_orm(column_type = "Text")]
    pub arcs: String,
    #[sea_orm(column_type = "Text")]
    pub confluences: String,
    #[sea_orm(column_type = "Text")]
    pub foreshadows: String,
    pub is_template: bool,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
