// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 同步历史表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sync_histories")]
pub struct Model {
    /// 历史 ID（UUID）
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 设备 ID
    pub device_id: String,
    /// 同步方向（push/pull/both）
    pub direction: String,
    /// 同步类型（full/incremental/manual）
    pub sync_type: String,
    /// 同步结果（JSON）
    pub result: String,
    /// 冲突详情（JSON 数组）
    pub conflicts: String,
    /// 开始时间（Unix 毫秒）
    pub started_at: i64,
    /// 结束时间（Unix 毫秒）
    pub completed_at: i64,
    /// 发起人
    pub initiated_by: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 属于设备
    #[sea_orm(
        belongs_to = "super::sync_device::Entity",
        from = "Column::DeviceId",
        to = "super::sync_device::Column::Id"
    )]
    Device,
}

impl Related<super::sync_device::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Device.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
