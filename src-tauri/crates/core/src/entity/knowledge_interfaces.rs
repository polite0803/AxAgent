use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "knowledge_interfaces")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub knowledge_base_id: String,
    pub name: String,
    pub interface_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub input_schema: Json,
    pub output_schema: Json,
    pub error_codes: Option<Json>,
    pub communication_pattern: Option<String>,
    pub version: Option<String>,
    pub metadata: Option<Json>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::knowledge_bases::Entity",
        from = "Column::KnowledgeBaseId",
        to = "super::knowledge_bases::Column::Id",
        on_delete = "Cascade"
    )]
    KnowledgeBase,
}

impl Related<super::knowledge_bases::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::KnowledgeBase.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
