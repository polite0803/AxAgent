// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_templates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Option<String>,
    pub version: i32,
    pub is_preset: bool,
    pub is_editable: bool,
    pub is_public: bool,
    pub trigger_config: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub nodes: String,
    #[sea_orm(column_type = "Text")]
    pub edges: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
    pub variables: Option<String>,
    pub error_config: Option<String>,
    pub composite_source: Option<String>,
    pub tool_defs: Option<String>,
    /// mission 哈希（SHA-256），用于 compile_mission_to_template 去重缓存。
    /// 当此模板由 mission 编译生成时填充；手动创建的模板该字段为 NULL。
    #[sea_orm(column_name = "mission_hash")]
    pub mission_hash: Option<String>,
    /// L2 集群 ID（三层路由第二层，对应 CapabilityCluster::cluster_id）
    #[sea_orm(column_name = "cluster_id")]
    pub cluster_id: Option<String>,
    /// 路由路径（三层路由地址，格式 /{domain}/{cluster}/{capability}）
    #[sea_orm(column_name = "route_path")]
    pub route_path: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
