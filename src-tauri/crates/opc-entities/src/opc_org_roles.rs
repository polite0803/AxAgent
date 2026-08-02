// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 组织角色实体（Self-Built 组织抽象）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_org_roles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub org_id: String,
    /// 角色 id（映射 agent_roles 或 opc-xxx 专家）
    pub role_id: String,
    pub name: String,
    /// 职责描述
    #[sea_orm(column_type = "Text")]
    pub responsibility: String,
    /// 汇报给的角色 id（None = 最高层）
    pub reports_to: Option<String>,
    /// 资历（junior/mid/senior/lead）
    pub seniority: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
