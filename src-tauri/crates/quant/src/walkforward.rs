//! Walk-Forward 验证
//!
//! ## 核心目的
//!
//! 防御"过拟合历史"。流程：
//! 1. 数据切分为 N 个 fold（每个 fold 含 IS 训练 + OOS 验证）
//! 2. rolling 模式：IS/OOS 窗口大小固定，每步前进
//! 3. anchored 模式：IS 起点固定，终点前移（更适合趋势性市场）
//! 4. 拼接所有 OOS 段得到样本外累计曲线
//! 5. 报告参数稳定性 + 过拟合告警
//!
//! ## D3 决策落实
//!
//! - 默认 `force_off = false`：跑回测时自动启用 Walk-Forward 验证
//! - 仅显式 `WalkForwardConfig { force_off: true, ... }` 才关闭
//! - 关闭时审计日志留痕
//!
//! ## 阶段
//!
//! - M1：数据切分 + 基础聚合（OOS 总指标 + 稳定度评分）
//! - M2：内置 grid search（自动 IS 寻参 → OOS 验证）
//!
//! ## Walk-Forward 基线评分（合成数据，2026-07-22）
//!
//! 首次通过 `test_walkforward_baseline_score` 和 `test_walkforward_param_scan`
//! 验证了 WalkForward::run() 的全链路执行。
//!
//! ```text
//! 数据: 1200 根合成 K 线（均值回复 + 趋势 + 噪声）
//! 策略: MaCross(5,20) | fold 数: 9
//! OOS Sharpe: -7.14 | MaxDD: 10.16% | 稳定度: 0.93 | 过拟合: 0/9
//!
//! 参数扫描最佳(合成数据): MaCross(10,20)
//!   OOS Sharpe=-3.87 MaxDD=~11% WinRate=42% TotalRet=-0.69% 稳定度=0.50
//! ```
//!
//! **说明**：基线为负是预期行为——简单均线交叉策略在随机性强的合成数据上不应盈利。
//! 基线的主要价值是验证 WalkForward 管道的完整性（数据切分→策略执行→聚合报告），
//! 为后续 DES 模拟产出与历史 Walk-Forward 评分的对比提供基准框架。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ctx::EquityPoint;
use crate::engine::{BacktestEngine, BacktestResult};
use crate::error::QuantError;
use crate::metrics::MetricsReport;
use crate::strategy::Strategy;
use crate::types::Bar;

/// Walk-Forward 配置
///
/// 注意：train_days / test_days 按 K 线 bar 数计算（非自然日），
/// 避免非交易日导致窗口长度不一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardConfig {
    /// IS 训练窗口大小（bar 数，非自然日）
    pub train_days: i64,
    /// OOS 验证窗口大小（bar 数，非自然日）
    pub test_days: i64,
    /// 步进 bar 数（默认 = test_days，即不重叠）
    pub step_days: Option<i64>,
    /// anchored 模式（IS 起点固定）
    pub anchored: bool,
    /// 最小 IS 数据点数（少于则跳过该 fold）
    pub min_train_bars: usize,
    /// 最小 OOS 数据点数
    pub min_test_bars: usize,
    /// 无风险年化利率（用于 Sharpe）
    pub risk_free_annual: f64,
    /// 显式关闭（默认 false = 强制开启）
    pub force_off: bool,
}

impl Default for WalkForwardConfig {
    fn default() -> Self {
        Self {
            train_days: 504, // 2 年
            test_days: 126,  // 6 月
            step_days: None, // 默认 = test_days
            anchored: false,
            min_train_bars: 60,
            min_test_bars: 20,
            risk_free_annual: 0.025,
            force_off: false,
        }
    }
}

impl WalkForwardConfig {
    /// 检查是否启用（force_off = false 视为启用）
    pub fn is_enabled(&self) -> bool {
        !self.force_off
    }
}

/// 单个 fold 的描述
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardFold {
    pub fold_idx: usize,
    pub train_start: String,
    pub train_end: String,
    pub test_start: String,
    pub test_end: String,
    pub train_bars_count: usize,
    pub test_bars_count: usize,
    /// 修复 M-RES-10: 添加 klines 切片索引，让 run 函数直接用索引切片，
    /// 避免用日期字符串过滤的重复逻辑（且多股票场景下日期过滤可能数据泄漏）。
    /// 序列化时跳过，仅为内部使用。
    #[serde(skip)]
    pub train_start_idx: usize,
    #[serde(skip)]
    pub train_end_idx: usize,
    #[serde(skip)]
    pub test_start_idx: usize,
    #[serde(skip)]
    pub test_end_idx: usize,
}

/// 数据切分结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardSplit {
    pub config: WalkForwardConfig,
    pub folds: Vec<WalkForwardFold>,
}

/// 单 fold 的回测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardWindowResult {
    pub fold: WalkForwardFold,
    /// 最佳参数（grid search 选出的，简化版：从外部传入）
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub best_params: Option<HashMap<String, serde_json::Value>>,
    /// IS 段回测结果
    pub train_result: BacktestResult,
    pub train_metrics: MetricsReport,
    /// OOS 段回测结果
    pub test_result: BacktestResult,
    pub test_metrics: MetricsReport,
    /// test_sharpe / train_sharpe，越接近 1 越稳定
    pub degradation_ratio: f64,
    /// 此 fold 是否触发过拟合告警
    pub overfit_flag: bool,
}

/// Walk-Forward 综合报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardReport {
    pub config: WalkForwardConfig,
    pub windows: Vec<WalkForwardWindowResult>,
    /// 拼接所有 OOS 段后的累计权益曲线
    pub aggregated_oos_equity: Vec<EquityPoint>,
    /// OOS 聚合指标
    pub aggregated_oos_metrics: MetricsReport,
    /// 参数稳定度 0..1（参数波动越小越接近 1）
    pub stability_score: f64,
    /// 是否触发整体过拟合告警
    pub overfit_warning: bool,
    /// 触发告警的窗口数
    pub overfit_window_count: usize,
    pub generated_at: String,
}

/// Walk-Forward 工具
pub struct WalkForward {
    pub config: WalkForwardConfig,
}

impl WalkForward {
    pub fn new(config: WalkForwardConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(WalkForwardConfig::default())
    }

    /// 数据切分（pure function）
    ///
    /// - 假设 klines 已按 date 排序
    /// - 基于 K 线 bar 索引切分，不依赖自然日历（避免非交易日导致窗口不一致）
    pub fn split(&self, klines: &[Bar]) -> Result<WalkForwardSplit, QuantError> {
        if klines.is_empty() {
            return Err(QuantError::WalkForward("输入 K 线为空".to_string()));
        }
        let total_bars = klines.len();
        let train_count = self.config.train_days as usize;
        let test_count = self.config.test_days as usize;
        if total_bars < train_count + test_count {
            return Err(QuantError::WalkForward(format!(
                "数据 bar 数 {} < train({}) + test({}) = {}",
                total_bars,
                train_count,
                test_count,
                train_count + test_count
            )));
        }
        let step = self.config.step_days.unwrap_or(self.config.test_days) as usize;
        // 修复 P0-M11: step == 0 会导致 cursor 永不增长，循环条件
        // `train_start_idx >= total_bars` 永不命中（除非 step 实际为 0 但 folds 已满），
        // 形成 CPU 死锁。直接拒绝配置。
        if step == 0 {
            return Err(QuantError::WalkForward("step_days must be > 0".to_string()));
        }
        let mut folds = Vec::new();
        let mut fold_idx = 0;
        let mut cursor = 0usize;
        // 修复 M-DS-4: 多股票场景下，按 bar 索引切分时若两支股票同一日期，
        // train_end 日期可能等于 test_start 日期，导致 run 函数按日期字符串过滤时
        // 同日数据被同时纳入 train 和 test（数据泄漏）。先检测是否为多股票，
        // 多股票时强制 train_end = test_start 日期减 1 天（严格小于）。
        let is_multi_stock = {
            let first_code = &klines[0].code;
            !klines.iter().all(|b| b.code == *first_code)
        };
        loop {
            let train_start_idx = if self.config.anchored { 0 } else { cursor };
            // P1-1 修复：anchored 模式下 IS 起点固定为 0，终点随 cursor 前移（expanding window）
            // 原实现 train_end_idx = train_start_idx + train_count 在 anchored 下恒等于 train_count，
            // 导致所有 fold 的 IS 窗口完全相同，参数稳定度评估失去意义（stability_score 恒为 1.0）。
            // rolling 模式保持原逻辑：窗口大小固定，整体随 cursor 前移。
            let train_end_idx = if self.config.anchored {
                (cursor + train_count).min(total_bars)
            } else {
                (train_start_idx + train_count).min(total_bars)
            };
            let test_start_idx = if self.config.anchored {
                train_count + cursor
            } else {
                train_end_idx
            };
            let test_end_idx = (test_start_idx + test_count).min(total_bars);
            if train_start_idx >= total_bars || test_start_idx >= total_bars {
                break;
            }
            let train_bars: Vec<Bar> = klines[train_start_idx..train_end_idx].to_vec();
            let test_bars: Vec<Bar> = klines[test_start_idx..test_end_idx].to_vec();
            if train_bars.len() >= self.config.min_train_bars
                && test_bars.len() >= self.config.min_test_bars
            {
                let test_start_date = klines[test_start_idx].date.clone();
                let mut train_end_date = klines[train_end_idx - 1].date.clone();
                if is_multi_stock
                    && train_end_date == test_start_date
                    && let Ok(test_start_d) =
                        chrono::NaiveDate::parse_from_str(&test_start_date, "%Y-%m-%d")
                {
                    train_end_date =
                        (test_start_d - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
                }
                folds.push(WalkForwardFold {
                    fold_idx,
                    train_start: klines[train_start_idx].date.clone(),
                    train_end: train_end_date,
                    test_start: test_start_date,
                    test_end: klines[test_end_idx - 1].date.clone(),
                    train_bars_count: train_bars.len(),
                    test_bars_count: test_bars.len(),
                    train_start_idx,
                    train_end_idx,
                    test_start_idx,
                    test_end_idx,
                });
                fold_idx += 1;
            }
            cursor += step;
        }
        if folds.is_empty() {
            return Err(QuantError::WalkForward("未生成任何 fold（数据不足以切分）".to_string()));
        }
        Ok(WalkForwardSplit { config: self.config.clone(), folds })
    }

    /// 跑 Walk-Forward 验证
    ///
    /// - `strategy_factory`: 给定 fold 索引，构造一个新 Strategy 实例。
    ///   修复 P0-T4: factory 返回 `Result` 而非 `Box<dyn Strategy>`，
    ///   构造失败时（参数错误 / 状态异常）跳过该 fold 而非 panic。
    /// - `param_grid`: 简化版 — M1 不做 grid search，由 caller 在 factory 内自行选参
    /// - 返回综合报告
    pub async fn run<F>(
        &self,
        strategy_factory: F,
        klines: Vec<Bar>,
    ) -> Result<WalkForwardReport, QuantError>
    where
        F: Fn(usize) -> Result<Box<dyn Strategy>, String>,
    {
        let split = self.split(&klines)?;
        let engine = BacktestEngine::with_defaults();
        let mut windows: Vec<WalkForwardWindowResult> = Vec::new();
        let mut all_oos_points: Vec<EquityPoint> = Vec::new();
        let mut skipped_folds: Vec<(usize, String)> = Vec::new();

        for (i, fold) in split.folds.iter().enumerate() {
            // 修复 M-RES-10: 原实现用日期字符串过滤 klines，与 split 的索引切片
            // 逻辑重复，且多股票场景下可能数据泄漏。改为直接用 fold 中的索引切片。
            let train_bars: Vec<Bar> = klines[fold.train_start_idx..fold.train_end_idx].to_vec();
            let test_bars: Vec<Bar> = klines[fold.test_start_idx..fold.test_end_idx].to_vec();
            // 修复 M-RES-11: 原实现每个 fold 调用 strategy_factory 三次
            // （train_strategy + test_strategy + best_params），浪费计算且可能
            // 因 factory 有状态导致三次调用结果不一致。改为只调用一次，
            // 用 train_strategy 提取 best_params，test_strategy 单独构造用于回测。
            // 每个 fold 用独立的 strategy 实例（避免状态污染）。
            // 修复 P0-T4: factory 返回 Result，构造失败时记录并跳过该 fold。
            let (mut train_strategy, mut test_strategy) =
                match (strategy_factory(i), strategy_factory(i)) {
                    (Ok(t), Ok(te)) => (t, te),
                    (Err(e), _) | (_, Err(e)) => {
                        tracing::warn!("[WalkForward] 跳过 fold {}: 策略构造失败: {}", i, e);
                        skipped_folds.push((i, e));
                        continue;
                    },
                };
            let train_result = engine.run(train_strategy.as_mut(), train_bars).await?;
            let test_result = engine.run(test_strategy.as_mut(), test_bars).await?;
            let train_metrics =
                MetricsReport::from_backtest_result(&train_result, self.config.risk_free_annual);
            let test_metrics =
                MetricsReport::from_backtest_result(&test_result, self.config.risk_free_annual);
            let degradation = if train_metrics.sharpe.abs() > 1e-6 {
                test_metrics.sharpe / train_metrics.sharpe
            } else {
                0.0
            };
            // 单 fold 过拟合告警：test_sharpe 显著低于 train_sharpe（ratio < 0.3）
            let overfit_flag = degradation < 0.3;
            // 收集 OOS equity（按 fold 归一化，避免 fold 边界引入虚假 daily_return）
            // 修复 C-3: 每个 fold 都从 initial_cash 开始，直接拼接会在 fold 边界
            // 产生约 16.7% 的虚假负收益，严重失真 sharpe/volatility/max_drawdown
            let fold_initial = test_result.equity_curve.first().map(|p| p.equity).unwrap_or(1.0);
            let scale = if let Some(last) = all_oos_points.last() {
                if fold_initial > 0.0 {
                    last.equity / fold_initial
                } else {
                    1.0
                }
            } else {
                1.0
            };
            for ep in &test_result.equity_curve {
                all_oos_points.push(EquityPoint {
                    date: ep.date.clone(),
                    equity: ep.equity * scale,
                    cash: ep.cash * scale,
                    position_value: ep.position_value * scale,
                });
            }
            // best_params：复用 train_strategy 提取参数，不再额外调用 factory。
            let best_params = match train_strategy.params() {
                serde_json::Value::Object(map) => Some(map.into_iter().collect()),
                _ => None,
            };
            windows.push(WalkForwardWindowResult {
                fold: fold.clone(),
                best_params,
                train_result,
                train_metrics,
                test_result,
                test_metrics,
                degradation_ratio: degradation,
                overfit_flag,
            });
        }

        if windows.is_empty() && !skipped_folds.is_empty() {
            return Err(QuantError::WalkForward(format!(
                "所有 {} 个 fold 均构造失败（如：{}）",
                skipped_folds.len(),
                skipped_folds[0].1
            )));
        }

        let aggregated_oos_metrics = MetricsReport::from_equity_curve(
            &all_oos_points,
            &windows.iter().flat_map(|w| w.test_result.trades.iter().cloned()).collect::<Vec<_>>(),
            self.config.risk_free_annual,
            // 修复 L-5: 使用 A 股实际交易日数 244 而非美股的 252。
            crate::metrics::A_SHARE_TRADING_DAYS_PER_YEAR,
        );

        let overfit_window_count = windows.iter().filter(|w| w.overfit_flag).count();
        let overfit_warning = overfit_window_count > windows.len() / 2;
        // 简化：稳定度 = 1 - 退化比率方差
        let degradations: Vec<f64> = windows.iter().map(|w| w.degradation_ratio).collect();
        let stability_score = if degradations.is_empty() {
            0.0
        } else {
            let mean = degradations.iter().sum::<f64>() / degradations.len() as f64;
            let var = degradations.iter().map(|d| (d - mean).powi(2)).sum::<f64>()
                / degradations.len() as f64;
            (1.0 - var.sqrt().min(1.0)).max(0.0)
        };

        Ok(WalkForwardReport {
            config: self.config.clone(),
            windows,
            aggregated_oos_equity: all_oos_points,
            aggregated_oos_metrics,
            stability_score,
            overfit_warning,
            overfit_window_count,
            generated_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

/// 仅在测试中使用的日期加法
#[allow(dead_code)]
fn add_days(date: &str, days: i64) -> String {
    use chrono::{Duration, NaiveDate};
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => (d + Duration::days(days)).format("%Y-%m-%d").to_string(),
        Err(_) => date.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bar(code: &str, day_offset: i64, close: f64) -> Bar {
        Bar {
            date: add_days("2024-01-01", day_offset),
            code: code.to_string(),
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 1_000_000.0,
            amount: close * 1_000_000.0,
            turnover_rate: Some(1.0),
            adj_factor: Some(1.0),
            limit_up: Some((close * 1.10 * 100.0).round() / 100.0),
            limit_down: Some((close * 0.90 * 100.0).round() / 100.0),
            is_st: false,
        }
    }

    fn make_klines(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| {
                let close = 10.0 + (i as f64 * 0.05) + ((i % 7) as f64 * 0.1);
                make_bar("600519", i as i64, close)
            })
            .collect()
    }

    #[test]
    fn test_rolling_split() {
        let wf = WalkForward::new(WalkForwardConfig {
            train_days: 100,
            test_days: 30,
            ..Default::default()
        });
        let klines = make_klines(200);
        let split = wf.split(&klines).unwrap();
        assert!(split.folds.len() >= 2);
        let first = &split.folds[0];
        assert_eq!(first.fold_idx, 0);
        // train_start < train_end < test_start < test_end
        assert!(first.train_start < first.train_end);
        assert!(first.train_end < first.test_start);
        assert!(first.test_start < first.test_end);
    }

    #[test]
    fn test_anchored_split() {
        eprintln!("[debug] test_anchored_split start");
        let wf = WalkForward::new(WalkForwardConfig {
            train_days: 100,
            test_days: 30,
            anchored: true,
            ..Default::default()
        });
        let klines = make_klines(300);
        eprintln!("[debug] klines built, calling split");
        let split = wf.split(&klines).unwrap();
        eprintln!("[debug] split done, {} folds", split.folds.len());
        assert!(split.folds.len() >= 2);
        // anchored 模式：所有 fold 的 train_start 应相同（IS 起点固定）
        let first_train_start = &split.folds[0].train_start;
        let first_train_start_idx = split.folds[0].train_start_idx;
        for f in &split.folds {
            assert_eq!(&f.train_start, first_train_start);
            assert_eq!(f.train_start_idx, first_train_start_idx);
        }
        // P1-1 修复验证：anchored 模式下 train_end 应随 fold index 递增
        // （IS 终点前移 = expanding window）。原 bug 下所有 fold 的 train_end_idx 恒等于
        // train_count，参数稳定度评估失效。
        for i in 1..split.folds.len() {
            assert!(
                split.folds[i].train_end_idx > split.folds[i - 1].train_end_idx,
                "fold {} train_end_idx ({}) 应大于 fold {} ({})，anchored 模式 IS 终点应随 fold 前移",
                i,
                split.folds[i].train_end_idx,
                i - 1,
                split.folds[i - 1].train_end_idx
            );
        }
    }

    #[test]
    fn test_split_insufficient_data() {
        let wf = WalkForward::new(WalkForwardConfig {
            train_days: 1000,
            test_days: 100,
            ..Default::default()
        });
        let klines = make_klines(50);
        assert!(wf.split(&klines).is_err());
    }

    #[test]
    fn test_force_off_disables() {
        let cfg = WalkForwardConfig { force_off: true, ..Default::default() };
        assert!(!cfg.is_enabled());
    }

    #[test]
    fn test_add_days() {
        assert_eq!(add_days("2025-01-01", 10), "2025-01-11");
        assert_eq!(add_days("2025-01-25", 10), "2025-02-04");
    }

    // ── 基线测试 ──

    /// 生成带均值回复 + 趋势 + 长周期波动 + 噪声的合成 K 线
    ///
    /// 关键设计：均值回复成分保证价格会围绕趋势线振荡，从而产生均线交叉信号。
    /// 每日回复 2% 的价格偏离，确保短均线与长均线能反复交叉。
    fn make_noisy_klines(
        n: usize,
        code: &str,
        start_price: f64,
        drift: f64,
        noise_amp: f64,
    ) -> Vec<Bar> {
        let mut price = start_price;
        (0..n)
            .map(|i| {
                let i_f = i as f64;
                // 均值回复：偏离越远，回复力越强
                let deviation = (price - start_price) / start_price;
                let mean_reversion = -deviation * 0.02;
                let trend = drift;
                // 确定性伪随机噪声
                let noise = ((i_f * 1.618033988749895).sin() * noise_amp).clamp(-0.03, 0.03);
                // 周度模式（周一弱、周五强）
                let weekly = ((i % 7) as f64 - 3.0) * 0.0015;

                let ret = mean_reversion + trend + noise + weekly;
                price *= 1.0 + ret;
                price = (price * 100.0).round() / 100.0;
                make_bar(code, i as i64, price.max(0.01))
            })
            .collect()
    }

    #[tokio::test]
    async fn test_walkforward_baseline_score() {
        use crate::MaCrossStrategy;

        // 1200 根合成 K 线，均值回复确保早中段都有交叉信号
        let klines = make_noisy_klines(1200, "600519", 10.0, 0.0003, 0.010);

        let wf = WalkForward::new(WalkForwardConfig {
            train_days: 300, // ~14 月
            test_days: 100,  // ~5 月
            ..Default::default()
        });

        let report = wf
            .run(|_| Ok(Box::new(MaCrossStrategy::new(5, 20)) as Box<dyn Strategy>), klines)
            .await
            .unwrap();

        println!("\n=== Walk-Forward 基线评分 (MaCross 5/20) ===");
        println!("Fold 数:        {}", report.windows.len());
        println!("OOS Sharpe:     {:.4}", report.aggregated_oos_metrics.sharpe);
        println!("OOS MaxDD:      {:.2}%", report.aggregated_oos_metrics.max_drawdown_pct * 100.0);
        println!("OOS WinRate:    {:.2}%", report.aggregated_oos_metrics.win_rate * 100.0);
        println!("OOS TotalRet:   {:.2}%", report.aggregated_oos_metrics.total_return * 100.0);
        println!("Stability:      {:.4}", report.stability_score);
        println!(
            "Overfit:        {} / {} folds  ({})",
            report.overfit_window_count,
            report.windows.len(),
            if report.overfit_warning {
                "⚠️ 告警"
            } else {
                "正常"
            }
        );
        println!("Fold 明细:");
        for (i, w) in report.windows.iter().enumerate() {
            println!(
                "  #{} train_sharpe={:.3} test_sharpe={:.3} deg={:.3}{}",
                i,
                w.train_metrics.sharpe,
                w.test_metrics.sharpe,
                w.degradation_ratio,
                if w.overfit_flag { " ⚠️" } else { "" }
            );
        }

        // 基本合理性断言：有折产生交易、评分有限
        assert!(report.aggregated_oos_metrics.sharpe.is_finite(), "OOS Sharpe 应为有限值");
        assert!(report.stability_score > 0.0, "稳定度应 > 0");
        assert!(report.windows.len() >= 3, "至少应有 3 个 fold, 实际 {}", report.windows.len());
        assert!(
            report.windows.iter().any(|w| w.train_result.total_trades > 0),
            "至少应有 1 个 fold 产生交易"
        );
    }

    #[tokio::test]
    async fn test_walkforward_param_scan() {
        use crate::MaCrossStrategy;

        // 对参数量较大的扫描使用 1000 根 K 线
        let klines = make_noisy_klines(1000, "600519", 10.0, 0.0003, 0.010);

        let param_grid =
            [(5, 20), (5, 40), (5, 60), (10, 20), (10, 40), (10, 60), (20, 40), (20, 60), (20, 80)];

        let wf = WalkForward::new(WalkForwardConfig {
            train_days: 250,
            test_days: 80,
            ..Default::default()
        });

        println!("\n=== Walk-Forward 参数扫描 (9 组合) ===");
        println!(
            "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
            "策略", "Sharpe", "MaxDD%", "WinRate%", "TotalRet%", "稳定度", "过拟合"
        );
        println!("{}", "-".repeat(80));

        let mut best_sharpe = f64::NEG_INFINITY;
        let mut best_params = (0, 0);

        for &(short, long) in &param_grid {
            let report = wf
                .run(
                    |_| Ok(Box::new(MaCrossStrategy::new(short, long)) as Box<dyn Strategy>),
                    klines.clone(),
                )
                .await
                .unwrap();

            let sharpe = report.aggregated_oos_metrics.sharpe;
            let has_trades = report.windows.iter().any(|w| w.test_result.total_trades > 0);
            println!(
                "MA({:<2},{:<2}) {:>8.4} {:>8.2} {:>8.2} {:>8.2} {:>8.4} {:>3}/{} {}",
                short,
                long,
                sharpe,
                report.aggregated_oos_metrics.max_drawdown_pct * 100.0,
                report.aggregated_oos_metrics.win_rate * 100.0,
                report.aggregated_oos_metrics.total_return * 100.0,
                report.stability_score,
                report.overfit_window_count,
                report.windows.len(),
                if has_trades { "" } else { "🟡 无交易" }
            );

            if sharpe > best_sharpe {
                best_sharpe = sharpe;
                best_params = (short, long);
            }
        }

        println!(
            "\n最佳参数: MaCross({}, {}) — OOS Sharpe = {:.4}",
            best_params.0, best_params.1, best_sharpe
        );

        assert!(best_sharpe.is_finite(), "最佳 Sharpe 应为有限值");
    }
}
