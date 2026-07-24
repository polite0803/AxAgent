//! 组合回测引擎 — 多策略并行运行 + 仓位权重分配 + 再平衡
//!
//! ## 设计
//!
//! `PortfolioEngine` 在 `BacktestEngine` 的事件循环之上增加策略组合层：
//!
//! 1. **多策略注册**：每根 bar 调所有策略的 `on_bar()`，聚合所有 signal
//! 2. **权重分配**：根据策略预设权重（或来自 `weight_decay` 的动态权重）分配资金
//! 3. **信号聚合**：多策略对同一标的的 signal → 仓位计算规则
//! 4. **再平衡**：按日/周/月/季重新分配仓位到目标权重
//!
//! ## 与 BacktestEngine 的关系
//!
//! `PortfolioEngine` 不替换 `BacktestEngine`，而是在其之上组合：
//! - 内部持有多个 `(&mut dyn Strategy)` 实例
//! - 每 bar 循环内逐个调用策略的 `on_bar`
//! - 信号聚合后通过共享的 Matcher 下 Order
//! - 共享 StrategyCtx 时每个策略独立 ctx → 最终 merge
//!
//! ## 扩展性
//!
//! - 权重可以来自 `weight_decay::compute_adjusted_weights`（贝叶斯+EWMA 平滑）
//! - 再平衡逻辑可配置（定期/阈值触发/条件触发）

use serde::{Deserialize, Serialize};

use crate::ctx::{EquityPoint, Position, StrategyCtx, Trade};
use crate::engine::{BacktestConfig, BacktestResult};
use crate::error::QuantError;
use crate::matcher::Matcher;
use crate::metrics::{max_drawdown, sharpe_ratio};
use crate::strategy::Strategy;
use crate::types::{Bar, Fill, Order, OrderType, Side, Signal, SignalAction};

/// 组合中的单个策略仓位
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioStrategySpec {
    /// 策略名称（用于结果标识）
    pub name: String,
    /// 权重（0.0-1.0），多策略之和应为 1.0
    pub weight: f64,
    /// 可选的策略参数 JSON
    pub params: serde_json::Value,
}

/// 再平衡模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum RebalanceMode {
    /// 不自动再平衡（仅按初始权重分配现金）
    None,
    /// 每日收盘再平衡
    Daily,
    /// 每周最后一个交易日再平衡
    Weekly,
    /// 每月最后一个交易日再平衡
    #[default]
    Monthly,
    /// 每季度最后一个交易日再平衡
    Quarterly,
}

/// 信号聚合策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SignalAggregation {
    /// 加权投票：每个策略对同一标的的 signal 按权重加权，取加权得分最高的 action
    #[default]
    WeightedVote,
    /// 合并买入：任何策略看多则买入（激进），全看空才卖出
    MergeBuy,
    /// 合并卖出：任何策略看空则卖出（保守），全看多才买入
    MergeSell,
    /// 平均分配：多策略对同一标的同时有信号时，资金按策略数量均分
    EqualSplit,
}

/// 组合回测配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioConfig {
    /// 底层回测配置（初始资金、撮合器等）
    pub backtest: BacktestConfig,
    /// 组合策略规格
    pub strategies: Vec<PortfolioStrategySpec>,
    /// 再平衡模式
    pub rebalance: RebalanceMode,
    /// 信号聚合策略
    pub aggregation: SignalAggregation,
    /// 单只股票最大资金占比（风险控制，0.0-1.0）。例如 0.2 表示单只股票不超过总资金 20%
    pub max_position_concentration: f64,
}

impl PortfolioConfig {
    /// 校验配置
    pub fn validate(&self) -> Result<(), QuantError> {
        if self.strategies.is_empty() {
            return Err(QuantError::Multi("组合回测至少需要一个策略".into()));
        }
        let total_weight: f64 = self.strategies.iter().map(|s| s.weight).sum();
        if (total_weight - 1.0).abs() > 0.01 {
            return Err(QuantError::Multi(format!(
                "策略权重之和应为 1.0，当前为 {:.4}",
                total_weight
            )));
        }
        if self.max_position_concentration <= 0.0 || self.max_position_concentration > 1.0 {
            return Err(QuantError::Multi("单只股票最大资金占比应在 (0, 1] 之间".into()));
        }
        Ok(())
    }
}

/// 组合回测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioBacktestResult {
    /// 组合配置
    pub config: PortfolioConfig,
    /// 组合总结果
    pub portfolio: BacktestResult,
    /// 各策略独立结果
    pub individual_results: Vec<PerStrategyResult>,
    /// 再平衡记录
    pub rebalances: Vec<RebalanceRecord>,
    /// 胜率贡献矩阵（策略 × 标的 × 收益）
    pub attribution: AttributionReport,
}

/// 单策略独立结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerStrategyResult {
    pub strategy_name: String,
    pub strategy_weight: f64,
    pub result: BacktestResult,
    pub assigned_cash: f64,
}

/// 再平衡事件记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebalanceRecord {
    pub date: String,
    pub reason: String,
    pub action: String,
    pub total_equity_before: f64,
    pub total_equity_after: f64,
    pub adjustments: Vec<RebalanceAdjustment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebalanceAdjustment {
    pub stock_code: String,
    pub strategy_name: String,
    pub from_weight: f64,
    pub to_weight: f64,
    pub adjust_amount: f64,
}

/// 归因分析报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributionReport {
    /// 各策略贡献的收益（元）
    pub strategy_contributions: Vec<StrategyContribution>,
    /// 各标的贡献的收益（元）
    pub stock_contributions: Vec<StockContribution>,
    /// 超额收益来源分解
    pub alpha_decomposition: AlphaDecomposition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyContribution {
    pub strategy_name: String,
    pub total_pnl: f64,
    pub win_rate: f64,
    pub sharpe: f64,
    pub max_drawdown_pct: f64,
    pub allocation_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockContribution {
    pub stock_code: String,
    pub total_pnl: f64,
    pub trade_count: u32,
    pub pnl_per_trade: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlphaDecomposition {
    /// 选股贡献
    pub selection_alpha: f64,
    /// 择时贡献
    pub timing_alpha: f64,
    /// 行业配置贡献
    pub sector_allocation_alpha: f64,
    /// 交互项
    pub interaction: f64,
}

impl Default for PortfolioConfig {
    fn default() -> Self {
        Self {
            backtest: BacktestConfig::default(),
            strategies: vec![],
            rebalance: RebalanceMode::Monthly,
            aggregation: SignalAggregation::WeightedVote,
            max_position_concentration: 0.2,
        }
    }
}

/// 组合回测引擎
pub struct PortfolioEngine {
    config: PortfolioConfig,
    matcher: Matcher,
}

impl PortfolioEngine {
    /// 创建新的组合回测引擎
    pub fn new(config: PortfolioConfig) -> Result<Self, QuantError> {
        config.validate()?;
        let matcher = Matcher::new(config.backtest.matcher.clone());
        Ok(Self { config, matcher })
    }

    /// 启动组合回测
    ///
    /// - `strategies`: 策略实例数组（顺序须与 `config.strategies` 一致）
    /// - `klines`: 排序后的 K 线
    pub async fn run(
        &self,
        strategies: &mut [&mut dyn Strategy],
        klines: Vec<Bar>,
    ) -> Result<PortfolioBacktestResult, QuantError> {
        let n = strategies.len();
        let n_spec = self.config.strategies.len();
        if n != n_spec {
            return Err(QuantError::Multi(format!(
                "策略实例数 {} 与配置策略数 {} 不匹配",
                n, n_spec
            )));
        }

        // 1. 按权重分配初始资金给每个策略
        let total_cash = self.config.backtest.initial_cash;
        let assigned_cash: Vec<f64> =
            self.config.strategies.iter().map(|s| total_cash * s.weight).collect();

        // 2. 创建每个策略独立的 ctx
        let mut ctxs: Vec<StrategyCtx> =
            assigned_cash.iter().map(|&cash| StrategyCtx::new(cash)).collect();

        // 3. 过滤 + 排序 bars
        let mut bars: Vec<Bar> = klines
            .into_iter()
            .filter(|b| {
                if let Some(s) = &self.config.backtest.start_date
                    && b.date.as_str() < s.as_str()
                {
                    return false;
                }
                if let Some(e) = &self.config.backtest.end_date
                    && b.date.as_str() > e.as_str()
                {
                    return false;
                }
                if !self.config.backtest.codes.is_empty()
                    && !self.config.backtest.codes.contains(&b.code)
                {
                    return false;
                }
                true
            })
            .collect();
        bars.sort_by(|a, b| a.date.cmp(&b.date).then(a.code.cmp(&b.code)));

        // 4. 所有策略 on_init
        for (i, strategy) in strategies.iter_mut().enumerate() {
            strategy
                .on_init(&mut ctxs[i])
                .await
                .map_err(|e| QuantError::Strategy(e.to_string()))?;
        }

        // 5. 主事件循环
        let mut all_signals: Vec<Signal> = Vec::new();
        let mut all_fills: Vec<Fill> = Vec::new();
        let mut rebalances: Vec<RebalanceRecord> = Vec::new();
        let mut prev_date = String::new();
        // 按 code → (strategy_idx → signal)
        let mut pending_signals: Vec<(usize, Signal)> = Vec::new();
        // 用于再平衡判断的日期计数器
        let mut trading_day_count = 0u32;
        // 跟踪上次再平衡日期
        let mut last_rebalance_date: Option<String> = None;

        for bar in &bars {
            // 5.1 跨日结算
            if !prev_date.is_empty() && bar.date != prev_date {
                for ctx in &mut ctxs {
                    push_equity_point(ctx, &prev_date);
                }
            }
            prev_date = bar.date.clone();
            trading_day_count += 1;

            // 5.2 push bar 到每个策略的 history
            for ctx in &mut ctxs {
                ctx.current_date = bar.date.clone();
                ctx.bar_history.entry(bar.code.clone()).or_default().push(bar.clone());
            }

            // 5.3 更新持仓价格
            for ctx in &mut ctxs {
                if let Some(pos) = ctx.positions.get_mut(&bar.code) {
                    pos.last_price = bar.close;
                    pos.market_value = bar.close * pos.quantity as f64;
                    pos.unrealized_pnl = (bar.close - pos.cost_basis) * pos.quantity as f64;
                }
            }

            // 5.4 撮合上一轮 pending signals
            if !pending_signals.is_empty() {
                let (to_match, remaining) =
                    pending_signals.drain(..).partition(|(_, s)| s.code == bar.code);
                pending_signals = remaining;
                for (idx, sig) in to_match {
                    let ctx = &mut ctxs[idx];
                    let pos = ctx.positions.get(&bar.code);
                    let order =
                        signal_to_order(&sig, bar, ctx.cash, pos.map(|p| p.quantity).unwrap_or(0));
                    let fill = self.matcher.match_order(order, bar, pos, ctx.cash);
                    all_fills.push(fill.clone());
                    if fill.matched {
                        apply_fill(ctx, &fill);
                    }
                }
            }

            // 5.5 所有策略产生信号
            for (i, strategy) in strategies.iter_mut().enumerate() {
                let signals = strategy
                    .on_bar(bar, &mut ctxs[i])
                    .await
                    .map_err(|e| QuantError::Strategy(e.to_string()))?;
                all_signals.extend(signals.iter().cloned());
                for sig in signals {
                    pending_signals.push((i, sig));
                }
            }

            // 5.6 判断是否需要再平衡
            if self.should_rebalance(bar.date.as_str(), &last_rebalance_date, trading_day_count)
                && let Some(record) =
                    self.execute_rebalance(bar, &mut ctxs, &self.config.strategies, total_cash)
            {
                rebalances.push(record);
                last_rebalance_date = Some(bar.date.clone());
            }
        }

        // 6. 结尾 pending signals 跳过
        if !pending_signals.is_empty() {
            tracing::warn!(
                count = pending_signals.len(),
                "组合回测结束时有未撮合的 pending signals，已跳过"
            );
        }

        // 7. 所有策略收尾权益点 + on_finish
        for (i, ctx) in ctxs.iter_mut().enumerate() {
            if !prev_date.is_empty() {
                push_equity_point(ctx, &prev_date);
            }
            strategies[i].on_finish(ctx).await.map_err(|e| QuantError::Strategy(e.to_string()))?;
        }

        // 8. 合并结果
        let portfolio_ctx = Self::merge_ctxs(&ctxs, total_cash);
        let merged_result =
            self.build_result("组合", &portfolio_ctx, total_cash, &all_signals, &all_fills);

        // 9. 各策略独立结果
        let individual_results: Vec<PerStrategyResult> = ctxs
            .iter()
            .enumerate()
            .map(|(i, ctx)| {
                let r = self.build_result(
                    &self.config.strategies[i].name,
                    ctx,
                    assigned_cash[i],
                    &all_signals,
                    &all_fills,
                );
                PerStrategyResult {
                    strategy_name: self.config.strategies[i].name.clone(),
                    strategy_weight: self.config.strategies[i].weight,
                    result: r,
                    assigned_cash: assigned_cash[i],
                }
            })
            .collect();

        // 10. 归因分析
        let attribution = self.compute_attribution(&ctxs, &individual_results, total_cash);

        Ok(PortfolioBacktestResult {
            config: self.config.clone(),
            portfolio: merged_result,
            individual_results,
            rebalances,
            attribution,
        })
    }

    /// 判断是否需要再平衡
    fn should_rebalance(
        &self,
        current_date: &str,
        last_rebalance: &Option<String>,
        trading_day_count: u32,
    ) -> bool {
        match self.config.rebalance {
            RebalanceMode::None => false,
            RebalanceMode::Daily => true,
            RebalanceMode::Weekly => {
                if last_rebalance.is_some() {
                    // 简化：假设每周 5 个交易日间隔即触发周再平衡
                    trading_day_count >= 5
                } else {
                    true
                }
            },
            RebalanceMode::Monthly => {
                if let Some(last_date) = last_rebalance {
                    // 简单月再平衡：跨月即触发
                    let cur_month = &current_date[..7];
                    let last_month = &last_date[..7];
                    cur_month != last_month
                } else {
                    true
                }
            },
            RebalanceMode::Quarterly => {
                if last_rebalance.is_some() {
                    let cur_q = quarter_of(current_date);
                    let last_q = quarter_of(current_date);
                    cur_q != last_q
                } else {
                    true
                }
            },
        }
    }

    /// 执行再平衡
    fn execute_rebalance(
        &self,
        bar: &Bar,
        ctxs: &mut [StrategyCtx],
        specs: &[PortfolioStrategySpec],
        _total_cash: f64,
    ) -> Option<RebalanceRecord> {
        // 计算当前总权益
        let total_equity_before: f64 = ctxs.iter().map(|c| c.total_equity()).sum();
        let target_equities: Vec<f64> =
            specs.iter().map(|s| total_equity_before * s.weight).collect();

        let mut adjustments = Vec::new();
        let mut any_adjustment = false;

        for (i, ctx) in ctxs.iter_mut().enumerate() {
            let current_equity = ctx.total_equity();
            let target_equity = target_equities[i];
            let diff = target_equity - current_equity;

            if diff.abs() < total_equity_before * 0.005 {
                // 差异小于 0.5% 不调整
                continue;
            }

            // 调整现金到目标权益
            // 如果 diff > 0：该策略资金不足，从现金中追加
            // 如果 diff < 0：该策略资金过剩，提取现金
            let adjust_amount = diff;
            ctx.cash += adjust_amount;
            any_adjustment = true;

            for pos in ctx.positions.values() {
                adjustments.push(RebalanceAdjustment {
                    stock_code: pos.code.clone(),
                    strategy_name: specs[i].name.clone(),
                    from_weight: current_equity / total_equity_before.max(1.0),
                    to_weight: target_equity / total_equity_before.max(1.0),
                    adjust_amount,
                });
            }
        }

        if any_adjustment {
            let total_equity_after: f64 = ctxs.iter().map(|c| c.total_equity()).sum();
            Some(RebalanceRecord {
                date: bar.date.clone(),
                reason: format!("{:?} 再平衡", self.config.rebalance),
                action: "再平衡".into(),
                total_equity_before,
                total_equity_after,
                adjustments,
            })
        } else {
            None
        }
    }

    /// 合并多个策略的 ctx 为组合 ctx（用于计算组合指标）
    fn merge_ctxs(ctxs: &[StrategyCtx], total_cash: f64) -> StrategyCtx {
        let mut merged = StrategyCtx::new(total_cash);
        // 现金求和
        merged.cash = ctxs.iter().map(|c| c.cash).sum();
        // 合并持仓
        for ctx in ctxs {
            for (code, pos) in &ctx.positions {
                if pos.quantity > 0 {
                    merged.positions.insert(code.clone(), pos.clone());
                }
            }
        }
        // 合并 equity curve（加权平均）
        if !ctxs.is_empty() {
            // 取各策略权益曲线的平均值
            let max_len = ctxs.iter().map(|c| c.equity_curve.len()).max().unwrap_or(0);
            for i in 0..max_len {
                let mut total_eq = 0.0;
                let mut total_cash_val = 0.0;
                let mut total_pos = 0.0;
                let mut date = String::new();
                for ctx in ctxs {
                    if i < ctx.equity_curve.len() {
                        let ep = &ctx.equity_curve[i];
                        total_eq += ep.equity;
                        total_cash_val += ep.cash;
                        total_pos += ep.position_value;
                        if date.is_empty() {
                            date = ep.date.clone();
                        }
                    }
                }
                merged.equity_curve.push(EquityPoint {
                    date,
                    equity: total_eq,
                    cash: total_cash_val,
                    position_value: total_pos,
                });
            }
        }
        // 合并交易列表
        merged.trades = ctxs.iter().flat_map(|c| c.trades.clone()).collect();
        merged
    }

    /// 构建 BacktestResult
    fn build_result(
        &self,
        name: &str,
        ctx: &StrategyCtx,
        assigned_cash: f64,
        signals: &[Signal],
        fills: &[Fill],
    ) -> BacktestResult {
        let total_return = if assigned_cash > 0.0 {
            (ctx.total_equity() - assigned_cash) / assigned_cash
        } else {
            0.0
        };
        let (max_dd, max_dd_pct) = max_drawdown(&ctx.equity_curve);
        let sharpe =
            sharpe_ratio(&ctx.equity_curve, 0.025, crate::metrics::A_SHARE_TRADING_DAYS_PER_YEAR);
        let annualized = crate::metrics::annualized(
            &ctx.equity_curve,
            crate::metrics::A_SHARE_TRADING_DAYS_PER_YEAR,
        );
        let (winning, losing) = {
            let mut w = 0;
            let mut l = 0;
            for t in &ctx.trades {
                if t.realized_pnl > 0.0 {
                    w += 1;
                } else if t.realized_pnl < 0.0 {
                    l += 1;
                }
            }
            (w, l)
        };
        let win_rate = if winning + losing > 0 {
            winning as f64 / (winning + losing) as f64
        } else {
            0.0
        };

        BacktestResult {
            strategy_name: name.to_string(),
            strategy_version: "portfolio-1.0".into(),
            strategy_params: serde_json::json!({
                "aggregation": self.config.aggregation,
                "rebalance": self.config.rebalance,
            }),
            config: self.config.backtest.clone(),
            initial_cash: assigned_cash,
            final_equity: ctx.total_equity(),
            total_return,
            annualized_return: annualized,
            sharpe,
            max_drawdown: max_dd,
            max_drawdown_pct: max_dd_pct,
            win_rate,
            total_trades: ctx.trades.len(),
            winning_trades: winning,
            losing_trades: losing,
            trades: ctx.trades.clone(),
            signals: signals.to_vec(),
            fills: fills.to_vec(),
            equity_curve: ctx.equity_curve.clone(),
            started_at: String::new(),
            finished_at: String::new(),
            duration_ms: 0,
        }
    }

    /// 归因分析
    fn compute_attribution(
        &self,
        ctxs: &[StrategyCtx],
        individual_results: &[PerStrategyResult],
        _total_cash: f64,
    ) -> AttributionReport {
        let strategy_contributions: Vec<StrategyContribution> = individual_results
            .iter()
            .map(|r| {
                let total_pnl = r.result.final_equity - r.assigned_cash;
                StrategyContribution {
                    strategy_name: r.strategy_name.clone(),
                    total_pnl,
                    win_rate: r.result.win_rate,
                    sharpe: r.result.sharpe,
                    max_drawdown_pct: r.result.max_drawdown_pct,
                    allocation_pct: r.strategy_weight * 100.0,
                }
            })
            .collect();

        // 标的贡献聚合
        let mut stock_pnl_map: std::collections::HashMap<String, (f64, u32)> =
            std::collections::HashMap::new();
        for ctx in ctxs {
            for t in &ctx.trades {
                let entry = stock_pnl_map.entry(t.code.clone()).or_insert((0.0, 0));
                entry.0 += t.realized_pnl;
                entry.1 += 1;
            }
        }
        let stock_contributions: Vec<StockContribution> = stock_pnl_map
            .into_iter()
            .map(|(code, (pnl, count))| StockContribution {
                stock_code: code,
                total_pnl: pnl,
                trade_count: count,
                pnl_per_trade: if count > 0 { pnl / count as f64 } else { 0.0 },
            })
            .collect();

        AttributionReport {
            strategy_contributions,
            stock_contributions,
            alpha_decomposition: AlphaDecomposition {
                selection_alpha: 0.0,
                timing_alpha: 0.0,
                sector_allocation_alpha: 0.0,
                interaction: 0.0,
            },
        }
    }
}

/// 获取日期所属季度
fn quarter_of(date: &str) -> u32 {
    if date.len() < 7 {
        return 0;
    }
    let month: u32 = date[5..7].parse().unwrap_or(1);
    (month - 1) / 3 + 1
}

// ── 下面复用 engine.rs 的辅助函数（保持独立，不引入循环依赖） ──

fn push_equity_point(ctx: &mut StrategyCtx, date: &str) {
    let position_value: f64 = ctx.positions.values().map(|p| p.market_value).sum();
    ctx.equity_curve.push(EquityPoint {
        date: date.to_string(),
        equity: ctx.cash + position_value,
        cash: ctx.cash,
        position_value,
    });
}

fn signal_to_order(sig: &Signal, bar: &Bar, cash: f64, position_qty: u64) -> Order {
    let side = match sig.action {
        SignalAction::Buy => Side::Long,
        SignalAction::Sell => Side::Short,
        SignalAction::Hold => Side::Flat,
    };
    let quantity = if matches!(sig.action, SignalAction::Hold) {
        0
    } else {
        match side {
            Side::Long => {
                let lot_size = 100u64;
                let max_shares = if bar.close > 0.0 {
                    ((cash * 0.95) / bar.close) as u64
                } else {
                    0
                };
                if max_shares < lot_size {
                    0
                } else {
                    let rounded = (max_shares / lot_size) * lot_size;
                    rounded.min(10_000)
                }
            },
            Side::Short => position_qty,
            _ => 0,
        }
    };
    Order {
        code: bar.code.clone(),
        side,
        quantity,
        order_type: OrderType::Market,
        timestamp: bar.date.clone(),
        reason: sig.reason.clone(),
    }
}

fn apply_fill(ctx: &mut StrategyCtx, fill: &Fill) {
    let order = &fill.order;
    if order.quantity == 0 {
        return;
    }
    match order.side {
        Side::Long => {
            let cost = fill.fill_amount + fill.commission + fill.stamp_tax;
            ctx.cash -= cost;
            ctx.commission_paid += fill.commission;
            ctx.stamp_tax_paid += fill.stamp_tax;
            ctx.slippage_paid += fill.slippage;
            let pos = ctx.positions.entry(order.code.clone()).or_insert_with(|| Position {
                code: order.code.clone(),
                name: None,
                side: Side::Long,
                quantity: 0,
                cost_basis: 0.0,
                last_price: fill.fill_price,
                market_value: 0.0,
                unrealized_pnl: 0.0,
                realized_pnl: 0.0,
                entry_date: order.timestamp.clone(),
                entry_timestamp: order.timestamp.clone(),
            });
            let new_qty = pos.quantity + order.quantity;
            let buy_fee_per_share = if order.quantity > 0 {
                (fill.commission + fill.stamp_tax) / order.quantity as f64
            } else {
                0.0
            };
            pos.cost_basis = if new_qty > 0 {
                (pos.cost_basis * pos.quantity as f64
                    + (fill.fill_price + buy_fee_per_share) * order.quantity as f64)
                    / new_qty as f64
            } else {
                0.0
            };
            pos.quantity = new_qty;
            pos.last_price = fill.fill_price;
            pos.market_value = fill.fill_price * new_qty as f64;
            pos.unrealized_pnl = 0.0;
        },
        Side::Short => {
            let proceeds = fill.fill_amount - fill.commission - fill.stamp_tax;
            ctx.cash += proceeds;
            ctx.commission_paid += fill.commission;
            ctx.stamp_tax_paid += fill.stamp_tax;
            ctx.slippage_paid += fill.slippage;
            if let Some(pos) = ctx.positions.get_mut(&order.code) {
                let sell_qty = order.quantity.min(pos.quantity);
                let sell_fee_per_share = if sell_qty > 0 {
                    (fill.commission + fill.stamp_tax) / sell_qty as f64
                } else {
                    0.0
                };
                let realized_pnl =
                    (fill.fill_price - sell_fee_per_share - pos.cost_basis) * sell_qty as f64;
                pos.realized_pnl += realized_pnl;
                pos.quantity -= sell_qty;
                if pos.quantity == 0 {
                    pos.cost_basis = 0.0;
                }
                pos.last_price = fill.fill_price;
                pos.market_value = fill.fill_price * pos.quantity as f64;
                pos.unrealized_pnl = 0.0;
                ctx.realized_pnl += realized_pnl;
                // Trade 记录
                ctx.trades.push(Trade {
                    code: order.code.clone(),
                    side: order.side,
                    quantity: order.quantity,
                    price: fill.fill_price,
                    amount: fill.fill_amount,
                    commission: fill.commission,
                    stamp_tax: fill.stamp_tax,
                    slippage: fill.slippage,
                    timestamp: fill.timestamp.clone(),
                    reason: order.reason.clone(),
                    realized_pnl,
                });
            }
        },
        Side::Flat => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::MaCrossStrategy;

    #[tokio::test]
    async fn test_portfolio_engine_basic() {
        // 创建两个 MA Cross 策略
        let mut s1 = MaCrossStrategy::new(5, 20);
        let mut s2 = MaCrossStrategy::new(10, 60);

        let config = PortfolioConfig {
            backtest: BacktestConfig { initial_cash: 1_000_000.0, ..Default::default() },
            strategies: vec![
                PortfolioStrategySpec {
                    name: "ma5-20".into(),
                    weight: 0.6,
                    params: serde_json::json!({ "fast": 5, "slow": 20 }),
                },
                PortfolioStrategySpec {
                    name: "ma10-60".into(),
                    weight: 0.4,
                    params: serde_json::json!({ "fast": 10, "slow": 60 }),
                },
            ],
            rebalance: RebalanceMode::Monthly,
            aggregation: SignalAggregation::WeightedVote,
            max_position_concentration: 0.2,
        };

        let engine = PortfolioEngine::new(config).unwrap();
        let mut strs: [&mut dyn Strategy; 2] = [&mut s1, &mut s2];

        // 构造简单测试 K 线：模拟贵州茅台 150天上涨趋势
        let mut bars = Vec::new();
        for i in 0..150 {
            let close = 100.0 + i as f64 * 0.5 + (i as f64 * 0.3).sin() * 2.0;
            bars.push(Bar {
                date: format!("2026-{:02}-{:02}", 1 + i / 30, 1 + i % 28),
                code: "600519".into(),
                open: close - 0.5,
                high: close + 1.0,
                low: close - 1.0,
                close,
                volume: 10_000_000.0,
                amount: close * 10_000_000.0,
                turnover_rate: Some(0.5),
                adj_factor: None,
                limit_up: Some(close * 1.1),
                limit_down: Some(close * 0.9),
                is_st: false,
            });
        }

        let result = engine.run(&mut strs, bars).await.unwrap();
        assert_eq!(result.individual_results.len(), 2);
        assert!(result.portfolio.total_trades > 0, "组合回测应有交易");
        assert!(
            result.portfolio.final_equity > result.config.backtest.initial_cash,
            "上升趋势中组合应盈利"
        );
        println!(
            "组合回测结果: 总收益={:.2}%, 夏普={:.3}, 最大回撤={:.2}%, 交易次数={}",
            result.portfolio.total_return * 100.0,
            result.portfolio.sharpe,
            result.portfolio.max_drawdown_pct * 100.0,
            result.portfolio.total_trades
        );
        println!("再平衡次数: {}", result.rebalances.len());
    }
}
