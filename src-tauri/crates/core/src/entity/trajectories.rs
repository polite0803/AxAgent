use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "trajectory_trajectories")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub session_id: String,
    pub user_id: String,
    pub topic: String,
    pub summary: String,
    pub outcome: String,
    pub duration_ms: i64,
    pub quality_overall: f64,
    pub quality_task_completion: f64,
    pub quality_tool_efficiency: f64,
    pub quality_reasoning_quality: f64,
    pub quality_user_satisfaction: f64,
    pub value_score: f64,
    pub patterns: String,
    pub created_at: String,
    pub replay_count: i32,
    pub last_replay_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
