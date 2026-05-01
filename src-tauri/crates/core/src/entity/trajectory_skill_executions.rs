use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "trajectory_skill_executions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub skill_id: String,
    pub trajectory_id: Option<String>,
    pub success: i32,
    pub execution_time_ms: i64,
    pub created_at: String,
    pub input_args: Option<String>,
    pub output_result: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
