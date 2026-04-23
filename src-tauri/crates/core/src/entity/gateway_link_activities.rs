use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gateway_link_activities")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub link_id: String,
    pub activity_type: String,
    pub description: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::gateway_links::Entity",
        from = "Column::LinkId",
        to = "super::gateway_links::Column::Id"
    )]
    GatewayLink,
}

impl Related<super::gateway_links::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GatewayLink.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
