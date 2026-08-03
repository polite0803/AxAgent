/// price_alerts 表 ↔ RealtimeMonitor 双向转换（v203 数据模型对齐）
pub mod alert_mapping;
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
pub mod evolution_optimizer;
pub mod exit_recommend;
pub mod factor_analysis;
pub mod hit_rate_backtest;
// G3 产业链传导映射（P2-8 从 astock-data 迁回）
pub mod industry_chain;
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
// P3-1: 跨股票信号聚合器 — 在 signals.rs（单股）和 portfolio_monitor.rs（持仓指标）之间填补信号聚合层
pub mod cross_stock_aggregator;
// P3-2: 板块联动分析 — 基于 ConceptIndex 识别同板块龙头-从属传导模式
pub mod key_levels;
pub mod market_mainline;
pub mod market_regime;
// G3 产业链 MCP 工具集（P2-8 从 astock-data 迁回）
pub mod mcp_tools;
pub mod monitor;
pub mod monthly_report;
pub mod paper_portfolio;
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
pub mod screenshot_diagnosis;
pub mod sector_coherence;
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

// ── 自改进分析循环（Loop Engineering）──
// 对接上游 harness::SelfImprovingRound trait 的股票领域实现，
// 在 wiring 层配合 axagent_agent::SelfImprovementExecutor 驱动多轮分析。
pub mod stock_analysis_round;
pub use stock_analysis_round::{AnalysisError as StockAnalysisRoundError, StockAnalysisRound};

// ── 股票行业编排（Orchestrator 集成）──
// 对接 axagent-orchestrator 的 IndustryAdapter trait，
// 为股票业务提供动态编排、反思、进化能力。
pub mod stock_orchestration;
pub use stock_orchestration::{
    create_stock_industry_adapter, register_stock_adapter, StockIndustryAdapter,
};

// ── 股票业务反思引擎（WorkflowReflector 集成）──
// 将 axagent-trajectory 的 WorkflowReflectorImpl 集成到股票业务，
// 提供领域特定的反思维度、报告生成和进化触发判定。
pub mod stock_reflection;
pub use stock_reflection::{
    AnalysisStepResult, DimensionScores, StockAnalysisOutcome, StockReflectionEngine,
    StockReflectionReport, StockReflectionWeights,
};

// ── 股票业务自我进化闭环（NumericEvolution + WorkflowEvolver 集成）──
// 基于反思结果驱动参数进化和流程进化，形成反思→诊断→进化→验证闭环。
pub mod stock_self_evolution;
pub use stock_self_evolution::{
    EvolutionHistory, EvolutionPlan, EvolutionTrigger, EvolutionType, StockEvolutionResult,
    StockSelfEvolutionEngine,
};

// ── 股票业务自适应引擎（Reflection + Evolution + Orchestration 整合）──
// 将反思、进化、编排三者整合为统一的自适应闭环系统，
// 实现分析→反思→诊断→进化→验证→应用的完整自适应流程。
pub mod stock_adaptive_engine;
pub use stock_adaptive_engine::{
    AdaptationRecord, AdaptationStatus, AdaptiveEngineConfig, AdaptiveResult, EvolutionValidator,
    StockAdaptiveEngine, ValidationResult,
};
