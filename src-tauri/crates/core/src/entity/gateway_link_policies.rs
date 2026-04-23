use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gateway_link_policies")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub link_id: String,
    pub route_strategy: String,
    pub model_fallback_enabled: i32,
    pub global_rpm: Option<i64>,
    pub per_model_rpm: Option<i64>,
    pub token_limit_per_minute: Option<i64>,
    pub key_rotation_strategy: String,
    pub key_failover_enabled: i32,
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
