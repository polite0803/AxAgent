pub mod backtest;
pub mod backtest_feedback;
pub mod backtest_strategy;
pub mod blackboard;
pub mod dashboard_report;
pub mod data_clean;
pub mod decision;
pub mod decision_tracker;
pub mod evidence_citation;
pub mod evidence_weight;
pub mod evolution_drift;
pub mod exit_recommend;
pub mod factor_analysis;
pub mod hit_rate_backtest;
pub mod intent_parser;
pub mod knowledge_loader;
// Phase 2: fundamentals_report 迁移到 astock-data 层(被 tools crate 依赖),
// 此处用 pub use 保持向后兼容。
// re-export conserved for backward compat
pub use axagent_astock_data::fundamentals_report;
// K 线形态和价量背离检测 — 权威实现在 astock-data crate，此处 re-export 保持向后兼容
pub use axagent_astock_data::{candlestick_pattern, divergence};
pub mod concept_index;
pub mod conditional_order;
pub mod key_levels;
pub mod market_regime;
pub mod monthly_report;
pub mod monitor;
pub mod plugin;
pub mod portfolio_formula;
pub mod portfolio_monitor;
pub mod portfolio_risk;
pub mod position_limits;
pub mod prompts;
pub mod quality;
pub mod recommender;
pub mod reflection_lesson_validator;
pub mod report;
pub mod review;
pub mod risk;
pub mod rules;
pub mod schema_serde_regression;
pub mod scoring;
pub mod screener;
pub mod sentiment_analysis;
pub mod signals;
pub mod strategy_pack;
pub mod trade_import;
pub mod trade_review;
pub mod trade_stats;
pub mod trading;
pub mod value;
pub mod value_investing;
pub mod vlm_import;
pub mod weight_decay;

// 以下两个模块原在 axagent-harness，属股票域契约，已迁出至本 crate（2026-07-16）：
pub mod stock_data_service;
pub use stock_data_service::StockDataService;
pub mod notification_channel;
pub use notification_channel::{
    AlertPayload, AlertSeverity, NotificationChannel, NotificationDispatchResult,
    NotificationDispatchSummary, NotificationPolicy, NotificationRoute, ReportPayload,
    ReportStockSummary, RouteConfig,
};
