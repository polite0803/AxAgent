// SPDX-License-Identifier: AGPL-3.0-only

//! Reading List Item 实体：阅读列表条目（论文/文档/外部链接）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "reading_list_items")]
pub struct Model {
    /// 唯一 ID（UUID）
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 关联 reading_lists.id
    pub reading_list_id: String,
    /// 关联 knowledge_documents.id（允许为空，方便外部链接）
    pub document_id: Option<String>,
    /// 外部链接（arxiv URL 等）
    pub external_url: Option<String>,
    /// 条目标题
    pub title: String,
    /// 用户备注
    #[sea_orm(column_type = "Text", nullable)]
    pub notes: Option<String>,
    /// 阅读状态：unread / reading / read / skipped
    pub status: String,
    /// 优先级 0-100，默认 50
    pub priority: i32,
    /// 在列表中的位置（用于自定义排序）
    pub position: i32,
    /// 任意元数据 JSON 字符串（authors/published_date 等）
    pub metadata_json: String,
    /// 添加时间（Unix 毫秒）
    pub added_at: i64,
    /// 更新时间（Unix 毫秒）
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多个条目属于一个阅读列表
    #[sea_orm(
        belongs_to = "super::reading_lists::Entity",
        from = "Column::ReadingListId",
        to = "super::reading_lists::Column::Id",
        on_delete = "Cascade"
    )]
    ReadingList,
}

impl Related<super::reading_lists::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ReadingList.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
