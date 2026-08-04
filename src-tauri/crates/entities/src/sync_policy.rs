// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 同步策略表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sync_policies")]
pub struct Model {
    /// 策略 ID（UUID）
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 策略描述
    pub description: Option<String>,
    /// 同步模式（manual/auto/scheduled）
    pub sync_mode: String,
    /// 冲突解决策略
    pub conflict_strategy: String,
    /// 同步间隔（毫秒）
    pub sync_interval_ms: i64,
    /// 允许的实体类型（JSON 数组）
    pub allowed_entity_types: String,
    /// 排除的实体类型（JSON 数组）
    pub excluded_entity_types: String,
    /// 压缩算法
    pub compression_algorithm: String,
    /// 最大传输大小（字节）
    pub max_transfer_size: i64,
    /// 是否加密传输
    pub encryption_enabled: bool,
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
