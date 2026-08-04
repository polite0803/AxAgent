// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 设备权限表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sync_permissions")]
pub struct Model {
    /// 权限 ID（UUID）
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 设备 ID
    pub device_id: String,
    /// 信任级别（backup_only/standard/full）
    pub trust_level: String,
    /// 是否允许推送
    pub can_push: bool,
    /// 是否允许拉取
    pub can_pull: bool,
    /// 是否允许全量同步
    pub can_full_sync: bool,
    /// 是否允许解决冲突
    pub can_resolve_conflicts: bool,
    /// 是否允许管理设备
    pub can_manage_devices: bool,
    /// 是否允许修改策略
    pub can_modify_policy: bool,
    /// 权限过期时间（Unix 毫秒，null 表示永不过期）
    pub expires_at: Option<i64>,
    /// 创建时间（Unix 毫秒）
    pub created_at: i64,
    /// 更新时间（Unix 毫秒）
    pub updated_at: i64,
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
