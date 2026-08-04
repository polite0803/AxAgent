// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 同步审计日志表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sync_audit_logs")]
pub struct Model {
    /// 审计 ID（UUID）
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 操作类型
    pub action: String,
    /// 操作目标类型
    pub target_type: String,
    /// 操作目标 ID
    pub target_id: String,
    /// 执行操作的设备 ID
    pub actor_device_id: String,
    /// 是否成功
    pub is_successful: bool,
    /// 操作详情（JSON）
    pub details: Option<String>,
    /// 错误信息
    pub error_message: Option<String>,
    /// 创建时间（Unix 毫秒）
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
