//! 股票全业务管道编排器模块
//!
//! 将股票发现（`recommend_stocks`）、单股分析（`run_single_stock_analysis`）、
//! 持仓再评估整合为每日自动触发的管道。反思阶段由现有 6h cron 接力。
//!
//! 重构后：通过 workflow_template + WorkEngine 执行，与股票分析工作流一致。

pub mod core;
pub mod seed_stock_pipeline;

pub use seed_stock_pipeline::seed_stock_pipeline_template;
