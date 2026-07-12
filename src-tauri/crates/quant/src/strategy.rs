//! Strategy trait — 已下沉到 harness
//!
//! `Strategy` trait 已下沉到 `axagent_harness::strategy_contract`，
//! 让 `market-sim`（consumer）可以不依赖 `axagent-quant` 而接入策略接口。
//!
//! trait 方法返回 `axagent_harness::core_error::Result`（即 `Result<_, AxAgentError>`）；
//! quant 内部实现可继续使用 `QuantError`，通过 `From<QuantError> for AxAgentError`
//! 自动 `?` 传播。
//!
//! 本模块仅做 re-export，保持 `crate::strategy::Strategy` 路径在 quant 内部继续可用。

pub use axagent_harness::strategy_contract::Strategy;
