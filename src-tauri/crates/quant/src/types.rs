//! 核心数据类型 — 已下沉到 harness
//!
//! 历史上 `Bar` / `Signal` / `Side` / `OrderType` / `Order` / `Fill`
//! / `SignalAction` / `CloseReason` 等类型定义在本 crate。
//! 为消除 `market-sim → quant` 的违规依赖（两者均为 consumer），
//! 这些共享类型与 `Strategy` trait 一起下沉到 `axagent_harness::strategy_contract`。
//!
//! 本模块仅做 re-export，保持 `crate::types::Bar` 等路径在 quant 内部继续可用，
//! 不破坏现有调用方。

pub use axagent_harness::strategy_contract::{
    Bar, CloseReason, Fill, Order, OrderType, Side, Signal, SignalAction,
};
