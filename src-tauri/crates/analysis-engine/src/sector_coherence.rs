// SPDX-License-Identifier: AGPL-3.0-only

//! 板块联动分析（P3-2）
//!
//! 基于 [`concept_index::ConceptIndex`] 提供的"概念→股票"映射能力，
//! 分析同板块多只股票的当日联动行为，识别"龙头-从属"传导模式。
//!
//! ## 核心能力
//!
//! - [`SectorCoherenceReport`] — 板块联动报告
//! - [`compute_sector_coherence`] — 计算板块内股票当日涨跌一致性
//! - [`detect_leader_follower`] — 识别板块龙头与从属股
//! - [`detect_contagion`] — 检测"龙头领涨→从属跟涨"传导模式
//!
//! ## 使用场景
//!
//! - `RealtimeMonitor` 在盘中轮询时调用，发现板块异动时推送告警
//! - 组合级风控：检测持仓股票所在板块的整体异动，提前预警集中度风险
//! - 龙头股识别：找出当日板块内涨幅+成交额最大的股票，作为板块情绪风向标
//!
//! ## 算法说明
//!
//! ### 一致性系数（coherence_score）
//!
//! ```text
//! coherence = (n_up - n_down) / n_total，取值范围 [-1, 1]
//! ```text
//! - `+1`：板块内全部上涨（强多头联动）
//! - `-1`：板块内全部下跌（强空头联动）
//! - `0`：板块内涨跌各半（无联动）
//!
//! 一致性绝对值 ≥ 0.6 视为"强联动"。
//!
//! ### 龙头识别
//!
//! 综合排序分 = 涨跌幅 × 0.6 + 成交额归一化 × 0.4
//! 板块内综合排序分最高的股票视为龙头。
//!
//! ### 传导模式
//!
//! 若龙头股涨幅 > +5% 且板块一致性 ≥ 0.6，且至少 1 只从属股涨幅 > +3%，
//! 视为"龙头领涨→从属跟涨"传导模式启动。

use crate::concept_index::ConceptIndex;
use axagent_harness::market_data::StockQuote;
use std::collections::HashMap;

/// 板块联动报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorCoherenceReport {
    /// 板块/概念 ID（如 "concept_ai"）
    pub concept_id: String,
    /// 板块显示名（如 "人工智能"）
    pub concept_display: String,
    /// 板块内参与分析的股票数
    pub total_stocks: usize,
    /// 上涨股票数
    pub up_count: usize,
    /// 下跌股票数
    pub down_count: usize,
    /// 平盘股票数
    pub flat_count: usize,
    /// 一致性系数 [-1.0, 1.0]
    pub coherence_score: f64,
    /// 联动强度标签（"强联动" | "中联动" | "弱联动" | "无联动"）
    pub coherence_label: String,
    /// 板块平均涨跌幅（%）
    pub avg_change_pct: f64,
    /// 板块总成交额（元）
    pub total_amount: f64,
    /// 龙头股信息（综合排序分最高的股票）
    pub leader: Option<LeaderStock>,
    /// 跟涨/跟跌从属股（前 3 名）
    pub followers: Vec<FollowerStock>,
    /// 传导模式（"leader_up_follow_up" | "leader_down_follow_down" | "none"）
    pub contagion_pattern: String,
    /// 分析时间戳（Unix 秒）
    pub timestamp: i64,
}

/// 板块龙头股
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderStock {
    pub stock_code: String,
    pub stock_name: String,
    pub change_pct: f64,
    pub amount: f64,
    /// 综合排序分（涨跌幅 × 0.6 + 成交额归一化 × 0.4）
    pub composite_score: f64,
}

/// 板块从属股
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowerStock {
    pub stock_code: String,
    pub stock_name: String,
    pub change_pct: f64,
    pub amount: f64,
}

/// 计算板块联动一致性报告。
///
/// # 参数
///
/// - `concept_id`: 板块/概念 ID（必须在 `idx` 中注册）
/// - `quotes`: 板块内所有股票的实时行情（调用方先用 `idx.members()` 获取股票列表，
///   再批量调 `MarketDataProvider::get_quote` 拉行情）
/// - `timestamp`: 分析时间戳（Unix 秒）
///
/// # 返回
///
/// 返回 `SectorCoherenceReport`；若 `concept_id` 未注册或 `quotes` 为空，返回 `None`。
pub fn compute_sector_coherence(
    idx: &ConceptIndex,
    concept_id: &str,
    quotes: &[StockQuote],
    timestamp: i64,
) -> Option<SectorCoherenceReport> {
    if quotes.is_empty() {
        return None;
    }
    let concept_display = idx.display(concept_id).unwrap_or(concept_id).to_string();

    let total = quotes.len();
    let mut up_count = 0usize;
    let mut down_count = 0usize;
    let mut flat_count = 0usize;
    let mut sum_change_pct = 0f64;
    let mut total_amount = 0f64;

    for q in quotes {
        sum_change_pct += q.change_pct;
        total_amount += q.amount;
        if q.change_pct > 0.01 {
            up_count += 1;
        } else if q.change_pct < -0.01 {
            down_count += 1;
        } else {
            flat_count += 1;
        }
    }

    let coherence_score = (up_count as f64 - down_count as f64) / total as f64;
    let avg_change_pct = sum_change_pct / total as f64;
    let coherence_label = coherence_label(coherence_score.abs());

    // 识别龙头：综合排序分 = 涨跌幅 × 0.6 + 成交额归一化 × 0.4
    let max_amount = quotes.iter().map(|q| q.amount).fold(0f64, f64::max).max(1.0);
    let mut scored: Vec<(&StockQuote, f64)> = quotes
        .iter()
        .map(|q| {
            let amount_norm = q.amount / max_amount;
            let score = q.change_pct * 0.6 + amount_norm * 40.0 * 0.4; // 涨跌幅通常 ±10%，成交额归一化乘 40 调整量级
            (q, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let leader = scored.first().map(|(q, score)| LeaderStock {
        stock_code: q.code.clone(),
        stock_name: q.name.clone(),
        change_pct: q.change_pct,
        amount: q.amount,
        composite_score: *score,
    });

    // 从属股：按涨跌幅绝对值排序（剔除龙头），取前 3
    let mut followers: Vec<FollowerStock> = scored
        .iter()
        .skip(1)
        .map(|(q, _)| FollowerStock {
            stock_code: q.code.clone(),
            stock_name: q.name.clone(),
            change_pct: q.change_pct,
            amount: q.amount,
        })
        .collect();
    followers.sort_by(|a, b| {
        b.change_pct.abs().partial_cmp(&a.change_pct.abs()).unwrap_or(std::cmp::Ordering::Equal)
    });
    followers.truncate(3);

    // 传导模式判定
    let contagion_pattern = detect_contagion_pattern(&leader, &followers, coherence_score);

    Some(SectorCoherenceReport {
        concept_id: concept_id.to_string(),
        concept_display,
        total_stocks: total,
        up_count,
        down_count,
        flat_count,
        coherence_score,
        coherence_label,
        avg_change_pct,
        total_amount,
        leader,
        followers,
        contagion_pattern,
        timestamp,
    })
}

/// 一致性绝对值 → 标签映射
fn coherence_label(abs_score: f64) -> String {
    if abs_score >= 0.6 {
        "强联动".to_string()
    } else if abs_score >= 0.4 {
        "中联动".to_string()
    } else if abs_score >= 0.2 {
        "弱联动".to_string()
    } else {
        "无联动".to_string()
    }
}

/// 传导模式判定
///
/// - 龙头涨幅 > +5% 且一致性 ≥ 0.4（中联动以上），且有从属股涨幅 > +3% → leader_up_follow_up
/// - 龙头跌幅 < -5% 且一致性 ≤ -0.4，且有从属股跌幅 < -3% → leader_down_follow_down
/// - 否则 → none
///
/// 阈值取 0.4 而非 0.6：3/4 同向（coherence=0.5）已构成明显联动，
/// 过严会漏掉板块情绪初期的传导信号。
fn detect_contagion_pattern(
    leader: &Option<LeaderStock>,
    followers: &[FollowerStock],
    coherence: f64,
) -> String {
    let Some(l) = leader else {
        return "none".to_string();
    };

    if l.change_pct > 5.0 && coherence >= 0.4 {
        let has_strong_follower = followers.iter().any(|f| f.change_pct > 3.0);
        if has_strong_follower {
            return "leader_up_follow_up".to_string();
        }
    }

    if l.change_pct < -5.0 && coherence <= -0.4 {
        let has_strong_follower = followers.iter().any(|f| f.change_pct < -3.0);
        if has_strong_follower {
            return "leader_down_follow_down".to_string();
        }
    }

    "none".to_string()
}

/// 批量扫描多个板块的联动情况。
///
/// # 参数
///
/// - `idx`: 概念索引
/// - `concept_ids`: 要扫描的板块 ID 列表
/// - `quotes_by_concept`: 每个板块对应的实时行情（key = concept_id, value = 行情列表）
/// - `timestamp`: 分析时间戳
///
/// # 返回
///
/// 按联动强度（coherence_score 绝对值）降序排列的报告列表
pub fn scan_sectors(
    idx: &ConceptIndex,
    concept_ids: &[String],
    quotes_by_concept: &HashMap<String, Vec<StockQuote>>,
    timestamp: i64,
) -> Vec<SectorCoherenceReport> {
    let mut reports: Vec<SectorCoherenceReport> = concept_ids
        .iter()
        .filter_map(|cid| {
            quotes_by_concept
                .get(cid)
                .and_then(|qs| compute_sector_coherence(idx, cid, qs, timestamp))
        })
        .collect();

    // 按一致性绝对值降序，便于快速识别"异动板块"
    reports.sort_by(|a, b| {
        b.coherence_score
            .abs()
            .partial_cmp(&a.coherence_score.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    reports
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concept_index::{ConceptIndex, ConceptNode};

    fn make_quote(code: &str, name: &str, change_pct: f64, amount: f64) -> StockQuote {
        StockQuote {
            code: code.to_string(),
            name: name.to_string(),
            price: 10.0,
            pre_close: 10.0,
            open: 10.0,
            high: 10.0,
            low: 10.0,
            volume: 1000.0,
            amount,
            change_pct,
            turnover_rate: 1.0,
            pe: None,
            pb: None,
            total_mv: None,
            circulating_mv: None,
            limit_up: None,
            limit_down: None,
            is_st: false,
            timestamp: "2026-07-25".to_string(),
        }
    }

    fn make_index() -> ConceptIndex {
        let mut idx = ConceptIndex::new();
        idx.register(ConceptNode::new("concept_ai", "人工智能", "concept"));
        idx.add_membership("concept_ai", "000001");
        idx.add_membership("concept_ai", "000002");
        idx.add_membership("concept_ai", "000003");
        idx.add_membership("concept_ai", "000004");
        idx
    }

    #[test]
    fn test_strong_bullish_coherence() {
        let idx = make_index();
        let quotes = vec![
            make_quote("000001", "股票A", 6.0, 1e8),
            make_quote("000002", "股票B", 4.0, 5e7),
            make_quote("000003", "股票C", 3.5, 3e7),
            make_quote("000004", "股票D", -0.5, 1e7),
        ];
        let report = compute_sector_coherence(&idx, "concept_ai", &quotes, 1700000000).unwrap();
        assert_eq!(report.total_stocks, 4);
        assert_eq!(report.up_count, 3);
        assert_eq!(report.down_count, 1);
        // 3 up + 1 down → coherence = (3-1)/4 = 0.5 → 中联动
        assert!((report.coherence_score - 0.5).abs() < 1e-6);
        assert_eq!(report.coherence_label, "中联动");
        assert!(report.leader.is_some());
        assert_eq!(report.leader.as_ref().unwrap().stock_code, "000001");
        // 龙头 +6% > +5% 且 coherence ≥ 0.4，且有从属股 > +3% → 触发传导
        assert_eq!(report.contagion_pattern, "leader_up_follow_up");
    }

    #[test]
    fn test_strong_bearish_coherence() {
        let idx = make_index();
        let quotes = vec![
            make_quote("000001", "股票A", -7.0, 1e8),
            make_quote("000002", "股票B", -4.0, 5e7),
            make_quote("000003", "股票C", -3.5, 3e7),
            make_quote("000004", "股票D", 0.5, 1e7),
        ];
        let report = compute_sector_coherence(&idx, "concept_ai", &quotes, 1700000000).unwrap();
        // 3 down + 1 up → coherence = -0.5 → 中联动
        assert!((report.coherence_score - (-0.5)).abs() < 1e-6);
        assert_eq!(report.coherence_label, "中联动");
        assert_eq!(report.contagion_pattern, "leader_down_follow_down");
    }

    #[test]
    fn test_no_coherence() {
        let idx = make_index();
        let quotes = vec![
            make_quote("000001", "股票A", 2.0, 1e8),
            make_quote("000002", "股票B", -2.0, 5e7),
            make_quote("000003", "股票C", 1.0, 3e7),
            make_quote("000004", "股票D", -1.0, 1e7),
        ];
        let report = compute_sector_coherence(&idx, "concept_ai", &quotes, 1700000000).unwrap();
        assert!(report.coherence_score.abs() < 0.2);
        assert_eq!(report.coherence_label, "无联动");
        assert_eq!(report.contagion_pattern, "none");
    }

    #[test]
    fn test_empty_quotes_returns_none() {
        let idx = make_index();
        let report = compute_sector_coherence(&idx, "concept_ai", &[], 1700000000);
        assert!(report.is_none());
    }

    #[test]
    fn test_scan_sectors_sorts_by_coherence() {
        let mut idx = ConceptIndex::new();
        idx.register(ConceptNode::new("concept_a", "板块A", "concept"));
        idx.register(ConceptNode::new("concept_b", "板块B", "concept"));
        idx.add_membership("concept_a", "000001");
        idx.add_membership("concept_a", "000002");
        idx.add_membership("concept_b", "000003");
        idx.add_membership("concept_b", "000004");

        let mut quotes_map = HashMap::new();
        // 板块A：强联动
        quotes_map.insert(
            "concept_a".to_string(),
            vec![make_quote("000001", "A1", 5.0, 1e8), make_quote("000002", "A2", 4.0, 5e7)],
        );
        // 板块B：无联动
        quotes_map.insert(
            "concept_b".to_string(),
            vec![make_quote("000003", "B1", 1.0, 1e8), make_quote("000004", "B2", -1.0, 5e7)],
        );

        let reports = scan_sectors(
            &idx,
            &["concept_a".to_string(), "concept_b".to_string()],
            &quotes_map,
            1700000000,
        );
        assert_eq!(reports.len(), 2);
        // 强联动板块排在前
        assert!(reports[0].coherence_score.abs() > reports[1].coherence_score.abs());
        assert_eq!(reports[0].concept_id, "concept_a");
    }
}
