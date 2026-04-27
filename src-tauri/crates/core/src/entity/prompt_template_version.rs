use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Eq, PartialOrd, Ord,
)]
#[sea_orm(table_name = "prompt_template_versions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub template_id: String,
    pub version: i32,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub variables_schema: Option<String>,
    pub changelog: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
