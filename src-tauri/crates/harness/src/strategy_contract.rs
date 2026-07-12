// SPDX-License-Identifier: AGPL-3.0-only

//! 量化策略契约层 — 纯 DTO + Trait 抽象
//!
//! 让 `market-sim` / `quant` 等 consumer crate 通过 trait 共享策略接口与数据类型，
//! 无需相互直接依赖。
//!
//! ## 下沉原因
//!
//! 原本 `Bar` / `Signal` / `Strategy` / `StrategyCtx` 等类型定义在 `axagent-quant`。
//! `market-sim`（consumer）为接入真实策略参与模拟，直接依赖了 `axagent-quant`（consumer），
//! 违反了 AGENTS.md 铁律 2「消费者禁止越过 harness」。
//!
//! 解决方案：将这些共享类型与 trait 下沉到 harness（foundation），
//! `quant` 与 `market-sim` 都通过 `axagent_harness::strategy_contract` 引用。
//!
//! ## 错误处理约定
//!
//! `Strategy` trait 的方法返回 `axagent_harness::core_error::Result<T>`，
//! 即 `Result<T, AxAgentError>`。具体策略实现可保留自有错误类型
//! （如 `QuantError`），通过 `From` 转换为 `AxAgentError` 后用 `?` 自动传播。

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core_error::Result;
use crate::market_data::{KLine, StockQuote};

// ── 核心数据 DTO ───────────────────────────────────────────────────────────

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
    ///
    /// 错误返回 `AxAgentError::Validation`。
    pub fn validate(&self) -> Result<()> {
        use crate::core_error::AxAgentError;
        // P1-2 修复：先检查 NaN/Inf
        // 原因：NaN 与任何数比较都返回 false（NaN <= 0.0 为 false），
        // 导致 NaN 会绕过下面的 price <= 0.0 检查，传播到 sma/ema/rsi 等指标
        // 计算结果，污染 cost_basis、market_value、equity_curve，最终使所有
        // 指标变为 NaN。sharpe_ratio 等函数的 std < 1e-10 检查也无法拦截 NaN。
        // f64::INFINITY 同理（Inf <= 0.0 为 false）。
        // 必须用 is_nan/is_infinite 显式检查。
        let fields = [
            (self.open, "open"),
            (self.high, "high"),
            (self.low, "low"),
            (self.close, "close"),
            (self.volume, "volume"),
        ];
        for (val, name) in fields {
            if val.is_nan() {
                return Err(AxAgentError::Validation(format!(
                    "Bar {} is NaN: code={} date={}",
                    name, self.code, self.date
                )));
            }
            if val.is_infinite() {
                return Err(AxAgentError::Validation(format!(
                    "Bar {} is infinite: code={} date={}",
                    name, self.code, self.date
                )));
            }
        }
        // adj_factor 若存在也需要检查（复权因子为 NaN/Inf 会污染所有复权价格）
        if let Some(adj) = self.adj_factor
            && (adj.is_nan() || adj.is_infinite())
        {
            return Err(AxAgentError::Validation(format!(
                "Bar adj_factor is NaN/Inf: code={} date={} adj_factor={}",
                self.code, self.date, adj
            )));
        }
        if self.open <= 0.0 || self.high <= 0.0 || self.low <= 0.0 || self.close <= 0.0 {
            return Err(AxAgentError::Validation(format!(
                "Bar 含非法价格: code={} date={} O={} H={} L={} C={}",
                self.code, self.date, self.open, self.high, self.low, self.close
            )));
        }
        if self.high < self.low {
            return Err(AxAgentError::Validation(format!(
                "Bar H<L: code={} date={} H={} L={}",
                self.code, self.date, self.high, self.low
            )));
        }
        if self.close > self.high + 1e-6 || self.close < self.low - 1e-6 {
            return Err(AxAgentError::Validation(format!(
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
/// `Strategy::on_bar` 返回 0..N 个 Signal；
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

// ── 策略运行时上下文 ───────────────────────────────────────────────────────

/// 策略运行上下文
///
/// 由 Engine / LiveRunner 维护，
/// Strategy 在 `on_bar` 中**只读访问**，由 Engine 在撮合完成后**回写**。
///
/// 关键字段：
/// - `cash` / `positions` / `equity_curve`: 资金与权益
/// - `bar_history`: 每只股票的历史 K 线（策略自行计算指标用）
/// - `indicators`: 预计算指标缓存 `(code, indicator) -> values`
/// - `asof_date` / `is_replay`: AsOf 时间锚（与 astock-data 联动）
/// - `trades`: 全部成交记录（用于绩效归因）
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyCtx {
    /// 现金（元）
    pub cash: f64,
    /// 持仓表（code -> Position）
    pub positions: HashMap<String, Position>,
    /// 每只股票的历史 K 线（按时间正序）
    pub bar_history: HashMap<String, Vec<Bar>>,
    /// 预计算指标缓存（key 格式 `{code}|{indicator_name}`，如 `600519|MA5`）
    /// 安全性前提：A 股代码为纯数字、指标名为字母数字组合，均不含 `|`，
    /// 故分隔符不会与内容碰撞；若未来支持含 `|` 的代码/指标名需改用不可见分隔符。
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
        Self { cash: initial_cash, ..Default::default() }
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

// ── Strategy Trait ─────────────────────────────────────────────────────────

/// 量化策略接口 — 单代码源
///
/// 同一份策略代码同时跑回测与实盘：
/// - 回测时：`BacktestEngine` 按 K 线序列逐 bar 调用 `on_bar`
/// - 实盘时：`LiveRunner` 订阅行情推送，按 tick/分钟 bar 调用 `on_bar`
///
/// 两条路径共享 trait，保证回测表现与实盘行为一致。
///
/// ## 错误类型
///
/// 返回 `axagent_harness::core_error::Result`（即 `Result<_, AxAgentError>`）。
/// 具体策略实现可保留自有错误类型（如 `QuantError`），通过 `From<Self::Err> for AxAgentError`
/// 实现后用 `?` 自动传播。
#[async_trait]
pub trait Strategy: Send + Sync {
    /// 策略名（DB 主键之一，建议英数下划线）
    fn name(&self) -> &str;

    /// 策略版本（语义化版本号，默认 "1.0.0"）
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// 策略描述（UI 展示用）
    fn description(&self) -> &str {
        ""
    }

    /// 暴露所有可调参数
    /// - 返回 JSON 对象，key 为参数名，value 为当前值
    /// - UI 用此渲染参数表单
    /// - Walk-Forward grid search 用此生成参数网格
    fn params(&self) -> Value;

    /// 运行时改参（UI 改参 / grid search 注入）
    ///
    /// 返回 `AxAgentError::Validation` 表示参数名不存在或类型不匹配
    fn set_param(&mut self, key: &str, value: Value) -> Result<()>;

    /// 每根 K 线收盘后回调（D2 决策：每 K 线收盘 = 默认频率）
    ///
    /// - bar: 当前 K 线（已含涨跌停信息）
    /// - ctx: 策略上下文（可读持仓/权益/历史 K 线/指标，**不要**直接修改 cash/positions）
    /// - 返回 0..N 个 Signal；Engine 收集本 bar 全部 Signal 后转 Order
    async fn on_bar(&mut self, bar: &Bar, ctx: &mut StrategyCtx) -> Result<Vec<Signal>>;

    /// 回测/实盘启动时调用一次（用于初始化指标历史等）
    async fn on_init(&mut self, _ctx: &mut StrategyCtx) -> Result<()> {
        Ok(())
    }

    /// 回测/实盘结束时调用一次（用于释放资源、打印统计）
    async fn on_finish(&mut self, _ctx: &mut StrategyCtx) -> Result<()> {
        Ok(())
    }
}
