// SPDX-License-Identifier: AGPL-3.0-only

//! 能力关系表 —— 统一能力模型第四层（CapabilityRelationship）的持久化载体。
//!
//! # 定位
//! 护照 `upstream`/`downstream` 字段是声明式内联依赖（一跳检索扩展的数据源）；
//! 本表是其**物化镜像**（启动时从护照声明重建）+ 关系元信息载体
//! （relationship_type / weight / context / metadata），供：
//! - 关系查询与审计（"哪些能力依赖 X"、"A 和 B 是什么关系"）
//! - 未来图遍历/分析（当前检索多跳 BFS 仍以内存护照图为源，不查库）
//! - 运行时注册的额外关系（预留，与护照声明并存）
//!
//! # 主键
//! 图边自然主键 = (source_id, target_id, relationship_type) 复合主键，
//! 避免 SQLite 下 BIGINT PRIMARY KEY 不自增的坑；upsert 用 ON CONFLICT。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "capability_relationships")]
pub struct Model {
    /// 源能力 ID（如 `tool:read_file`）
    #[sea_orm(primary_key)]
    #[sea_orm(column_name = "source_id")]
    pub source_id: String,
    /// 目标能力 ID
    #[sea_orm(primary_key)]
    #[sea_orm(column_name = "target_id")]
    pub target_id: String,
    /// 关系类型（snake_case：depends_on / uses / alternative_to / conflicts_with / parent_of / precedes / follows / requires_knowledge / superseded_by）
    #[sea_orm(primary_key)]
    #[sea_orm(column_name = "relationship_type")]
    pub relationship_type: String,
    /// 关系权重（0.0-1.0，检索排序用；默认 1.0）
    pub weight: f64,
    /// 关系描述上下文
    #[sea_orm(column_name = "context")]
    pub context: Option<String>,
    /// 扩展元信息（JSON 对象，TEXT 存储）
    #[sea_orm(column_name = "metadata")]
    pub metadata: Option<String>,
    /// 创建时间（Unix 毫秒）
    #[sea_orm(column_name = "created_at")]
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
