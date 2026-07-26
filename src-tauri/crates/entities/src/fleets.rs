// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 舰队（办公室）— 一个正在运行的 AI 团队
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fleets")]
pub struct Model {
    /// 唯一 ID（UUID）
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 场景模板 slug（可选，下游业务系统可填）
    pub scene_template_slug: Option<String>,
    /// 舰队状态：active / paused / stopped
    pub status: String,
    /// 创建时间（Unix 毫秒）
    pub created_at: i64,
    /// 更新时间（Unix 毫秒）
    pub updated_at: i64,
    /// 业务元数据 JSON
    pub metadata_json: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 一个舰队有多个成员
    #[sea_orm(has_many = "super::fleet_members::Entity")]
    FleetMembers,
}

impl Related<super::fleet_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FleetMembers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
