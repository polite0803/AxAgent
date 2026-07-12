//! 股票全业务管道编排器模块
//!
//! 将股票发现（`recommend_stocks`）、单股分析（`run_single_stock_analysis`）、
//! 持仓再评估整合为每日自动触发的管道。反思阶段由现有 6h cron 接力。

pub mod core;
