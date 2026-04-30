use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "plans")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub conversation_id: String,
    pub user_message_id: String,
    pub title: String,
    /// JSON-encoded Vec<PlanStep>
    pub steps_json: String,
    pub status: String,
    pub is_active: i32,
    pub created_under_strategy: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::conversations::Entity",
        from = "Column::ConversationId",
        to = "super::conversations::Column::Id"
    )]
    Conversation,
}

impl Related<super::conversations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Conversation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
