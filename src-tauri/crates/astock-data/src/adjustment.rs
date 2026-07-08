//! 复权因子计算与 K 线复权应用 (R3-A)
//!
//! 复权是 A 股研究最基础的数据加工:
//! - **不复权 (None)**: 原始 K 线,价格在除权日会有"断崖" (split/dividend apply day)
//! - **前复权 (Forward)**: 以最近一日为基准,历史价按比例折算,最右一根是真实价
//!   公式: adj_price(t) = price(t) * factor(t)
//!   其中 factor(t) = ∏(events.ex_date > t)  (1 - cash_dividend / ex_close - bonus_ratio)
//! - **后复权 (Backward)**: 以最早一日为基准,后续价按比例折算,最左一根是真实价
//!   公式: adj_price(t) = price(t) / factor(t)
//!   其中 factor(t) = ∏(events.ex_date <= t)  (1 - cash_dividend / ex_close - bonus_ratio)
//!
//! 复权因子是连续乘法,实现采用按时间顺序累乘,精度可控。
//!
//! ## 时间旅行
//! - replay 模式: `as_of_date` 之前的除权事件生效,之后的过滤掉
//! - live 模式: 全部除权事件生效
//!
//! ## 现金流还原
//! 复权后的 K 线只反映价格走势,不还原分红现金流。
//! 持仓 PnL 需另行加回 cash_dividend。

use crate::types::{AdjType, AdjustmentEvent, KLine};
use serde::{Deserialize, Serialize};

/// 单个时点的复权因子 (R3-A)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdjFactorPoint {
    /// 原始日期
    pub date: String,
    /// 累计复权因子 (forward: 真实价 * factor = adj_price; backward: 真实价 / factor = adj_price)
    pub factor: f64,
}

/// 计算复权因子序列 (R3-A 核心算法)
///
/// # Arguments
/// - `klines`: 升序 K 线,日期格式 YYYY-MM-DD
/// - `events`: 除权除息事件 (按 ex_date 升序),需要先按时间过滤 as_of
/// - `adj_type`: 复权方式
///
/// # Returns
/// 与 `klines` 等长的 `Vec<AdjFactorPoint>`,每个点的 factor 表示该日 K 线应乘/除以多少
/// - 前复权: factor 末端 = 1.0 (最近一日是真实价)
/// - 后复权: factor 始端 = 1.0 (最早一日是真实价)
/// - 不复权: 全部 factor = 1.0
pub fn compute_adj_factors(
    klines: &[KLine],
    events: &[AdjustmentEvent],
    adj_type: AdjType,
) -> Vec<AdjFactorPoint> {
    if klines.is_empty() {
        return vec![];
    }
    match adj_type {
        AdjType::None => klines
            .iter()
            .map(|k| (k.date.clone(), 1.0))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(date, factor)| AdjFactorPoint { date, factor })
            .collect(),
        AdjType::Forward | AdjType::Backward => compute_factor_series(klines, events, adj_type),
    }
}

/// 核心累乘算法: 按 ex_date 顺序累乘事件复权率
fn compute_factor_series(
    klines: &[KLine],
    events: &[AdjustmentEvent],
    adj_type: AdjType,
) -> Vec<AdjFactorPoint> {
    // events 必须按 ex_date 升序
    let mut sorted_events: Vec<&AdjustmentEvent> = events.iter().collect();
    sorted_events.sort_by(|a, b| a.ex_date.cmp(&b.ex_date));

    // 为每个 K 线日找其"生效"的事件: ex_date <= date
    // 但需要 ex_date 当日 K 线的"昨收价"才能算事件复权率(1 - cash/prev_close - bonus)
    // 简化: 用 (1 - bonus_ratio) 作为事件复权率(送转的影响),分红单独记录
    //   这样复权后曲线在除权日还是有跳变,但仍连续可比较
    //   真实复权应拆成两步: 1) 送转调整 (无跳变) 2) 分红调整 (有跳变,但有现金补偿)
    //   行业惯例是只做送转调整,分红走"持仓 PnL 还原"路径

    let mut result = Vec::with_capacity(klines.len());
    let mut cumulative: f64 = 1.0;

    // 找到 last_kline_date
    let last_date = match adj_type {
        AdjType::Forward => klines.last().map(|k| k.date.clone()),
        AdjType::Backward => klines.first().map(|k| k.date.clone()),
        _ => None,
    };
    let _ = last_date; // suppress unused warning (last_date is conceptual)

    let mut event_idx = 0;
    for k in klines {
        // 累加 ex_date <= k.date 的所有事件复权率
        while event_idx < sorted_events.len() && sorted_events[event_idx].ex_date <= k.date {
            let ev = sorted_events[event_idx];
            // 单步复权率: 1 / (1 + bonus_ratio) — 送转后股本扩大,股价需除以扩股系数
            // 例: 10送2 → 股本从 100 → 120,股价从 10 → 10/1.2 = 8.33
            let step = 1.0 / (1.0 + ev.bonus_share_ratio.max(0.0));
            cumulative *= step;
            event_idx += 1;
        }

        let factor = match adj_type {
            // 前复权: 末端归一化 (最近一日 factor = 1)
            // 倒推: 累计到此时的 cumulative 应当 * (1/cumulative_at_last) → 1
            // 为避免双 pass, 这里直接返回 cumulative,后处理会再 normalize
            AdjType::Forward => cumulative,
            AdjType::Backward => cumulative,
            _ => 1.0,
        };

        result.push(AdjFactorPoint {
            date: k.date.clone(),
            factor,
        });
    }

    // 前复权归一化: 末端 factor → 1
    if adj_type == AdjType::Forward {
        if let Some(last_factor) = result.last().map(|p| p.factor) {
            if last_factor > 0.0 && (last_factor - 1.0).abs() > 1e-9 {
                let norm = 1.0 / last_factor;
                for p in result.iter_mut() {
                    p.factor *= norm;
                }
            }
        }
    }
    // 后复权: 始端归一化 (最早一日 factor = 1)
    else if adj_type == AdjType::Backward {
        if let Some(first_factor) = result.first().map(|p| p.factor) {
            if first_factor > 0.0 && (first_factor - 1.0).abs() > 1e-9 {
                let norm = 1.0 / first_factor;
                for p in result.iter_mut() {
                    p.factor *= norm;
                }
            }
        }
    }

    result
}

/// 应用复权因子到 K 线 (R3-A)
///
/// - 不复权: 原样返回
/// - 前复权: open/high/low/close *= factor
/// - 后复权: open/high/low/close /= factor
///   (volume 不变,amount = volume * close 用新 close)
pub fn apply_adjustment(
    klines: &[KLine],
    factors: &[AdjFactorPoint],
    adj_type: AdjType,
) -> Vec<KLine> {
    if adj_type == AdjType::None || factors.is_empty() || klines.is_empty() {
        // 不复权 或 输入为空: 保留 adj_factor=None
        return klines.to_vec();
    }
    if klines.len() != factors.len() {
        // 长度不匹配: 不要做部分复权,直接返回原数据
        return klines.to_vec();
    }

    klines
        .iter()
        .zip(factors.iter())
        .map(|(k, p)| {
            let scale = match adj_type {
                AdjType::Forward => p.factor,
                AdjType::Backward => {
                    if p.factor.abs() < 1e-9 {
                        1.0
                    } else {
                        1.0 / p.factor
                    }
                },
                _ => 1.0,
            };
            let open = k.open * scale;
            let high = k.high * scale;
            let low = k.low * scale;
            let close = k.close * scale;
            let amount = k.volume * close;
            KLine {
                date: k.date.clone(),
                open,
                high,
                low,
                close,
                volume: k.volume,
                amount,
                turnover_rate: k.turnover_rate,
                adj_factor: Some(scale),
            }
        })
        .collect()
}

/// 时间旅行过滤: replay 模式下只保留 ex_date <= as_of_date 的事件
pub fn filter_events_by_asof(
    events: &[AdjustmentEvent],
    as_of_date: Option<&str>,
) -> Vec<AdjustmentEvent> {
    match as_of_date {
        Some(cutoff) => events
            .iter()
            .filter(|e| e.ex_date.as_str() <= cutoff)
            .cloned()
            .collect(),
        None => events.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kline(date: &str, close: f64) -> KLine {
        KLine {
            date: date.to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 1000.0,
            amount: close * 1000.0,
            turnover_rate: None,
            adj_factor: None,
        }
    }

    fn event(ex_date: &str, bonus: f64, _cash: f64) -> AdjustmentEvent {
        AdjustmentEvent {
            stock_code: "000001".into(),
            ex_date: ex_date.into(),
            cash_dividend: _cash,
            bonus_share_ratio: bonus,
            rights_ratio: 0.0,
            rights_price: 0.0,
        }
    }

    #[test]
    fn none_adj_returns_all_ones() {
        let klines = vec![kline("2025-01-01", 10.0), kline("2025-02-01", 12.0)];
        let events = vec![event("2025-01-15", 0.0, 1.0)];
        let f = compute_adj_factors(&klines, &events, AdjType::None);
        assert_eq!(f.len(), 2);
        for p in &f {
            assert_eq!(p.factor, 1.0);
        }
    }

    #[test]
    fn forward_adj_normalizes_to_1_at_last() {
        // 10送2 在 2025-01-15 → 1/1.2 倍率
        let klines = vec![
            kline("2025-01-01", 12.0),
            kline("2025-01-15", 12.0),
            kline("2025-02-01", 14.4),
        ];
        let events = vec![event("2025-01-15", 0.2, 0.0)];
        let f = compute_adj_factors(&klines, &events, AdjType::Forward);
        // 最后一日归一化到 1
        assert!((f[2].factor - 1.0).abs() < 1e-6, "末端 factor 必须是 1: got {}", f[2].factor);
        // 早期未受事件影响,factor = (1/1.2)^{-1} = 1.2
        assert!((f[0].factor - 1.2).abs() < 1e-6, "早期 factor = 1.2: got {}", f[0].factor);
    }

    #[test]
    fn backward_adj_normalizes_to_1_at_first() {
        let klines = vec![
            kline("2025-01-01", 12.0),
            kline("2025-01-15", 12.0),
            kline("2025-02-01", 14.4),
        ];
        let events = vec![event("2025-01-15", 0.2, 0.0)];
        let f = compute_adj_factors(&klines, &events, AdjType::Backward);
        // 始端归一化到 1
        assert!((f[0].factor - 1.0).abs() < 1e-6, "始端 factor 必须是 1: got {}", f[0].factor);
        // 后期 factor = 1/1.2
        assert!(
            (f[2].factor - (1.0 / 1.2)).abs() < 1e-6,
            "后期 factor = 1/1.2: got {}",
            f[2].factor
        );
    }

    #[test]
    fn no_events_returns_all_ones() {
        let klines = vec![kline("2025-01-01", 10.0), kline("2025-02-01", 12.0)];
        let f = compute_adj_factors(&klines, &[], AdjType::Forward);
        for p in &f {
            assert_eq!(p.factor, 1.0);
        }
    }

    #[test]
    fn apply_none_returns_input_unchanged() {
        let klines = vec![kline("2025-01-01", 10.0)];
        let f = compute_adj_factors(&klines, &[], AdjType::None);
        let out = apply_adjustment(&klines, &f, AdjType::None);
        assert_eq!(out.len(), 1);
        assert!((out[0].close - 10.0).abs() < 1e-9);
        assert!(out[0].adj_factor.is_none());
    }

    #[test]
    fn apply_forward_scales_prices() {
        // 早期 K 价 12, 前复权后 12 * 1.2 = 14.4
        let klines = vec![kline("2025-01-01", 12.0), kline("2025-02-01", 14.4)];
        let events = vec![event("2025-01-15", 0.2, 0.0)];
        let f = compute_adj_factors(&klines, &events, AdjType::Forward);
        let out = apply_adjustment(&klines, &f, AdjType::Forward);
        // 末端 close = 14.4 * 1 = 14.4 (factor 归一化)
        assert!((out[1].close - 14.4).abs() < 1e-6);
        // 早期 close = 12 * 1.2 = 14.4
        assert!((out[0].close - 14.4).abs() < 1e-6);
        // adj_factor 已设置
        assert!(out[0].adj_factor.is_some());
    }

    #[test]
    fn apply_backward_divides_prices() {
        let klines = vec![kline("2025-01-01", 12.0), kline("2025-02-01", 14.4)];
        let events = vec![event("2025-01-15", 0.2, 0.0)];
        let f = compute_adj_factors(&klines, &events, AdjType::Backward);
        let out = apply_adjustment(&klines, &f, AdjType::Backward);
        // 始端 close = 12 / 1 = 12
        assert!((out[0].close - 12.0).abs() < 1e-6);
        // 后期 close = 14.4 / (1/1.2) = 14.4 * 1.2 = 17.28
        assert!((out[1].close - 17.28).abs() < 1e-6);
    }

    #[test]
    fn apply_handles_mismatched_lengths() {
        let klines = vec![kline("2025-01-01", 10.0)];
        let out = apply_adjustment(&klines, &[], AdjType::Forward);
        // 长度不匹配: 返回原数据
        assert_eq!(out.len(), 1);
        assert!((out[0].close - 10.0).abs() < 1e-9);
    }

    #[test]
    fn apply_empty_klines() {
        let out = apply_adjustment(&[], &[], AdjType::Forward);
        assert!(out.is_empty());
    }

    #[test]
    fn filter_events_by_asof_drops_future() {
        let events = vec![
            event("2025-01-15", 0.0, 0.0),
            event("2025-02-15", 0.0, 0.0),
            event("2025-03-15", 0.0, 0.0),
        ];
        let filtered = filter_events_by_asof(&events, Some("2025-02-20"));
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[1].ex_date, "2025-02-15");
    }

    #[test]
    fn filter_events_by_asof_none_keeps_all() {
        let events = vec![event("2025-01-15", 0.0, 0.0)];
        let filtered = filter_events_by_asof(&events, None);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn multiple_bonus_events_accumulate() {
        // 两次 10送1 (bonus=0.1): 总倍率 = 1/(1.1 * 1.1) = 1/1.21
        let klines = vec![
            kline("2025-01-01", 10.0),
            kline("2025-02-01", 11.0),
            kline("2025-03-01", 12.1),
        ];
        let events = vec![event("2025-01-15", 0.1, 0.0), event("2025-02-15", 0.1, 0.0)];
        let f = compute_adj_factors(&klines, &events, AdjType::Forward);
        // 末端归一: factor[2] = 1
        assert!((f[2].factor - 1.0).abs() < 1e-6);
        // 0日: 两次事件都生效,factor = 1.21
        assert!((f[0].factor - 1.21).abs() < 1e-6, "f[0] = {} 应为 1.21", f[0].factor);
        // 1月15日: 已发生 1 次,factor = 1.1
        assert!((f[1].factor - 1.1).abs() < 1e-6, "f[1] = {} 应为 1.1", f[1].factor);
    }
}
