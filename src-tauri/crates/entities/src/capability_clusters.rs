// SPDX-License-Identifier: AGPL-3.0-only
//! 能力集群实体 — L2 簇路由规则的持久化存储
//!
//! 与 harness 层的静态 `CapabilityCluster` 对应，
//! 此实体用于动态配置的集群规则（用户可在 UI 中增删改）。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 能力集群表
///
/// 存储可动态配置的 L2 簇路由规则。
/// harness 层的 `CapabilityCluster` 提供静态默认值，
/// 此表提供运行时可修改的扩展规则。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "capability_clusters")]
pub struct Model {
    /// 集群唯一 ID，格式 `{domain}_{cluster}`，如 `core_file_ops`
    #[sea_orm(primary_key)]
    pub cluster_id: String,
    /// 所属业务域（CapabilityDomain 的 as_str() 值）
    pub domain: String,
    /// 集群中文名称
    pub cluster_name: String,
    /// 集群描述
    pub description: Option<String>,
    /// 路径段（用于 RoutingPath）
    pub path_segment: String,
    /// 关键词列表（JSON 数组，用于从能力护照推导所属集群）
    #[sea_orm(column_type = "Text")]
    pub keywords: String,
    /// 排除关键词列表（JSON 数组，命中则不匹配）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[sea_orm(column_type = "Text", nullable)]
    pub exclude_keywords: Option<String>,
    /// 规则优先级（数字越大优先级越高）
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 是否为内置预设（不可删除）
    #[serde(default)]
    pub is_preset: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

fn default_priority() -> i32 {
    50
}

fn default_enabled() -> bool {
    true
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
