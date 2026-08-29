// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;

/// 叙事结构（v126）—— 弧线/交汇点/伏笔的持久化实体。
///
/// `structure` 列存 JSON 文本，与前端 `src/types/narrative.ts` 的
/// `NarrativeStructure`（arcs / confluences / foreshadows）一一对应；
/// 命令层负责 JSON ↔ Value 转换，实体层保持纯文本。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "narrative_structures")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub genre: String,
    pub structure: String,
    pub is_template: bool,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
