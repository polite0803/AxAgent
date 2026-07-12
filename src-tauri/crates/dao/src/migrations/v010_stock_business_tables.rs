//! v010 — AxInvest 股票业务核心表批量建表
//!
//! ## 背景
//!
//! 此前存在两套迁移系统：
//! - `crates/migration/src/m20250514_*`（sea-orm-migration 风格，完全孤立未注册）
//! - `crates/dao/src/migrations/v0*`（自定义版本化框架，实际生效）
//!
//! 导致 stock_analyses、trades、portfolio_holdings 等 20+ 表
//! 从未被 CREATE TABLE。本迁移一次性补齐所有缺失的股票业务表，
//! 并合并 v004_news_archive、v005_node_results_snapshot、
//! v006_llm_decision_json、v007_reco_pick_data、v008_reflection_structured、
//! v009_reflection_lessons、v010_agreement_score、v011_fix_reflection_lessons_fk
//! 这 8 个未注册迁移的内容。
//!
//! ## 表清单
//!
//! 1. stock_analyses — 股票分析记录
//! 2. stock_reflections — 反思记录
//! 3. watchlist_items — 自选股
//! 4. portfolio_holdings — 持仓
//! 5. trades — 手动交易记录
//! 6. price_alerts — 价格提醒
//! 7. reco_picks — 荐股推荐持久化
//! 8. earnings_events — 财报披露事件
//! 9. fund_transfers — 银证转账出入金记录
//! 10. financial_snapshots — 每日估值快照
//! 11. portfolio_correlation_snapshot — 两两相关性快照
//! 12. portfolio_metrics_daily — 每日 EOD 组合快照
//! 13. quant_runs — 回测运行记录
//! 14. quant_strategies — 量化策略元数据
//! 15. quant_signals — 信号历史
//! 16. quant_paper_trades — 纸面成交记录
//! 17. decision_validations — 决策事后验证表
//! 18. divergence_logs — 分歧日志审计表
//! 19. strategy_performance — 策略实际表现
//! 20. strategy_weight_history — 权重调整留痕
//! 21. reflection_lessons — 反思教训规则化表
//!
//! 所有表均使用 `CREATE TABLE IF NOT EXISTS`（幂等），
//! 字段类型严格对齐 entity 定义。
//!
//! ## 与未注册迁移的合并策略
//!
//! - news_archive（原 v004_news_archive）：独立表，本迁移中创建
//! - stock_analyses.node_results_snapshot（原 v005）：本迁移建表时直接包含
//! - stock_analyses.llm_decision_json（原 v006）：本迁移建表时直接包含
//! - reco_picks.pick_data（原 v007）：本迁移建表时直接包含
//! - stock_reflections 结构化字段（原 v008）：本迁移建表时直接包含
//! - reflection_lessons 表（原 v009+v011 合并）：本迁移中创建（已修复 FK 问题）
//! - strategy_performance.agreement_score（原 v010）：本迁移建表时直接包含

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // ── 1. stock_analyses ──
    // 已含 node_results_snapshot / llm_decision_json 列（合并原 v005/v006）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS stock_analyses (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, \
            stock_name TEXT NOT NULL, \
            analysis_date TEXT NOT NULL, \
            provider_id TEXT NOT NULL, \
            conversation_id TEXT NOT NULL, \
            status TEXT NOT NULL, \
            decision_action TEXT, \
            decision_position_pct REAL, \
            decision_reasoning TEXT, \
            decision_json TEXT, \
            blackboard_snapshot TEXT, \
            config_id TEXT, \
            analysis_kind TEXT NOT NULL DEFAULT 'live', \
            as_of_date TEXT, \
            decision_time_horizon TEXT, \
            decision_expected_holding_days INTEGER, \
            model_version TEXT, \
            data_snapshot_id TEXT, \
            outcome TEXT, \
            llm_decision_json TEXT, \
            node_results_snapshot TEXT, \
            created_at INTEGER NOT NULL, \
            updated_at INTEGER NOT NULL\
        )",
    )
    .await?;
    // 索引
    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_stock_analyses_code_date ON stock_analyses(stock_code, analysis_date DESC)",
        "CREATE INDEX IF NOT EXISTS idx_stock_analyses_status ON stock_analyses(status)",
        "CREATE INDEX IF NOT EXISTS idx_stock_analyses_conv ON stock_analyses(conversation_id)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // ── 2. stock_reflections ──
    // 已含原 v008 的结构化字段（raw_return / alpha_return / holding_days 等）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS stock_reflections (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, \
            stock_name TEXT NOT NULL, \
            original_analysis_id TEXT NOT NULL, \
            as_of_date TEXT NOT NULL, \
            hindsight_date TEXT NOT NULL, \
            min_confidence_threshold INTEGER NOT NULL, \
            reflection_depth TEXT NOT NULL, \
            actual_outcome TEXT NOT NULL, \
            raw_return REAL, \
            alpha_return REAL, \
            holding_days INTEGER, \
            benchmark_name TEXT, \
            verdict TEXT, \
            alpha_cited TEXT, \
            lesson_summary TEXT, \
            what_went_wrong TEXT, \
            missed_signals TEXT, \
            fix_for_future TEXT, \
            parameter_suggestions_json TEXT, \
            decision_json TEXT, \
            blackboard_snapshot TEXT, \
            model_version TEXT, \
            status TEXT NOT NULL, \
            created_at INTEGER NOT NULL, \
            updated_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_stock_reflections_ticker_created \
         ON stock_reflections(stock_code, created_at DESC)",
    )
    .await?;

    // ── 3. watchlist_items ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS watchlist_items (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, \
            stock_name TEXT NOT NULL, \
            notes TEXT, \
            created_at INTEGER NOT NULL, \
            updated_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_watchlist_code ON watchlist_items(stock_code)",
    )
    .await?;

    // ── 4. portfolio_holdings ──
    // entity 中 cost_price 列名映射到 avg_cost 字段
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS portfolio_holdings (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, \
            stock_name TEXT NOT NULL, \
            shares REAL NOT NULL, \
            cost_price REAL NOT NULL, \
            notes TEXT, \
            created_at INTEGER NOT NULL, \
            updated_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_holdings_code ON portfolio_holdings(stock_code)",
    )
    .await?;

    // ── 5. trades ──
    // 已含 strategy 列（原 m20250514_000005 缺失）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS trades (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, \
            stock_name TEXT NOT NULL, \
            direction TEXT NOT NULL, \
            price REAL NOT NULL, \
            quantity INTEGER NOT NULL, \
            trade_date TEXT NOT NULL, \
            trade_time TEXT NOT NULL, \
            fee REAL, \
            realized_pnl REAL, \
            strategy TEXT, \
            notes TEXT, \
            created_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_trades_code_date ON trades(stock_code, trade_date DESC)",
    )
    .await?;

    // ── 6. price_alerts ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS price_alerts (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, \
            stock_name TEXT NOT NULL, \
            condition TEXT NOT NULL, \
            target_price REAL NOT NULL, \
            is_triggered INTEGER NOT NULL DEFAULT 0, \
            triggered_at INTEGER, \
            created_at INTEGER NOT NULL, \
            updated_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_price_alerts_code ON price_alerts(stock_code)",
    )
    .await?;

    // ── 7. reco_picks ──
    // 已含 pick_data 列（原 v007）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS reco_picks (\
            id TEXT NOT NULL PRIMARY KEY, \
            generated_at TEXT NOT NULL, \
            period TEXT NOT NULL, \
            stock_code TEXT NOT NULL, \
            stock_name TEXT NOT NULL, \
            style TEXT NOT NULL, \
            confidence INTEGER NOT NULL, \
            synthetic INTEGER NOT NULL DEFAULT 0, \
            seed_pool_json TEXT, \
            strategy_weights_json TEXT, \
            pick_data TEXT, \
            created_at TEXT NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_reco_picks_period ON reco_picks(period, created_at DESC)",
    )
    .await?;

    // ── 8. earnings_events ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS earnings_events (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, \
            stock_name TEXT NOT NULL, \
            event_date TEXT NOT NULL, \
            event_type TEXT NOT NULL, \
            period TEXT, \
            detail TEXT, \
            source TEXT, \
            created_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_earnings_code_date ON earnings_events(stock_code, event_date DESC)",
    ).await?;

    // ── 9. fund_transfers ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS fund_transfers (\
            id TEXT NOT NULL PRIMARY KEY, \
            transfer_type TEXT NOT NULL, \
            amount REAL NOT NULL, \
            transfer_date TEXT NOT NULL, \
            fee REAL, \
            notes TEXT, \
            created_at INTEGER NOT NULL\
        )",
    )
    .await?;

    // ── 10. financial_snapshots ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS financial_snapshots (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, \
            snapshot_date TEXT NOT NULL, \
            pe_ttm REAL, \
            pb REAL, \
            ps_ttm REAL, \
            pcf REAL, \
            ev_ebitda REAL, \
            roe REAL, \
            gross_margin REAL, \
            debt_ratio REAL, \
            revenue_yoy REAL, \
            profit_yoy REAL, \
            source TEXT, \
            created_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_fin_snap_code_date ON financial_snapshots(stock_code, snapshot_date DESC)",
    ).await?;

    // ── 11. portfolio_correlation_snapshot ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS portfolio_correlation_snapshot (\
            id TEXT NOT NULL PRIMARY KEY, \
            snapshot_date TEXT NOT NULL, \
            lookback_days INTEGER NOT NULL, \
            code_a TEXT NOT NULL, \
            code_b TEXT NOT NULL, \
            correlation REAL NOT NULL, \
            created_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_corr_snap_date ON portfolio_correlation_snapshot(snapshot_date DESC)",
    ).await?;

    // ── 12. portfolio_metrics_daily ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS portfolio_metrics_daily (\
            id TEXT NOT NULL PRIMARY KEY, \
            snapshot_date TEXT NOT NULL, \
            total_market_value REAL NOT NULL, \
            cash_pct REAL NOT NULL, \
            total_pnl REAL NOT NULL, \
            total_pnl_pct REAL NOT NULL, \
            max_drawdown_pct REAL NOT NULL, \
            beta REAL, \
            sharpe_30d REAL, \
            correlation_avg REAL, \
            top_concentration_pct REAL NOT NULL, \
            sector_exposure_json TEXT NOT NULL, \
            stress_test_json TEXT, \
            created_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_portfolio_metrics_date ON portfolio_metrics_daily(snapshot_date DESC)",
    ).await?;

    // ── 13. quant_runs ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS quant_runs (\
            id TEXT NOT NULL PRIMARY KEY, \
            strategy_id TEXT NOT NULL, \
            name TEXT, \
            start_date TEXT NOT NULL, \
            end_date TEXT NOT NULL, \
            initial_cash REAL NOT NULL, \
            config_json TEXT NOT NULL, \
            status TEXT NOT NULL, \
            result_json TEXT, \
            walk_forward_enabled INTEGER NOT NULL DEFAULT 0, \
            walk_forward_folds INTEGER, \
            walk_forward_overfit_warning INTEGER, \
            walk_forward_stability_score REAL, \
            started_at INTEGER NOT NULL, \
            finished_at INTEGER, \
            error_message TEXT\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_quant_runs_strategy ON quant_runs(strategy_id, started_at DESC)",
    ).await?;

    // ── 14. quant_strategies ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS quant_strategies (\
            id TEXT NOT NULL PRIMARY KEY, \
            name TEXT NOT NULL, \
            version TEXT NOT NULL, \
            strategy_type TEXT NOT NULL, \
            description TEXT, \
            script_source TEXT, \
            params_json TEXT, \
            walk_forward_enabled INTEGER NOT NULL DEFAULT 0, \
            created_at INTEGER NOT NULL, \
            updated_at INTEGER NOT NULL\
        )",
    )
    .await?;

    // ── 15. quant_signals ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS quant_signals (\
            id TEXT NOT NULL PRIMARY KEY, \
            run_id TEXT NOT NULL, \
            code TEXT NOT NULL, \
            action TEXT NOT NULL, \
            strength REAL NOT NULL, \
            reason TEXT, \
            close_reason TEXT, \
            timestamp TEXT NOT NULL, \
            created_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_quant_signals_run ON quant_signals(run_id, timestamp)",
    )
    .await?;

    // ── 16. quant_paper_trades ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS quant_paper_trades (\
            id TEXT NOT NULL PRIMARY KEY, \
            run_id TEXT NOT NULL, \
            code TEXT NOT NULL, \
            side TEXT NOT NULL, \
            quantity INTEGER NOT NULL, \
            price REAL NOT NULL, \
            amount REAL NOT NULL, \
            commission REAL NOT NULL, \
            stamp_tax REAL NOT NULL, \
            slippage REAL NOT NULL, \
            timestamp TEXT NOT NULL, \
            reason TEXT, \
            realized_pnl REAL NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_quant_paper_trades_run ON quant_paper_trades(run_id, timestamp)",
    ).await?;

    // ── 17. decision_validations ──
    // 已含 agreement_score（原 v010）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS decision_validations (\
            id TEXT NOT NULL PRIMARY KEY, \
            pick_id TEXT NOT NULL, \
            stock_code TEXT NOT NULL, \
            stock_name TEXT NOT NULL, \
            style TEXT NOT NULL, \
            period TEXT NOT NULL, \
            t_plus_n INTEGER NOT NULL, \
            generated_at TEXT NOT NULL, \
            validated_at TEXT NOT NULL, \
            entry_price REAL NOT NULL, \
            target_price REAL NOT NULL, \
            stop_loss REAL NOT NULL, \
            position_pct REAL NOT NULL, \
            confidence INTEGER NOT NULL, \
            inferred_action TEXT NOT NULL, \
            t_plus_n_price REAL, \
            max_price REAL, \
            min_price REAL, \
            max_return_pct REAL, \
            max_drawdown_pct REAL, \
            final_return_pct REAL, \
            hit_stop_loss INTEGER, \
            hit_target INTEGER, \
            hit_outcome TEXT, \
            factor_snapshot TEXT, \
            data_source TEXT NOT NULL, \
            created_at TEXT NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_decision_val_code ON decision_validations(stock_code, generated_at DESC)",
    ).await?;

    // ── 18. divergence_logs ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS divergence_logs (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, \
            session_id TEXT NOT NULL, \
            dimension TEXT NOT NULL, \
            source_a TEXT NOT NULL, \
            source_b TEXT NOT NULL, \
            magnitude REAL NOT NULL, \
            direction TEXT NOT NULL, \
            rule_id TEXT, \
            llm_proposal TEXT, \
            rejection_reason TEXT, \
            resolved_by TEXT NOT NULL, \
            resolution_type TEXT NOT NULL, \
            prev_hash TEXT, \
            current_hash TEXT NOT NULL, \
            decision_ts TEXT NOT NULL, \
            created_at TEXT NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_divergence_logs_code ON divergence_logs(stock_code, decision_ts DESC)",
    ).await?;

    // ── 19. strategy_performance ──
    // 已含 agreement_score（原 v010）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS strategy_performance (\
            id TEXT NOT NULL PRIMARY KEY, \
            strategy_id TEXT NOT NULL, \
            period TEXT NOT NULL, \
            stock_code TEXT NOT NULL, \
            stock_name TEXT NOT NULL, \
            decision_at INTEGER NOT NULL, \
            exit_at INTEGER NOT NULL, \
            holding_days INTEGER NOT NULL, \
            return_pct REAL NOT NULL, \
            was_correct INTEGER NOT NULL, \
            decision_confidence INTEGER NOT NULL, \
            horizon_pnl_json TEXT, \
            agreement_score INTEGER, \
            created_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_strategy_perf_strategy ON strategy_performance(strategy_id, decision_at DESC)",
    ).await?;

    // ── 20. strategy_weight_history ──
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS strategy_weight_history (\
            id TEXT NOT NULL PRIMARY KEY, \
            strategy_id TEXT NOT NULL, \
            period TEXT NOT NULL, \
            old_weight REAL NOT NULL, \
            new_weight REAL NOT NULL, \
            delta_pct REAL NOT NULL, \
            trigger TEXT NOT NULL, \
            source_reflection_id TEXT, \
            sample_size INTEGER NOT NULL, \
            win_rate REAL NOT NULL, \
            rationale TEXT, \
            applied_at INTEGER NOT NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_weight_history_strategy ON strategy_weight_history(strategy_id, applied_at DESC)",
    ).await?;

    // ── 21. reflection_lessons ──
    // 合并原 v009 + v011（已修复 FK 问题：移除 stock_code FK）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS reflection_lessons (\
            id TEXT NOT NULL PRIMARY KEY, \
            lesson_summary TEXT NOT NULL, \
            rule_pattern TEXT, \
            source_reflection_id TEXT, \
            stock_code TEXT, \
            applicable_scenarios TEXT, \
            times_applied INTEGER NOT NULL DEFAULT 0, \
            success_count INTEGER NOT NULL DEFAULT 0, \
            confidence REAL NOT NULL DEFAULT 0.5, \
            status TEXT NOT NULL DEFAULT 'active', \
            created_at INTEGER NOT NULL, \
            updated_at INTEGER NOT NULL, \
            FOREIGN KEY (source_reflection_id) REFERENCES stock_reflections(id) ON DELETE SET NULL\
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_reflection_lessons_ticker_status_conf \
         ON reflection_lessons(stock_code, status, confidence DESC)",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_reflection_lessons_global_status_conf \
         ON reflection_lessons(confidence DESC) WHERE stock_code IS NULL",
    )
    .await?;

    // ── 22. news_archive ──（原 v004_news_archive）
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS news_archive (\
            id TEXT NOT NULL PRIMARY KEY, \
            source TEXT NOT NULL, \
            article_code TEXT, \
            title TEXT NOT NULL, \
            summary TEXT, \
            url TEXT, \
            media_name TEXT, \
            publish_time INTEGER NOT NULL, \
            stock_code TEXT, \
            keyword TEXT, \
            fetched_at INTEGER NOT NULL, \
            sentiment_score REAL, \
            UNIQUE(source, article_code))",
    )
    .await?;
    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_news_archive_publish ON news_archive(publish_time)",
        "CREATE INDEX IF NOT EXISTS idx_news_archive_stock ON news_archive(stock_code, publish_time)",
        "CREATE INDEX IF NOT EXISTS idx_news_archive_keyword ON news_archive(keyword, publish_time)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    Ok(())
}
