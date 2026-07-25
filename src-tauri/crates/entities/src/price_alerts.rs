use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "price_alerts")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub stock_code: String,
    pub stock_name: String,
    /// 老字段（兼容保留）: "above" 或 "below"
    /// 新代码应读 [`Self::alert_type`]，本字段仅在迁移过渡期使用。
    pub condition: String,
    /// 老字段（兼容保留）: 仅 price 类告警有效。
    /// 新代码应读 [`Self::threshold`]。
    pub target_price: f64,
    /// 新字段（v203）: 对齐 RealtimeMonitor 的 6 类 alert_type
    /// 取值: `stop_loss` / `take_profit` / `resistance` / `support` / `change` / `volume`
    /// 老数据在 v203 迁移中已回填，可安全读取。
    pub alert_type: Option<String>,
    /// 新字段（v203）: 阈值语义
    /// 取值: `price` / `change_pct` / `turnover_rate`
    pub condition_type: Option<String>,
    /// 新字段（v203）: 通用阈值
    /// - condition_type=price → 绝对价格
    /// - condition_type=change_pct → 涨跌幅百分比
    /// - condition_type=turnover_rate → 换手率百分比
    pub threshold: Option<f64>,
    pub is_triggered: i32,
    pub triggered_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
