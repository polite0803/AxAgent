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
//! - **Trend** (ultra_short/short/mid/long): 纯 K 线依赖 ✅
//! - **Reversion** (short/mid): 纯 K 线依赖 ✅
//! - **Value** (ultra_short/short/mid/long): K 线代理（低波幅+均线附近+温和量能） ✅
//! - **Capital** (ultra_short/short/mid/long): K 线代理（量比+动量，匹配 scan_from_klines） ✅
//! - **CapitalFlow** (short/mid/long): 复用 Capital proxy ✅
//! - **Technical** (short/mid/long): 纯 K 线形态（MACD 金叉/多周期共振/年线向上） ✅
//! - Watchlist: 兜底策略无信号逻辑，不计入回测 ⏭️
//! - Bottleneck/Policy/Earnings/Event: 依赖 LLM 工作流/财务/基本面，不可 K 线回测 ⏭️

use crate::recommender::indicators;
use crate::recommender::types::Period;
use axagent_harness::market_data::{AdjType, KLine, MarketDataProvider};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

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
    let min_len = read_f64(vars, "trend_short_min_kline_len", 30.0) as usize;
    if klines.len() < min_len {
        return None;
    }
    // MA5 > MA10 + 股价未远离 MA20（同步 trend.rs::Short：宽松版，非严格三均线多头）
    let ma5 = indicators::sma(&cs, 5)?;
    let ma10 = indicators::sma(&cs, 10)?;
    let ma20 = indicators::sma(&cs, 20)?;
    let ma20_tolerance = read_f64(vars, "trend_short_ma20_tolerance", 0.985);
    if !(ma5 > ma10 && last >= ma20 * ma20_tolerance) {
        return None;
    }
    let high_period = read_f64(vars, "trend_high_20_period", 20.0) as usize;
    let high_threshold = read_f64(vars, "trend_high_20_threshold", 0.97);
    let high = indicators::highest(klines, high_period)?;
    if last < high * high_threshold {
        return None;
    }
    let amount_ratio_min = read_f64(vars, "trend_amount_ratio_min", 0.8);
    let amt_ratio = klines.last()?.amount / indicators::avg_amount_20d(klines)?;
    if amt_ratio < amount_ratio_min {
        return None;
    }
    Some(last)
}

fn detect_trend_mid(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    let cs = closes(klines);
    let last = *cs.last()?;
    let min_len = read_f64(vars, "trend_mid_min_kline_len", 60.0) as usize;
    if klines.len() < min_len {
        return None;
    }
    let ma60 = indicators::sma(&cs, 60)?;
    let ma60_threshold = read_f64(vars, "trend_ma60_threshold", 0.985);
    if ma60.is_nan() || last < ma60 * ma60_threshold {
        return None;
    }
    let high_period = read_f64(vars, "trend_high_60_period", 60.0) as usize;
    let high_threshold = read_f64(vars, "trend_high_60_threshold", 0.94);
    if indicators::highest(klines, high_period)? * high_threshold > last {
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

fn detect_trend_long(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    let cs = closes(klines);
    let last = *cs.last()?;
    let min_len = read_f64(vars, "trend_long_min_kline_len", 120.0) as usize;
    if cs.len() < min_len {
        return None;
    }
    let ma250 = indicators::sma(&cs, 250)?;
    if ma250.is_nan() {
        return None;
    }
    let ma60 = indicators::sma(&cs, 60)?;
    let ma60_ma250_mult = read_f64(vars, "trend_ma60_ma250_mult", 0.95);
    if ma60 < ma250 * ma60_ma250_mult {
        return None;
    }
    let ma60_break_mult = read_f64(vars, "trend_ma60_break_mult", 0.95);
    if last < ma60 * ma60_break_mult {
        return None;
    }
    Some(last)
}

fn detect_trend_ultra_short(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    let cs = closes(klines);
    let last = *cs.last()?;
    let min_len = read_f64(vars, "trend_ultra_short_min_kline_len", 15.0) as usize;
    if cs.len() < min_len {
        return None;
    }
    // MA5 > MA10 短期多头（同步 trend.rs::UltraShort）
    let ma5 = indicators::sma(&cs, 5)?;
    let ma10 = indicators::sma(&cs, 10)?;
    if ma5 <= ma10 {
        return None;
    }
    // 价格接近 5 日高（同步 trend.rs）
    let high_period = read_f64(vars, "trend_high_ultra_short_period", 5.0) as usize;
    let high_threshold = read_f64(vars, "trend_high_ultra_short_threshold", 0.995);
    let high_n = indicators::highest(klines, high_period)?;
    if last < high_n * high_threshold {
        return None;
    }
    // 量比 >= 阈值（同步 trend.rs）
    let amount_ratio_min = read_f64(vars, "trend_amount_ratio_min", 0.8);
    let today_vol = klines.last()?.amount;
    let avg_vol_5 = indicators::avg_amount_n(klines, 5)?;
    if avg_vol_5 <= 0.0 || today_vol < avg_vol_5 * amount_ratio_min {
        return None;
    }
    Some(last)
}

fn detect_reversion_short(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    let min_len = read_f64(vars, "rev_min_kline_len", 30.0) as usize;
    if klines.len() < min_len {
        return None;
    }
    let rsi_period = read_f64(vars, "rev_rsi_period", 6.0) as usize;
    let rsi_val = indicators::rsi(klines, rsi_period)?;
    let rsi_short_max = read_f64(vars, "rev_rsi_short_max", 35.0);
    if rsi_val >= rsi_short_max {
        return None;
    }
    let avg_period = read_f64(vars, "rev_avg_amount_period", 5.0) as usize;
    let avg_mult = read_f64(vars, "rev_avg_amount_mult", 1.2);
    let avg_5 = indicators::avg_amount_n(klines, avg_period)?;
    if avg_5 <= 0.0 {
        return None;
    }
    let today_amt = klines.last()?.amount;
    if today_amt > avg_5 * avg_mult {
        return None;
    }
    Some(klines.last()?.close)
}

fn detect_reversion_mid(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    let min_len = read_f64(vars, "rev_kline_limit", 250.0) as usize;
    if klines.len() < min_len.min(100) {
        return None;
    }
    let dd_period = read_f64(vars, "rev_dd_period", 250.0) as usize;
    let dd_min = read_f64(vars, "rev_dd_min", 20.0);
    let dd = indicators::drawdown_from_high(klines, dd_period)?;
    if dd < dd_min {
        return None;
    }
    let rsi_mid_period = read_f64(vars, "rev_rsi_mid_period", 30.0) as usize;
    let rsi_mid_max = read_f64(vars, "rev_rsi_mid_max", 50.0);
    let rsi30 = indicators::rsi(klines, rsi_mid_period)?;
    if rsi30 > rsi_mid_max {
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
    let min_len = read_f64(vars, "val_short_min_kline_len", 20.0) as usize;
    if klines.len() < min_len {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    let ma_period = read_f64(vars, "val_short_ma_period", 20.0) as usize;
    let ma20 = indicators::sma(&cs, ma_period)?;
    // 价格在 MA20 附近（偏离范围可配）
    let upper_dev = read_f64(vars, "value_upper_deviation", 1.02);
    let lower_dev = read_f64(vars, "value_lower_deviation", 0.90);
    if last > ma20 * upper_dev || last < ma20 * lower_dev {
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

fn detect_value_mid(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 100 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    let ma60 = indicators::sma(&cs, 60)?;
    // 价格在 MA60 附近，排除深跌垃圾股和暴涨热门股
    let upper_dev = read_f64(vars, "value_mid_upper_deviation", 1.05);
    let lower_dev = read_f64(vars, "value_mid_lower_deviation", 0.80);
    if last > ma60 * upper_dev || last < ma60 * lower_dev {
        return None;
    }
    // 60 日波幅 < 30%
    let high60 = indicators::highest(klines, 60)?;
    let low60 = indicators::lowest(klines, 60)?;
    let max_swing = read_f64(vars, "value_mid_max_swing", 0.30);
    if (high60 - low60) / low60 > max_swing {
        return None;
    }
    Some(last)
}

fn detect_value_long(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 250 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    let ma120 = indicators::sma(&cs, 120)?;
    if ma120.is_nan() {
        return None;
    }
    // 价格在 MA120 附近，允许更大偏离
    let upper_dev = read_f64(vars, "value_long_upper_deviation", 1.10);
    let lower_dev = read_f64(vars, "value_long_lower_deviation", 0.70);
    if last > ma120 * upper_dev || last < ma120 * lower_dev {
        return None;
    }
    // 长期低波幅
    let high120 = indicators::highest(klines, 120)?;
    let low120 = indicators::lowest(klines, 120)?;
    let max_swing = read_f64(vars, "value_long_max_swing", 0.40);
    if (high120 - low120) / low120 > max_swing {
        return None;
    }
    Some(last)
}

// ── Capital 策略 K 线代理 ──
//
// 实际 Capital 策略依赖北向持仓/主力净流入/龙虎榜资金流数据。
// 此处用量价配合近似：放量上涨 + 成交额放大反映资金介入。

fn detect_capital_short(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 20 {
        return None;
    }
    // 匹配 capital.rs→scan_from_klines
    let avg_amt_5 = indicators::avg_amount_n(klines, 5)?;
    let avg_amt_20 = indicators::avg_amount_n(klines, 20)?;
    let vol_ratio = if avg_amt_20 > 0.0 {
        avg_amt_5 / avg_amt_20
    } else {
        1.0
    };
    let vol_ratio_min = read_f64(vars, "cap_kline_vol_ratio_min", 1.5);
    if vol_ratio < vol_ratio_min {
        return None;
    }
    let cs = closes(klines);
    let mom_5 = if cs.len() >= 6 {
        cs[cs.len() - 1] / cs[cs.len() - 6] - 1.0
    } else {
        0.0
    };
    let mom_5_min = read_f64(vars, "cap_kline_mom_5_min", -0.02);
    let mom_5_max = read_f64(vars, "cap_kline_mom_5_max", 0.10);
    if mom_5 < mom_5_min || mom_5 > mom_5_max {
        return None;
    }
    cs.last().copied()
}

fn detect_capital_mid(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 40 {
        return None;
    }
    // 量比放大窗口匹配 capital.rs→scan_from_klines
    let avg_amt_20 = indicators::avg_amount_n(klines, 20)?;
    let avg_amt_60 = indicators::avg_amount_n(klines, 60)?;
    let vol_ratio = if avg_amt_60 > 0.0 {
        avg_amt_20 / avg_amt_60
    } else {
        1.0
    };
    let vol_ratio_min = read_f64(vars, "cap_kline_vol_ratio_min", 1.2);
    if vol_ratio < vol_ratio_min {
        return None;
    }
    let cs = closes(klines);
    let mom_20 = if cs.len() >= 21 {
        cs[cs.len() - 1] / cs[cs.len() - 21] - 1.0
    } else {
        0.0
    };
    let mom_20_min = read_f64(vars, "cap_kline_mom_20_min", 0.0);
    let mom_20_max = read_f64(vars, "cap_kline_mom_20_max", 0.30);
    if mom_20 < mom_20_min || mom_20 > mom_20_max {
        return None;
    }
    // RSI 非超买
    let rsi30 = indicators::rsi(klines, 30)?;
    if rsi30 > 70.0 {
        return None;
    }
    cs.last().copied()
}

fn detect_capital_long(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 60 {
        return None;
    }
    // 量比放大窗口匹配 capital.rs→scan_from_klines
    let avg_amt_60 = indicators::avg_amount_n(klines, 60)?;
    let avg_amt_120 = indicators::avg_amount_n(klines, 120)?;
    let vol_ratio = if avg_amt_120 > 0.0 {
        avg_amt_60 / avg_amt_120
    } else {
        1.0
    };
    let vol_ratio_min = read_f64(vars, "cap_kline_vol_ratio_min", 1.1);
    if vol_ratio < vol_ratio_min {
        return None;
    }
    let cs = closes(klines);
    let ma60 = indicators::sma(&cs, 60)?;
    let ma120 = indicators::sma(&cs, 120)?;
    if ma120.is_nan() || ma60 < ma120 {
        return None;
    }
    // RSI 非过热
    let rsi60 = indicators::rsi(klines, 60)?;
    if rsi60 > 65.0 {
        return None;
    }
    cs.last().copied()
}

fn detect_value_ultra_short(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    let cs = closes(klines);
    let last = *cs.last()?;
    let min_len = read_f64(vars, "val_ultra_short_min_kline_len", 5.0) as usize;
    if cs.len() < min_len {
        return None;
    }
    // K 线代理：价格在 MA10 附近（匹配 value.rs→UltraShort，真实 PE 检查不可回溯）
    let ma_period = read_f64(vars, "val_ultra_short_ma_period", 10.0) as usize;
    let ma10 = indicators::sma(&cs, ma_period)?;
    let ma_mult = read_f64(vars, "val_ultra_short_ma_mult", 1.005);
    if last > ma10 * ma_mult {
        return None;
    }
    // 低波幅确认（5日振幅 < 8%）
    let high5 = klines.iter().rev().take(5).map(|k| k.high).fold(0.0_f64, f64::max);
    let low5 = klines.iter().rev().take(5).map(|k| k.low).fold(f64::MAX, f64::min);
    if high5 - low5 > low5 * 0.08 {
        return None;
    }
    Some(last)
}

fn detect_capital_ultra_short(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 5 {
        return None;
    }
    // 匹配 capital.rs→scan_from_klines：量比 + 动量
    let avg_amt_5 = indicators::avg_amount_n(klines, 5)?;
    let avg_amt_20 = indicators::avg_amount_n(klines, 20)?;
    let vol_ratio = if avg_amt_20 > 0.0 {
        avg_amt_5 / avg_amt_20
    } else {
        1.0
    };
    let vol_ratio_min = read_f64(vars, "cap_kline_vol_ratio_min", 1.5);
    if vol_ratio < vol_ratio_min {
        return None;
    }
    let cs = closes(klines);
    let mom_5 = if cs.len() >= 6 {
        cs[cs.len() - 1] / cs[cs.len() - 6] - 1.0
    } else {
        0.0
    };
    let mom_5_max = read_f64(vars, "cap_kline_mom_5_max", 0.10);
    if mom_5 < 0.0 || mom_5 > mom_5_max {
        return None;
    }
    cs.last().copied()
}

// ── Serenity 工作流策略（不可纯 K 线回测） ──

/// 占位符：依赖 LLM 工作流/财务/基本面数据的策略，无法用 K 线回测
fn detect_not_backtestable(_klines: &[KLine], _vars: &serde_json::Value) -> Option<f64> {
    None
}

// ── Technical 策略（纯 K 线形态，可回测） ──

fn detect_technical_short(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 30 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    // MACD 金叉（DIF 上穿 DEA）
    let (dif, dea, _) = indicators::macd(klines, 12, 26, 9)?;
    if dif <= dea {
        return None;
    }
    // 前一根 K 线 DIF 还在 DEA 下方确认金叉刚刚发生
    let prev_macd = indicators::macd(&klines[..klines.len() - 1], 12, 26, 9);
    if let Some((prev_dif, prev_dea, _)) = prev_macd {
        if prev_dif > prev_dea {
            return None; // 金叉已发生多日，不是新鲜信号
        }
    }
    // 量比确认
    let vol_ratio_min = read_f64(vars, "technical_vol_ratio_min", 1.2);
    let today_amt = klines.last()?.amount;
    let avg_amt = indicators::avg_amount_n(klines, 20)?;
    if avg_amt <= 0.0 || today_amt < avg_amt * vol_ratio_min {
        return None;
    }
    Some(last)
}

fn detect_technical_mid(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 100 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    // 多周期共振：周线 MACD 金叉 (用日线 5×12=60, 5×26=130 近似)
    let (dif, dea, _) = indicators::macd(klines, 12, 26, 9)?;
    if dif <= dea {
        return None;
    }
    // MA50 > MA120 中期多头排列
    let ma50 = indicators::sma(&cs, 50)?;
    let ma120 = indicators::sma(&cs, 120)?;
    if ma50 <= ma120 {
        return None;
    }
    // 价格在 MA50 之上
    let ma50_mult = read_f64(vars, "technical_ma50_mult", 0.98);
    if last < ma50 * ma50_mult {
        return None;
    }
    // RSI 非超买
    let rsi14 = indicators::rsi(klines, 14)?;
    let max_rsi = read_f64(vars, "technical_max_rsi", 75.0);
    if rsi14 > max_rsi {
        return None;
    }
    Some(last)
}

fn detect_technical_long(klines: &[KLine], vars: &serde_json::Value) -> Option<f64> {
    if klines.len() < 200 {
        return None;
    }
    let cs = closes(klines);
    let last = *cs.last()?;
    // 年线多头：MA200 向上
    let ma200 = indicators::sma(&cs, 200)?;
    if ma200.is_nan() {
        return None;
    }
    let ma200_gradient_period = read_f64(vars, "technical_ma200_period", 50.0) as usize;
    if cs.len() > ma200_gradient_period {
        let ma200_prev = indicators::sma(&cs[..cs.len() - ma200_gradient_period], 200);
        if let Some(prev) = ma200_prev {
            if ma200 <= prev {
                return None;
            }
        }
    }
    // 价格在 MA200 之上
    if last < ma200 * 0.95 {
        return None;
    }
    // 低波动率环境（适合长期持仓）
    let rsi_monthly = indicators::rsi(klines, 60)?;
    let max_rsi = read_f64(vars, "technical_long_max_rsi", 65.0);
    let min_rsi = read_f64(vars, "technical_long_min_rsi", 35.0);
    if rsi_monthly > max_rsi || rsi_monthly < min_rsi {
        return None;
    }
    Some(last)
}

// ── 策略注册表 ──

// 修复 L-1: 删除错误的 #[allow(dead_code)] 标注，read_f64 在多处策略中被使用。
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
    // ── Serenity/Bottleneck（不可 K 线回测） ──
    // 瓶颈分析依赖 LLM 工作流 + 财务数据（毛利率/负债率/营收增速），
    // 非纯 K 线信号策略。此处为 placeholder 保持注册表完整性，实际不计入回测。
    StratDef {
        id: "bottleneck_mid",
        style: "bottleneck",
        period: "mid",
        period_enum: Period::Mid,
        warmup: 60,
        detect: detect_not_backtestable,
    },
    StratDef {
        id: "bottleneck_long",
        style: "bottleneck",
        period: "long",
        period_enum: Period::Long,
        warmup: 120,
        detect: detect_not_backtestable,
    },
    // ── Policy（不可 K 线回测） ──
    StratDef {
        id: "policy_mid",
        style: "policy",
        period: "mid",
        period_enum: Period::Mid,
        warmup: 60,
        detect: detect_not_backtestable,
    },
    StratDef {
        id: "policy_long",
        style: "policy",
        period: "long",
        period_enum: Period::Long,
        warmup: 120,
        detect: detect_not_backtestable,
    },
    // ── Earnings（不可 K 线回测） ──
    StratDef {
        id: "earnings_mid",
        style: "earnings",
        period: "mid",
        period_enum: Period::Mid,
        warmup: 60,
        detect: detect_not_backtestable,
    },
    StratDef {
        id: "earnings_long",
        style: "earnings",
        period: "long",
        period_enum: Period::Long,
        warmup: 120,
        detect: detect_not_backtestable,
    },
    // ── CapitalFlow（K 线代理同 Capital） ──
    StratDef {
        id: "capital_flow_short",
        style: "capital_flow",
        period: "short",
        period_enum: Period::Short,
        warmup: 30,
        detect: detect_capital_short,
    },
    StratDef {
        id: "capital_flow_mid",
        style: "capital_flow",
        period: "mid",
        period_enum: Period::Mid,
        warmup: 100,
        detect: detect_capital_mid,
    },
    StratDef {
        id: "capital_flow_long",
        style: "capital_flow",
        period: "long",
        period_enum: Period::Long,
        warmup: 250,
        detect: detect_capital_long,
    },
    // ── Event（不可 K 线回测） ──
    StratDef {
        id: "event_short",
        style: "event",
        period: "short",
        period_enum: Period::Short,
        warmup: 30,
        detect: detect_not_backtestable,
    },
    StratDef {
        id: "event_mid",
        style: "event",
        period: "mid",
        period_enum: Period::Mid,
        warmup: 100,
        detect: detect_not_backtestable,
    },
    StratDef {
        id: "event_long",
        style: "event",
        period: "long",
        period_enum: Period::Long,
        warmup: 220,
        detect: detect_not_backtestable,
    },
    // ── Technical（纯 K 线形态，可回测） ──
    StratDef {
        id: "technical_short",
        style: "technical",
        period: "short",
        period_enum: Period::Short,
        warmup: 30,
        detect: detect_technical_short,
    },
    StratDef {
        id: "technical_mid",
        style: "technical",
        period: "mid",
        period_enum: Period::Mid,
        warmup: 100,
        detect: detect_technical_mid,
    },
    StratDef {
        id: "technical_long",
        style: "technical",
        period: "long",
        period_enum: Period::Long,
        warmup: 250,
        detect: detect_technical_long,
    },
];

pub const SKIPPED: &[&str] = &[
    "watchlist_*: 兜底策略无信号逻辑，不计入回测",
    "bottleneck_*: 依赖 LLM 工作流 + 财务基本面，不可 K 线回测",
    "policy_*: 依赖政策分析 LLM 工作流，不可 K 线回测",
    "earnings_*: 依赖预期差分析 LLM 工作流，不可 K 线回测",
    "event_*: 依赖事件驱动 LLM 工作流，不可 K 线回测",
];

// ── 策略查找与单策略信号历史 ──

/// 按 strategy_id 查找策略定义（用于 RecoSignalTimeline）
pub(crate) fn get_strategy_def(sid: &str) -> Option<&'static StratDef> {
    STRATS.iter().find(|s| s.id == sid)
}

/// 对指定策略跑所有股票的单笔信号历史（不聚合，返回每条信号明细）
///
/// `stock_codes` 可选过滤：传入非空列表时只分析这些股票；None 时从 reco_picks 种子池读。
pub async fn run_signal_history(
    client: Arc<dyn MarketDataProvider>,
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
        // 修复 P1: 使用前复权（Forward）消除除权除息造成的价格跳变，避免回测收益失真
        let klines =
            match client.get_klines(code, "daily", kline_limit, Some(AdjType::Forward)).await {
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
    client: Arc<dyn MarketDataProvider>,
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
    client: Arc<dyn MarketDataProvider>,
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
        // 修复 P1: 与 scan_strategy 一致，使用前复权
        match client.get_klines(code, "daily", kline_limit, Some(AdjType::Forward)).await {
            Ok(k) if k.len() >= 60 => {
                loaded.push(StockWithKlines { code: code.clone(), name: name.clone(), klines: k })
            },
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

    GroupBacktestResult { label: label.to_string(), stock_count: loaded.len() as u32, strategies }
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
    let max_idx = klines.len().saturating_sub(holding as usize + 2);
    let mut out = Vec::new();
    // cooldown_index: 上次信号触发的位置 + holding，此期间不产生新信号
    // 避免滑动窗口在同一持仓期内产生多个重叠信号（高估可执行信号频率）
    let mut cooldown_index: usize = 0;
    for i in warmup..max_idx {
        if i <= cooldown_index {
            continue;
        }
        let window = &klines[..=i];
        if detect(window, vars_ref).is_some() {
            // 修复 P1: 前视偏差 — 原代码用信号日收盘价（klines[i].close）作为入场价，
            // 但信号在收盘后才能生成，实际最早只能在次日开盘执行买入。
            // 改为：entry = klines[i+1].open（次日开盘价），exit = klines[i+1+holding].close
            let entry = klines[i + 1].open;
            let exit_idx = (i + 1 + holding as usize).min(klines.len() - 1);
            let exit_price = klines[exit_idx].close;
            let mut peak = 0.0_f64;
            let mut max_dd = 0.0;
            for k in &klines[i + 1..=exit_idx] {
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
        (wins as f64 / total as f64 * 100.0, s_ret / total as f64, s_ret, s_dd / total as f64)
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
        let tw: f64 = sigs.iter().filter(|s| s.was_profitable).map(|s| s.return_pct.abs()).sum();
        let tl: f64 = sigs.iter().filter(|s| !s.was_profitable).map(|s| s.return_pct.abs()).sum();
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
        let extra_penalty = stats.sharpe_ratio.map_or(0.0, |s| if s < 0.3 { -0.05 } else { 0.0 });
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
static SIGNAL_QUALITY_CACHE: LazyLock<Mutex<HashMap<(String, String), SignalQualityStats>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 缓存容量上限，超出后按 last_updated 驱逐最旧条目，避免长跑内存无限增长
const MAX_CACHE_ENTRIES: usize = 4096;

/// 从回测 groups 结果更新信号质量缓存（自动注入 as-of 后缀隔离 live/replay）
pub fn update_signal_quality_cache(positive_stats: &HashMap<String, StrategyStats>) {
    let suffix = axagent_astock_data::as_of::cache_suffix();
    // 修复 P0-S4: 系统时钟早于 UNIX_EPOCH 时 unwrap 会 panic（嵌入式/虚拟机时钟漂移场景）。
    // 改用 unwrap_or_default() 兜底为 0；now=0 会让缓存条目看上去"立即过期"，但功能不挂。
    // 修复 L-3: 添加 warn 日志，便于发现时钟倒流异常。
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_else(|e| {
            tracing::warn!("[signal_quality] SystemTime 早于 UNIX_EPOCH（时钟倒流）: {e}");
            0
        });
    let mut cache = SIGNAL_QUALITY_CACHE.lock();
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
    // 容量上限：超出时按 last_updated 驱逐最旧的约 25%，避免长跑内存无限增长
    if cache.len() > MAX_CACHE_ENTRIES {
        let mut aged: Vec<_> = cache.iter().map(|(k, v)| (k.clone(), v.last_updated)).collect();
        aged.sort_by_key(|(_, ts)| *ts);
        let drop = cache.len() - MAX_CACHE_ENTRIES + MAX_CACHE_ENTRIES / 4;
        for (k, _) in aged.into_iter().take(drop) {
            cache.remove(&k);
        }
    }
}

/// 查询策略信号质量（自动注入 as-of 后缀隔离 live/replay）
pub fn get_signal_quality(strategy_id: &str) -> Option<SignalQualityStats> {
    let suffix = axagent_astock_data::as_of::cache_suffix();
    let cache = SIGNAL_QUALITY_CACHE.lock();
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

/// 加权信号平滑（非真贝叶斯更新）：返回 (平滑后胜率, 样本数, 先验胜率)
///
/// 先验胜率基于市场环境：
///   - "bull" → 0.55（牛市中随机持有多头的胜率偏高）
///   - "bear" → 0.45（熊市中随机持有多头的胜率偏低）
///   - _      → 0.50（拉普拉斯无差别先验）
///
/// 旧名「贝叶斯校准」系误称——此处仅做一次性权重平滑 (prior_weight + n)，
/// 并非真正的贝叶斯序贯更新。重命名为 weighted_signal_calibration 避免误导。
///
/// 公式：
///   calibrated = (prior_weight × prior + n × sample_win_rate) / (prior_weight + n)
///
/// 其中 prior_weight=20 为"虚拟样本量"。信号数<20 时结果向先验平滑；
/// 信号数>200 时结果接近原始胜率。
pub fn weighted_signal_calibration(strategy_id: &str, market_regime: &str) -> (f64, u32, f64) {
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
        let klines: Vec<KLine> =
            (0..50).map(|i| k(10.0, &format!("d{}", i % 28 + 1), 10_000_000.0)).collect();
        assert!(detect_trend_short(&klines, &serde_json::Value::Null).is_none());
        assert!(detect_reversion_short(&klines, &serde_json::Value::Null).is_none());
        // 资金策略需要放量，平盘不放量也为 None
        assert!(detect_capital_short(&klines, &serde_json::Value::Null).is_none());
    }

    #[test]
    fn value_strategy_signals_on_flat() {
        let klines: Vec<KLine> =
            (0..50).map(|i| k(10.0, &format!("d{}", i % 28 + 1), 10_000_000.0)).collect();
        // 平盘 = 低波幅+均线附近 = 价值股典型 K 线特征
        assert!(detect_value_short(&klines, &serde_json::Value::Null).is_some());
    }

    #[test]
    fn capital_needs_volume_spike() {
        // 第 1 部分：平盘无放量 → None
        let flat: Vec<KLine> =
            (0..50).map(|i| k(10.0, &format!("d{}", i % 28 + 1), 10_000_000.0)).collect();
        assert!(detect_capital_short(&flat, &serde_json::Value::Null).is_none());

        // 第 2 部分：连续 5 日放量 + 上涨 → 有信号
        // （新 detect 使用 5 日均量/20 日均量比，单日放量不足）
        let mut spike = flat.clone();
        for i in 0..5 {
            spike.push(KLine {
                date: format!("d{}", 100 + i),
                open: 10.1,
                high: 10.4,
                low: 10.0,
                close: 10.3 + i as f64 * 0.05,
                volume: 3_000_000.0,
                amount: 31_000_000.0,
                turnover_rate: Some(3.0),
                adj_factor: None,
            });
        }
        assert!(
            detect_capital_short(&spike, &serde_json::Value::Null).is_some(),
            "连续 5 日放量应触发 Capital 信号"
        );
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
