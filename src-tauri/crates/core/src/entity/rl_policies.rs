use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "rl_policies")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub policy_type: String,
    pub model_id: String,
    pub reward_signals_json: String,
    pub experiences_json: String,
    pub total_experiences: i32,
    pub episodes_completed: i32,
    pub avg_reward: f32,
    pub last_update: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
