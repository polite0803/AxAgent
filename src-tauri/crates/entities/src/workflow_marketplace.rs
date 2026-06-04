use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_marketplace")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub template_id: String,
    pub author_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub icon: String,
    pub tags: Option<String>,
    pub downloads: i64,
    pub rating_average: f64,
    pub rating_count: i32,
    pub is_featured: bool,
    pub is_verified: bool,
    pub is_public: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
