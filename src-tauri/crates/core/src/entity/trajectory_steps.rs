use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "trajectory_steps")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub trajectory_id: String,
    pub step_index: i32,
    pub timestamp_ms: i64,
    pub role: String,
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Option<String>,
    pub tool_results: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
