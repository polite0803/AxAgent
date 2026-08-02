// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 组织实体（Self-Built 组织抽象）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_orgs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    /// 公司画像（业务/行业/规模描述）
    #[sea_orm(column_type = "Text")]
    pub company_profile: String,
    /// 组织拓扑（扁平/层级/矩阵等）
    pub topology: String,
    /// 最终决策角色 id（CEO 等）
    pub final_decider_role_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
