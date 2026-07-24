//! Agent 系统模块。
//!
//! - `traits` — SimAgent trait, AgentType, MessageBody, AgentContext, AgentAction
//! - `exchange` — ExchangeAgent（中央交易所，维护订单簿）
//! - `market_maker` — 做市商（双边报价）
//! - `momentum` — 动量交易者（追涨杀跌）
//! - `value` — 价值交易者（逆势）
//! - `noise` — 噪声交易者（随机下单）
//! - `rhai` — Rhai 脚本定义的 Agent（用户自定义行为）
//! - `institutional` — 机构投资者（TWAP/VWAP 拆单执行）
//! - `background` — 后台监控 Agent
//! - `event_driven` — 事件驱动 Agent（P2-C9：基于新闻/公告/财报事件交易）

pub mod background;
pub mod event_driven;
pub mod exchange;
pub mod institutional;
pub mod market_maker;
pub mod momentum;
pub mod noise;
pub mod quant_bridge;
pub mod rhai;
pub mod strategy;
pub mod traits;
pub mod value;

pub use background::BackgroundAgent;
pub use event_driven::EventDrivenAgent;
pub use exchange::ExchangeAgent;
pub use institutional::InstitutionalAgent;
pub use market_maker::MarketMakerAgent;
pub use momentum::MomentumAgent;
pub use noise::NoiseAgent;
pub use quant_bridge::QuantStrategyAgent;
pub use rhai::RhaiAgent;
pub use strategy::StrategyAgent;
pub use traits::{AgentAction, AgentContext, AgentMessage, AgentType, MessageBody, SimAgent};
pub use value::ValueAgent;
