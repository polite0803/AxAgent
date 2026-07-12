//! 策略运行时上下文 — 已下沉到 harness
//!
//! `StrategyCtx` / `Position` / `Trade` / `EquityPoint` 等共享类型
//! 已下沉到 `axagent_harness::strategy_contract`，消除 `market-sim → quant`
//! 的违规依赖（两者均为 consumer，应仅依赖 harness）。
//!
//! 本模块仅做 re-export，保持 `crate::ctx::StrategyCtx` 等路径在 quant 内部继续可用。

pub use axagent_harness::strategy_contract::{EquityPoint, Position, StrategyCtx, Trade};
