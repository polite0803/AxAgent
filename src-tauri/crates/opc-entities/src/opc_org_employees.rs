// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 组织员工实体（Self-Built 组织抽象）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "opc_org_employees")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub org_id: String,
    /// 员工 id（agent id 或占位）
    pub employee_id: String,
    pub role_id: String,
    /// 绑定的专家 id（agency_experts / opc-xxx）
    pub expert_id: Option<String>,
    /// 状态（active/on_leave/terminated）
    pub status: String,
    /// 经验档案引用（opc_experience_records 关联 key）
    pub experience_ref: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
