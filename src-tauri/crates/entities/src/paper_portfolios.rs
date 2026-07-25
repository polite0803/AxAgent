// SPDX-License-Identifier: AGPL-3.0-only
//! 模拟观察组合（Paper Trading Portfolio）实体
//!
//! 对应迁移：v204_paper_portfolio
//! 用途：承接 DojoAgents 场景 1/2/3 的研究观察列表 / 模拟建仓实体：
//! - 场景 1：把市场异动摘要沉淀成研究观察列表
//! - 场景 2：按消息发布日价格虚拟建仓
//! - 场景 3：从持仓诊断结果生成新观察列表

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 模拟组合主表（一个组合 = 一次研究观察）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "paper_portfolios")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 组合名称（用户输入或自动生成）
    pub name: String,
    /// 来源事件描述（如 "英伟达隔夜大跌" / "Meta 卖算力"）
    pub source_event: String,
    /// 关联 news_archive.id，实现新闻→组合溯源（可空）
    pub source_news_id: Option<String>,
    /// 关联 screenshot_diagnoses.id（G6 用，可空）
    pub source_screenshot_diagnosis_id: Option<String>,
    /// active / closed / archived
    #[sea_orm(default_value = "active")]
    pub status: String,
    /// 创建时间戳（ms）
    pub created_at: i64,
    /// 关闭时间戳（ms，可空）
    pub closed_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 一个组合有多个虚拟持仓
    #[sea_orm(has_many = "super::paper_positions::Entity")]
    PaperPositions,
}

impl Related<super::paper_positions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PaperPositions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
