use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "knowledge_attributes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub knowledge_base_id: String,
    pub entity_id: String,
    pub name: String,
    pub attribute_type: String,
    pub data_type: String,
    pub description: Option<String>,
    pub is_required: bool,
    pub default_value: Option<String>,
    pub constraints: Option<Json>,
    pub validation_rules: Option<Json>,
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
    #[sea_orm(
        belongs_to = "super::knowledge_entities::Entity",
        from = "Column::EntityId",
        to = "super::knowledge_entities::Column::Id",
        on_delete = "Cascade"
    )]
    KnowledgeEntity,
}

impl Related<super::knowledge_bases::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::KnowledgeBase.def()
    }
}

impl Related<super::knowledge_entities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::KnowledgeEntity.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
