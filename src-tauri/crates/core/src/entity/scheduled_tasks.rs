use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scheduled_tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: String,
    #[sea_orm(column_type = "Text")]
    pub task_type: String,
    pub workflow_id: Option<String>,
    pub cron_expression: Option<String>,
    pub interval_seconds: Option<i64>,
    pub next_run_at: i64,
    pub last_run_at: Option<i64>,
    pub last_result: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub status: String,
    #[sea_orm(column_type = "Text")]
    pub config: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}