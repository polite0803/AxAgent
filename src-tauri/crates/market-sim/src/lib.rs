//! ABIDES-inspired 市场模拟 crate — Phase 2: DES Kernel。
//!
//! ## 模块
//!
//! - `orderbook` — 中央限价订单簿（LOB），价格-时间优先撮合
//! - `kernel` — 离散事件模拟内核（DES），事件驱动的多 Agent 仿真
//! - `agent` — Agent trait + ExchangeAgent
//! - `config` — 模拟配置 + 延迟矩阵
//! - `types` — 核心类型
//! - `error` — 错误类型
//!
//! ## Phase 3 预览
//!
//! - 内置 Agent 类型：做市商/动量/价值/噪声/Rhai 脚本
//! - Oracle 信号注入
//! - Stylized Facts 验证
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use axagent_market_sim::{SimKernel, SimConfig, ExchangeAgent};
//!
//! let mut kernel = SimKernel::new(SimConfig::default());
//! kernel.register(Box::new(ExchangeAgent::new("exchange")));
//! let result = kernel.run().unwrap();
//! println!("处理了 {} 个事件", result.total_events);
//! ```

pub mod agent;
pub mod calibration;
pub mod config;
pub mod error;
pub mod events;
pub mod kernel;
pub mod monte_carlo;
pub mod oracle;
pub mod orderbook;
pub mod stylized_facts;
pub mod types;

// Phase 1 re-exports
pub use error::SimError;
pub use orderbook::OrderBook;
pub use types::{
    BookLevel, BookSnapshot, BookStats, FillResult, LimitOrder, MarketOrder, OrderId, OrderResult,
    OrderSide, Price, Quantity, SimTimestamp, TradeRecord,
};

// Phase 2+3 re-exports
pub use agent::{
    AgentAction, AgentContext, AgentMessage, AgentType, BackgroundAgent, EventDrivenAgent,
    ExchangeAgent, InstitutionalAgent, MarketMakerAgent, MessageBody, MomentumAgent, NoiseAgent,
    QuantStrategyAgent, RhaiAgent, SimAgent, StrategyAgent, ValueAgent,
};
pub use calibration::{BEST_PARAMS, CalibrationParam, CalibrationResult, CalibrationRunner};
pub use config::{LatencyMatrix, SimConfig};
pub use kernel::{SimKernel, SimResult, SimStats};
pub use stylized_facts::{StylizedFacts, TargetRange};

// P2-C9: 外部事件 DTO + 事件注入接口
pub use events::{ExternalEvent, ExternalEventKind};
