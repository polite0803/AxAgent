//! Agent 系统模块。
//!
//! - `traits` — SimAgent trait, AgentType, MessageBody, AgentContext, AgentAction
//! - `exchange` — ExchangeAgent（中央交易所，维护订单簿）
//! - `market_maker` — 做市商（双边报价）
//! - `momentum` — 动量交易者（追涨杀跌）
//! - `value` — 价值交易者（逆势）
//! - `noise` — 噪声交易者（随机下单）

pub mod exchange;
pub mod market_maker;
pub mod momentum;
pub mod noise;
pub mod traits;
pub mod value;

pub use exchange::ExchangeAgent;
pub use market_maker::MarketMakerAgent;
pub use momentum::MomentumAgent;
pub use noise::NoiseAgent;
pub use traits::{AgentAction, AgentContext, AgentMessage, AgentType, MessageBody, SimAgent};
pub use value::ValueAgent;
