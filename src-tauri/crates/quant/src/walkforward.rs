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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ctx::EquityPoint;
use crate::engine::{BacktestConfig, BacktestEngine, BacktestResult};
use crate::error::QuantError;
use crate::metrics::MetricsReport;
use crate::strategy::Strategy;
use crate::types::Bar;

/// Walk-Forward 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardConfig {
    /// IS 训练窗口大小（自然日，简化处理）
    pub train_days: i64,
    /// OOS 验证窗口大小（自然日）
    pub test_days: i64,
    /// 步进大小（默认 = test_days，即不重叠）
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
    /// - 简化：按自然日切分（不做交易日历）
    pub fn split(&self, klines: &[Bar]) -> Result<WalkForwardSplit, QuantError> {
        if klines.is_empty() {
            return Err(QuantError::WalkForward("输入 K 线为空".to_string()));
        }
        let first_date = &klines[0].date;
        let last_date = &klines[klines.len() - 1].date;
        let total_days = approx_days(first_date, last_date);
        if total_days < self.config.train_days + self.config.test_days {
            return Err(QuantError::WalkForward(format!(
                "数据跨度 {} 天 < train({}) + test({}) = {} 天",
                total_days,
                self.config.train_days,
                self.config.test_days,
                self.config.train_days + self.config.test_days
            )));
        }
        let step = self.config.step_days.unwrap_or(self.config.test_days);
        let mut folds = Vec::new();
        let mut fold_idx = 0;
        let mut cursor = 0i64;
        loop {
            let train_start_offset = if self.config.anchored { 0 } else { cursor };
            let train_end_offset = train_start_offset + self.config.train_days;
            let test_start_offset = if self.config.anchored {
                self.config.train_days + cursor
            } else {
                train_end_offset
            };
            let test_end_offset = test_start_offset + self.config.test_days;
            if test_end_offset > total_days {
                break;
            }
            let train_start = add_days(first_date, train_start_offset);
            let train_end = add_days(first_date, train_end_offset.saturating_sub(1));
            let test_start = add_days(first_date, test_start_offset);
            let test_end = add_days(first_date, test_end_offset.saturating_sub(1));
            let train_bars: Vec<Bar> = klines
                .iter()
                .filter(|b| {
                    b.date.as_str() >= train_start.as_str() && b.date.as_str() <= train_end.as_str()
                })
                .cloned()
                .collect();
            let test_bars: Vec<Bar> = klines
                .iter()
                .filter(|b| {
                    b.date.as_str() >= test_start.as_str() && b.date.as_str() <= test_end.as_str()
                })
                .cloned()
                .collect();
            if train_bars.len() >= self.config.min_train_bars
                && test_bars.len() >= self.config.min_test_bars
            {
                folds.push(WalkForwardFold {
                    fold_idx,
                    train_start,
                    train_end,
                    test_start,
                    test_end,
                    train_bars_count: train_bars.len(),
                    test_bars_count: test_bars.len(),
                });
                fold_idx += 1;
            }
            cursor += step;
        }
        if folds.is_empty() {
            return Err(QuantError::WalkForward("未生成任何 fold（数据不足以切分）".to_string()));
        }
        Ok(WalkForwardSplit {
            config: self.config.clone(),
            folds,
        })
    }

    /// 跑 Walk-Forward 验证
    ///
    /// - `strategy_factory`: 给定参数，构造一个新 Strategy 实例（grid search 时为每个候选构造）
    /// - `param_grid`: 简化版 — M1 不做 grid search，由 caller 在 factory 内自行选参
    /// - 返回综合报告
    pub async fn run<F>(
        &self,
        strategy_factory: F,
        klines: Vec<Bar>,
    ) -> Result<WalkForwardReport, QuantError>
    where
        F: Fn(usize) -> Box<dyn Strategy>,
    {
        let split = self.split(&klines)?;
        let engine = BacktestEngine::with_defaults();
        let mut windows: Vec<WalkForwardWindowResult> = Vec::new();
        let mut all_oos_points: Vec<EquityPoint> = Vec::new();

        for (i, fold) in split.folds.iter().enumerate() {
            let train_bars: Vec<Bar> = klines
                .iter()
                .filter(|b| {
                    b.date.as_str() >= fold.train_start.as_str()
                        && b.date.as_str() <= fold.train_end.as_str()
                })
                .cloned()
                .collect();
            let test_bars: Vec<Bar> = klines
                .iter()
                .filter(|b| {
                    b.date.as_str() >= fold.test_start.as_str()
                        && b.date.as_str() <= fold.test_end.as_str()
                })
                .cloned()
                .collect();
            // 每个 fold 用独立的 strategy 实例（避免状态污染）
            let mut train_strategy = strategy_factory(i);
            let mut test_strategy = strategy_factory(i);
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
            // 收集 OOS equity（简化：直接拼接 OOS equity）
            all_oos_points.extend(test_result.equity_curve.clone());
            // best_params（M1 简化：取该 fold 的 strategy 参数；M2 阶段接 grid search）
            let best_params = match strategy_factory(i).params() {
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

        let aggregated_oos_metrics = MetricsReport::from_equity_curve(
            &all_oos_points,
            &windows
                .iter()
                .flat_map(|w| w.test_result.trades.iter().cloned())
                .collect::<Vec<_>>(),
            self.config.risk_free_annual,
            252.0,
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

// 借用 BacktestConfig（保持 API 一致性，M1 不直接使用）
#[allow(dead_code)]
fn _ensure_used(_: BacktestConfig) {}

fn approx_days(start: &str, end: &str) -> i64 {
    use chrono::NaiveDate;
    let s = NaiveDate::parse_from_str(start, "%Y-%m-%d").ok();
    let e = NaiveDate::parse_from_str(end, "%Y-%m-%d").ok();
    match (s, e) {
        (Some(s), Some(e)) => (e - s).num_days(),
        _ => 0,
    }
}

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
        // anchored 模式：所有 fold 的 train_start 应相同
        let first_train_start = &split.folds[0].train_start;
        for f in &split.folds {
            assert_eq!(&f.train_start, first_train_start);
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
        let cfg = WalkForwardConfig {
            force_off: true,
            ..Default::default()
        };
        assert!(!cfg.is_enabled());
    }

    #[test]
    fn test_add_days() {
        assert_eq!(add_days("2025-01-01", 10), "2025-01-11");
        assert_eq!(add_days("2025-01-25", 10), "2025-02-04");
    }
}
