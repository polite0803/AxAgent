// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "wiki_edit_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub conversation_id: Option<String>,
    pub wiki_id: String,
    pub note_id: String,
    pub operation: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub before_snippet: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub after_snippet: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub reason: Option<String>,
    pub quality_score: Option<f64>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
