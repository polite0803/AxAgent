// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Eq, PartialOrd, Ord,
)]
#[sea_orm(table_name = "prompt_templates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub variables_schema: Option<String>,
    pub version: i32,
    pub is_active: bool,
    pub ab_test_enabled: bool,
    pub ab_test_variant: Option<String>,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub author: Option<String>,
    pub source: Option<String>,
    pub source_type: Option<String>,
    pub format: Option<String>,
    pub metadata_json: Option<String>,
    pub usage_count: i32,
    pub is_favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
