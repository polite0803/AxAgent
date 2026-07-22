// SPDX-License-Identifier: AGPL-3.0-only

//! 决策事后验证（V55 新增）—— hit_rate / 假阳性率 / 9 因子 IC 测算
//!
//! ## 背景
//! 股票分析系统的最大问题是"决策可采信度低"。本模块用历史数据反推当前决策系统的
//! 真实命中率与因子有效性，把"事后验证"做成系统的一等公民。
//!
//! ## 数据流
//! ```text
//! reco_picks 表 (决策输出)
//!     ↓ run_decision_backtest 命令
//! 拉取 T+5 / T+20 / T+60 实际价格 (行情 API)
//!     ↓
//! validate_pick(): 推断 action → 计算 hit_outcome → 写 decision_validations
//!     ↓
//! compute_hit_rate_report(): 聚合 hit_rate + 9 因子 IC
//!     ↓
//! 输出给前端 / 反馈到 portfolio-mgr.rhai 因子权重
//! ```
//!
//! ## 关键设计
//! - 推断 action（不依赖显式 action 字段）：target_price > price × 1.03 → "buy"
//!   target_price < price × 0.97 → "sell"，否则 "hold"
//! - hit_outcome 判定（buy）：final_return_pct > 0 → "hit"；< -5% → "false_hit"；
//!   触及 stop_loss → "miss"；触及 target → "hit"；介于 -5%~0 → "partial"
//! - factor IC = Spearman(因子值, 实际收益) — 衡量该因子对未来收益的预测能力
//! - 9 因子权重若无 IC 标定则全为 0.15/0.20/0.25 等"专家拍脑袋"值，
//!   这是 portfolio-mgr.rhai 可信度低的根因之一。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::recommender::types::RecoPick;

// ── 数据结构 ──

/// 单条决策的事后验证结果（对应 decision_validations 表的一行）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickValidation {
    pub pick_id: String,
    pub stock_code: String,
    pub stock_name: String,
    pub style: String,
    pub period: String,
    pub generated_at: String,
    pub t_plus_n: i32,
    pub entry_price: f64,
    pub target_price: f64,
    pub stop_loss: f64,
    pub position_pct: f64,
    pub confidence: i32,
    /// 由 target_price vs price 推断（"buy" | "sell" | "hold"）
    pub inferred_action: String,
    pub t_plus_n_price: Option<f64>,
    pub max_price: Option<f64>,
    pub min_price: Option<f64>,
    pub max_return_pct: Option<f64>,
    pub max_drawdown_pct: Option<f64>,
    pub final_return_pct: Option<f64>,
    pub hit_stop_loss: Option<i32>,
    pub hit_target: Option<i32>,
    /// "hit" | "miss" | "false_hit" | "partial" | "insufficient"
    pub hit_outcome: Option<String>,
    /// 9 因子快照（key=factor_id, value=0-1 数值），供 IC 重标定
    pub factor_snapshot: Option<HashMap<String, f64>>,
    pub data_source: String,
}

/// 按 action / style / t_plus_n 维度的统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionStats {
    pub total: usize,
    pub hit: usize,
    pub miss: usize,
    pub false_hit: usize,
    pub partial: usize,
    pub insufficient: usize,
    pub avg_return_pct: f64,
    pub median_return_pct: f64,
    /// hit_rate = hit / (hit + miss + false_hit)，不含 partial/insufficient
    pub hit_rate: f64,
}

/// 整体 hit_rate 报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitRateReport {
    pub total: usize,
    pub generated_at: String,
    /// key=action, value=stats
    pub by_action: HashMap<String, ActionStats>,
    /// key=style, value=stats
    pub by_style: HashMap<String, ActionStats>,
    /// key=t_plus_n (5/20/60), value=stats
    pub by_t_plus_n: HashMap<String, ActionStats>,
    /// 9 因子 IC（Spearman 等级相关）
    /// key=factor_id (f1_technical, f2_consensus, ...), value=IC in [-1, 1]
    /// IC 接近 0 = 该因子无预测力
    /// IC > 0.1 = 有正向预测力
    /// IC < -0.05 = 反向预测（应做空或反权重）
    pub factor_ic: HashMap<String, f64>,
    /// IC 绝对值排序：最有预测力的因子 → 最没预测力的因子
    pub factor_ic_ranked: Vec<(String, f64)>,
    /// 命中最好的 5 条决策（用于"亮点"展示）
    pub best_picks: Vec<PickValidation>,
    /// 命中最差的 5 条决策（用于"教训"展示）
    pub worst_picks: Vec<PickValidation>,
}

// ── 核心函数 ──

/// 推断 action：从 target_price / price 关系推断（不依赖显式 action 字段）
///
/// 规则：
/// - target_price / price > 1.01 → "buy"（隐含预期上涨 1%+）
/// - target_price / price < 0.99 → "sell"（隐含预期下跌 1%+）
/// - 其他 → "hold"
///
/// 修复 P1: 原阈值 3% 偏高 —— A 股 T+1 持有至少 1 日，策略计算出的 target_price
/// 通常已包含最小预期涨幅。3% 阈值会把"轻微看涨"（1-3%）误判为 "hold"，
/// 导致命中率统计分母偏大。降到 1% 更贴合实际信号强度。
pub fn infer_action(price: f64, target_price: f64) -> &'static str {
    if price <= 0.0 {
        return "hold";
    }
    let ratio = target_price / price;
    if ratio > 1.01 {
        "buy"
    } else if ratio < 0.99 {
        "sell"
    } else {
        "hold"
    }
}

/// 计算单条决策的 hit_outcome（基于 T+N 实际价格）
///
/// ## 判定规则
/// - **buy**:
///   - final_return_pct > 0% → "hit"
///   - final_return_pct <= -5% → "false_hit"（"看多却跌" 是最严重的误判）
///   - 触及 stop_loss → "miss"
///   - 触及 target → "hit"
///   - 介于 -5% ~ 0% → "partial"
/// - **sell**:
///   - final_return_pct < 0% → "hit"（看空看对了）
///   - final_return_pct > 5% → "false_hit"
///   - 其他 → "partial"
/// - **hold**: 窄幅震荡判断
///   - |final_return_pct| <= 1% → "hit"（震荡判断正确，价格确实未明显变动）
///   - |final_return_pct| > 5% → "false_hit"（判断震荡但市场走出趋势）
///   - 其他 → "partial"
/// - **数据不足**: 任意 None → "insufficient"
pub fn compute_hit_outcome(
    action: &str,
    final_return_pct: Option<f64>,
    hit_stop_loss: Option<i32>,
    hit_target: Option<i32>,
) -> Option<String> {
    let Some(ret) = final_return_pct else {
        return Some("insufficient".to_string());
    };
    match action {
        "buy" => {
            if hit_stop_loss == Some(1) {
                Some("miss".to_string())
            } else if hit_target == Some(1) || ret > 0.0 {
                Some("hit".to_string())
            } else if ret < -5.0 {
                Some("false_hit".to_string())
            } else {
                Some("partial".to_string())
            }
        },
        "sell" => {
            if ret < 0.0 {
                Some("hit".to_string())
            } else if ret > 5.0 {
                Some("false_hit".to_string())
            } else {
                Some("partial".to_string())
            }
        },
        // 修复 P1: hold 原一律 "partial"，未反映震荡判断的正确性。
        // 窄幅震荡（|ret| ≤ 1%）→ hit；明显趋势（|ret| > 5%）→ false_hit。
        "hold" => {
            let abs_ret = ret.abs();
            if abs_ret <= 1.0 {
                Some("hit".to_string())
            } else if abs_ret > 5.0 {
                Some("false_hit".to_string())
            } else {
                Some("partial".to_string())
            }
        },
        _ => Some("insufficient".to_string()),
    }
}

/// 从 K 线数据计算验证指标（T+N 收盘价 / 期间最大最小价 / 收益等）
///
/// ## 入参
/// - `entry_price`: 决策时的价格
/// - `closes`: 从决策日 T+1 到 T+N 的每日收盘价（按时间升序）
/// - `highs`: 对应每日最高价
/// - `lows`: 对应每日最低价
/// - `target_price`: 目标位
/// - `stop_loss`: 止损
///
/// ## 返回
/// 返回的字段都是 Option —— 数据缺失（k 线不足）时为 None
/// `compute_price_metrics` 的返回结果（全部为 Option，数据缺失时为 None）
#[derive(Debug, Clone, Copy, Default)]
pub struct PriceMetrics {
    pub t_plus_n_price: Option<f64>,
    pub max_price: Option<f64>,
    pub min_price: Option<f64>,
    pub max_return_pct: Option<f64>,
    pub max_drawdown_pct: Option<f64>,
    pub final_return_pct: Option<f64>,
    pub hit_stop_loss: Option<i32>,
    pub hit_target: Option<i32>,
}

pub fn compute_price_metrics(
    entry_price: f64,
    closes: &[f64],
    highs: &[f64],
    lows: &[f64],
    target_price: f64,
    stop_loss: f64,
) -> PriceMetrics {
    if closes.is_empty() || entry_price <= 0.0 {
        return PriceMetrics::default();
    }

    let t_plus_n_price = *closes.last().unwrap();

    let max_price = highs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_price = lows.iter().copied().fold(f64::INFINITY, f64::min);

    let max_return_pct = (max_price - entry_price) / entry_price * 100.0;

    // 修复 P0: max_drawdown 原误算为"最大亏损幅度"（从入场价到最低价的跌幅），
    // 正确定义是"从持有期间最高峰值到后续最低谷值的跌幅"。
    // 反例: entry=10, closes=[12,8,11]
    //   误算 = (8-10)/10*100 = -20%（从入场价到最低价）
    //   正确 = (12-8)/12*100 = 33.3%（从峰值 12 到谷值 8 的回撤）
    let mut peak = entry_price;
    let mut max_dd_pct = 0.0_f64;
    for &close in closes {
        if close > peak {
            peak = close;
        }
        if peak > 0.0 {
            let dd = (peak - close) / peak * 100.0;
            if dd > max_dd_pct {
                max_dd_pct = dd;
            }
        }
    }
    let max_drawdown_pct = max_dd_pct;
    let final_return_pct = (t_plus_n_price - entry_price) / entry_price * 100.0;

    let hit_stop_loss = min_price <= stop_loss;
    let hit_target = max_price >= target_price;

    PriceMetrics {
        t_plus_n_price: Some(t_plus_n_price),
        max_price: Some(max_price),
        min_price: Some(min_price),
        max_return_pct: Some(max_return_pct),
        max_drawdown_pct: Some(max_drawdown_pct),
        final_return_pct: Some(final_return_pct),
        hit_stop_loss: Some(if hit_stop_loss { 1 } else { 0 }),
        hit_target: Some(if hit_target { 1 } else { 0 }),
    }
}

/// 聚合一组 PickValidation 生成 hit_rate 报告
pub fn compute_hit_rate_report(validations: &[PickValidation]) -> HitRateReport {
    let mut by_action: HashMap<String, Vec<&PickValidation>> = HashMap::new();
    let mut by_style: HashMap<String, Vec<&PickValidation>> = HashMap::new();
    let mut by_t_plus_n: HashMap<String, Vec<&PickValidation>> = HashMap::new();

    for v in validations {
        by_action.entry(v.inferred_action.clone()).or_default().push(v);
        by_style.entry(v.style.clone()).or_default().push(v);
        by_t_plus_n.entry(v.t_plus_n.to_string()).or_default().push(v);
    }

    let action_stats = |group: &[&PickValidation]| -> ActionStats {
        let mut stats = ActionStats { total: group.len(), ..Default::default() };
        let mut returns: Vec<f64> = Vec::new();
        for v in group {
            match v.hit_outcome.as_deref() {
                Some("hit") => stats.hit += 1,
                Some("miss") => stats.miss += 1,
                Some("false_hit") => stats.false_hit += 1,
                Some("partial") => stats.partial += 1,
                _ => stats.insufficient += 1,
            }
            if let Some(ret) = v.final_return_pct {
                returns.push(ret);
            }
        }
        let denom = stats.hit + stats.miss + stats.false_hit;
        stats.hit_rate = if denom > 0 {
            stats.hit as f64 / denom as f64
        } else {
            0.0
        };
        if !returns.is_empty() {
            stats.avg_return_pct = returns.iter().sum::<f64>() / returns.len() as f64;
            let mut sorted = returns.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            stats.median_return_pct = sorted[sorted.len() / 2];
        }
        stats
    };

    let mut report = HitRateReport {
        total: validations.len(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        by_action: by_action.iter().map(|(k, v)| (k.clone(), action_stats(v))).collect(),
        by_style: by_style.iter().map(|(k, v)| (k.clone(), action_stats(v))).collect(),
        by_t_plus_n: by_t_plus_n.iter().map(|(k, v)| (k.clone(), action_stats(v))).collect(),
        factor_ic: HashMap::new(),
        factor_ic_ranked: Vec::new(),
        best_picks: Vec::new(),
        worst_picks: Vec::new(),
    };

    // 计算 9 因子 IC（Spearman 等级相关系数）
    report.factor_ic = compute_factor_ic(validations);
    let mut ranked: Vec<(String, f64)> =
        report.factor_ic.iter().map(|(k, v)| (k.clone(), *v)).collect();
    ranked.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
    report.factor_ic_ranked = ranked;

    // 排序 best / worst picks（按 final_return_pct 降序 / 升序）
    let mut sorted: Vec<PickValidation> =
        validations.iter().filter(|v| v.final_return_pct.is_some()).cloned().collect();
    sorted.sort_by(|a, b| {
        b.final_return_pct
            .unwrap_or(0.0)
            .partial_cmp(&a.final_return_pct.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    report.best_picks = sorted.iter().take(5).cloned().collect();
    sorted.reverse();
    report.worst_picks = sorted.iter().take(5).cloned().collect();

    report
}

/// 计算每个因子 vs 实际收益的 Spearman 等级相关系数（IC）
///
/// IC 越接近 0 → 该因子对未来收益无预测力
/// IC > 0.1 → 有正向预测力，应增加该因子权重
/// IC < -0.05 → 反向预测，应反向使用或减权
///
/// ## 算法
/// 对每个因子（factor_id）：
/// 1. 收集所有验证样本的 (factor_value, final_return_pct) 对
/// 2. 各自排序得等级
/// 3. Spearman = Pearson(等级_x, 等级_y)
pub fn compute_factor_ic(validations: &[PickValidation]) -> HashMap<String, f64> {
    let mut factor_to_pairs: HashMap<String, Vec<(f64, f64)>> = HashMap::new();

    for v in validations {
        let Some(ret) = v.final_return_pct else { continue };
        let Some(ref factors) = v.factor_snapshot else { continue };
        for (factor_id, factor_val) in factors {
            factor_to_pairs.entry(factor_id.clone()).or_default().push((*factor_val, ret));
        }
    }

    let mut ics = HashMap::new();
    for (factor_id, pairs) in factor_to_pairs {
        if pairs.len() < 5 {
            // 样本太少，IC 不可靠
            ics.insert(factor_id, 0.0);
            continue;
        }
        let xs: Vec<f64> = pairs.iter().map(|(x, _)| *x).collect();
        let ys: Vec<f64> = pairs.iter().map(|(_, y)| *y).collect();
        let rank_x = rank_average(&xs);
        let rank_y = rank_average(&ys);
        ics.insert(factor_id, pearson_correlation(&rank_x, &rank_y));
    }
    ics
}

/// 平均秩次（处理 ties：取所有相同值的平均秩）
fn rank_average(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    let mut indexed: Vec<(usize, f64)> = values.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ranks = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n && (indexed[j].1 - indexed[i].1).abs() < 1e-9 {
            j += 1;
        }
        // i..j 都是相同的值
        let avg_rank = ((i + 1) + j) as f64 / 2.0; // 1-indexed 平均
        for k in i..j {
            ranks[indexed[k].0] = avg_rank;
        }
        i = j;
    }
    ranks
}

/// Pearson 相关系数
fn pearson_correlation(xs: &[f64], ys: &[f64]) -> f64 {
    if xs.len() != ys.len() || xs.is_empty() {
        return 0.0;
    }
    let n = xs.len() as f64;
    let mean_x = xs.iter().sum::<f64>() / n;
    let mean_y = ys.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for i in 0..xs.len() {
        let dx = xs[i] - mean_x;
        let dy = ys[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }
    let denom = (var_x * var_y).sqrt();
    if denom < 1e-9 {
        0.0
    } else {
        cov / denom
    }
}

// ── 工具函数：把 RecoPick 转 PickValidation ──

/// 从 RecoPick + 已拉取的 T+N K 线数据构建 PickValidation
pub fn build_pick_validation(
    pick: &RecoPick,
    pick_id: &str,
    t_plus_n: i32,
    closes: &[f64],
    highs: &[f64],
    lows: &[f64],
    data_source: &str,
) -> PickValidation {
    let action = infer_action(pick.price, pick.target_price).to_string();
    let m =
        compute_price_metrics(pick.price, closes, highs, lows, pick.target_price, pick.stop_loss);
    let hit_outcome =
        compute_hit_outcome(&action, m.final_return_pct, m.hit_stop_loss, m.hit_target);

    PickValidation {
        pick_id: pick_id.to_string(),
        stock_code: pick.stock_code.clone(),
        stock_name: pick.stock_name.clone(),
        style: format!("{:?}", pick.style).to_lowercase(),
        period: format!("{:?}", pick.period).to_lowercase(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        t_plus_n,
        entry_price: pick.price,
        target_price: pick.target_price,
        stop_loss: pick.stop_loss,
        position_pct: pick.position_pct,
        confidence: pick.confidence as i32,
        inferred_action: action,
        t_plus_n_price: m.t_plus_n_price,
        max_price: m.max_price,
        min_price: m.min_price,
        max_return_pct: m.max_return_pct,
        max_drawdown_pct: m.max_drawdown_pct,
        final_return_pct: m.final_return_pct,
        hit_stop_loss: m.hit_stop_loss,
        hit_target: m.hit_target,
        hit_outcome,
        factor_snapshot: None, // TODO: 从 portfolio-mgr 节点结果中提取
        data_source: data_source.to_string(),
    }
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_action_buy() {
        assert_eq!(infer_action(10.0, 12.0), "buy"); // 20% 上涨
        assert_eq!(infer_action(10.0, 10.5), "buy"); // 5% 上涨
    }

    #[test]
    fn test_infer_action_sell() {
        assert_eq!(infer_action(10.0, 8.0), "sell");
        assert_eq!(infer_action(10.0, 9.0), "sell");
    }

    #[test]
    fn test_infer_action_hold() {
        assert_eq!(infer_action(10.0, 10.0), "hold");
        assert_eq!(infer_action(10.0, 10.05), "hold"); // 0.5% < 1%
        assert_eq!(infer_action(0.0, 10.0), "hold"); // 边界
    }

    #[test]
    fn test_hit_outcome_hold_narrow_range() {
        // 修复 P1: hold 窄幅震荡判定
        assert_eq!(compute_hit_outcome("hold", Some(0.5), None, None), Some("hit".to_string()));
        assert_eq!(compute_hit_outcome("hold", Some(-0.8), None, None), Some("hit".to_string()));
        assert_eq!(compute_hit_outcome("hold", Some(3.0), None, None), Some("partial".to_string()));
        assert_eq!(
            compute_hit_outcome("hold", Some(6.0), None, None),
            Some("false_hit".to_string())
        );
    }

    #[test]
    fn test_hit_outcome_buy_hit() {
        assert_eq!(compute_hit_outcome("buy", Some(2.0), None, None), Some("hit".to_string()));
        assert_eq!(compute_hit_outcome("buy", Some(0.5), None, Some(1)), Some("hit".to_string()));
    }

    #[test]
    fn test_hit_outcome_buy_false_hit() {
        assert_eq!(
            compute_hit_outcome("buy", Some(-7.0), None, None),
            Some("false_hit".to_string())
        );
    }

    #[test]
    fn test_hit_outcome_buy_miss_stop_loss() {
        assert_eq!(compute_hit_outcome("buy", Some(-3.0), Some(1), None), Some("miss".to_string()));
    }

    #[test]
    fn test_hit_outcome_sell_hit() {
        assert_eq!(compute_hit_outcome("sell", Some(-2.0), None, None), Some("hit".to_string()));
    }

    #[test]
    fn test_hit_outcome_sell_false_hit() {
        assert_eq!(
            compute_hit_outcome("sell", Some(8.0), None, None),
            Some("false_hit".to_string())
        );
    }

    #[test]
    fn test_hit_outcome_insufficient() {
        assert_eq!(compute_hit_outcome("buy", None, None, None), Some("insufficient".to_string()));
    }

    #[test]
    fn test_compute_price_metrics_uptrend() {
        let closes = vec![10.5, 11.0, 11.5, 12.0];
        let highs = vec![10.8, 11.2, 11.7, 12.2];
        let lows = vec![10.3, 10.9, 11.3, 11.8];
        let m = compute_price_metrics(10.0, &closes, &highs, &lows, 12.0, 9.5);
        assert_eq!(m.t_plus_n_price, Some(12.0));
        assert_eq!(m.max_price, Some(12.2));
        assert_eq!(m.min_price, Some(10.3));
        assert!((m.max_return_pct.unwrap() - 22.0).abs() < 1e-6);
        // 纯上行（closes 严格递增）无回撤，peak-to-trough 定义为 0
        assert!((m.max_drawdown_pct.unwrap() - 0.0).abs() < 1e-6);
        assert!((m.final_return_pct.unwrap() - 20.0).abs() < 1e-6);
        assert_eq!(m.hit_target, Some(1));
    }

    #[test]
    fn test_compute_price_metrics_drawdown_peak_to_trough() {
        // 验证 P0 修复：最大回撤 = 持有期间峰值到后续谷值的跌幅，
        // 而非「从入场价到最低价的跌幅」。
        // entry=10, closes=[12,8,11]:
        //   正确 = (12-8)/12*100 ≈ 33.333%（峰值 12 → 谷值 8）
        //   旧误算 = (10-8)/10*100 = 20%（从入场价到谷值，漏算入场后涨幅）
        let closes = vec![12.0, 8.0, 11.0];
        let highs = vec![12.0, 8.0, 11.0];
        let lows = vec![12.0, 8.0, 11.0];
        let m = compute_price_metrics(10.0, &closes, &highs, &lows, 20.0, 5.0);
        assert!((m.max_drawdown_pct.unwrap() - 100.0 * (12.0 - 8.0) / 12.0).abs() < 1e-6);
        // 回归守卫：绝不能回到旧实现的 20.0
        assert!((m.max_drawdown_pct.unwrap() - 20.0).abs() >= 1e-6);
    }

    #[test]
    fn test_compute_price_metrics_stop_loss_hit() {
        let closes = vec![9.8, 9.5, 9.2];
        let highs = vec![10.0, 9.7, 9.4];
        let lows = vec![9.5, 9.3, 9.0]; // 9.0 < stop_loss 9.5
        let m = compute_price_metrics(10.0, &closes, &highs, &lows, 12.0, 9.5);
        assert_eq!(m.hit_stop_loss, Some(1));
    }

    #[test]
    fn test_factor_ic_positive() {
        // 完美正相关：因子值越大，收益越大
        let mut validations = Vec::new();
        for (f, r) in [(0.1, -5.0), (0.3, -2.0), (0.5, 1.0), (0.7, 3.0), (0.9, 6.0)] {
            let mut factors = HashMap::new();
            factors.insert("f1_technical".to_string(), f);
            validations.push(PickValidation {
                pick_id: "p".into(),
                stock_code: "x".into(),
                stock_name: "x".into(),
                style: "trend".into(),
                period: "short".into(),
                generated_at: "t".into(),
                t_plus_n: 20,
                entry_price: 10.0,
                target_price: 12.0,
                stop_loss: 9.0,
                position_pct: 10.0,
                confidence: 50,
                inferred_action: "buy".into(),
                t_plus_n_price: Some(10.0 + r / 10.0),
                max_price: None,
                min_price: None,
                max_return_pct: None,
                max_drawdown_pct: None,
                final_return_pct: Some(r),
                hit_stop_loss: None,
                hit_target: None,
                hit_outcome: None,
                factor_snapshot: Some(factors),
                data_source: "test".into(),
            });
        }
        let ic = compute_factor_ic(&validations);
        let v = ic.get("f1_technical").copied().unwrap_or(0.0);
        assert!(v > 0.9, "expected strong positive IC, got {v}");
    }

    #[test]
    fn test_factor_ic_no_correlation() {
        let mut validations = Vec::new();
        // 因子值 vs 收益完全无关
        for (f, r) in [(0.1, 1.0), (0.3, -1.0), (0.5, 1.0), (0.7, -1.0), (0.9, 1.0)] {
            let mut factors = HashMap::new();
            factors.insert("f1".to_string(), f);
            validations.push(PickValidation {
                pick_id: "p".into(),
                stock_code: "x".into(),
                stock_name: "x".into(),
                style: "trend".into(),
                period: "short".into(),
                generated_at: "t".into(),
                t_plus_n: 20,
                entry_price: 10.0,
                target_price: 12.0,
                stop_loss: 9.0,
                position_pct: 10.0,
                confidence: 50,
                inferred_action: "buy".into(),
                t_plus_n_price: None,
                max_price: None,
                min_price: None,
                max_return_pct: None,
                max_drawdown_pct: None,
                final_return_pct: Some(r),
                hit_stop_loss: None,
                hit_target: None,
                hit_outcome: None,
                factor_snapshot: Some(factors),
                data_source: "test".into(),
            });
        }
        let ic = compute_factor_ic(&validations);
        let v = ic.get("f1").copied().unwrap_or(0.0);
        assert!(v.abs() < 0.3, "expected low IC for random data, got {v}");
    }

    #[test]
    fn test_compute_hit_rate_report_basic() {
        let make = |outcome: &str, ret: f64| PickValidation {
            pick_id: "p".into(),
            stock_code: "x".into(),
            stock_name: "x".into(),
            style: "trend".into(),
            period: "short".into(),
            generated_at: "t".into(),
            t_plus_n: 5,
            entry_price: 10.0,
            target_price: 12.0,
            stop_loss: 9.0,
            position_pct: 10.0,
            confidence: 50,
            inferred_action: "buy".into(),
            t_plus_n_price: Some(10.0 + ret / 100.0),
            max_price: None,
            min_price: None,
            max_return_pct: None,
            max_drawdown_pct: None,
            final_return_pct: Some(ret),
            hit_stop_loss: None,
            hit_target: None,
            hit_outcome: Some(outcome.into()),
            factor_snapshot: None,
            data_source: "test".into(),
        };
        let validations = vec![
            make("hit", 3.0),
            make("hit", 5.0),
            make("miss", -2.0),
            make("false_hit", -8.0),
            make("partial", -1.0),
        ];
        let report = compute_hit_rate_report(&validations);
        assert_eq!(report.total, 5);
        let buy_stats = report.by_action.get("buy").unwrap();
        assert_eq!(buy_stats.hit, 2);
        assert_eq!(buy_stats.miss, 1);
        assert_eq!(buy_stats.false_hit, 1);
        // hit_rate = 2 / (2+1+1) = 0.5
        assert!((buy_stats.hit_rate - 0.5).abs() < 1e-9);
    }
}
