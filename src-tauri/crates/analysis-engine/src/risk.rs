//! 风险模型与估值指标 — 独立纯函数，可注册为工作流 Tool handler。
//!
//! 包含：最大回撤、夏普比率、VaR、PE 分位数、PEG、凯利公式、风险平价。

use serde::Serialize;

// P3-C8: 年化因子改用 harness 统一常量（A 股 244 天），消除 252/244 混用。
// 保留 `ANNUALIZATION_FACTOR_DAILY` 名称作为对外 API 稳定性兼容（portfolio_monitor 等下游引用），
// 但语义已从"美股 252"切换为"A 股 244"。
pub use axagent_harness::indicators::A_SHARE_TRADING_DAYS_PER_YEAR as ANNUALIZATION_FACTOR_DAILY;
/// 凯利公式默认重仓阈值
pub const KELLY_HEAVY_THRESHOLD: f64 = 0.25;
/// 凯利公式默认中仓阈值
pub const KELLY_MEDIUM_THRESHOLD: f64 = 0.1;

// ── 最大回撤 ──

/// 峰值到谷底的最大回撤比例 (0.0~1.0)。
///
/// 本模块内最大回撤的唯一核心实现；`portfolio_monitor::compute_max_drawdown_pct`
/// 复用本函数（结果 ×100 得到百分比）。
pub(crate) fn peak_trough_drawdown(prices: &[f64]) -> f64 {
    if prices.is_empty() || prices.iter().all(|&p| p <= 0.0) {
        return 0.0;
    }
    let mut peak = prices.iter().find(|&&p| p > 0.0).copied().unwrap_or(0.0);
    let mut max_dd = 0.0;
    for &p in prices.iter() {
        if p > peak {
            peak = p;
        }
        if peak > 0.0 {
            let dd = (peak - p) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

/// 计算峰值到谷底的最大回撤比例 (0.0~1.0)，复用 `peak_trough_drawdown`。
pub fn max_drawdown(prices: &[f64]) -> f64 {
    peak_trough_drawdown(prices)
}

// ── 夏普比率 ──

/// 计算夏普比率：(mean_return - risk_free) / stddev_return。
/// 使用 `ANNUALIZATION_FACTOR_DAILY` 作为默认年化因子。
pub fn sharpe_ratio(returns: &[f64], risk_free: f64) -> SharpeResult {
    sharpe_ratio_with_annualization(returns, risk_free, ANNUALIZATION_FACTOR_DAILY)
}

/// 夏普比率核心计算：返回 (sharpe, annualized, mean_return, stddev)。
///
/// P3-C8: 委托 `axagent_harness::indicators::sharpe_components` 统一实现，
/// 消除本 crate 与 astock-data/tools/quant 的算法分叉（252/244、n/n-1）。
/// `portfolio_monitor::compute_sharpe` 复用本函数避免重复实现。
///
/// 保留四舍五入到 3-4 位小数的历史行为，确保下游序列化输出稳定。
pub(crate) fn sharpe_components(
    returns: &[f64],
    risk_free: f64,
    annualization_factor: f64,
) -> (f64, f64, f64, f64) {
    let c =
        axagent_harness::indicators::sharpe_components(returns, risk_free, annualization_factor);
    (
        (c.sharpe * 1000.0).round() / 1000.0,
        (c.annualized * 1000.0).round() / 1000.0,
        (c.mean_return * 10000.0).round() / 100.0,
        (c.stddev * 10000.0).round() / 100.0,
    )
}

/// 带自定义年化因子的夏普比率。
/// `annualization_factor` 为年化时的周期数（A 股日频=244，周频=52，月频=12）。
pub fn sharpe_ratio_with_annualization(
    returns: &[f64],
    risk_free: f64,
    annualization_factor: f64,
) -> SharpeResult {
    let (sharpe, annualized, mean_return, stddev) =
        sharpe_components(returns, risk_free, annualization_factor);
    SharpeResult { sharpe, annualized, mean_return, stddev }
}

#[derive(Debug, Clone, Serialize)]
pub struct SharpeResult {
    pub sharpe: f64,
    pub annualized: f64,
    pub mean_return: f64,
    pub stddev: f64,
}

// ── VaR (Value at Risk) ──

/// 历史模拟法 VaR：将收益率排序后取第 (1-confidence) 分位数。
/// 返回正数表示损失的百分比。
pub fn value_at_risk(returns: &[f64], confidence: f64) -> VarResult {
    let n = returns.len();
    if n < 5 {
        return VarResult { var_pct: 0.0, confidence, cvar_pct: 0.0 };
    }
    let mut sorted = returns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((1.0 - confidence) * (n as f64 + 1.0)).floor() as usize;
    let var_idx = if idx == 0 { 0 } else { idx - 1 };
    let var_val = if var_idx < n { -sorted[var_idx] } else { 0.0 };
    let tail: f64 = sorted[..=var_idx.min(n - 1)].iter().map(|r| -r).sum::<f64>();
    let cvar = tail / (var_idx + 1) as f64;
    VarResult {
        var_pct: (var_val * 100.0).round() / 100.0,
        confidence,
        cvar_pct: (cvar * 100.0).round() / 100.0,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VarResult {
    pub var_pct: f64,
    pub confidence: f64,
    pub cvar_pct: f64,
}

// ── PE 分位数 ──

/// 计算当前 PE 在历史 PE 序列中的分位数 (0-100)。
pub fn pe_percentile(current_pe: f64, historical_pes: &[f64]) -> PEPercentileResult {
    let mut sorted = historical_pes.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let below = sorted.iter().filter(|&&pe| pe <= current_pe).count();
    let pct = if sorted.is_empty() {
        50.0
    } else {
        below as f64 / sorted.len() as f64 * 100.0
    };
    let level = if pct < 20.0 {
        "极低"
    } else if pct < 40.0 {
        "偏低"
    } else if pct < 60.0 {
        "合理"
    } else if pct < 80.0 {
        "偏高"
    } else {
        "极高"
    };
    PEPercentileResult {
        percentile: (pct * 10.0).round() / 10.0,
        level: level.into(),
        median: if !sorted.is_empty() {
            sorted[sorted.len() / 2]
        } else {
            current_pe
        },
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PEPercentileResult {
    pub percentile: f64,
    pub level: String,
    pub median: f64,
}

// ── PEG ──

/// PEG = PE / 增长率。增长率以 % 表示（如 25 表示 25%）。
pub fn peg_ratio(pe: f64, growth_rate: f64) -> PEGResult {
    if growth_rate <= 0.0 {
        return PEGResult { peg: f64::INFINITY, level: "无意义".into(), pe, growth_rate };
    }
    let peg = pe / growth_rate;
    let level = if peg < 0.5 {
        "严重低估"
    } else if peg < 1.0 {
        "低估"
    } else if peg < 2.0 {
        "合理"
    } else {
        "高估"
    };
    PEGResult { peg: (peg * 100.0).round() / 100.0, level: level.into(), pe, growth_rate }
}

#[derive(Debug, Clone, Serialize)]
pub struct PEGResult {
    pub peg: f64,
    pub level: String,
    pub pe: f64,
    pub growth_rate: f64,
}

// ── 凯利公式 ──

/// Kelly Criterion: f* = p - q / (W/L) = p - (1-p) / (avg_win / avg_loss)
/// 返回建议仓位比例。使用 `KELLY_HEAVY_THRESHOLD` / `KELLY_MEDIUM_THRESHOLD` 作为默认阈值。
pub fn kelly_criterion(win_rate: f64, avg_win: f64, avg_loss: f64) -> KellyResult {
    kelly_criterion_with_thresholds(
        win_rate,
        avg_win,
        avg_loss,
        KELLY_HEAVY_THRESHOLD,
        KELLY_MEDIUM_THRESHOLD,
    )
}

/// 带自定义仓位信号阈值的凯利公式。
/// - `heavy_threshold`: 超过此值视为"重仓"（默认 0.25）
/// - `medium_threshold`: 超过此值视为"中等"（默认 0.1），低于此值且 >0 为"轻仓"
pub fn kelly_criterion_with_thresholds(
    win_rate: f64,
    avg_win: f64,
    avg_loss: f64,
    heavy_threshold: f64,
    medium_threshold: f64,
) -> KellyResult {
    if avg_loss <= 0.0 || avg_win <= 0.0 || win_rate <= 0.0 {
        return KellyResult {
            kelly_fraction: 0.0,
            half_kelly: 0.0,
            position_pct: 0.0,
            signal: "不适用".into(),
        };
    }
    let odds = avg_win / avg_loss;
    let kelly = ((win_rate * (odds + 1.0) - 1.0) / odds).max(0.0);
    let half = kelly / 2.0;
    let signal = if kelly > heavy_threshold {
        "重仓"
    } else if kelly > medium_threshold {
        "中等"
    } else if kelly > 0.0 {
        "轻仓"
    } else {
        "不建议"
    };
    KellyResult {
        kelly_fraction: (kelly * 1000.0).round() / 1000.0,
        half_kelly: (half * 1000.0).round() / 1000.0,
        position_pct: (half * 10000.0).round() / 100.0,
        signal: signal.into(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KellyResult {
    pub kelly_fraction: f64,
    pub half_kelly: f64,
    pub position_pct: f64,
    pub signal: String,
}

// ── 风险平价 ──

/// 风险平价权重：每项资产权重 ∝ 1/volatility，归一化到总和=1。
pub fn risk_parity_weights(volatilities: &[f64], correlations_json: &str) -> RiskParityResult {
    let n = volatilities.len();
    if n == 0 {
        return RiskParityResult { weights: vec![], divers_ratio: 0.0 };
    }
    let corr_matrix: Option<Vec<Vec<f64>>> = serde_json::from_str(correlations_json)
        .ok()
        .filter(|m: &Vec<Vec<f64>>| m.len() == n && m.iter().all(|r| r.len() == n));
    let inv_vols: Vec<f64> =
        volatilities.iter().map(|&v| if v > 0.0 { 1.0 / v } else { 0.0 }).collect();
    let total: f64 = inv_vols.iter().sum();
    let weights = if let Some(corr) = corr_matrix {
        let mut w: Vec<f64> = inv_vols.clone();
        let w_sum: f64 = w.iter().sum();
        if w_sum > 0.0 {
            for wi in w.iter_mut() {
                *wi /= w_sum;
            }
        }
        for _ in 0..20 {
            let mut risk_contrib = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    risk_contrib[i] += w[i] * w[j] * volatilities[i] * volatilities[j] * corr[i][j];
                }
            }
            let total_risk: f64 = risk_contrib.iter().sum();
            if total_risk <= 0.0 {
                break;
            }
            let target = total_risk / n as f64;
            for i in 0..n {
                if risk_contrib[i] > 0.0 {
                    w[i] *= (target / risk_contrib[i]).sqrt().clamp(0.5, 2.0);
                }
            }
            let ws: f64 = w.iter().sum();
            if ws > 0.0 {
                for wi in w.iter_mut() {
                    *wi /= ws;
                }
            }
        }
        w.iter().map(|&x| (x * 10000.0).round() / 10000.0).collect()
    } else if total > 0.0 {
        inv_vols.iter().map(|&w| (w / total * 10000.0).round() / 10000.0).collect()
    } else {
        vec![1.0 / n as f64; n]
    };
    let hhi: f64 = weights.iter().map(|w| w * w).sum();
    let divers_ratio = if hhi > 0.0 {
        (1.0 / (hhi * n as f64)).min(1.0)
    } else {
        1.0
    };
    RiskParityResult { weights, divers_ratio: (divers_ratio * 100.0).round() / 100.0 }
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskParityResult {
    pub weights: Vec<f64>,
    pub divers_ratio: f64,
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_drawdown_normal() {
        let prices = vec![100.0, 110.0, 90.0, 95.0, 105.0];
        let dd = max_drawdown(&prices);
        assert!((dd - 0.1818).abs() < 0.01, "expected ~0.1818, got {dd}"); // (110-90)/110
    }

    #[test]
    fn test_max_drawdown_empty() {
        assert_eq!(max_drawdown(&[]), 0.0);
    }

    #[test]
    fn test_sharpe_ratio() {
        let returns = vec![0.01, 0.02, -0.01, 0.005, 0.015];
        // P3-C8: 年化因子切换为 A 股 244 天
        let r = sharpe_ratio(&returns, 0.02 / 244.0);
        assert!(r.sharpe > 0.0, "positive mean return should give positive sharpe");
    }

    #[test]
    fn test_var() {
        let returns = vec![0.01, -0.02, 0.03, -0.01, -0.03, 0.02, -0.01, 0.01, -0.05, 0.02];
        let r = value_at_risk(&returns, 0.95);
        assert!(r.var_pct > 0.0);
        assert!(r.cvar_pct >= r.var_pct);
    }

    #[test]
    fn test_pe_percentile() {
        let pes = vec![10.0, 12.0, 15.0, 18.0, 20.0, 22.0, 25.0, 30.0];
        let r = pe_percentile(16.0, &pes);
        assert!(r.percentile > 30.0 && r.percentile < 60.0);
    }

    #[test]
    fn test_peg_ratio() {
        let r = peg_ratio(20.0, 25.0);
        assert!((r.peg - 0.8).abs() < 0.01);
        assert_eq!(r.level, "低估");
    }

    #[test]
    fn test_kelly() {
        let r = kelly_criterion(0.55, 0.08, 0.05);
        assert!(r.kelly_fraction > 0.0);
        assert!(r.half_kelly > 0.0);
    }

    #[test]
    fn test_risk_parity() {
        let vols = vec![0.2, 0.3, 0.4];
        let r = risk_parity_weights(&vols, "[]");
        assert_eq!(r.weights.len(), 3);
        assert!((r.weights.iter().sum::<f64>() - 1.0).abs() < 0.001);
        // 高波动资产权重应更低
        assert!(r.weights[2] < r.weights[0]);
    }
}
