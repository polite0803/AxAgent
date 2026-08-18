// SPDX-License-Identifier: AGPL-3.0-only
//! v200_axinvest_stock_tables: AxInvest 独有的股票业务表
//!
//! ## 背景
//!
//! AxInvest fork 在上游 v100_consolidated 之上扩展了 4 张股票业务表 + 2 处
//! CHECK 约束扩展。为避免与上游未来 v101–v199 区间的迁移冲突，本地独有
//! 迁移从 v200 起单调递增（详见 project_memory.md「AxInvest 本地数据库
//! 迁移版本号策略」）。
//!
//! ## 包含内容
//!
//! - `stock_analyses`：单股分析结果主表
//! - `stock_reflections`：复盘反思记录
//! - `stock_pipeline_runs`：批量管道执行记录
//! - `strategy_performance`：策略级实际表现（用于复盘→进化）
//! - `agency_experts.category` CHECK 约束扩展：加入 `'stock-analysis'`
//! - `agent_profiles.category` CHECK 约束扩展：加入 `'stock-analysis'`
//!
//! ## DDL 风格
//!
//! 与 v100_consolidated 保持一致：直接写 PG 语法（BIGINT/DOUBLE PRECISION/
//! BOOLEAN/BIGSERIAL），SQLite 侧由 [`sqlite_ddl`](super::pg_ddl::sqlite_ddl)
//! 自动转换。

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub use super::pg_ddl::exec_ddl;

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let is_pg = db.get_database_backend() == DbBackend::Postgres;
    let backend = db.get_database_backend();

    // ========================================================================
    // PHASE 1: AxInvest 独有的股票业务表
    //   DDL 直接写 PG 语法，exec_ddl 在 SQLite 下自动转换 BIGSERIAL/to_char。
    // ========================================================================

    for sql in &[
        // stock_analyses：单股分析主表
        "CREATE TABLE IF NOT EXISTS stock_analyses (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
            analysis_date TEXT NOT NULL, \
            provider_id TEXT NOT NULL, conversation_id TEXT NOT NULL, \
            status TEXT NOT NULL, \
            decision_action TEXT, decision_position_pct DOUBLE PRECISION, \
            decision_reasoning TEXT, decision_json TEXT, \
            blackboard_snapshot TEXT, config_id TEXT, \
            analysis_kind TEXT NOT NULL DEFAULT 'live', \
            as_of_date TEXT, \
            decision_time_horizon TEXT, \
            decision_expected_holding_days BIGINT, \
            model_version TEXT, data_snapshot_id TEXT, \
            outcome TEXT, \
            llm_decision_json TEXT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // stock_reflections：复盘反思表
        "CREATE TABLE IF NOT EXISTS stock_reflections (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
            original_analysis_id TEXT NOT NULL, \
            as_of_date TEXT NOT NULL, hindsight_date TEXT NOT NULL, \
            min_confidence_threshold INTEGER NOT NULL, \
            reflection_depth TEXT NOT NULL, \
            actual_outcome TEXT NOT NULL, \
            raw_return DOUBLE PRECISION, alpha_return DOUBLE PRECISION, \
            holding_days INTEGER, benchmark_name TEXT, \
            verdict TEXT, alpha_cited TEXT, lesson_summary TEXT, \
            what_went_wrong TEXT, missed_signals TEXT, fix_for_future TEXT, \
            parameter_suggestions_json TEXT, \
            decision_json TEXT, blackboard_snapshot TEXT, \
            model_version TEXT, \
            status TEXT NOT NULL, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // stock_pipeline_runs：批量管道执行记录
        "CREATE TABLE IF NOT EXISTS stock_pipeline_runs (\
            id TEXT NOT NULL PRIMARY KEY, \
            run_date TEXT NOT NULL, as_of_date TEXT, \
            status TEXT NOT NULL, \
            candidates_json TEXT, new_analyses_json TEXT, \
            reassessed_json TEXT, summary_json TEXT, \
            error_message TEXT, \
            started_at BIGINT NOT NULL, completed_at BIGINT, \
            created_at BIGINT NOT NULL)",
        // strategy_performance：策略级实际表现
        "CREATE TABLE IF NOT EXISTS strategy_performance (\
            id TEXT NOT NULL PRIMARY KEY, \
            strategy_id TEXT NOT NULL, period TEXT NOT NULL, \
            stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
            decision_at BIGINT NOT NULL, exit_at BIGINT NOT NULL, \
            holding_days INTEGER NOT NULL, \
            return_pct DOUBLE PRECISION NOT NULL, \
            was_correct INTEGER NOT NULL, \
            decision_confidence INTEGER NOT NULL, \
            horizon_pnl_json TEXT, agreement_score INTEGER, \
            created_at BIGINT NOT NULL)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // ========================================================================
    // PHASE 1b: 其余 AxInvest 本地股票业务表
    //
    // 覆盖：reco_picks / portfolio_holdings / trades / watchlist_items /
    // price_alerts / decision_validations / financial_snapshots / earnings_events /
    // news_archive / quant_runs / quant_strategies / quant_signals /
    // quant_paper_trades / strategy_weight_history
    //
    // 背景：这些表原由已被合并删除的旧 AxInvest 迁移建表，本仓库此前无 DDL 可
    // 比对，导致全新库缺表（"table does not exist"）。此处统一补
    // CREATE TABLE IF NOT EXISTS（对已有表为 no-op，不改动现有列类型）。
    // DDL 由实体类型派生，遵循 PG 约定：BIGINT↔i64、DOUBLE PRECISION↔f64、
    // INTEGER↔i32、布尔列 INTEGER（实体已统一用 i32 / Option<i32>）。
    // ========================================================================

    for sql in &[
        // reco_picks：荐股推荐持久化
        "CREATE TABLE IF NOT EXISTS reco_picks (\
            id TEXT NOT NULL PRIMARY KEY, \
            generated_at TEXT NOT NULL, period TEXT NOT NULL, \
            stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
            style TEXT NOT NULL, confidence INTEGER NOT NULL, \
            synthetic INTEGER NOT NULL, \
            seed_pool_json TEXT, strategy_weights_json TEXT, pick_data TEXT, \
            created_at TEXT NOT NULL)",
        // portfolio_holdings：持仓
        "CREATE TABLE IF NOT EXISTS portfolio_holdings (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
            shares DOUBLE PRECISION NOT NULL, cost_price DOUBLE PRECISION NOT NULL, \
            notes TEXT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // trades：手动交易记录
        "CREATE TABLE IF NOT EXISTS trades (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
            direction TEXT NOT NULL, price DOUBLE PRECISION NOT NULL, \
            quantity INTEGER NOT NULL, trade_date TEXT NOT NULL, \
            trade_time TEXT NOT NULL, fee DOUBLE PRECISION, realized_pnl DOUBLE PRECISION, \
            strategy TEXT, notes TEXT, created_at BIGINT NOT NULL)",
        // watchlist_items：自选
        "CREATE TABLE IF NOT EXISTS watchlist_items (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
            notes TEXT, created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // price_alerts：价格预警（is_triggered 为 INTEGER 布尔列）
        "CREATE TABLE IF NOT EXISTS price_alerts (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
            condition TEXT NOT NULL, target_price DOUBLE PRECISION NOT NULL, \
            is_triggered INTEGER NOT NULL, triggered_at BIGINT, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // decision_validations：决策事后验证（hit_stop_loss/hit_target 为 INTEGER 布尔列）
        "CREATE TABLE IF NOT EXISTS decision_validations (\
            id TEXT NOT NULL PRIMARY KEY, \
            pick_id TEXT NOT NULL, stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
            style TEXT NOT NULL, period TEXT NOT NULL, t_plus_n INTEGER NOT NULL, \
            generated_at TEXT NOT NULL, validated_at TEXT NOT NULL, \
            entry_price DOUBLE PRECISION NOT NULL, target_price DOUBLE PRECISION NOT NULL, \
            stop_loss DOUBLE PRECISION NOT NULL, position_pct DOUBLE PRECISION NOT NULL, \
            confidence INTEGER NOT NULL, inferred_action TEXT NOT NULL, \
            t_plus_n_price DOUBLE PRECISION, max_price DOUBLE PRECISION, \
            min_price DOUBLE PRECISION, max_return_pct DOUBLE PRECISION, \
            max_drawdown_pct DOUBLE PRECISION, final_return_pct DOUBLE PRECISION, \
            hit_stop_loss INTEGER, hit_target INTEGER, hit_outcome TEXT, \
            factor_snapshot TEXT, data_source TEXT NOT NULL, created_at TEXT NOT NULL)",
        // financial_snapshots：每日估值快照
        "CREATE TABLE IF NOT EXISTS financial_snapshots (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, snapshot_date TEXT NOT NULL, \
            pe_ttm DOUBLE PRECISION, pb DOUBLE PRECISION, ps_ttm DOUBLE PRECISION, \
            pcf DOUBLE PRECISION, ev_ebitda DOUBLE PRECISION, roe DOUBLE PRECISION, \
            gross_margin DOUBLE PRECISION, debt_ratio DOUBLE PRECISION, \
            revenue_yoy DOUBLE PRECISION, profit_yoy DOUBLE PRECISION, \
            source TEXT, created_at BIGINT NOT NULL)",
        // earnings_events：财报披露事件
        "CREATE TABLE IF NOT EXISTS earnings_events (\
            id TEXT NOT NULL PRIMARY KEY, \
            stock_code TEXT NOT NULL, stock_name TEXT NOT NULL, \
            event_date TEXT NOT NULL, event_type TEXT NOT NULL, \
            period TEXT, detail TEXT, source TEXT, created_at BIGINT NOT NULL)",
        // news_archive：本地新闻语料库
        "CREATE TABLE IF NOT EXISTS news_archive (\
            id TEXT NOT NULL PRIMARY KEY, \
            source TEXT NOT NULL, article_code TEXT NOT NULL, title TEXT NOT NULL, \
            summary TEXT, url TEXT, media_name TEXT, \
            publish_time BIGINT NOT NULL, stock_code TEXT, keyword TEXT, \
            fetched_at BIGINT NOT NULL, sentiment_score DOUBLE PRECISION, \
            UNIQUE(source, article_code))",
        // quant_runs：回测运行记录（walk_forward_* 为 INTEGER 布尔列）
        "CREATE TABLE IF NOT EXISTS quant_runs (\
            id TEXT NOT NULL PRIMARY KEY, \
            strategy_id TEXT NOT NULL, name TEXT, \
            start_date TEXT NOT NULL, end_date TEXT NOT NULL, \
            initial_cash DOUBLE PRECISION NOT NULL, config_json TEXT NOT NULL, \
            status TEXT NOT NULL, result_json TEXT, \
            walk_forward_enabled INTEGER NOT NULL, walk_forward_folds INTEGER, \
            walk_forward_overfit_warning INTEGER, walk_forward_stability_score DOUBLE PRECISION, \
            started_at BIGINT NOT NULL, finished_at BIGINT, error_message TEXT)",
        // quant_strategies：量化策略元数据（walk_forward_enabled 为 INTEGER 布尔列）
        "CREATE TABLE IF NOT EXISTS quant_strategies (\
            id TEXT NOT NULL PRIMARY KEY, \
            name TEXT NOT NULL, version TEXT NOT NULL, strategy_type TEXT NOT NULL, \
            description TEXT, script_source TEXT, params_json TEXT, \
            walk_forward_enabled INTEGER NOT NULL, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // quant_signals：信号历史
        "CREATE TABLE IF NOT EXISTS quant_signals (\
            id TEXT NOT NULL PRIMARY KEY, \
            run_id TEXT NOT NULL, code TEXT NOT NULL, action TEXT NOT NULL, \
            strength DOUBLE PRECISION NOT NULL, reason TEXT, close_reason TEXT, \
            timestamp TEXT NOT NULL, created_at BIGINT NOT NULL)",
        // quant_paper_trades：纸面成交记录
        "CREATE TABLE IF NOT EXISTS quant_paper_trades (\
            id TEXT NOT NULL PRIMARY KEY, \
            run_id TEXT NOT NULL, code TEXT NOT NULL, side TEXT NOT NULL, \
            quantity BIGINT NOT NULL, price DOUBLE PRECISION NOT NULL, \
            amount DOUBLE PRECISION NOT NULL, commission DOUBLE PRECISION NOT NULL, \
            stamp_tax DOUBLE PRECISION NOT NULL, slippage DOUBLE PRECISION NOT NULL, \
            timestamp TEXT NOT NULL, reason TEXT, realized_pnl DOUBLE PRECISION NOT NULL)",
        // strategy_weight_history：权重调整留痕
        "CREATE TABLE IF NOT EXISTS strategy_weight_history (\
            id TEXT NOT NULL PRIMARY KEY, \
            strategy_id TEXT NOT NULL, period TEXT NOT NULL, \
            old_weight DOUBLE PRECISION NOT NULL, new_weight DOUBLE PRECISION NOT NULL, \
            delta_pct DOUBLE PRECISION NOT NULL, trigger TEXT NOT NULL, \
            source_reflection_id TEXT, sample_size INTEGER NOT NULL, \
            win_rate DOUBLE PRECISION NOT NULL, rationale TEXT, \
            applied_at BIGINT NOT NULL)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // ========================================================================
    // PHASE 1c: 实体存在但历史迁移缺建表的补漏表
    //
    // 覆盖：reflection_lessons / fund_transfers /
    // portfolio_correlation_snapshot / portfolio_metrics_daily /
    // gateway_message_queue
    //
    // 背景：这 5 张表在 crates/entities（前 4 张）和 crates/runtime
    // （gateway_message_queue）有 SeaORM 实体，但 v100/v200/runtime 均无
    // CREATE TABLE，导致全新库缺表崩溃（与 total_time_ms 同源：旧库靠历史
    // 迁移留表能跑，全新库炸）。此处统一补 CREATE TABLE IF NOT EXISTS
    // （对已有表 no-op）。DDL 由实体类型派生，遵循 PG 约定：BIGINT↔i64、
    // DOUBLE PRECISION↔f64、INTEGER↔i32。
    //
    // 注：gateway_message_queue 属 runtime 通用能力（非 AxInvest 独有），
    // 放此处仅为补齐全新库缺表；因 IF NOT EXISTS 幂等，上游若日后自带建表
    // 迁移也不会冲突或崩溃。
    // ========================================================================

    for sql in &[
        // reflection_lessons：F1 反思教训规则化表
        "CREATE TABLE IF NOT EXISTS reflection_lessons (\
            id TEXT NOT NULL PRIMARY KEY, \
            lesson_summary TEXT NOT NULL, rule_pattern TEXT, \
            source_reflection_id TEXT, stock_code TEXT, \
            applicable_scenarios TEXT, \
            times_applied INTEGER NOT NULL, success_count INTEGER NOT NULL, \
            confidence DOUBLE PRECISION NOT NULL, status TEXT NOT NULL, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL)",
        // fund_transfers：银证转账出入金流水
        "CREATE TABLE IF NOT EXISTS fund_transfers (\
            id TEXT NOT NULL PRIMARY KEY, \
            transfer_type TEXT NOT NULL, amount DOUBLE PRECISION NOT NULL, \
            transfer_date TEXT NOT NULL, fee DOUBLE PRECISION, notes TEXT, \
            created_at BIGINT NOT NULL)",
        // portfolio_correlation_snapshot：两两相关性快照
        "CREATE TABLE IF NOT EXISTS portfolio_correlation_snapshot (\
            id TEXT NOT NULL PRIMARY KEY, \
            snapshot_date TEXT NOT NULL, lookback_days INTEGER NOT NULL, \
            code_a TEXT NOT NULL, code_b TEXT NOT NULL, \
            correlation DOUBLE PRECISION NOT NULL, created_at BIGINT NOT NULL)",
        // portfolio_metrics_daily：每日组合快照
        "CREATE TABLE IF NOT EXISTS portfolio_metrics_daily (\
            id TEXT NOT NULL PRIMARY KEY, \
            snapshot_date TEXT NOT NULL, \
            total_market_value DOUBLE PRECISION NOT NULL, cash_pct DOUBLE PRECISION NOT NULL, \
            total_pnl DOUBLE PRECISION NOT NULL, total_pnl_pct DOUBLE PRECISION NOT NULL, \
            max_drawdown_pct DOUBLE PRECISION NOT NULL, beta DOUBLE PRECISION, \
            sharpe_30d DOUBLE PRECISION, correlation_avg DOUBLE PRECISION, \
            top_concentration_pct DOUBLE PRECISION NOT NULL, \
            sector_exposure_json TEXT NOT NULL, stress_test_json TEXT, \
            created_at BIGINT NOT NULL)",
        // gateway_message_queue：runtime 持久化消息队列
        "CREATE TABLE IF NOT EXISTS gateway_message_queue (\
            id TEXT NOT NULL PRIMARY KEY, \
            from_agent TEXT NOT NULL, to_agent TEXT NOT NULL, \
            payload_type TEXT NOT NULL, payload TEXT NOT NULL, status TEXT NOT NULL, \
            retry_count INTEGER NOT NULL DEFAULT 0, max_retries INTEGER NOT NULL, \
            created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL, \
            expires_at BIGINT, correlation_id TEXT, reply_to TEXT)",
    ] {
        exec_ddl(&db, is_pg, sql).await?;
    }

    // ========================================================================
    // PHASE 2: 索引
    // ========================================================================

    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_stock_analyses_code_date \
         ON stock_analyses(stock_code, analysis_date)",
        "CREATE INDEX IF NOT EXISTS idx_stock_analyses_status \
         ON stock_analyses(status)",
        "CREATE INDEX IF NOT EXISTS idx_stock_analyses_outcome \
         ON stock_analyses(outcome)",
        "CREATE INDEX IF NOT EXISTS idx_stock_reflections_original \
         ON stock_reflections(original_analysis_id)",
        "CREATE INDEX IF NOT EXISTS idx_stock_reflections_code_asof \
         ON stock_reflections(stock_code, as_of_date)",
        "CREATE INDEX IF NOT EXISTS idx_stock_pipeline_runs_date \
         ON stock_pipeline_runs(run_date)",
        "CREATE INDEX IF NOT EXISTS idx_stock_pipeline_runs_status \
         ON stock_pipeline_runs(status)",
        "CREATE INDEX IF NOT EXISTS idx_strategy_performance_strategy_period \
         ON strategy_performance(strategy_id, period)",
        "CREATE INDEX IF NOT EXISTS idx_strategy_performance_stock \
         ON strategy_performance(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_strategy_performance_decision_at \
         ON strategy_performance(decision_at)",
        // ── PHASE 1b 新表索引 ──
        "CREATE INDEX IF NOT EXISTS idx_reco_picks_stock_code \
         ON reco_picks(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_reco_picks_generated_at \
         ON reco_picks(generated_at)",
        "CREATE INDEX IF NOT EXISTS idx_portfolio_holdings_stock_code \
         ON portfolio_holdings(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_trades_stock_code \
         ON trades(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_trades_trade_date \
         ON trades(trade_date)",
        "CREATE INDEX IF NOT EXISTS idx_watchlist_items_stock_code \
         ON watchlist_items(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_price_alerts_stock_code \
         ON price_alerts(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_decision_validations_pick_id \
         ON decision_validations(pick_id)",
        "CREATE INDEX IF NOT EXISTS idx_decision_validations_stock_code \
         ON decision_validations(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_financial_snapshots_stock_code \
         ON financial_snapshots(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_financial_snapshots_stock_date \
         ON financial_snapshots(stock_code, snapshot_date)",
        "CREATE INDEX IF NOT EXISTS idx_earnings_events_stock_code \
         ON earnings_events(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_news_archive_stock_code \
         ON news_archive(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_news_archive_publish_time \
         ON news_archive(publish_time)",
        "CREATE INDEX IF NOT EXISTS idx_quant_runs_strategy_id \
         ON quant_runs(strategy_id)",
        "CREATE INDEX IF NOT EXISTS idx_quant_runs_status \
         ON quant_runs(status)",
        "CREATE INDEX IF NOT EXISTS idx_quant_strategies_name \
         ON quant_strategies(name)",
        "CREATE INDEX IF NOT EXISTS idx_quant_signals_run_id \
         ON quant_signals(run_id)",
        "CREATE INDEX IF NOT EXISTS idx_quant_paper_trades_run_id \
         ON quant_paper_trades(run_id)",
        "CREATE INDEX IF NOT EXISTS idx_strategy_weight_history_strategy_id \
         ON strategy_weight_history(strategy_id)",
        // ── PHASE 1c 新表索引 ──
        "CREATE INDEX IF NOT EXISTS idx_reflection_lessons_stock_code \
         ON reflection_lessons(stock_code)",
        "CREATE INDEX IF NOT EXISTS idx_reflection_lessons_status \
         ON reflection_lessons(status)",
        "CREATE INDEX IF NOT EXISTS idx_fund_transfers_transfer_date \
         ON fund_transfers(transfer_date)",
        "CREATE INDEX IF NOT EXISTS idx_portfolio_correlation_snapshot_date \
         ON portfolio_correlation_snapshot(snapshot_date)",
        "CREATE INDEX IF NOT EXISTS idx_portfolio_metrics_daily_date \
         ON portfolio_metrics_daily(snapshot_date)",
        "CREATE INDEX IF NOT EXISTS idx_gateway_message_queue_status \
         ON gateway_message_queue(status)",
        "CREATE INDEX IF NOT EXISTS idx_gateway_message_queue_to_agent \
         ON gateway_message_queue(to_agent)",
    ] {
        db.execute_unprepared(sql).await?;
    }

    // ========================================================================
    // PHASE 3: 扩展 agency_experts / agent_profiles 的 category CHECK 约束
    //   上游 v100 的 CHECK 列表没有 'stock-analysis'，AxInvest 的 stock
    //   profile 插入会失败。此阶段在 PG 下用 ALTER CONSTRAINT 替换约束。
    //
    //   注意：v100 原约束含 'opc-company'/'opc-experts'（OPC 业务使用），
    //   重写时必须一并保留，否则 OPC 种子插入会违反
    //   agency_experts_category_check（见 [opc-company] Seed failed 日志）。
    //
    //   - PG: DROP CONSTRAINT + ADD CONSTRAINT（幂等，不存在则跳过）
    //   - SQLite: CHECK 约束在 CREATE TABLE 时固定，无法 ALTER；
    //     SQLite 不强制 CHECK 约束（除非 PRAGMA writable_schema=ON），
    //     所以 stock-analysis 值可直接插入，无需处理。
    // ========================================================================

    if is_pg {
        // agency_experts：旧约束名约定为 agency_experts_category_check（PG 自动生成）
        let _ = db
            .execute_raw(Statement::from_string(
                backend,
                "ALTER TABLE agency_experts DROP CONSTRAINT IF EXISTS agency_experts_category_check",
            ))
            .await;
        db.execute_raw(Statement::from_string(
            backend,
            "ALTER TABLE agency_experts ADD CONSTRAINT agency_experts_category_check \
             CHECK (category IN ('general','development','security','data','finance',\
             'devops','design','writing','business','opc-company','opc-experts',\
             'opc-industry','opc-domain','stock-analysis'))",
        ))
        .await?;

        // agent_profiles
        let _ = db
            .execute_raw(Statement::from_string(
                backend,
                "ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS agent_profiles_category_check",
            ))
            .await;
        db.execute_raw(Statement::from_string(
            backend,
            "ALTER TABLE agent_profiles ADD CONSTRAINT agent_profiles_category_check \
             CHECK (category IN ('general','development','security','data','finance',\
             'devops','design','writing','business','opc-company','opc-experts',\
             'opc-industry','opc-domain','stock-analysis'))",
        ))
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn v200_is_self_idempotent() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        // v200 依赖 v100 已建好的 agency_experts / agent_profiles 表
        super::super::v100_consolidated::up(db.clone()).await.unwrap();
        up(db.clone()).await.unwrap();
        // 第二次跑：所有 CREATE 都是 IF NOT EXISTS，ALTER 也是幂等
        up(db).await.expect("v200 must be re-runnable");
    }
}
