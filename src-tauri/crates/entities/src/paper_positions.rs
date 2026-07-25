// SPDX-License-Identifier: AGPL-3.0-only
//! 模拟观察组合持仓（Paper Position）实体
//!
//! 对应迁移：v204_paper_portfolio
//! 用途：模拟组合内的虚拟持仓（按事件日价格建仓）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 模拟组合内的虚拟持仓
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "paper_positions")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 所属组合 ID
    pub portfolio_id: String,
    /// 股票代码
    pub symbol: String,
    /// 市场：A / US / HK / ETF
    #[sea_orm(default_value = "A")]
    pub market: String,
    /// 虚拟建仓价
    pub entry_price: f64,
    /// 虚拟建仓日（YYYY-MM-DD）
    pub entry_date: String,
    /// 虚拟数量
    pub quantity: f64,
    /// 虚拟平仓价（可空）
    pub exit_price: Option<f64>,
    /// 虚拟平仓日（可空）
    pub exit_date: Option<String>,
    /// open / closed
    #[sea_orm(default_value = "open")]
    pub status: String,
    /// 备注（如 "AI 算力链" / "光模块龙头"）
    pub note: Option<String>,
    /// 创建时间戳（ms）
    pub created_at: i64,
    /// 更新时间戳（ms）
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多个持仓属于一个组合
    #[sea_orm(
        belongs_to = "super::paper_portfolios::Entity",
        from = "Column::PortfolioId",
        to = "super::paper_portfolios::Column::Id"
    )]
    PaperPortfolio,
}

impl Related<super::paper_portfolios::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PaperPortfolio.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
