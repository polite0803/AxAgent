use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "atomic_skills")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
    pub entry_type: String,
    pub entry_ref: String,
    pub category: String,
    pub tags: Option<String>,
    pub version: String,
    pub enabled: bool,
    pub source: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
