//! 荐股策略历史信号回测（两组对比）
//!
//! ## 输入
//! - **正向样本**: reco_picks 表中 synthetic=0 的推荐记录
//! - **负向样本**: 同次荐股的候选池中，未被推荐（或只被兜底合成）的股票
//!
//! ## 输出
//! 两组分别计算各策略的信号历史表现，并给出差异分析。
//!
//! ## 覆盖
//! - **Trend** (short/mid/long): 纯 K 线依赖 ✅
//! - **Reversion** (short/mid): 纯 K 线依赖 ✅
//! - **Value** (short/mid/long): K 线代理（低波幅+均线附近+温和量能） ✅
//! - **Capital** (short/mid/long): K 线代理（放量上涨+量价配合） ✅
//! - Watchlist: 兜底策略无信号逻辑，不计入回测 ⏭️

use crate::recommender::indicators;
use crate::recommender::types::Period;
use axagent_astock_data::{AStockClient, KLine};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::RwLock;

// ── 类型 ──

/// 单次信号回测结果（用于内部聚合）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategySignalResult {
    pub strategy_id: String,
    pub stock_code: String,
    pub stock_name: String,
    pub signal_date: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub holding_days: u32,
    pub return_pct: f64,
    pub was_profitable: bool,
    pub max_drawdown_pct: f64,
}

/// 单组（正向/负向）回测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupBacktestResult {
    pub label: String,
    pub stock_count: u32,
    pub strategies: HashMap<String, StrategyStats>,
}

/// 单个策略的统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyStats {
    pub strategy_id: String,
    pub style: String,
    pub period: String,
    pub total_signals: u32,
    pub win_count: u32,
    pub loss_count: u32,
    pub win_rate_pct: f64,
    pub avg_return_pct: f64,
    pub total_return_pct: f64,
    pub avg_max_drawdown_pct: f64,
    pub max_consecutive_losses: u32,
    pub sharpe_ratio: Option<f64>,
    pub profit_factor: Option<f64>,
}

/// 回测响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestComparisonResponse {
    /// 正向组（被推荐的股票）
    pub positive: GroupBacktestResult,
    /// 负向组（候选池中未被推荐的股票）
    pub negative: GroupBacktestResult,
    /// 正向样本股票列表
    pub positive_stocks: Vec<String>,
    /// 负向样本股票列表
    pub negative_stocks: Vec<String>,
    /// 被跳过的策略及原因
    pub skipped: Vec<String>,
}

// ── 信号检测 ──

fn closes(klines: &[KLine]) -> Vec<f64> {
    klines.iter().map(|k| k.close).collect()
}

fn detect_trend_short(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    let cs = closes(klines);
    let last = *cs.last()?;
    if klines.len() < 30 {
        return None;
    }
    let ma5 = indicators::sma(&cs, 5)?;
    let ma10 = indicators::sma(&cs, 10)?;
    let ma20 = indicators::sma(&cs, 20)?;
    if !(ma5 > ma10 && ma10 > ma20) {
        return None;
    }
    let high = indicators::highest(klines, 20)?;
    if last < high * read_f64(vars, "trend_high_20_threshold", 0.99) {
        return None;
    }
    let amt_ratio = klines.last()?.amount / indicators::avg_amount_20d(klines)?;
    if amt_ratio < read_f64(vars, "trend_vol_ratio_threshold", 0.8) {
        return None;
    }
    Some(last)
}

fn detect_trend_mid(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    let cs = closes(klines);
    let last = *cs.last()?;
    if klines.len() < 100 {
        return None;
    }
    let ma60 = indicators::sma(&cs, 60)?;
    if ma60.is_nan() || last < ma60 * read_f64(vars, "trend_ma60_threshold", 0.995) {
        return None;
    }
    if indicators::highest(klines, 60)? * read_f64(vars, "trend_high_60_threshold", 0.98) > last {
        return None;
    }
    if let Some((dif, dea, _)) = indicators::macd(klines, 12, 26, 9) {
        if dif <= dea {
            return None;
        }
    } else {
        return None;
    }
    Some(last)
}

fn detect_trend_long(klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    let cs = closes(klines);
    let last = *cs.last()?;
    if cs.len() < 250 {
        return None;
    }
    let ma250 = indicators::sma(&cs, 250)?;
    if ma250.is_nan() {
        return None;
    }
    let ma60 = indicators::sma(&cs, 60)?;
    if ma60 < ma250 * 0.95 {
        return None;
    }
    if last < ma60 * 0.95 {
        return None;
    }
    Some(last)
}

fn detect_trend_ultra_short(klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    let cs = closes(klines);
    let last = *cs.last()?;
    if cs.len() < 10 {
        return None;
    }
    // MA3 站上 + 今日涨幅 > 0
    let ma3 = indicators::sma(&cs, 3)?;
    if last < ma3 || cs[cs.len() - 1] <= cs[cs.len() - 2] {
        return None;
    }
    // 前一日也站上 MA3（确认短期动量持续）
    if cs[cs.len() - 2] < indicators::sma(&cs[..cs.len() - 1], 3)? {
        return None;
    }
    // 量价配合：当日成交量 > 5日均量
    let today_vol = klines.last()?.amount;
    let avg_vol_5 = indicators::avg_amount_n(klines, 5)?;
    if avg_vol_5 <= 0.0 || today_vol < avg_vol_5 * 0.8 {
        return None;
    }
    Some(last)
}

fn detect_reversion_short(klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 30 {
        return None;
    }
    let rsi_val = indicators::rsi(klines, 6)?;
    if rsi_val >= 35.0 {
        return None;
    }
    let avg_5 = indicators::avg_amount_n(klines, 5)?;
    if avg_5 <= 0.0 {
        return None;
    }
    let today_amt = klines.last()?.amount;
    if today_amt > avg_5 * 1.2 {
        return None;
    }
    Some(klines.last()?.close)
}

fn detect_reversion_mid(klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 100 {
        return None;
    }
    let dd = indicators::drawdown_from_high(klines, 250)?;
    if dd < 20.0 {
        return None;
    }
    let rsi30 = indicators::rsi(klines, 30)?;
    if rsi30 > 50.0 {
        return None;
    }
    Some(klines.last()?.close)
}

// ── Value 策略 K 线代理 ──
//
// 实际 Value 策略依赖 PE/PB/TTM 基本面数据，历史回测不可得。
// 此处用 K 线形态近似：低波幅 + 价格在均线附近 + 无异常放量，
// 反映"低估值股票"在 K 线上的典型特征（稳定、非投机）。

fn detect_value_short(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 30 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    let ma20 = indicators::sma(&cs, 20)?;
    // 价格在 MA20 附近（偏离 < 2%），不在上涨趋势中
    if last > ma20 * read_f64(vars, "value_upper_deviation", 1.02)
        || last < ma20 * read_f64(vars, "value_lower_deviation", 0.90)
    {
        return None;
    }
    // 低波幅：20 日振幅 < 15%
    let high20 = indicators::highest(klines, 20)?;
    let low20 = indicators::lowest(klines, 20)?;
    if (high20 - low20) / low20 > 0.15 {
        return None;
    }
    // 成交温和（非投机放量）
    let avg_amt = indicators::avg_amount_n(klines, 20)?;
    let today_amt = klines.last()?.amount;
    if today_amt > avg_amt * 2.0 {
        return None;
    }
    Some(last)
}

fn detect_value_mid(klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 100 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    let ma60 = indicators::sma(&cs, 60)?;
    // 价格在 MA60 附近（偏离 -20% ~ +5%），排除深跌垃圾股和暴涨热门股
    if last > ma60 * 1.05 || last < ma60 * 0.80 {
        return None;
    }
    // 60 日波幅 < 30%
    let high60 = indicators::highest(klines, 60)?;
    let low60 = indicators::lowest(klines, 60)?;
    if (high60 - low60) / low60 > 0.30 {
        return None;
    }
    Some(last)
}

fn detect_value_long(klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 250 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    let ma120 = indicators::sma(&cs, 120)?;
    if ma120.is_nan() {
        return None;
    }
    // 价格在 MA120 附近（-30% ~ +10%），允许更大基本面偏离
    if last > ma120 * 1.10 || last < ma120 * 0.70 {
        return None;
    }
    // 长期低波幅：120 日波幅 < 40%
    let high120 = indicators::highest(klines, 120)?;
    let low120 = indicators::lowest(klines, 120)?;
    if (high120 - low120) / low120 > 0.40 {
        return None;
    }
    Some(last)
}

// ── Capital 策略 K 线代理 ──
//
// 实际 Capital 策略依赖北向持仓/主力净流入/龙虎榜资金流数据。
// 此处用量价配合近似：放量上涨 + 成交额放大反映资金介入。

fn detect_capital_short(klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 30 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    // 放量：当日成交额 > 20 日均值 1.5 倍
    let avg_amt = indicators::avg_amount_n(klines, 20)?;
    let today_amt = klines.last()?.amount;
    if today_amt < avg_amt * 1.5 {
        return None;
    }
    // 上涨：当日收盘价 > 前日收盘价 * 1.01（至少 1% 涨幅确认）
    let prev = cs[cs.len() - 2];
    if last < prev * 1.01 {
        return None;
    }
    // 不极度追高：涨幅 < 10%
    if last > prev * 1.10 {
        return None;
    }
    Some(last)
}

fn detect_capital_mid(klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 100 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    // 中期放量：20 日均成交额 > 60 日均值
    let avg_amt_20 = indicators::avg_amount_n(klines, 20)?;
    let avg_amt_60 = indicators::avg_amount_n(klines, 60)?;
    if avg_amt_60 <= 0.0 || avg_amt_20 < avg_amt_60 * 1.2 {
        return None;
    }
    // 趋势向上：MA20 > MA60（资金驱动的中期上涨）
    let ma20 = indicators::sma(&cs, 20)?;
    let ma60 = indicators::sma(&cs, 60)?;
    if last < ma20 || ma20 < ma60 * 0.95 {
        return None;
    }
    // RSI 非超买 (< 70)
    let rsi30 = indicators::rsi(klines, 30)?;
    if rsi30 > 70.0 {
        return None;
    }
    Some(last)
}

fn detect_capital_long(klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 250 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    // 长期放量：60 日均成交额 > 120 日均值
    let avg_amt_60 = indicators::avg_amount_n(klines, 60)?;
    let avg_amt_120 = indicators::avg_amount_n(klines, 120)?;
    if avg_amt_120 <= 0.0 || avg_amt_60 < avg_amt_120 * 1.1 {
        return None;
    }
    // 持续上升趋势
    let ma120 = indicators::sma(&cs, 120)?;
    let ma60 = indicators::sma(&cs, 60)?;
    if ma120.is_nan() || ma60 < ma120 {
        return None;
    }
    // RSI 非过热 (< 65)
    let rsi60 = indicators::rsi(klines, 60)?;
    if rsi60 > 65.0 {
        return None;
    }
    Some(last)
}

fn detect_value_ultra_short(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    let cs = closes(klines);
    let last = *cs.last()?;
    if cs.len() < 15 {
        return None;
    }
    // 超短线价值回归代理：RSI超卖 + 价格低于均线 + 低波幅
    let rsi6 = indicators::rsi(klines, 6)?;
    if rsi6 > 35.0 {
        return None;
    } // RSI 超卖区
    let ma5 = indicators::sma(&cs, 5)?;
    let ma10 = indicators::sma(&cs, 10)?;
    if last > ma5.min(ma10) * read_f64(vars, "value_ultra_short_entry_threshold", 0.998) {
        return None;
    } // 价格低于短均
      // 低波幅确认（不是单纯暴跌）
    let high5 = klines
        .iter()
        .rev()
        .take(5)
        .map(|k| k.high)
        .fold(0.0_f64, f64::max);
    let low5 = klines
        .iter()
        .rev()
        .take(5)
        .map(|k| k.low)
        .fold(f64::MAX, f64::min);
    if high5 - low5 > low5 * 0.08 {
        return None;
    } // 5日振幅 > 8% 则跳过（太剧烈）
    Some(last)
}

fn detect_capital_ultra_short(klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    let cs = closes(klines);
    let last = *cs.last()?;
    if cs.len() < 5 {
        return None;
    }
    // 超短线资金驱动：放量 + 上涨
    let avg_amt_5 = indicators::avg_amount_n(klines, 5)?;
    let today_amt = klines.last()?.amount;
    if avg_amt_5 <= 0.0 || today_amt < avg_amt_5 * 1.5 {
        return None;
    }
    let ma3 = indicators::sma(&cs, 3)?;
    if last < ma3 {
        return None;
    }
    Some(last)
}

// ── 策略注册表 ──

#[allow(dead_code)]
/// Read a f64 variable from vars with fallback default
fn read_f64(vars: &serde_json::Value, name: &str, default: f64) -> f64 {
    vars.get(name).and_then(|v| v.as_f64()).unwrap_or(default)
}

pub(crate) struct StratDef {
    id: &'static str,
    style: &'static str,
    period: &'static str,
    period_enum: Period,
    warmup: usize,
    detect: fn(&[KLine], &serde_json::Value) -> Option<f64>,
}

const STRATS: &[StratDef] = &[
    StratDef {
        id: "trend_ultra_short",
        style: "trend",
        period: "ultra_short",
        period_enum: Period::UltraShort,
        warmup: 5,
        detect: detect_trend_ultra_short,
    },
    StratDef {
        id: "trend_short",
        style: "trend",
        period: "short",
        period_enum: Period::Short,
        warmup: 30,
        detect: detect_trend_short,
    },
    StratDef {
        id: "trend_mid",
        style: "trend",
        period: "mid",
        period_enum: Period::Mid,
        warmup: 100,
        detect: detect_trend_mid,
    },
    StratDef {
        id: "trend_long",
        style: "trend",
        period: "long",
        period_enum: Period::Long,
        warmup: 280,
        detect: detect_trend_long,
    },
    StratDef {
        id: "rev_short",
        style: "reversion",
        period: "short",
        period_enum: Period::Short,
        warmup: 30,
        detect: detect_reversion_short,
    },
    StratDef {
        id: "rev_mid",
        style: "reversion",
        period: "mid",
        period_enum: Period::Mid,
        warmup: 100,
        detect: detect_reversion_mid,
    },
    // ── Value（K 线代理） ──
    StratDef {
        id: "value_short",
        style: "value",
        period: "short",
        period_enum: Period::Short,
        warmup: 30,
        detect: detect_value_short,
    },
    StratDef {
        id: "value_mid",
        style: "value",
        period: "mid",
        period_enum: Period::Mid,
        warmup: 100,
        detect: detect_value_mid,
    },
    StratDef {
        id: "value_long",
        style: "value",
        period: "long",
        period_enum: Period::Long,
        warmup: 250,
        detect: detect_value_long,
    },
    StratDef {
        id: "value_ultra_short",
        style: "value",
        period: "ultra_short",
        period_enum: Period::UltraShort,
        warmup: 5,
        detect: detect_value_ultra_short,
    },
    // ── Capital（K 线代理） ──
    StratDef {
        id: "capital_short",
        style: "capital",
        period: "short",
        period_enum: Period::Short,
        warmup: 30,
        detect: detect_capital_short,
    },
    StratDef {
        id: "capital_mid",
        style: "capital",
        period: "mid",
        period_enum: Period::Mid,
        warmup: 100,
        detect: detect_capital_mid,
    },
    StratDef {
        id: "capital_long",
        style: "capital",
        period: "long",
        period_enum: Period::Long,
        warmup: 250,
        detect: detect_capital_long,
    },
    StratDef {
        id: "capital_ultra_short",
        style: "capital",
        period: "ultra_short",
        period_enum: Period::UltraShort,
        warmup: 5,
        detect: detect_capital_ultra_short,
    },
];

pub const SKIPPED: &[&str] = &["watchlist_*: 兜底策略无信号逻辑，不计入回测"];

// ── 策略查找与单策略信号历史 ──

/// 按 strategy_id 查找策略定义（用于 RecoSignalTimeline）
pub(crate) fn get_strategy_def(sid: &str) -> Option<&'static StratDef> {
    STRATS.iter().find(|s| s.id == sid)
}

/// 对指定策略跑所有股票的单笔信号历史（不聚合，返回每条信号明细）
///
/// `stock_codes` 可选过滤：传入非空列表时只分析这些股票；None 时从 reco_picks 种子池读。
pub async fn run_signal_history(
    client: Arc<AStockClient>,
    sid: &str,
    stock_codes: Option<&[(String, String)]>,
) -> Result<Vec<StrategySignalResult>, String> {
    let strat = get_strategy_def(sid).ok_or_else(|| format!("未知策略: {}", sid))?;
    let kline_limit = 500u32;
    let holding = strat.period_enum.default_holding_days();

    let stocks = match stock_codes {
        Some(list) => list.to_vec(),
        None => return Ok(vec![]), // 调用方应自行提供股票列表
    };

    let mut results = Vec::new();
    for (code, name) in &stocks {
        let klines = match client.get_klines(code, "daily", kline_limit).await {
            Ok(k) if k.len() >= strat.warmup => k,
            _ => continue,
        };
        let sigs = scan_one(
            &klines,
            code,
            name,
            sid,
            strat.detect,
            holding,
            strat.warmup,
            &serde_json::Value::Null,
        );
        results.extend(sigs);
    }
    results.sort_by(|a, b| b.signal_date.cmp(&a.signal_date));
    Ok(results)
}

// ── 主入口 ──

/// 对正/负两组分别跑策略信号回测
///
/// `stock_codes`: `[(code, name)]`
pub async fn backtest_two_groups(
    client: Arc<AStockClient>,
    positive_stocks: &[(String, String)],
    negative_stocks: &[(String, String)],
) -> Result<BacktestComparisonResponse, String> {
    let pos_result = run_group(client.clone(), "推荐命中", positive_stocks).await;
    let neg_result = run_group(client.clone(), "候选池未中", negative_stocks).await;

    Ok(BacktestComparisonResponse {
        positive: pos_result,
        negative: neg_result,
        positive_stocks: positive_stocks.iter().map(|(c, _)| c.clone()).collect(),
        negative_stocks: negative_stocks.iter().map(|(c, _)| c.clone()).collect(),
        skipped: SKIPPED.iter().map(|s| s.to_string()).collect(),
    })
}

async fn run_group(
    client: Arc<AStockClient>,
    label: &str,
    stocks: &[(String, String)],
) -> GroupBacktestResult {
    let kline_limit = 500u32;

    // 加载所有 K 线
    struct StockWithKlines {
        code: String,
        name: String,
        klines: Vec<KLine>,
    }
    let mut loaded = Vec::new();
    for (code, name) in stocks {
        match client.get_klines(code, "daily", kline_limit).await {
            Ok(k) if k.len() >= 60 => loaded.push(StockWithKlines {
                code: code.clone(),
                name: name.clone(),
                klines: k,
            }),
            _ => {},
        }
    }

    // 逐策略扫描
    let mut results = HashMap::new();
    for strat in STRATS {
        let mut all_sigs = Vec::new();
        let holding = strat.period_enum.default_holding_days();
        for s in &loaded {
            let sigs = scan_one(
                &s.klines,
                &s.code,
                &s.name,
                strat.id,
                strat.detect,
                holding,
                strat.warmup,
                &serde_json::Value::Null,
            );
            all_sigs.extend(sigs);
        }
        results.insert(strat.id.to_string(), all_sigs);
    }

    // 聚合
    let mut strategies = HashMap::new();
    for strat in STRATS {
        let sigs = results.get(strat.id).map_or(&[] as &[_], |v| v.as_slice());
        if sigs.is_empty() {
            continue;
        }
        strategies
            .insert(strat.id.to_string(), aggregate(strat.id, strat.style, strat.period, sigs));
    }

    GroupBacktestResult {
        label: label.to_string(),
        stock_count: loaded.len() as u32,
        strategies,
    }
}

// ── 滑动窗口扫描 ──

#[allow(clippy::too_many_arguments)]
fn scan_one(
    klines: &[KLine],
    code: &str,
    name: &str,
    sid: &str,
    detect: fn(&[KLine], &serde_json::Value) -> Option<f64>,
    holding: u32,
    warmup: usize,
    vars_ref: &serde_json::Value,
) -> Vec<StrategySignalResult> {
    let max_idx = klines.len().saturating_sub(holding as usize + 1);
    let mut out = Vec::new();
    // cooldown_index: 上次信号触发的位置 + holding，此期间不产生新信号
    // 避免滑动窗口在同一持仓期内产生多个重叠信号（高估可执行信号频率）
    let mut cooldown_index: usize = 0;
    for i in warmup..max_idx {
        if i <= cooldown_index {
            continue;
        }
        let window = &klines[..=i];
        if let Some(entry) = detect(window, vars_ref) {
            let exit_idx = (i + holding as usize).min(klines.len() - 1);
            let exit_price = klines[exit_idx].close;
            let mut peak = 0.0_f64;
            let mut max_dd = 0.0;
            for k in &klines[i..=exit_idx] {
                if k.close > peak {
                    peak = k.close;
                }
                if peak > 0.0 {
                    let dd = (peak - k.close) / peak;
                    if dd > max_dd {
                        max_dd = dd;
                    }
                }
            }
            let ret = if entry > 0.0 {
                ((exit_price - entry) / entry) * 100.0
            } else {
                0.0
            };
            out.push(StrategySignalResult {
                strategy_id: sid.into(),
                stock_code: code.into(),
                stock_name: name.into(),
                signal_date: klines[i].date.clone(),
                entry_price: entry,
                exit_price,
                holding_days: holding,
                return_pct: ret,
                was_profitable: ret > 0.0,
                max_drawdown_pct: max_dd * 100.0,
            });
            // 设置冷却指数：本次信号触发+持有期内不产生新信号
            cooldown_index = i + holding as usize;
        }
    }
    out
}

// ── 聚合统计 ──

fn aggregate(sid: &str, style: &str, period: &str, sigs: &[StrategySignalResult]) -> StrategyStats {
    let total = sigs.len() as u32;
    let wins = sigs.iter().filter(|s| s.was_profitable).count() as u32;
    let losses = total - wins;
    let (wr, avg_ret, total_ret, avg_dd) = if total > 0 {
        let s_ret: f64 = sigs.iter().map(|s| s.return_pct).sum();
        let s_dd: f64 = sigs.iter().map(|s| s.max_drawdown_pct).sum();
        (
            wins as f64 / total as f64 * 100.0,
            s_ret / total as f64,
            s_ret,
            s_dd / total as f64,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    let mut streak = 0u32;
    let mut max_streak = 0u32;
    for s in sigs {
        if s.was_profitable {
            streak = 0;
        } else {
            streak += 1;
            if streak > max_streak {
                max_streak = streak;
            }
        }
    }

    // 夏普比率（已修正）：
    //   Sharpe = (E[R] - Rf) / σ(R)
    //   其中 E[R] = 信号平均单笔收益率
    //         Rf  = 0 (假设无风险收益率为 0，对回测信号序列更合理)
    //         σ   = 样本收益率标准差
    //   注意：这是"信号级"夏普，非年化。信号周期不同时不能直接对比年化夏普。
    let sharpe = if total > 1 {
        let returns: Vec<f64> = sigs.iter().map(|s| s.return_pct / 100.0).collect();
        let n = returns.len() as f64;
        let avg_r = returns.iter().sum::<f64>() / n;
        let var = returns.iter().map(|r| (r - avg_r).powi(2)).sum::<f64>() / (n - 1.0);
        if var > 0.0 {
            let std = var.sqrt();
            Some(avg_r / std) // 信号级夏普，无风险利率=0
        } else {
            None
        }
    } else {
        None
    };

    let pf = if total > 0 && losses > 0 {
        let tw: f64 = sigs
            .iter()
            .filter(|s| s.was_profitable)
            .map(|s| s.return_pct.abs())
            .sum();
        let tl: f64 = sigs
            .iter()
            .filter(|s| !s.was_profitable)
            .map(|s| s.return_pct.abs())
            .sum();
        if tl > 0.0 && tw > 0.0 {
            Some(tw / tl)
        } else {
            None
        }
    } else {
        None
    };

    StrategyStats {
        strategy_id: sid.into(),
        style: style.into(),
        period: period.into(),
        total_signals: total,
        win_count: wins,
        loss_count: losses,
        win_rate_pct: wr,
        avg_return_pct: avg_ret,
        total_return_pct: total_ret,
        avg_max_drawdown_pct: avg_dd,
        max_consecutive_losses: max_streak,
        sharpe_ratio: sharpe,
        profit_factor: pf,
    }
}

// ── 测试 ──

// ── 权重自动调整 ──

/// 根据正向组回测统计自动计算策略权重
///
/// `existing_weights` 可选：传入当前模板中已有的权重，权重为 0.0 的策略将被跳过（用户禁用保护）。
/// 规则（基于置信度校准 + 夏普修正）：
///   win_rate >= 55% → 1.0 + (wr-50)/100 × 0.5  （60%→1.05, 70%→1.10）
///   win_rate 45-55% → 1.0 （维持不变）
///   win_rate < 45%  → 1.0 - (50-wr)/100 × 0.8  （40%→0.92, 30%→0.84）
///   额外：Shapre < 0.3 → -0.05（收益不稳降权）
///   上下限：[0.5, 1.5]
pub fn adjust_strategy_weights(
    strategies: &HashMap<String, StrategyStats>,
    existing_weights: Option<&BTreeMap<String, f64>>,
) -> Result<BTreeMap<String, f64>, String> {
    let mut weights: BTreeMap<String, f64> = BTreeMap::new();
    for (sid, stats) in strategies {
        if stats.total_signals < 5 {
            continue; // 样本太少，不调整
        }
        // 用户禁用保护：权重为 0.0 的策略跳过（用户已手动禁用的不自动恢复）
        if let Some(existing) = existing_weights {
            if existing.get(sid) == Some(&0.0) {
                weights.insert(sid.clone(), 0.0);
                continue;
            }
        }
        let offset = if stats.win_rate_pct >= 55.0 {
            (stats.win_rate_pct - 50.0) / 100.0 * 0.5
        } else if stats.win_rate_pct >= 45.0 {
            0.0
        } else {
            -(50.0 - stats.win_rate_pct) / 100.0 * 0.8
        };
        let extra_penalty = stats
            .sharpe_ratio
            .map_or(0.0, |s| if s < 0.3 { -0.05 } else { 0.0 });
        let weight = (1.0 + offset + extra_penalty).clamp(0.5, 1.5);
        // 保留两位小数
        weights.insert(sid.clone(), (weight * 100.0).round() / 100.0);
    }
    if weights.is_empty() {
        return Ok(weights); // 无策略可调整时返回空 map，非错误
    }
    Ok(weights)
}

// ── 信号质量缓存（回测历史胜率 → 推荐置信度校准） ──
// 回测系统已经计算了每个策略的 win_rate/avg_return/sharpe。
// 这些统计值不应只用于权重调整，还应直接反馈到单次信号的质量判断上。
// 例如：trend_short 历史胜率 38% → 新产生的信号置信度应下调。

use std::sync::LazyLock;

/// 信号质量快照：每个策略的历史统计表现
#[derive(Debug, Clone)]
pub struct SignalQualityStats {
    pub strategy_id: String,
    pub period: String,
    pub total_signals: u32,
    pub win_rate_pct: f64,
    pub avg_return_pct: f64,
    pub last_updated: u64, // Unix timestamp ms
}

/// 全局信号质量缓存（按 (strategy_id, as_of_suffix) 索引，live/replay 隔离）
static SIGNAL_QUALITY_CACHE: LazyLock<RwLock<HashMap<(String, String), SignalQualityStats>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 从回测 groups 结果更新信号质量缓存（自动注入 as-of 后缀隔离 live/replay）
pub fn update_signal_quality_cache(positive_stats: &HashMap<String, StrategyStats>) {
    let suffix = axagent_astock_data::as_of::cache_suffix();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let mut cache = SIGNAL_QUALITY_CACHE
        .write()
        .unwrap_or_else(|e| e.into_inner());
    for (sid, stats) in positive_stats {
        if stats.total_signals < 5 {
            continue;
        }
        cache.insert(
            (sid.clone(), suffix.clone()),
            SignalQualityStats {
                strategy_id: sid.clone(),
                period: stats.period.clone(),
                total_signals: stats.total_signals,
                win_rate_pct: stats.win_rate_pct,
                avg_return_pct: stats.avg_return_pct,
                last_updated: now,
            },
        );
    }
}

/// 查询策略信号质量（自动注入 as-of 后缀隔离 live/replay）
pub fn get_signal_quality(strategy_id: &str) -> Option<SignalQualityStats> {
    let suffix = axagent_astock_data::as_of::cache_suffix();
    let cache = SIGNAL_QUALITY_CACHE
        .read()
        .unwrap_or_else(|e| e.into_inner());
    cache.get(&(strategy_id.to_string(), suffix)).cloned()
}

/// 信号质量调整系数：将历史胜率映射到 [0.7, 1.3] 的乘数
/// - win_rate >= 55% → 乘数 ≥ 1.0（提升置信度）
/// - win_rate 45-55% → 乘数 ≈ 1.0（不调整）
/// - win_rate < 45%  → 乘数 ≤ 1.0（压低置信度）
/// - 无数据（缓存未命中）→ 乘数 = 1.0（不调整）
pub fn signal_quality_multiplier(strategy_id: &str) -> f64 {
    match get_signal_quality(strategy_id) {
        Some(q) if q.total_signals >= 5 => {
            let factor = q.win_rate_pct / 50.0; // 50% 为基线
            factor.clamp(0.7, 1.3).max(0.4) // 极端保护
        },
        _ => 1.0, // 无数据 → 不调整
    }
}

/// 贝叶斯收缩版信号质量：返回 (收缩后胜率, 样本数, 先验胜率)
///
/// 先验胜率基于市场环境：
///   - "bull" → 0.55（牛市中随机持有多头的胜率偏高）
///   - "bear" → 0.45（熊市中随机持有多头的胜率偏低）
///   - _      → 0.50（拉普拉斯无差别先验）
///
/// 贝叶斯后验公式：
///   posterior = (prior_weight × prior + n × sample_win_rate) / (prior_weight + n)
///
/// 其中 prior_weight=20 为"虚拟样本量"，代表我们对先验的信赖程度。
/// 信号数<20 时后验显著向先验收缩；信号数>200 时后验接近原始胜率。
pub fn bayesian_signal_quality(strategy_id: &str, market_regime: &str) -> (f64, u32, f64) {
    let prior = match market_regime {
        "bull" => 0.55,
        "bear" => 0.45,
        _ => 0.50,
    };
    const PRIOR_WEIGHT: f64 = 20.0;

    match get_signal_quality(strategy_id) {
        Some(q) if q.total_signals >= 5 => {
            let n = q.total_signals as f64;
            let posterior =
                (PRIOR_WEIGHT * prior + n * q.win_rate_pct / 100.0) / (PRIOR_WEIGHT + n);
            (posterior, q.total_signals, prior)
        },
        _ => (prior, 0, prior), // 无数据 → 只用先验
    }
}

/// 反身性系数：根据风险等级和拥挤度调整信号权重
///
/// - 低风险（β<0.8）：乘数 1.1（防御性标的，信号可靠度提升）
/// - 中风险（β~1.0）：乘数 1.0（市场平均）
/// - 高风险（β>1.2）：乘数 0.85（高波动下的信号噪声大）
/// - 极高（β>2.0）：乘数 0.6（极高博弈性，反转风险大）
///
/// 原理：高拥挤/高波动环境下，技术信号的反身性效应显著——
///   信号本身改变了市场参与者的行为，使信号的可预测性下降。
pub fn reflexivity_discount(risk_level: &str) -> f64 {
    if risk_level.contains("极高") {
        0.60
    } else if risk_level.contains("高风险") || risk_level.contains("高") {
        0.85
    } else if risk_level.contains("低风险") || risk_level.contains("低") {
        1.10
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(c: f64, d: &str, a: f64) -> KLine {
        KLine {
            date: d.into(),
            open: c * 0.99,
            high: c * 1.02,
            low: c * 0.98,
            close: c,
            volume: 1_000_000.0,
            amount: a,
            turnover_rate: Some(1.0),
            adj_factor: None,
        }
    }

    #[test]
    fn momentum_strategies_no_signal_on_flat() {
        let klines: Vec<KLine> = (0..50)
            .map(|i| k(10.0, &format!("d{}", i % 28 + 1), 10_000_000.0))
            .collect();
        assert!(detect_trend_short(&klines, &serde_json::Value::Null).is_none());
        assert!(detect_reversion_short(&klines, &serde_json::Value::Null).is_none());
        // 资金策略需要放量，平盘不放量也为 None
        assert!(detect_capital_short(&klines, &serde_json::Value::Null).is_none());
    }

    #[test]
    fn value_strategy_signals_on_flat() {
        let klines: Vec<KLine> = (0..50)
            .map(|i| k(10.0, &format!("d{}", i % 28 + 1), 10_000_000.0))
            .collect();
        // 平盘 = 低波幅+均线附近 = 价值股典型 K 线特征
        assert!(detect_value_short(&klines, &serde_json::Value::Null).is_some());
    }

    #[test]
    fn capital_needs_volume_spike() {
        // 第 1 部分：平盘无放量 → None
        let flat: Vec<KLine> = (0..50)
            .map(|i| k(10.0, &format!("d{}", i % 28 + 1), 10_000_000.0))
            .collect();
        assert!(detect_capital_short(&flat, &serde_json::Value::Null).is_none());

        // 第 2 部分：最后一日放量 + 上涨 → 有信号
        let mut spike = flat.clone();
        spike.push(KLine {
            date: "d100".into(),
            open: 10.1,
            high: 10.4,
            low: 10.0,
            close: 10.3,
            volume: 3_000_000.0,
            amount: 31_000_000.0,
            turnover_rate: Some(3.0),
            adj_factor: None,
        });
        assert!(detect_capital_short(&spike, &serde_json::Value::Null).is_some());
    }

    #[test]
    fn aggregate_empty() {
        let s = aggregate("test", "test", "test", &[]);
        assert_eq!(s.total_signals, 0);
    }

    #[test]
    fn adjust_weights_high_winrate() {
        let mut strategies = HashMap::new();
        strategies.insert(
            "trend_short".into(),
            StrategyStats {
                strategy_id: "trend_short".into(),
                style: "trend".into(),
                period: "short".into(),
                total_signals: 20,
                win_count: 14,
                loss_count: 6,
                win_rate_pct: 70.0,
                avg_return_pct: 3.5,
                total_return_pct: 70.0,
                avg_max_drawdown_pct: 5.0,
                max_consecutive_losses: 2,
                sharpe_ratio: Some(1.2),
                profit_factor: Some(2.3),
            },
        );
        let result = adjust_strategy_weights(&strategies, None).unwrap();
        // wr=70% → 偏移 (70-50)/100*0.5=0.10 → 1.10；Sharpe>0.3 无惩罚
        let w = result.get("trend_short").copied().unwrap_or(0.0);
        assert!((w - 1.10).abs() < 0.01, "trend_short weight expected 1.10 got {w}");
    }

    #[test]
    fn adjust_weights_low_winrate() {
        let mut strategies = HashMap::new();
        strategies.insert(
            "capital_long".into(),
            StrategyStats {
                strategy_id: "capital_long".into(),
                style: "capital".into(),
                period: "long".into(),
                total_signals: 15,
                win_count: 5,
                loss_count: 10,
                win_rate_pct: 33.3,
                avg_return_pct: -2.0,
                total_return_pct: -30.0,
                avg_max_drawdown_pct: 12.0,
                max_consecutive_losses: 5,
                sharpe_ratio: Some(0.1),
                profit_factor: Some(0.6),
            },
        );
        let result = adjust_strategy_weights(&strategies, None).unwrap();
        // wr=33.3% → 偏移 -(50-33.3)/100*0.8=-0.134 → 0.866
        // Sharpe<0.3 → -0.05 → 0.8167
        let w = result.get("capital_long").copied().unwrap_or(0.0);
        assert!((w - 0.82).abs() < 0.02, "capital_long weight expected ~0.82 got {w}");
    }

    #[test]
    fn adjust_weights_insufficient_samples() {
        let mut strategies = HashMap::new();
        strategies.insert(
            "trend_short".into(),
            StrategyStats {
                strategy_id: "trend_short".into(),
                style: "trend".into(),
                period: "short".into(),
                total_signals: 3, // < 5，不满足最小样本
                win_count: 3,
                loss_count: 0,
                win_rate_pct: 100.0,
                avg_return_pct: 10.0,
                total_return_pct: 30.0,
                avg_max_drawdown_pct: 2.0,
                max_consecutive_losses: 0,
                sharpe_ratio: Some(3.0),
                profit_factor: Some(999.0),
            },
        );
        let result = adjust_strategy_weights(&strategies, None).unwrap();
        // 样本不足 5 → 跳过，返回空 map
        assert!(result.is_empty(), "应该跳过不足 5 个信号的策略");
    }

    #[test]
    fn adjust_weights_preserves_user_disabled() {
        let mut strategies = HashMap::new();
        strategies.insert(
            "value_long".into(),
            StrategyStats {
                strategy_id: "value_long".into(),
                style: "value".into(),
                period: "long".into(),
                total_signals: 20,
                win_count: 16,
                loss_count: 4,
                win_rate_pct: 80.0,
                avg_return_pct: 5.0,
                total_return_pct: 100.0,
                avg_max_drawdown_pct: 4.0,
                max_consecutive_losses: 1,
                sharpe_ratio: Some(1.5),
                profit_factor: Some(3.0),
            },
        );
        let mut existing = BTreeMap::new();
        existing.insert("value_long".into(), 0.0); // 用户已禁用
        let result = adjust_strategy_weights(&strategies, Some(&existing)).unwrap();
        // 用户禁用策略即使胜率高也应保持 0.0
        let w = result.get("value_long").copied().unwrap_or(-1.0);
        assert_eq!(w, 0.0, "用户禁用的策略 weight 必须保持 0.0");
    }
}
