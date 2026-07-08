//! 工作流驱动的股票分析 — 基于持久化 WorkflowTemplate + WorkEngine DAG 执行。
//!
//! 启动时种子化 stock-analysis 工作流模板到 workflow_templates 表，
//! 每次分析从模板加载 DAG 结构，注入实时行情数据，由 WorkEngine 并行执行。
//!
//! 子模块：
//! - decision: 决策质量预检、决策提取、重跑决策命令
//! - core: 股票分析工作流核心（run_stock_workflow, run_single_stock_analysis）
//! - reflection: 反思工作流（run_reflection_workflow, run_batch_reflection）
//! - serenity: Serenity 瓶颈筛选工作流
//! - reco_history: 推荐历史记录管理
//! - misc: 导出和回测命令

pub mod core;
pub mod decision;
pub mod misc;
pub mod reco_history;
pub mod reflection;
pub mod serenity;

// Re-export all #[tauri::command] functions so they remain accessible via stock_workflow::
pub use core::run_single_stock_analysis;
pub use reflection::run_reflection_workflow;
// Internal helpers called from outside the module
pub use reflection::run_batch_reflection_inner;

// Re-export public structs
