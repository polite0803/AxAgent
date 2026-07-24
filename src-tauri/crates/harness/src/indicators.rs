//! 技术指标纯函数模块（SMA / EMA / RSI / stddev）
//!
//! P2-C7: 将原本散落在 `astock-data`、`quant`、`market-sim`、`stock-analysis`
//! 的重复实现统一收口到 harness foundation 层。所有共享数据模型的 crate
//! （implementor / consumer / hybrid / wiring）均可通过 `pub use` 引用，
//! 消除 DRY 违规，确保算法一致性。
//!
//! ## 算法约定
//!
//! - **SMA**: 取最近 `period` 个数据的算术平均；数据不足返回 `None`
//! - **EMA 序列**: 首值用前 `period` 个数据的 SMA 初始化（标准 EMA 初始化），
//!   返回与输入等长的序列
//! - **RSI (Wilder 平滑)**: 首轮简单平均，后续用 `(n-1)/n` 指数平滑；
//!   数据不足（`len < period + 1` 或 `period == 0`）返回 `None`
//! - **样本标准差**: n-1 分母（Bessel 校正），用于布林带等统计场景
//!
//! ## 返回值语义
//!
//! - `Option<f64>` 版本：数据不足返回 `None`，调用方自行决定中性默认值
//! - 序列版本：输入为空或 `period == 0` 返回 `vec![0.0]`（保持与历史调用方兼容）
//!
//! ## 不变量
//!
//! - 所有函数对 `period == 0` 做防御性处理，不会 panic
//! - 输入为空切片时不会 panic
//! - 不依赖任何外部 crate，仅用 std（符合 foundation 层零 axagent-* 依赖约束）

#![allow(dead_code)]

// ===================== SMA =====================

/// 简单移动平均（取最后 `period` 个数据点的算术平均）
///
/// - 数据不足（`data.len() < period`）或 `period == 0` 时返回 `None`
/// - 调用方需自行决定回退值（如用最新收盘价或 50.0 中性值）
///
/// # 示例
///
/// ```
/// use axagent_harness::indicators::sma;
/// assert_eq!(sma(&[1.0, 2.0, 3.0, 4.0], 2), Some(3.5));
/// assert_eq!(sma(&[1.0, 2.0], 5), None);
/// assert_eq!(sma(&[1.0, 2.0, 3.0], 0), None);
/// ```
pub fn sma(data: &[f64], period: usize) -> Option<f64> {
    if data.len() < period || period == 0 {
        return None;
    }
    let start = data.len() - period;
    Some(data[start..].iter().sum::<f64>() / period as f64)
}

// ===================== EMA =====================

/// 构建完整 EMA 序列（与输入等长）
///
/// 首值用前 `period` 个数据的 SMA 初始化（标准 EMA 初始化），
/// 后续按 `multiplier = 2 / (period + 1)` 递推。
///
/// - 输入为空或 `period == 0` 时返回 `vec![0.0]`（保持与历史调用方兼容）
/// - 返回序列长度等于输入长度
///
/// # 示例
///
/// ```
/// use axagent_harness::indicators::build_ema_series;
/// let series = build_ema_series(&[1.0, 2.0, 3.0, 4.0], 2);
/// assert_eq!(series.len(), 4);
/// // 首值 = SMA(1, 2) = 1.5
/// assert!((series[0] - 1.5).abs() < 1e-10);
/// ```
pub fn build_ema_series(data: &[f64], period: usize) -> Vec<f64> {
    if data.is_empty() || period == 0 {
        return vec![0.0];
    }
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut result = Vec::with_capacity(data.len());
    let init_n = period.min(data.len());
    let init_sma: f64 = data[..init_n].iter().sum::<f64>() / init_n as f64;
    let mut ema_val = init_sma;
    result.push(ema_val);
    for &val in &data[1..] {
        ema_val = (val - ema_val) * multiplier + ema_val;
        result.push(ema_val);
    }
    result
}

/// 仅取 EMA 序列的末值（便捷函数）
///
/// 等价于 `build_ema_series(data, period).last().copied().unwrap_or(0.0)`，
/// 但避免分配整个 Vec。数据为空时返回 `0.0`。
#[inline]
pub fn ema_last(data: &[f64], period: usize) -> f64 {
    if data.is_empty() || period == 0 {
        return 0.0;
    }
    let multiplier = 2.0 / (period as f64 + 1.0);
    let init_n = period.min(data.len());
    let init_sma: f64 = data[..init_n].iter().sum::<f64>() / init_n as f64;
    let mut ema_val = init_sma;
    for &val in &data[1..] {
        ema_val = (val - ema_val) * multiplier + ema_val;
    }
    ema_val
}

// ===================== RSI (Wilder 平滑) =====================

/// RSI 指标（Wilder 平滑法）
///
/// 首轮对前 `period` 个涨跌幅做简单平均，后续用 `(n-1)/n` 指数平滑。
/// 数据不足（`len < period + 1` 或 `period == 0`）返回 `None`，
/// 调用方自行决定中性默认值（如 50.0）。
///
/// # 边界情况
///
/// - `avg_loss < 1e-10`（持续上涨无回调）返回 `Some(100.0)`
/// - 数据不足返回 `None`
///
/// # 示例
///
/// ```
/// use axagent_harness::indicators::rsi_wilder;
/// // 持续上涨 → RSI = 100
/// let closes = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
/// assert_eq!(rsi_wilder(&closes, 5), Some(100.0));
/// // 数据不足
/// assert_eq!(rsi_wilder(&[1.0, 2.0], 5), None);
/// ```
pub fn rsi_wilder(closes: &[f64], period: usize) -> Option<f64> {
    if closes.len() < period + 1 || period == 0 {
        return None;
    }
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..=period {
        let diff = closes[i] - closes[i - 1];
        if diff > 0.0 {
            avg_gain += diff;
        } else {
            avg_loss += -diff;
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;
    for i in (period + 1)..closes.len() {
        let diff = closes[i] - closes[i - 1];
        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { -diff } else { 0.0 };
        avg_gain = (avg_gain * (period - 1) as f64 + gain) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + loss) / period as f64;
    }
    if avg_loss < 1e-10 {
        return Some(100.0);
    }
    let rs = avg_gain / avg_loss;
    Some(100.0 - (100.0 / (1.0 + rs)))
}

// ===================== 样本标准差 =====================

/// 样本标准差（n-1 分母，Bessel 校正）
///
/// 用于布林带等统计场景。数据少于 2 个返回 `0.0`。
///
/// # 示例
///
/// ```
/// use axagent_harness::indicators::stddev_sample;
/// let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
/// let mean = data.iter().sum::<f64>() / data.len() as f64;
/// let sd = stddev_sample(&data, mean);
/// assert!((sd - 2.138).abs() < 0.01);
/// ```
pub fn stddev_sample(data: &[f64], mean: f64) -> f64 {
    let n = data.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let variance = data.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    variance.sqrt()
}

// ===================== Sharpe Ratio（P3-C8 统一实现）=====================

/// A 股每年实际交易日数（约 244 天，而非美股的 252 天）。
///
/// P3-C8: 将原本散落在 `stock-analysis/risk.rs` (252)、`astock-data/mcp_tools.rs` (252)、
/// `tools/finance.rs` (252)、`quant/metrics.rs` (244) 的年化因子统一收口。
/// 所有 A 股相关计算应使用本常量，避免 252/244 混用导致的 Sharpe / 波动率偏差。
pub const A_SHARE_TRADING_DAYS_PER_YEAR: f64 = 244.0;

/// 默认年无风险利率（2.5%，参考 10 年期国债收益率中枢）。
///
/// 各调用方可根据自身语义覆盖（如 `astock-data/mcp_tools.rs` 历史使用 3.0%）。
pub const RISK_FREE_ANNUAL_DEFAULT: f64 = 0.025;

/// Sharpe 计算的完整结果（与历史 `SharpeResult` / `SharpeR` 字段对齐）。
///
/// P3-C8: 统一 DTO，消除 stock-analysis/risk.rs `SharpeResult` 与 tools/finance.rs `SharpeR`
/// 两套同义结构体的 DRY 违规。下游 crate 通过 `pub use axagent_harness::indicators::SharpeComponents`
/// 复用，避免重复定义。
#[derive(Debug, Clone, serde::Serialize)]
pub struct SharpeComponents {
    /// 日频 Sharpe：(mean - risk_free_daily) / stddev
    pub sharpe: f64,
    /// 年化 Sharpe：`sharpe * sqrt(annualization)`
    pub annualized: f64,
    /// 日均收益率（原始值，未缩放）
    pub mean_return: f64,
    /// 日收益率样本标准差（n-1 分母）
    pub stddev: f64,
}

/// 夏普比率核心计算 —— 接受 **日频** 无风险利率。
///
/// 统一算法约定：
/// - 样本方差（n-1 分母，Bessel 校正）
/// - 数据 < 2 个返回全零 `SharpeComponents`
/// - `stddev == 0`（常数序列）返回全零，避免除零
/// - 不做四舍五入，由调用方按需 round（保留精度供下游复用）
///
/// # 参数
///
/// - `returns`: 日收益率切片（如 0.01 表示 +1%）
/// - `risk_free_daily`: **日频** 无风险利率（如 0.03/244 ≈ 0.000123）
/// - `annualization`: 年化因子（A 股 = 244，美股 = 252，周频 = 52，月频 = 12）
///
/// # 示例
///
/// ```
/// use axagent_harness::indicators::{sharpe_components, A_SHARE_TRADING_DAYS_PER_YEAR};
/// let returns = vec![0.01, 0.02, -0.01, 0.005, 0.015];
/// let r = sharpe_components(&returns, 0.03 / A_SHARE_TRADING_DAYS_PER_YEAR, A_SHARE_TRADING_DAYS_PER_YEAR);
/// assert!(r.sharpe > 0.0, "正均值应有正 sharpe");
/// assert!(r.annualized > r.sharpe, "年化值应放大");
/// ```
pub fn sharpe_components(
    returns: &[f64],
    risk_free_daily: f64,
    annualization: f64,
) -> SharpeComponents {
    let n = returns.len();
    if n < 2 {
        return SharpeComponents { sharpe: 0.0, annualized: 0.0, mean_return: 0.0, stddev: 0.0 };
    }
    let mean: f64 = returns.iter().sum::<f64>() / n as f64;
    let variance: f64 = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let stddev = variance.sqrt();
    if stddev == 0.0 {
        return SharpeComponents { sharpe: 0.0, annualized: 0.0, mean_return: mean, stddev: 0.0 };
    }
    let excess = mean - risk_free_daily;
    let sharpe = excess / stddev;
    let annualized = sharpe * annualization.sqrt();
    SharpeComponents { sharpe, annualized, mean_return: mean, stddev }
}

/// 便捷函数：A 股日频夏普比率（年化），使用默认 244 天年化。
///
/// 等价于 `sharpe_components(returns, risk_free_daily, A_SHARE_TRADING_DAYS_PER_YEAR).annualized`。
/// 数据不足或常数序列返回 `0.0`。
#[inline]
pub fn sharpe_ratio(returns: &[f64], risk_free_daily: f64) -> f64 {
    sharpe_components(returns, risk_free_daily, A_SHARE_TRADING_DAYS_PER_YEAR).annualized
}

/// 便捷函数：带自定义年化因子的夏普比率（年化）。
///
/// 适用于周频（52）、月频（12）或美股日频（252）等非 A 股场景。
#[inline]
pub fn sharpe_ratio_with_annualization(
    returns: &[f64],
    risk_free_daily: f64,
    annualization: f64,
) -> f64 {
    sharpe_components(returns, risk_free_daily, annualization).annualized
}

/// 便捷函数：接受 **年频** 无风险利率的夏普比率（年化）。
///
/// 内部将年利率转换为日利率（`risk_free_annual / days_per_year`）后调用核心函数。
/// 适用于 `quant::metrics::sharpe_ratio(curve, risk_free_annual, days_per_year)` 这类
/// 以年利率为输入的回测场景。
#[inline]
pub fn sharpe_ratio_annual(returns: &[f64], risk_free_annual: f64, days_per_year: f64) -> f64 {
    if days_per_year <= 0.0 {
        return 0.0;
    }
    let daily_rf = risk_free_annual / days_per_year;
    sharpe_components(returns, daily_rf, days_per_year).annualized
}

// ===================== 单元测试 =====================

#[cfg(test)]
mod tests {
    use super::*;

    // ── SMA ──

    #[test]
    fn sma_basic() {
        assert_eq!(sma(&[1.0, 2.0, 3.0, 4.0], 2), Some(3.5));
        assert_eq!(sma(&[1.0, 2.0, 3.0], 3), Some(2.0));
    }

    #[test]
    fn sma_insufficient_data_returns_none() {
        assert_eq!(sma(&[1.0, 2.0], 5), None);
    }

    #[test]
    fn sma_zero_period_returns_none() {
        assert_eq!(sma(&[1.0, 2.0, 3.0], 0), None);
    }

    #[test]
    fn sma_empty_input_returns_none() {
        assert_eq!(sma(&[], 1), None);
    }

    // ── EMA 序列 ──

    #[test]
    fn ema_series_basic() {
        let series = build_ema_series(&[1.0, 2.0, 3.0, 4.0], 2);
        assert_eq!(series.len(), 4);
        // 首值 = SMA(1, 2) = 1.5
        assert!((series[0] - 1.5).abs() < 1e-10);
        // multiplier = 2/3, ema[1] = (2 - 1.5) * 2/3 + 1.5 = 1.8333...
        assert!((series[1] - 1.8333_3333).abs() < 1e-6);
    }

    #[test]
    fn ema_series_empty_input() {
        assert_eq!(build_ema_series(&[], 5), vec![0.0]);
    }

    #[test]
    fn ema_series_zero_period() {
        assert_eq!(build_ema_series(&[1.0, 2.0], 0), vec![0.0]);
    }

    #[test]
    fn ema_last_matches_series_end() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let series = build_ema_series(&data, 3);
        let last = ema_last(&data, 3);
        assert!((last - series.last().copied().unwrap_or(0.0)).abs() < 1e-10);
    }

    // ── RSI ──

    #[test]
    fn rsi_all_up_is_100() {
        // 持续上涨：avg_loss = 0 → RSI = 100
        let closes = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        assert_eq!(rsi_wilder(&closes, 5), Some(100.0));
    }

    #[test]
    fn rsi_all_down_is_0() {
        // 持续下跌：avg_gain = 0, rs = 0 → RSI = 0
        let closes = vec![6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
        assert_eq!(rsi_wilder(&closes, 5), Some(0.0));
    }

    #[test]
    fn rsi_insufficient_data_returns_none() {
        assert_eq!(rsi_wilder(&[1.0, 2.0], 5), None);
    }

    #[test]
    fn rsi_zero_period_returns_none() {
        assert_eq!(rsi_wilder(&[1.0, 2.0, 3.0], 0), None);
    }

    #[test]
    fn rsi_mixed_market_in_range() {
        // 涨跌交替：RSI 应在 (0, 100) 之间
        // 注意：Wilder 平滑用指数递归，对涨跌顺序敏感，不保证对称涨跌返回 50
        let closes = vec![10.0, 11.0, 10.0, 11.0, 10.0, 11.0];
        let rsi = rsi_wilder(&closes, 5).expect("数据充足应返回 Some");
        assert!(rsi > 0.0 && rsi < 100.0, "RSI 应在 (0, 100) 区间, 实际: {}", rsi);
    }

    // ── 样本标准差 ──

    #[test]
    fn stddev_basic() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let sd = stddev_sample(&data, mean);
        // 经典样本标准差 = 2.138...
        assert!((sd - 2.138).abs() < 0.01, "sd = {}", sd);
    }

    #[test]
    fn stddev_single_element_is_zero() {
        assert_eq!(stddev_sample(&[5.0], 5.0), 0.0);
    }

    #[test]
    fn stddev_empty_is_zero() {
        assert_eq!(stddev_sample(&[], 0.0), 0.0);
    }

    #[test]
    fn stddev_constant_series_is_zero() {
        // 常数序列方差为 0
        let data = vec![5.0, 5.0, 5.0, 5.0];
        assert_eq!(stddev_sample(&data, 5.0), 0.0);
    }

    // ── Sharpe Ratio (P3-C8) ──

    #[test]
    fn sharpe_components_basic() {
        // 正均值 → 正 sharpe
        let returns = vec![0.01, 0.02, -0.01, 0.005, 0.015];
        let r = sharpe_components(&returns, 0.0, 244.0);
        assert!(r.sharpe > 0.0, "正均值应有正 sharpe, got {}", r.sharpe);
        assert!(r.annualized > r.sharpe, "年化值应放大, sharpe={}, ann={}", r.sharpe, r.annualized);
        assert!(r.mean_return > 0.0);
        assert!(r.stddev > 0.0);
    }

    #[test]
    fn sharpe_components_insufficient_data() {
        let r = sharpe_components(&[0.01], 0.0, 244.0);
        assert_eq!(r.sharpe, 0.0);
        assert_eq!(r.annualized, 0.0);
        assert_eq!(r.mean_return, 0.0);
        assert_eq!(r.stddev, 0.0);
    }

    #[test]
    fn sharpe_components_empty() {
        let r = sharpe_components(&[], 0.0, 244.0);
        assert_eq!(r.sharpe, 0.0);
    }

    #[test]
    fn sharpe_components_constant_series_returns_zero_sharpe() {
        // 常数序列 stddev=0 → sharpe=0，但 mean_return 保留
        let r = sharpe_components(&[0.01, 0.01, 0.01], 0.0, 244.0);
        assert_eq!(r.sharpe, 0.0);
        assert_eq!(r.annualized, 0.0);
        assert_eq!(r.stddev, 0.0);
        assert!((r.mean_return - 0.01).abs() < 1e-10, "mean_return 应保留, got {}", r.mean_return);
    }

    #[test]
    fn sharpe_components_uses_sample_variance() {
        // 验证使用 n-1 而非 n 分母
        // 数据: [1, 2, 3, 4, 5]
        // mean = 3, Σ(x-mean)² = 4+1+0+1+4 = 10
        // 样本方差 = 10/4 = 2.5 → stddev = √2.5 ≈ 1.5811
        // 总体方差 = 10/5 = 2.0 → stddev = √2 ≈ 1.4142
        let returns = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let r = sharpe_components(&returns, 0.0, 1.0);
        let expected_stddev = (2.5_f64).sqrt();
        assert!(
            (r.stddev - expected_stddev).abs() < 1e-10,
            "应使用样本方差 n-1, expected {}, got {}",
            expected_stddev,
            r.stddev
        );
    }

    #[test]
    fn sharpe_ratio_convenience_uses_a_share_default() {
        // sharpe_ratio 应默认使用 244 天年化
        let returns = vec![0.01, 0.02, -0.01, 0.005, 0.015];
        let convenience = sharpe_ratio(&returns, 0.0);
        let explicit =
            sharpe_ratio_with_annualization(&returns, 0.0, A_SHARE_TRADING_DAYS_PER_YEAR);
        assert!((convenience - explicit).abs() < 1e-10);
    }

    #[test]
    fn sharpe_ratio_annual_converts_rf_correctly() {
        // sharpe_ratio_annual(rf_annual=0.03, days=244) 应等价于
        // sharpe_components(daily_rf=0.03/244, annualization=244).annualized
        let returns = vec![0.01, 0.02, -0.01, 0.005, 0.015];
        let annual = sharpe_ratio_annual(&returns, 0.03, 244.0);
        let daily = sharpe_components(&returns, 0.03 / 244.0, 244.0).annualized;
        assert!((annual - daily).abs() < 1e-12, "annual={}, daily={}", annual, daily);
    }

    #[test]
    fn sharpe_ratio_annual_zero_days_returns_zero() {
        let returns = vec![0.01, 0.02, -0.01];
        assert_eq!(sharpe_ratio_annual(&returns, 0.03, 0.0), 0.0);
    }

    #[test]
    fn sharpe_matches_astock_data_legacy_formula_after_fix() {
        // 验证修复 astock-data 总体方差 bug 后的算法等价性:
        // 修复前(bug): variance = Σ(x-mean)² / n
        // 修复后(correct): variance = Σ(x-mean)² / (n-1) — 本 harness 实现
        let returns = vec![0.01, 0.02, -0.01, 0.005, 0.015, 0.0, 0.012, -0.005];
        let n = returns.len() as f64;
        let mean = returns.iter().sum::<f64>() / n;
        let variance_sample = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let variance_population = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
        // 样本方差 > 总体方差（n-1 < n）， stddev 也更大 → Sharpe 绝对值更小
        assert!(variance_sample > variance_population);
        let r = sharpe_components(&returns, 0.0, 244.0);
        assert!((r.stddev - variance_sample.sqrt()).abs() < 1e-12);
    }
}
