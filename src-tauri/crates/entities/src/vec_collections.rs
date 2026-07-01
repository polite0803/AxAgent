// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const INDEX_TYPE_FLAT: &str = "flat";
pub const INDEX_TYPE_HNSW: &str = "hnsw";

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "vec_collections")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub collection_id: String,
    pub dimensions: i32,
    #[sea_orm(column_type = "Text", nullable)]
    pub embedding_model: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub index_type: String,
    pub hnsw_ef_construction: Option<i32>,
    pub hnsw_m: Option<i32>,
    pub hnsw_ef_search: Option<i32>,
    pub vector_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_indexed_at: Option<i64>,
    #[sea_orm(column_type = "Text", nullable)]
    pub metadata: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
