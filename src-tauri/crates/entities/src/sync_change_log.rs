// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 变更日志表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sync_change_logs")]
pub struct Model {
    /// 变更 ID（UUID）
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 发起设备 ID
    pub device_id: String,
    /// 实体类型（conversation/message/setting 等）
    pub entity_type: String,
    /// 实体 ID
    pub entity_id: String,
    /// 操作类型（insert/update/delete）
    pub operation: String,
    /// 操作数据（JSON）
    pub data: String,
    /// 版本号（用于 CRDT）
    pub version: i64,
    /// 父版本 ID（可选）
    pub parent_version_id: Option<String>,
    /// 变更时间（Unix 毫秒）
    pub created_at: i64,
    /// 是否已同步
    pub is_synced: bool,
    /// 同步到的设备列表（JSON 数组）
    pub synced_to: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 属于一个设备
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
