//! 估值带（Valuation Band）— R3-C
//!
//! 给定一只股票历史财务快照(`FinancialSnapshot`)和当前快照,计算 PE / PB / PS 的
//! 5/10/25/50/75/90/95 分位带 + 当前值在分布中的位置。
//!
//! 用途：
//! - 估值带用于估值偏离度判断:当 PE 当前分位 < 25% 时认为是"历史低位"；> 75% 时为"历史高位"。
//! - 可视化为"PE/PB/PS 历年分布 + 当前红点"。

use serde::{Deserialize, Serialize};

/// 单一指标的"分位带"
///
/// `percentile` 数组:索引 0..6 对应 [5, 10, 25, 50, 75, 90, 95] 分位值;
/// `current` = 当前快照该指标;`current_percentile` = 当前值在历史样本中的分位(0..100)。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetricBand {
    /// 5 / 10 / 25 / 50 / 75 / 90 / 95 百分位
    pub percentiles: [f64; 7],
    pub current: Option<f64>,
    pub current_percentile: Option<f64>,
    /// 样本数(去掉 None 后的有效值)
    pub sample_size: usize,
}

/// 估值带综合结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValuationBand {
    pub stock_code: String,
    pub metric_pe: MetricBand,
    pub metric_pb: MetricBand,
    pub metric_ps: MetricBand,
    /// 历史样本的时间范围(最早日期 ~ 最晚日期)
    pub sample_start: Option<String>,
    pub sample_end: Option<String>,
    /// 评估结论
    pub verdict: String,
    /// 数据来源描述
    pub note: Option<String>,
}

const PERCENTILE_KEYS: [f64; 7] = [5.0, 10.0, 25.0, 50.0, 75.0, 90.0, 95.0];

/// 从一组有效值计算分位带。
///
/// 内部使用 nearest-rank 方法(简单稳健);要求输入非空。
fn compute_percentiles(values: &[f64], keys: &[f64; 7]) -> [f64; 7] {
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let mut out = [0.0; 7];
    if n == 0 {
        return out;
    }
    for (i, &p) in keys.iter().enumerate() {
        // rank = ceil(p/100 * n), 用 1-indexed; clamp 到 [1, n]
        let rank_f = (p / 100.0) * (n as f64);
        let mut rank = rank_f.ceil() as usize;
        if rank < 1 {
            rank = 1;
        }
        if rank > n {
            rank = n;
        }
        out[i] = sorted[rank - 1];
    }
    out
}

/// 计算"当前值"在历史样本中的百分位。
///
/// 用 `(n_lower + 0.5 * n_equal) / n_total * 100` 近似(连续分布假设),
/// 返回 0..100 范围;当 `current` 不在样本范围时返回 0 或 100。
pub fn current_percentile(values: &[f64], current: f64) -> f64 {
    let n = values.len();
    if n == 0 {
        return 50.0;
    }
    let mut lower = 0usize;
    let mut equal = 0usize;
    for &v in values {
        if v < current {
            lower += 1;
        } else if v == current {
            equal += 1;
        }
    }
    let rank = (lower as f64) + 0.5 * (equal as f64);
    (rank / (n as f64)) * 100.0
}

/// 单指标计算:从 snapshots 提取有效值 → 计算分位 → 用 current 计算当前位置。
pub fn metric_band_from(samples: &[Option<f64>], current: Option<f64>) -> MetricBand {
    let values: Vec<f64> = samples.iter().filter_map(|v| *v).collect();
    let sample_size = values.len();
    if sample_size == 0 {
        return MetricBand {
            percentiles: [0.0; 7],
            current,
            current_percentile: current.map(|_| 50.0),
            sample_size: 0,
        };
    }
    let percentiles = compute_percentiles(&values, &PERCENTILE_KEYS);
    let current_percentile = current.map(|c| current_percentile(&values, c));
    MetricBand {
        percentiles,
        current,
        current_percentile,
        sample_size,
    }
}

/// 从历史快照 + 当前快照,计算完整估值带。
///
/// 建议 historical 长度 ≥ 20 才有意义;不足时会设置 verdict = "insufficient"。
pub fn compute_valuation_band<S: FinancialSnapshotLike>(
    stock_code: &str,
    historical: &[S],
    current: Option<&S>,
) -> ValuationBand {
    let pe_vals: Vec<Option<f64>> = historical.iter().map(|s| s.pe_ttm()).collect();
    let pb_vals: Vec<Option<f64>> = historical.iter().map(|s| s.pb()).collect();
    let ps_vals: Vec<Option<f64>> = historical.iter().map(|s| s.ps_ttm()).collect();

    let pe = metric_band_from(&pe_vals, current.and_then(|c| c.pe_ttm()));
    let pb = metric_band_from(&pb_vals, current.and_then(|c| c.pb()));
    let ps = metric_band_from(&ps_vals, current.and_then(|c| c.ps_ttm()));

    let sample_start = historical
        .iter()
        .map(|s| s.snapshot_date().to_string())
        .min();
    let sample_end = historical
        .iter()
        .map(|s| s.snapshot_date().to_string())
        .max();

    let verdict = if pe.sample_size < 20 || pb.sample_size < 20 {
        "insufficient"
    } else {
        verdict_from_bands(&pe, &pb)
    };

    let note = if pe.sample_size < 20 {
        Some(format!("历史样本不足(PE {} < 20),分位仅供参考", pe.sample_size))
    } else {
        None
    };

    ValuationBand {
        stock_code: stock_code.to_string(),
        metric_pe: pe,
        metric_pb: pb,
        metric_ps: ps,
        sample_start,
        sample_end,
        verdict: verdict.to_string(),
        note,
    }
}

fn verdict_from_bands(pe: &MetricBand, pb: &MetricBand) -> &'static str {
    let pe_pct = pe.current_percentile.unwrap_or(50.0);
    let pb_pct = pb.current_percentile.unwrap_or(50.0);
    let avg = (pe_pct + pb_pct) / 2.0;
    if avg < 25.0 {
        "deep_value"
    } else if avg < 40.0 {
        "undervalued"
    } else if avg > 75.0 {
        "overvalued"
    } else if avg > 60.0 {
        "expensive"
    } else {
        "fair"
    }
}

/// 最小化的"快照"接口,避免与 ORM 模型耦合。
///
/// `FinancialSnapshot` 实体实现此 trait,测试中的 mock struct 也实现。
pub trait FinancialSnapshotLike {
    fn snapshot_date(&self) -> &str;
    fn pe_ttm(&self) -> Option<f64>;
    fn pb(&self) -> Option<f64>;
    fn ps_ttm(&self) -> Option<f64>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用 mock
    #[derive(Debug, Clone)]
    struct MockSnap {
        date: String,
        pe: Option<f64>,
        pb: Option<f64>,
        ps: Option<f64>,
    }

    impl FinancialSnapshotLike for MockSnap {
        fn snapshot_date(&self) -> &str {
            &self.date
        }
        fn pe_ttm(&self) -> Option<f64> {
            self.pe
        }
        fn pb(&self) -> Option<f64> {
            self.pb
        }
        fn ps_ttm(&self) -> Option<f64> {
            self.ps
        }
    }

    fn sample_data(n: usize) -> Vec<MockSnap> {
        (0..n)
            .map(|i| MockSnap {
                date: format!("2024-{:02}-01", (i % 12) + 1),
                pe: Some(10.0 + (i as f64) * 0.5), // 10, 10.5, 11.0 ... 10 + 0.5*(n-1)
                pb: Some(1.0 + (i as f64) * 0.05),
                ps: Some(2.0 + (i as f64) * 0.02),
            })
            .collect()
    }

    #[test]
    fn percentiles_sorted() {
        let v: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let p = compute_percentiles(&v, &PERCENTILE_KEYS);
        // 5% → 5
        assert!((p[0] - 5.0).abs() < 1e-6, "p5={}", p[0]);
        // 50% → 50
        assert!((p[3] - 50.0).abs() < 1e-6, "p50={}", p[3]);
        // 95% → 95
        assert!((p[6] - 95.0).abs() < 1e-6, "p95={}", p[6]);
        // 升序
        for i in 0..6 {
            assert!(p[i] <= p[i + 1] + 1e-6);
        }
    }

    #[test]
    fn current_percentile_mid() {
        let v: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        // 5.5 不在样本中,应在中间
        let p = current_percentile(&v, 5.5);
        // lower=5, equal=0, rank=5, 5/10=50%
        assert!((p - 50.0).abs() < 1e-6, "p={}", p);
    }

    #[test]
    fn current_percentile_below() {
        let v: Vec<f64> = (10..=20).map(|i| i as f64).collect();
        let p = current_percentile(&v, 5.0);
        // 全部都 > 5, lower=0, 0/11=0
        assert!((p - 0.0).abs() < 1e-6, "p={}", p);
    }

    #[test]
    fn band_full_sample() {
        let samples = sample_data(60);
        // 60 个样本,pe 范围 10..39.5,pb 范围 1.0..3.95
        // pe=30 落在 (30-10)/0.5 = 第 40 名 → 40/60 = 66.7% → 触发 "expensive" (>60)
        let current = MockSnap {
            date: "2025-01-01".to_string(),
            pe: Some(30.0),
            pb: Some(3.0),
            ps: Some(3.0),
        };
        let band = compute_valuation_band("000001", &samples, Some(&current));
        assert_eq!(band.stock_code, "000001");
        assert_eq!(band.metric_pe.sample_size, 60);
        // PE 30 落在 40/60 = 66.7%, PB 3.0 落在 (3.0-1.0)/0.05 = 第 40 名 → 66.7%
        let pe_pct = band.metric_pe.current_percentile.unwrap();
        assert!(pe_pct > 60.0 && pe_pct < 75.0, "pe_pct={}", pe_pct);
        let pb_pct = band.metric_pb.current_percentile.unwrap();
        assert!(pb_pct > 60.0 && pb_pct < 75.0, "pb_pct={}", pb_pct);
        assert_eq!(band.verdict, "expensive");
    }

    #[test]
    fn band_insufficient_sample() {
        let samples = sample_data(5);
        let band = compute_valuation_band("000002", &samples, None);
        assert_eq!(band.verdict, "insufficient");
        assert!(band.note.is_some());
    }

    #[test]
    fn band_deep_value() {
        // 让 current 远低于历史
        let samples = sample_data(60);
        let current = MockSnap {
            date: "2025-01-01".to_string(),
            pe: Some(11.0),
            pb: Some(1.05),
            ps: Some(2.05),
        };
        let band = compute_valuation_band("000003", &samples, Some(&current));
        assert_eq!(band.verdict, "deep_value");
    }

    #[test]
    fn band_empty() {
        let band: ValuationBand = compute_valuation_band::<MockSnap>("000004", &[], None);
        assert_eq!(band.verdict, "insufficient");
        assert_eq!(band.metric_pe.sample_size, 0);
    }

    #[test]
    fn band_overvalued() {
        let samples = sample_data(60);
        // 60 个样本 pe=10..39.5, 给一个 38 的 PE
        let current = MockSnap {
            date: "2025-01-01".to_string(),
            pe: Some(38.0),
            pb: Some(3.8),
            ps: Some(4.0),
        };
        let band = compute_valuation_band("000005", &samples, Some(&current));
        assert_eq!(band.verdict, "overvalued");
    }

    #[test]
    fn band_handles_none_in_samples() {
        // 插入一些 None 后有效样本 = 20,刚好达到 20 阈值,verdict 应为 "fair" 或正常输出
        let mut samples = sample_data(30);
        for s in samples.iter_mut().step_by(3) {
            s.pe = None;
            s.pb = None;
        }
        let band = compute_valuation_band("000006", &samples, None);
        assert_eq!(band.metric_pe.sample_size, 20);
        // 20 >= 20 阈值,verdict 不再是 "insufficient"
        assert_ne!(band.verdict, "insufficient");
    }

    #[test]
    fn band_handles_too_few_after_filter() {
        // 30 个里 15 个被过滤 → 15 < 20,应 insufficient
        let mut samples = sample_data(30);
        for s in samples.iter_mut().step_by(2) {
            s.pe = None;
            s.pb = None;
        }
        let band = compute_valuation_band("000007", &samples, None);
        assert_eq!(band.metric_pe.sample_size, 15);
        assert_eq!(band.verdict, "insufficient");
        assert!(band.note.is_some());
    }
}
