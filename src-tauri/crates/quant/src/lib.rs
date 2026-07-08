//! Quant 量化交易与回测 crate
//!
//! A 股市场量化策略开发、回测评估、Walk-Forward 验证的统一入口。
//!
//! ## 核心原则
//!
//! - **单代码源（Single Code Source）**: 同一套策略代码同时跑回测与实盘，
//!   策略接口抽象一致，回测与实盘共用一个 `Strategy` trait。
//! - **默认反过拟合**: Walk-Forward 验证强制默认开启，不可关闭
//!   （除非显式 `ForceWalkForward::Off` 显式 opt-in，并在审计日志留痕）。
//! - **A 股市场微观结构内建**: 撮合器实现 T+1、涨跌停、印花税、佣金、滑点；
//!   不允许"美式撮合偷看未来"。
//! - **复盘/回测统一**: 通过 AsOf 时间锚，复盘模式与回测走同一份代码路径，
//!   复用 `MarketDataProvider trait` 的 replay routing。
//!
//! ## 模块组织
//!
//! - `builtin`: 5 个内置技术分析策略 (MA cross / MACD / RSI / BOLL / Turtle)
//! - `ctx`: 策略运行时上下文 (StrategyCtx / Position / Trade / EquityPoint)
//! - `engine`: BacktestEngine 事件循环
//! - `error`: 统一错误类型
//! - `matcher`: A 股撮合器（T+1 / 涨跌停 / 印花税 / 佣金 / 滑点）
//! - `metrics`: 完整绩效指标 (Sharpe / Sortino / MaxDD / WinRate / ProfitFactor)
//! - `script`: Rhai 策略加载器（热加载 + sandbox）
//! - `strategy`: Strategy trait (单代码源接口)
//! - `types`: 核心数据类型 (Bar / Signal / Order / Fill)
//! - `walkforward`: Walk-Forward 验证 (rolling / anchored / 反过拟合)
//!
//! ## M1 实施状态
//!
//! 后端 quant crate 已完成全部核心模块，可独立跑回测。
//! 待办：DB 实体 / Tauri 命令 / 前端 UI。

#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]

mod builtin;
mod ctx;
mod engine;
mod error;
mod matcher;
mod metrics;
mod script;
mod strategy;
mod types;
mod walkforward;

pub use builtin::{BollStrategy, MaCrossStrategy, MacdStrategy, RsiStrategy, TurtleStrategy};
pub use ctx::{EquityPoint, Position, StrategyCtx, Trade};
pub use engine::{BacktestConfig, BacktestEngine, BacktestResult};
pub use error::QuantError;
pub use matcher::{Matcher, MatcherConfig};
pub use metrics::MetricsReport;
pub use script::RhaiStrategy;
pub use strategy::Strategy;
pub use types::{Bar, CloseReason, Fill, Order, OrderType, Side, Signal, SignalAction};
pub use walkforward::{
    WalkForward, WalkForwardConfig, WalkForwardFold, WalkForwardReport, WalkForwardSplit,
    WalkForwardWindowResult,
};

pub mod prelude {
    pub use crate::builtin::{
        BollStrategy, MaCrossStrategy, MacdStrategy, RsiStrategy, TurtleStrategy,
    };
    pub use crate::ctx::{EquityPoint, Position, StrategyCtx, Trade};
    pub use crate::engine::{BacktestConfig, BacktestEngine, BacktestResult};
    pub use crate::error::QuantError;
    pub use crate::matcher::{Matcher, MatcherConfig};
    pub use crate::metrics::MetricsReport;
    pub use crate::script::RhaiStrategy;
    pub use crate::strategy::Strategy;
    pub use crate::types::{Bar, CloseReason, Fill, Order, OrderType, Side, Signal, SignalAction};
    pub use crate::walkforward::{
        WalkForward, WalkForwardConfig, WalkForwardFold, WalkForwardReport, WalkForwardSplit,
        WalkForwardWindowResult,
    };
}
