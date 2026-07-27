// SPDX-License-Identifier: AGPL-3.0-only

//! Reading List 实体：用户收藏的论文/文档集合

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "reading_lists")]
pub struct Model {
    /// 唯一 ID（UUID）
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 列表名称
    pub name: String,
    /// 描述
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    /// 所有者用户 ID（可选，多用户场景预留）
    pub owner_user_id: Option<String>,
    /// 状态：active / archived
    pub status: String,
    /// 排序序号
    pub sort_order: i32,
    /// 创建时间（Unix 毫秒）
    pub created_at: i64,
    /// 更新时间（Unix 毫秒）
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 一个阅读列表有多个条目
    #[sea_orm(
        has_many = "super::reading_list_items::Entity",
        from = "Column::Id",
        to = "super::reading_list_items::Column::ReadingListId"
    )]
    ReadingListItems,
}

impl Related<super::reading_list_items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ReadingListItems.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
