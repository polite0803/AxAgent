// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 同步设备表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sync_devices")]
pub struct Model {
    /// 设备 ID（UUID）
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 设备名称
    pub name: String,
    /// 设备类型（desktop/mobile/server）
    pub device_type: String,
    /// 操作系统
    pub os: String,
    /// 应用版本
    pub app_version: String,
    /// 唯一设备标识符
    pub unique_id: String,
    /// 公钥（用于加密通信）
    pub public_key: String,
    /// IP 地址
    pub ip_address: Option<String>,
    /// 配对状态
    pub is_paired: bool,
    /// 受信任级别（backup_only/standard/full）
    pub trust_level: String,
    /// 最后同步时间（Unix 毫秒）
    pub last_synced_at: Option<i64>,
    /// 最后心跳时间（Unix 毫秒）
    pub last_heartbeat_at: Option<i64>,
    /// 是否启用
    pub is_enabled: bool,
    /// 创建时间（Unix 毫秒）
    pub created_at: i64,
    /// 更新时间（Unix 毫秒）
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
