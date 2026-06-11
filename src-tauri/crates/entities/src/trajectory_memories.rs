// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "trajectory_memories")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub memory_type: String,
    pub content: String,
    pub updated_at: String,
    pub tier: String,
    pub importance: f64,
    pub access_count: i32,
    pub last_accessed: Option<String>,
    pub decay_rate: f64,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub source_conversation_id: Option<String>,
    pub source_message_id: Option<String>,
    pub memory_nature: String,
    pub tags: String,
    pub namespace_id: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
