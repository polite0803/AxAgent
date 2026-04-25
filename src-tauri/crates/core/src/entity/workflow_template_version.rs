use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_template_versions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub template_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Option<String>,
    pub version: i32,
    pub is_preset: bool,
    pub is_editable: bool,
    pub is_public: bool,
    pub trigger_config: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub nodes: String,
    #[sea_orm(column_type = "Text")]
    pub edges: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
    pub variables: Option<String>,
    pub error_config: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
