use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "wiki_pages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub wiki_id: String,
    pub note_id: String,
    pub page_type: String,
    pub title: String,
    pub source_ids: Option<Json>,
    pub quality_score: Option<f64>,
    pub last_linted_at: Option<i64>,
    pub last_compiled_at: i64,
    pub compiled_source_hash: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::notes::Entity",
        from = "Column::NoteId",
        to = "super::notes::Column::Id",
        on_delete = "Cascade"
    )]
    Note,
}

impl Related<super::notes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Note.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}