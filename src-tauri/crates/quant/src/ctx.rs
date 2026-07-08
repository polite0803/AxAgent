//! 策略运行时上下文
//!
//! StrategyCtx 由 Engine / LiveRunner 维护，
//! Strategy 在 `on_bar` 中**只读访问**，由 Engine 在撮合完成后**回写**。
//!
//! 关键字段：
//! - `cash` / `positions` / `equity_curve`: 资金与权益
//! - `bar_history`: 每只股票的历史 K 线（策略自行计算指标用）
//! - `indicators`: 预计算指标缓存 `(code, indicator) -> values`
//! - `asof_date` / `is_replay`: AsOf 时间锚（与 astock-data 联动）
//! - `trades`: 全部成交记录（用于绩效归因）

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::{Bar, Order, Side};

/// 策略运行上下文
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyCtx {
    /// 现金（元）
    pub cash: f64,
    /// 持仓表（code -> Position）
    pub positions: HashMap<String, Position>,
    /// 每只股票的历史 K 线（按时间正序）
    pub bar_history: HashMap<String, Vec<Bar>>,
    /// 预计算指标（key 格式 `{code}|{indicator_name}`，如 `600519|MA5`）
    pub indicators: HashMap<String, Vec<f64>>,
    /// 当前回测/复盘日期（YYYY-MM-DD）
    pub current_date: String,
    /// ISO 8601 时间戳
    pub current_time: String,
    /// 是否为复盘 / replay 模式
    pub is_replay: bool,
    /// AsOf 时间锚（replay / backtest_sweep 模式下设置）
    pub asof_date: Option<String>,
    /// 待撮合订单（本 bar 撮合后由 Engine 清空）
    pub pending_orders: Vec<Order>,
    /// 累计已实现盈亏（元）
    pub realized_pnl: f64,
    /// 累计已付佣金
    pub commission_paid: f64,
    /// 累计已付印花税
    pub stamp_tax_paid: f64,
    /// 累计滑点损失
    pub slippage_paid: f64,
    /// 全部成交记录
    pub trades: Vec<Trade>,
    /// 权益曲线点（撮合后由 Engine 追加）
    pub equity_curve: Vec<EquityPoint>,
}

impl StrategyCtx {
    pub fn new(initial_cash: f64) -> Self {
        Self {
            cash: initial_cash,
            ..Default::default()
        }
    }

    /// 当前总权益 = 现金 + 持仓市值
    pub fn total_equity(&self) -> f64 {
        let position_value: f64 = self.positions.values().map(|p| p.market_value).sum();
        self.cash + position_value
    }

    /// 获取指定股票持仓
    pub fn position(&self, code: &str) -> Option<&Position> {
        self.positions.get(code)
    }

    /// 持仓代码列表
    pub fn position_codes(&self) -> Vec<String> {
        self.positions.keys().cloned().collect()
    }
}

/// 持仓
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    pub side: Side,
    /// 持仓数量（股，A 股 100 的整数倍）
    pub quantity: u64,
    /// 加权平均成本价
    pub cost_basis: f64,
    /// 最新价（回测时由 Engine 用 bar.close 更新）
    pub last_price: f64,
    /// 持仓市值 = last_price * quantity
    pub market_value: f64,
    /// 浮动盈亏（未实现）
    pub unrealized_pnl: f64,
    /// 累计已实现盈亏（仅平仓部分加总）
    pub realized_pnl: f64,
    /// 建仓日期
    pub entry_date: String,
    /// 建仓时间戳
    pub entry_timestamp: String,
}

impl Position {
    /// 浮动盈亏率
    pub fn unrealized_pnl_pct(&self) -> f64 {
        if self.cost_basis <= 0.0 {
            0.0
        } else {
            (self.last_price - self.cost_basis) / self.cost_basis
        }
    }
}

/// 成交记录（含手续费、滑点）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub code: String,
    pub side: Side,
    pub quantity: u64,
    pub price: f64,
    pub amount: f64,
    pub commission: f64,
    pub stamp_tax: f64,
    pub slippage: f64,
    pub timestamp: String,
    pub reason: String,
    /// 该笔对应的已实现盈亏（开仓为 0，平仓时为该笔对应的盈亏）
    #[serde(default)]
    pub realized_pnl: f64,
}

/// 权益曲线点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityPoint {
    pub date: String,
    /// 总资产 = 现金 + 持仓市值
    pub equity: f64,
    pub cash: f64,
    pub position_value: f64,
}
