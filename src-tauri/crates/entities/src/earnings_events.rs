use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 财报披露事件(R3-B)
///
/// 每条记录对应一只股票的一次财报披露事件(业绩预告/快报/正式财报/股东大会)。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "earnings_events")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub stock_code: String,
    pub stock_name: String,
    /// YYYY-MM-DD
    pub event_date: String,
    /// "preliminary" | "express" | "formal" | "shareholders_meeting" | "other"
    pub event_type: String,
    /// 可选:财报期间(2025Q3 / 2025年报)
    pub period: Option<String>,
    /// 可选:摘要/标题
    pub detail: Option<String>,
    /// vendor 标识
    pub source: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
