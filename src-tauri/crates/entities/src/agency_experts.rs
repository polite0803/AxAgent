use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "agency_experts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    #[sea_orm(column_name = "system_prompt")]
    pub system_prompt: String,
    pub color: Option<String>,
    #[sea_orm(column_name = "source_dir")]
    pub source_dir: String,
    pub is_enabled: i32,
    pub imported_at: i64,
    #[sea_orm(column_name = "recommended_workflows")]
    pub recommended_workflows: Option<String>,
    #[sea_orm(column_name = "recommended_tools")]
    pub recommended_tools: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
