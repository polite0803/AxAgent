use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_marketplace_reviews")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub marketplace_id: String,
    pub user_id: String,
    pub rating: i32,
    pub comment: Option<String>,
    pub is_hidden: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workflow_marketplace::Entity",
        from = "Column::MarketplaceId",
        to = "super::workflow_marketplace::Column::Id"
    )]
    Marketplace,
}

impl ActiveModelBehavior for ActiveModel {}
