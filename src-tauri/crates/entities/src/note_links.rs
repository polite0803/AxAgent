use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "note_links")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub vault_id: String,
    pub source_note_id: String,
    pub target_note_id: String,
    pub link_text: String,
    pub link_type: String,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::notes::Entity",
        from = "Column::SourceNoteId",
        to = "super::notes::Column::Id",
        on_delete = "Cascade"
    )]
    SourceNote,
    #[sea_orm(
        belongs_to = "super::notes::Entity",
        from = "Column::TargetNoteId",
        to = "super::notes::Column::Id",
        on_delete = "Cascade"
    )]
    TargetNote,
}

impl Related<super::notes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SourceNote.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
