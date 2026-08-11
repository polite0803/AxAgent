// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "wikis")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub root_path: String,
    pub schema_version: String,
    pub note_count: i32,
    pub source_count: i32,
    pub embedding_provider: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub retrieval_threshold: Option<f32>,
    pub retrieval_top_k: Option<i32>,
    /// v118: 关联的知识库 ID，建立 Wiki 与 KB 的 1:1 关联
    #[sea_orm(column_name = "knowledge_base_id")]
    pub knowledge_base_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
