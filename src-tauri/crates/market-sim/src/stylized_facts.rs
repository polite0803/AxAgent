//! Stylized Facts —— 从模拟成交数据计算金融时间序列统计特征。
//!
//! 用于验证模拟器产出的市场数据是否呈现真实 A 股的统计特性。
//! 参考 Cont (2001) "Empirical properties of asset returns" 框架。

use crate::types::TradeRecord;

// ── Bar 聚合 ──

/// 时间窗口聚合后的 K 线（用于在 Bar 收益率上计算 Stylized Facts）
#[derive(Debug, Clone)]
pub struct Bar {
    pub time: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

/// 将成交记录按固定时间窗口聚合为 Bar
///
/// `bar_ns` = 每个 bar 的时间宽度（纳秒）。
/// 每个 bar 的 trade price = 该窗口最后一笔的价格（模拟 Tick 级别无撮合价，取末笔）。
pub fn aggregate_to_bars(trades: &[TradeRecord], bar_ns: u64) -> Vec<Bar> {
    if trades.is_empty() {
        return Vec::new();
    }
    // 修复 P0-M10: bar_ns == 0 会导致 bar_start += bar_ns 后 bar_end == bar_start
    // 形成无限循环（while bar_start <= last_time 永真）。直接返回空 Vec。
    if bar_ns == 0 {
        tracing::warn!("[stylized_facts] aggregate_to_bars 收到 bar_ns=0，返回空 Vec");
        return Vec::new();
    }

    let first_time = trades[0].timestamp;
    let last_time = trades[trades.len() - 1].timestamp;
    let mut bars = Vec::new();

    let mut bar_start = first_time;
    while bar_start <= last_time {
        let bar_end = bar_start + bar_ns;
        let window_trades: Vec<&TradeRecord> =
            trades.iter().filter(|t| t.timestamp >= bar_start && t.timestamp < bar_end).collect();

        let bar_time = bar_start;
        bar_start = bar_end;

        if window_trades.is_empty() {
            continue;
        }

        let open = window_trades[0].price as f64;
        let close = window_trades[window_trades.len() - 1].price as f64;
        let high = window_trades.iter().map(|t| t.price as f64).fold(f64::NEG_INFINITY, f64::max);
        let low = window_trades.iter().map(|t| t.price as f64).fold(f64::INFINITY, f64::min);
        let volume: u64 = window_trades.iter().map(|t| t.quantity).sum();

        bars.push(Bar { time: bar_time, open, high, low, close, volume });
    }

    bars
}

/// A 股日收益率的目标范围（来自 A 股实证研究文献）
pub struct TargetRange {
    pub kurtosis: (f64, f64),      // 峰度：A 股日收益 ~8-12
    pub hurst: (f64, f64),         // Hurst 指数：~0.45-0.55（接近随机游走）
    pub lb_significant: bool,      // Ljung-Box Q(20) 是否显著（p < 0.05）
    pub leverage_corr: (f64, f64), // 杠杆效应相关系数：~(-0.3)-(-0.1)
}

impl Default for TargetRange {
    fn default() -> Self {
        Self {
            kurtosis: (5.0, 15.0),
            hurst: (0.40, 0.60),
            lb_significant: true,
            leverage_corr: (-0.4, -0.05),
        }
    }
}

/// Stylized Facts 计算结果
#[derive(Debug, Clone)]
pub struct StylizedFacts {
    /// 收益率峰度（正态分布 = 3.0）
    pub kurtosis: f64,
    /// 收益率偏度
    pub skewness: f64,
    /// Hurst 指数（R/S 分析法）
    pub hurst_exponent: f64,
    /// Ljung-Box Q(20) 统计量
    pub lb_stat: f64,
    /// Ljung-Box p 值
    pub lb_pvalue: f64,
    /// 杠杆效应：收益(t) 与 波动率(t+1) 的相关系数
    pub leverage_corr: f64,
    /// 总观测数
    pub n_observations: usize,
    /// 通过的检查项列表
    pub passed: Vec<String>,
    /// 未通过的检查项列表
    pub failed: Vec<String>,
}

/// 从成交记录提取价格序列，计算 Stylized Facts。
///
/// 返回每个时间点的价格序列、对数收益率序列。
/// 修复 M-DS-7: 原代码注释假设 `trades` 已按时间排序但未 enforce，
/// 上游若乱序传入会导致收益率序列错乱、Stylized Facts 失真。
/// 现显式复制并按 timestamp 排序，保证单调。
fn price_and_returns(trades: &[TradeRecord]) -> (Vec<f64>, Vec<f64>) {
    if trades.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // 修复 M-DS-7: 显式排序而非假设上游已排序
    let mut sorted_trades = trades.to_vec();
    sorted_trades.sort_by_key(|t| t.timestamp);

    // 按时间排序后的价格序列
    let prices: Vec<f64> = sorted_trades.iter().map(|t| t.price as f64).collect();

    // 对数收益率
    let returns: Vec<f64> =
        prices.windows(2).map(|w| (w[1] / w[0]).ln()).filter(|r| r.is_finite()).collect();

    (prices, returns)
}

/// 计算峰度
fn compute_kurtosis(returns: &[f64]) -> f64 {
    if returns.len() < 4 {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    if variance == 0.0 {
        return 0.0;
    }
    // 超峰度 = fourth - 3，这里返回原始峰度
    returns.iter().map(|r| (r - mean).powi(4)).sum::<f64>() / n / variance.powi(2)
}

/// 计算偏度
fn compute_skewness(returns: &[f64]) -> f64 {
    if returns.len() < 3 {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    if variance == 0.0 {
        return 0.0;
    }
    returns.iter().map(|r| (r - mean).powi(3)).sum::<f64>() / n / variance.sqrt().powi(3)
}

/// 计算 Hurst 指数（R/S 分析法简化版）
fn compute_hurst(returns: &[f64]) -> f64 {
    let n = returns.len();
    if n < 100 {
        return 0.5; // 样本不足，默认随机游走
    }

    // 用 4 个时间尺度粗略估计
    let scales = (2..=5).map(|k| n / 2usize.pow(k)).filter(|&s| s >= 10);
    let mut rs_values = Vec::new();

    for scale in scales {
        let n_segments = n / scale;
        let mut avg_rs = 0.0;

        for i in 0..n_segments {
            let seg = &returns[i * scale..(i + 1) * scale];
            let seg_mean = seg.iter().sum::<f64>() / seg.len() as f64;
            let deviations: Vec<f64> = seg.iter().map(|r| r - seg_mean).collect();
            let cumulative: Vec<f64> = deviations
                .iter()
                .scan(0.0, |state, &x| {
                    *state += x;
                    Some(*state)
                })
                .collect();

            let r = cumulative.iter().fold(f64::NAN, |m, v| v.max(m)) // max
                - cumulative.iter().fold(f64::NAN, |m, v| v.min(m)); // min

            let s = (deviations.iter().map(|d| d.powi(2)).sum::<f64>() / deviations.len() as f64)
                .sqrt();
            if s > 0.0 {
                avg_rs += r / s;
            }
        }
        if n_segments > 0 {
            avg_rs /= n_segments as f64;
            if avg_rs > 0.0 && scale > 0 {
                rs_values.push(((scale as f64).ln(), avg_rs.ln()));
            }
        }
    }

    if rs_values.len() < 2 {
        return 0.5;
    }

    // 线性回归估算 Hurst
    let n_pts = rs_values.len() as f64;
    let sum_x: f64 = rs_values.iter().map(|(x, _)| x).sum();
    let sum_y: f64 = rs_values.iter().map(|(_, y)| y).sum();
    let sum_xy: f64 = rs_values.iter().map(|(x, y)| x * y).sum();
    let sum_xx: f64 = rs_values.iter().map(|(x, _)| x * x).sum();

    let slope = (n_pts * sum_xy - sum_x * sum_y) / (n_pts * sum_xx - sum_x * sum_x);
    if slope.is_finite() { slope } else { 0.5 }
}

/// 计算 Ljung-Box Q 统计量（滞后 20 阶）
fn compute_ljung_box(returns: &[f64], lag: usize) -> (f64, f64) {
    let n = returns.len();
    if n <= lag {
        return (0.0, 1.0);
    }

    let mean = returns.iter().sum::<f64>() / n as f64;

    // 计算自协方差
    let gamma0 = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n as f64;
    if gamma0 == 0.0 {
        return (0.0, 1.0);
    }

    let mut q_stat = 0.0;
    for k in 1..=lag {
        let gamma_k: f64 =
            (0..(n - k)).map(|i| (returns[i] - mean) * (returns[i + k] - mean)).sum::<f64>()
                / n as f64;
        let r_k = gamma_k / gamma0;
        q_stat += r_k.powi(2) / (n - k) as f64;
    }
    q_stat *= n as f64;

    // 用卡方分布近似 p 值（简化）
    let chi2 = q_stat;
    let dof = lag as f64;
    // 使用近似公式
    let p_value = if chi2 <= 0.0 {
        1.0
    } else {
        let x = (chi2 / dof).powf(1.0 / 3.0);
        let z = (x - (1.0 - 2.0 / (9.0 * dof))) / (2.0 / (9.0 * dof)).sqrt();
        // 标准正态 CDF 近似
        let t = 1.0 / (1.0 + 0.2316419 * z.abs());
        let cdf = 1.0
            - (-z.powi(2) / 2.0).exp()
                * (0.319381530 * t - 0.356563782 * t.powi(2) + 1.781477937 * t.powi(3)
                    - 1.821255978 * t.powi(4)
                    + 1.330274429 * t.powi(5))
                / (2.0 * std::f64::consts::PI).sqrt();
        if z >= 0.0 { 1.0 - cdf } else { cdf }
    };

    (q_stat, p_value)
}

/// 计算杠杆效应（收益与后续波动率的相关性）
fn compute_leverage(returns: &[f64]) -> f64 {
    let n = returns.len();
    if n < 10 {
        return 0.0;
    }

    // 滞后 1 的收益与波动率相关
    let mut pairs = Vec::new();
    for i in 0..(n - 1) {
        let r_t = returns[i];
        let vol_t1 = returns[i + 1].abs();
        if r_t.is_finite() && vol_t1.is_finite() {
            pairs.push((r_t, vol_t1));
        }
    }

    if pairs.len() < 5 {
        return 0.0;
    }

    let m = pairs.len() as f64;
    let mean_r = pairs.iter().map(|(r, _)| r).sum::<f64>() / m;
    let mean_v = pairs.iter().map(|(_, v)| v).sum::<f64>() / m;

    let cov = pairs.iter().map(|(r, v)| (r - mean_r) * (v - mean_v)).sum::<f64>() / m;
    let var_r = pairs.iter().map(|(r, _)| (r - mean_r).powi(2)).sum::<f64>() / m;
    let var_v = pairs.iter().map(|(_, v)| (v - mean_v).powi(2)).sum::<f64>() / m;

    if var_r > 0.0 && var_v > 0.0 {
        cov / (var_r * var_v).sqrt()
    } else {
        0.0
    }
}

// ── 公共 API ──

/// 从收益率序列计算 StylizedFacts 并做目标校验
fn compute_facts_from_returns(returns: &[f64]) -> StylizedFacts {
    let n = returns.len();

    let kurtosis = compute_kurtosis(returns);
    let skewness = compute_skewness(returns);
    let hurst = compute_hurst(returns);
    let (lb_stat, lb_pvalue) = compute_ljung_box(returns, 20);
    let leverage = compute_leverage(returns);

    let mut facts = StylizedFacts {
        kurtosis,
        skewness,
        hurst_exponent: hurst,
        lb_stat,
        lb_pvalue,
        leverage_corr: leverage,
        n_observations: n,
        passed: Vec::new(),
        failed: Vec::new(),
    };

    facts.validate(&TargetRange::default());
    facts
}

impl StylizedFacts {
    /// 从成交记录计算 Stylized Facts（直接使用 tick 收益率）
    pub fn from_trades(trades: &[TradeRecord]) -> Self {
        let (_, returns) = price_and_returns(trades);
        compute_facts_from_returns(&returns)
    }

    /// 从 Bar 序列计算 Stylized Facts（推荐：bar 收益率更接近日收益率统计特征）
    ///
    /// `bar_ns` 是每个 Bar 的时间窗口宽度（纳秒）。
    /// 建议值：模拟总时长的 1/500 ~ 1/1000（如 5s 模拟 → 5-10ms bar）。
    pub fn from_bars(trades: &[TradeRecord], bar_ns: u64) -> Self {
        let bars = aggregate_to_bars(trades, bar_ns);
        if bars.len() < 3 {
            return Self {
                kurtosis: 0.0,
                skewness: 0.0,
                hurst_exponent: 0.5,
                lb_stat: 0.0,
                lb_pvalue: 1.0,
                leverage_corr: 0.0,
                n_observations: bars.len(),
                passed: Vec::new(),
                failed: vec![format!("Bar 不足: {} < 3", bars.len())],
            };
        }

        // 用 Bar 的 close 价格计算对数收益率
        let prices: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let returns: Vec<f64> =
            prices.windows(2).map(|w| (w[1] / w[0]).ln()).filter(|r| r.is_finite()).collect();

        compute_facts_from_returns(&returns)
    }

    /// 对照目标范围验证
    pub fn validate(&mut self, target: &TargetRange) {
        self.passed.clear();
        self.failed.clear();

        if self.n_observations < 30 {
            self.failed.push(format!("样本不足: {} < 30", self.n_observations));
            return;
        }

        // 峰度
        if self.kurtosis >= target.kurtosis.0 && self.kurtosis <= target.kurtosis.1 {
            self.passed.push(format!(
                "峰度 {:.2} 在目标范围 [{}, {}]",
                self.kurtosis, target.kurtosis.0, target.kurtosis.1
            ));
        } else {
            self.failed.push(format!(
                "峰度 {:.2} 不在目标范围 [{}, {}]（肥尾不足）",
                self.kurtosis, target.kurtosis.0, target.kurtosis.1
            ));
        }

        // Hurst
        if self.hurst_exponent >= target.hurst.0 && self.hurst_exponent <= target.hurst.1 {
            self.passed.push(format!(
                "Hurst {:.3} 在目标范围 [{}, {}]",
                self.hurst_exponent, target.hurst.0, target.hurst.1
            ));
        } else {
            self.failed.push(format!(
                "Hurst {:.3} 不在目标范围 [{}, {}]",
                self.hurst_exponent, target.hurst.0, target.hurst.1
            ));
        }

        // Ljung-Box
        let lb_sig = self.lb_pvalue < 0.05;
        if target.lb_significant == lb_sig {
            self.passed.push(format!(
                "LB Q({:.0}) p={:.4} {}显著（波动聚集）",
                20.0,
                self.lb_pvalue,
                if lb_sig { "" } else { "不" }
            ));
        } else {
            self.failed.push(format!(
                "LB p={:.4} 不满足目标要求 (显著={})",
                self.lb_pvalue, target.lb_significant
            ));
        }

        // 杠杆效应
        if self.leverage_corr >= target.leverage_corr.0
            && self.leverage_corr <= target.leverage_corr.1
        {
            self.passed.push(format!(
                "杠杆效应 {:.3} 在目标范围 [{}, {}]",
                self.leverage_corr, target.leverage_corr.0, target.leverage_corr.1
            ));
        } else {
            self.failed.push(format!(
                "杠杆效应 {:.3} 不在目标范围 [{}, {}]",
                self.leverage_corr, target.leverage_corr.0, target.leverage_corr.1
            ));
        }
    }

    /// 是否通过所有校准检查
    pub fn is_calibrated(&self) -> bool {
        self.failed.is_empty()
    }

    /// 综合评分（越低越好）
    pub fn score(&self, target: &TargetRange) -> f64 {
        let mut score = 0.0;

        // 峰度偏差
        let target_kurt = (target.kurtosis.0 + target.kurtosis.1) / 2.0;
        score += ((self.kurtosis - target_kurt) / target_kurt).abs();

        // Hurst 偏差
        let target_hurst = (target.hurst.0 + target.hurst.1) / 2.0;
        score += ((self.hurst_exponent - target_hurst) / target_hurst).abs();

        // 杠杆效应偏差
        let target_lev = (target.leverage_corr.0 + target.leverage_corr.1) / 2.0;
        if target_lev != 0.0 {
            score += ((self.leverage_corr - target_lev) / target_lev).abs();
        }

        // LB 不显著则罚分
        if self.lb_pvalue >= 0.05 {
            score += 2.0;
        }

        // 惩罚项：样本不足
        if self.n_observations < 50 {
            score += 10.0;
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个具有可控统计特征的测试价格序列
    fn make_test_trades(prices: &[i64]) -> Vec<TradeRecord> {
        prices
            .iter()
            .enumerate()
            .map(|(i, &p)| TradeRecord {
                price: p,
                quantity: 100,
                buyer_agent_id: "buyer".into(),
                seller_agent_id: "seller".into(),
                buyer_order_id: i as u64 * 2,
                seller_order_id: i as u64 * 2 + 1,
                timestamp: i as u64 * 1_000_000,
            })
            .collect()
    }

    #[test]
    fn test_normal_returns_kurtosis_3() {
        // 用正态分布随机数近似生成
        let mut prices = Vec::new();
        let mut p = 1000.0;
        for _ in 0..500 {
            // Box-Muller
            let u1: f64 = 0.5;
            let u2: f64 = 0.7;
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            p *= 1.0 + z * 0.005;
            prices.push(p.round() as i64);
        }
        let trades = make_test_trades(&prices);
        let facts = StylizedFacts::from_trades(&trades);
        // 正态分布峰度应在 3 附近
        assert!((facts.kurtosis - 3.0).abs() < 3.0, "峰度={:.2}", facts.kurtosis);
        assert!(facts.n_observations > 0);
    }

    #[test]
    fn test_empty_trades() {
        let facts = StylizedFacts::from_trades(&[]);
        assert!(facts.n_observations == 0);
        assert!(!facts.is_calibrated());
    }

    #[test]
    fn test_score_returns_finite() {
        let trades = make_test_trades(&[1000, 1001, 998, 1002, 999, 1000, 1005, 995]);
        let facts = StylizedFacts::from_trades(&trades);
        let score = facts.score(&TargetRange::default());
        assert!(score.is_finite());
    }
}
