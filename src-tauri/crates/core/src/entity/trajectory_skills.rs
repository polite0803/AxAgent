use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "trajectory_skills")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: String,
    pub skill_type: String,
    pub content: String,
    pub category: String,
    pub tags: String,
    pub scenarios: String,
    pub parameters: String,
    pub created_at: String,
    pub updated_at: String,
    pub usage_count: i32,
    pub success_rate: f64,
    pub avg_execution_time_ms: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
