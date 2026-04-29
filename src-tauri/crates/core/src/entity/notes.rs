use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default,
)]
#[sea_orm(table_name = "notes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub vault_id: String,
    pub title: String,
    pub file_path: String,
    pub content: String,
    pub content_hash: String,
    pub author: String,
    pub page_type: Option<String>,
    pub source_refs: Option<Json>,
    pub related_pages: Option<Json>,
    pub quality_score: Option<f64>,
    pub last_linted_at: Option<i64>,
    pub last_compiled_at: Option<i64>,
    pub compiled_source_hash: Option<String>,
    pub user_edited: i32,
    pub user_edited_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_deleted: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::note_links::Entity")]
    NoteLink,
    #[sea_orm(has_many = "super::note_backlinks::Entity")]
    Backlink,
}

impl Related<super::note_links::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NoteLink.def()
    }
}

impl Related<super::note_backlinks::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Backlink.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}