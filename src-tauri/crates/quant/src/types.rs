//! 核心数据类型
//!
//! - `Bar`: 字段对齐 `axagent_harness::market_data::KLine`，
//!   扩展涨跌停上下限 + ST 标记（来自 StockQuote）
//! - `Order` / `Fill`: 订单与成交回报
//! - `Signal` / `SignalAction` / `CloseReason`: 策略信号
//! - `Side` / `OrderType`: 持仓方向与订单类型

use serde::{Deserialize, Serialize};

use axagent_harness::market_data::{KLine, StockQuote};

use crate::error::QuantError;

/// 统一 K 线结构
///
/// 字段对齐 `axagent_harness::market_data::KLine`，额外扩展：
/// - `code`: 股票代码（多标的回测时由 Engine 注入）
/// - `limit_up` / `limit_down`: 涨跌停价（来自 StockQuote，未载入时为 None）
/// - `is_st`: 是否 ST/*ST 股票
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bar {
    pub date: String,
    pub code: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub amount: f64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub turnover_rate: Option<f64>,
    /// 累计复权因子；None 表示未应用复权
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub adj_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limit_up: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limit_down: Option<f64>,
    #[serde(default)]
    pub is_st: bool,
}

impl Bar {
    /// 从 KLine 构造（无涨跌停信息，用于纯 K 线回测）
    pub fn from_kline(code: impl Into<String>, k: &KLine) -> Self {
        Self {
            code: code.into(),
            date: k.date.clone(),
            open: k.open,
            high: k.high,
            low: k.low,
            close: k.close,
            volume: k.volume,
            amount: k.amount,
            turnover_rate: k.turnover_rate,
            adj_factor: k.adj_factor,
            limit_up: None,
            limit_down: None,
            is_st: false,
        }
    }

    /// 从 KLine + StockQuote 构造（带涨跌停上下限，撮合器依赖此信息）
    pub fn from_kline_with_quote(code: impl Into<String>, k: &KLine, q: &StockQuote) -> Self {
        Self {
            code: code.into(),
            date: k.date.clone(),
            open: k.open,
            high: k.high,
            low: k.low,
            close: k.close,
            volume: k.volume,
            // 优先使用 quote.amount（更准，包含集合竞价）
            amount: if q.amount > 0.0 { q.amount } else { k.amount },
            turnover_rate: k.turnover_rate.or(Some(q.turnover_rate)),
            adj_factor: k.adj_factor,
            limit_up: q.limit_up,
            limit_down: q.limit_down,
            is_st: q.is_st,
        }
    }

    /// 收盘价是否触及涨停（含误差容忍）
    pub fn is_limit_up(&self) -> bool {
        match self.limit_up {
            Some(lu) if lu > 0.0 => {
                (self.close - lu).abs() < 0.0001 * lu.max(1.0) || self.close >= lu
            },
            _ => false,
        }
    }

    /// 收盘价是否触及跌停
    pub fn is_limit_down(&self) -> bool {
        match self.limit_down {
            Some(ld) if ld > 0.0 => {
                (self.close - ld).abs() < 0.0001 * ld.max(1.0) || self.close <= ld
            },
            _ => false,
        }
    }

    /// 校验 Bar 数据合理性（撮合器在写入时调用）
    pub fn validate(&self) -> Result<(), QuantError> {
        if self.open <= 0.0 || self.high <= 0.0 || self.low <= 0.0 || self.close <= 0.0 {
            return Err(QuantError::Data(format!(
                "Bar 含非法价格: code={} date={} O={} H={} L={} C={}",
                self.code, self.date, self.open, self.high, self.low, self.close
            )));
        }
        if self.high < self.low {
            return Err(QuantError::Data(format!(
                "Bar H<L: code={} date={} H={} L={}",
                self.code, self.date, self.high, self.low
            )));
        }
        if self.close > self.high + 1e-6 || self.close < self.low - 1e-6 {
            return Err(QuantError::Data(format!(
                "Bar 收盘价超出 H/L 范围: code={} date={} C={} H={} L={}",
                self.code, self.date, self.close, self.high, self.low
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Long,
    Short,
    Flat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum OrderType {
    /// 市价单
    /// - 回测：下一根 K 线开盘价成交（避免偷看未来）
    /// - 实盘：直接报单，按对手价成交
    Market,
    /// 限价单
    /// - 回测：当根 K 线 H/L 触及限价时按限价成交
    /// - 实盘：挂单等待
    Limit { price: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub code: String,
    pub side: Side,
    /// 数量（A 股 100 的整数倍，撮合器负责整手校验）
    pub quantity: u64,
    pub order_type: OrderType,
    /// ISO 8601 时间戳（回测时为 bar.date）
    pub timestamp: String,
    pub reason: String,
}

/// 策略信号
///
/// Strategy::on_bar 返回 0..N 个 Signal；
/// Engine 收集本 bar 全部 Signal 后转 Order，再交由 Matcher 撮合。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Signal {
    pub code: String,
    pub action: SignalAction,
    /// 信号强度 0..1，撮合器按 strength 排序
    pub strength: f64,
    pub reason: String,
    /// 目标权重（0..1），仅在策略使用 target-weight 模式时设置
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub target_weight: Option<f64>,
    /// 平仓原因（仅 action=Sell 时有效）
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub close_reason: Option<CloseReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalAction {
    Buy,
    Sell,
    Hold,
}

/// 平仓原因（用于绩效归因 + UI 展示）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    TakeProfit,
    StopLoss,
    SignalReverse,
    RiskControl,
    EndOfBacktest,
    Manual,
}

/// 成交回报
///
/// 撮合器对每张 Order 返回一个 Fill。
/// `matched=false` 表示撤单/未成交（涨跌停不可买入/卖出、资金不足、停牌等）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fill {
    pub order: Order,
    /// 实际成交价（已含滑点）
    pub fill_price: f64,
    /// 实际成交金额 = fill_price * quantity
    pub fill_amount: f64,
    pub commission: f64,
    /// 印花税（仅卖出收取）
    pub stamp_tax: f64,
    /// 滑点损失（与 fill_price 与理论价的差）
    pub slippage: f64,
    pub timestamp: String,
    pub matched: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reject_reason: Option<String>,
}
