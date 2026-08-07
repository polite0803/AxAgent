// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 人才模板实体（Self-Built 招聘决策数据源）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_talent_templates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub category: String,
    pub name: String,
    /// 描述
    #[sea_orm(column_type = "Text")]
    pub description: String,
    /// 来源仓库（agency-agents-src / 市场包）
    pub source_repo: String,
    /// 提示词引用（json 数组）
    pub prompt_refs: Option<String>,
    /// 技能引用（json 数组）
    pub skill_refs: Option<String>,
    /// 标签（json 数组）
    pub tags: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
