// technical-veto.rhai — 技术面硬约束否决规则
//
// 规则体系：
//   R-401：RSI>80 + LLM 看多 → 否决（追高风险）
//   R-402：RSI<20 + LLM 看空 → 否决（恐慌杀跌）
//   R-403：MACD 顶背离 + LLM 看多 → 否决
//   R-404：MACD 底背离 + LLM 看空 → 否决
//   R-405：RSI>80 + MACD 顶背离同时触发 → 否决
//
// 输入：由 Rust 端 (core.rs) 计算的技术指标注入
// 返回：veto_result { vetoed, rule_id, reason, trail_entry }

fn present(x) { type_of(x) != "()" }

let vetoed = false;
let rule_id = "";
let reason = "";
let trail_entries = [];

// ── 辅助：构建 trail entry ──
fn make_trail(rid, verdict, detail) {
    #{ "rule_id": rid, "status": verdict, "detail": detail, "timestamp": now_ts }
}

// ── 技术指标提取 ──
let rsi_14 = if present(rsi_14) && type_of(rsi_14) == "f64" { rsi_14 }
    else if present(rsi) && type_of(rsi) == "f64" { rsi }
    else { 50.0 };

let rsi_6 = if present(rsi_6) && type_of(rsi_6) == "f64" { rsi_6 } else { 50.0 };

let macd_dif = if present(macd_dif) && type_of(macd_dif) == "f64" { macd_dif }
    else if present(macd_dif_value) && type_of(macd_dif_value) == "f64" { macd_dif_value }
    else { 0.0 };

let macd_dea = if present(macd_dea) && type_of(macd_dea) == "f64" { macd_dea }
    else if present(macd_dea_value) && type_of(macd_dea_value) == "f64" { macd_dea_value }
    else { 0.0 };

let macd_hist = if present(macd_hist) && type_of(macd_hist) == "f64" { macd_hist }
    else if present(macd_bar) && type_of(macd_bar) == "f64" { macd_bar }
    else { 0.0 };

// ── MACD 背离检测（需要历史数列注入）──
let has_divergence_top = false;   // 顶背离：价格新高，MACD 下降
let has_divergence_bottom = false; // 底背离：价格新低，MACD 上升

if present(price_high_series) && type_of(price_high_series) == "array" && price_high_series.len() >= 20
   && present(macd_hist_series) && type_of(macd_hist_series) == "array" && macd_hist_series.len() >= 20 {
    let n = price_high_series.len();
    let m = macd_hist_series.len();

    // 取最近 20 根 K 线的高点和 MACD 柱
    let lookback = 20;
    if n >= lookback && m >= lookback {
        let price_now = 0.0;
        let price_max = 0.0;
        let macd_now = 0.0;
        let macd_at_max = 0.0;

        for i in range(n-lookback, n-1) {
            let p = if type_of(price_high_series[i]) == "f64" { price_high_series[i] } else { 0.0 };
            if p > price_max { price_max = p; }
        }
        price_now = if type_of(price_high_series[n-1]) == "f64" { price_high_series[n-1] } else { 0.0 };

        // 最近 5 日 MACD 柱平均值 vs 前 15 日最大值
        let macd_recent_sum = 0.0;
        let macd_early_max = 0.0;
        for i in range(m-lookback, m-1) {
            let mv = if type_of(macd_hist_series[i]) == "f64" { macd_hist_series[i] } else { 0.0 };
            if i >= m-6 { macd_recent_sum += mv; }
            if i < m-6 && mv > macd_early_max { macd_early_max = mv; }
        }

        let macd_recent_avg = macd_recent_sum / 5.0;

        // 顶背离：价格在 20 日内高点附近（98%+），但 MACD 柱状体明显低于前高
        if price_now >= price_max * 0.98 && macd_recent_avg < macd_early_max * 0.7 {
            has_divergence_top = true;
        }

        // 底背离检测（用最低价）
        if present(price_low_series) && type_of(price_low_series) == "array" && price_low_series.len() >= lookback {
            let price_min = 999999.0;
            let price_now_low = 0.0;
            for i in range(price_low_series.len()-lookback, price_low_series.len()-1) {
                let p = if type_of(price_low_series[i]) == "f64" { price_low_series[i] } else { 999999.0 };
                if p < price_min { price_min = p; }
            }
            price_now_low = if type_of(price_low_series[price_low_series.len()-1]) == "f64" {
                price_low_series[price_low_series.len()-1]
            } else { 999999.0 };

            // MACD 底：近期平均 vs 早期低点
            let macd_early_min = 999999.0;
            for i in range(m-lookback, m-6) {
                let mv = if type_of(macd_hist_series[i]) == "f64" { macd_hist_series[i] } else { 0.0 };
                if mv < macd_early_min { macd_early_min = mv; }
            }
            if price_now_low <= price_min * 1.02 && macd_recent_avg > macd_early_min * 1.3 {
                has_divergence_bottom = true;
            }
        }
    }
}

// ── 获取 LLM 交易信号方向 ──
let llm_direction = if present(llm_trade_signal) && type_of(llm_trade_signal) == "string" {
    llm_trade_signal.to_lower()
} else if present(llm_direction) && type_of(llm_direction) == "string" {
    llm_direction.to_lower()
} else if present(llm_action) && type_of(llm_action) == "string" {
    llm_action.to_lower()
} else { "" };

let llm_is_bullish = llm_direction.contains("buy") || llm_direction.contains("bull") || llm_direction.contains("看多")
    || llm_direction.contains("买入") || llm_direction.contains("增持") || llm_direction.contains("long");
let llm_is_bearish = llm_direction.contains("sell") || llm_direction.contains("bear") || llm_direction.contains("看空")
    || llm_direction.contains("卖出") || llm_direction.contains("减持") || llm_direction.contains("short");

let now_ts = if present(current_timestamp) && type_of(current_timestamp) == "f64" { current_timestamp }
    else if present(current_timestamp) && type_of(current_timestamp) == "int" { current_timestamp as f64 }
    else { 0.0 };

// ── 否决规则 ──

// R-401: RSI>80 + LLM 看多 → 否决
if !vetoed && rsi_14 > 80.0 && llm_is_bullish {
    vetoed = true;
    rule_id = "R-401";
    reason = `RSI(14)=${(rsi_14*100.0).round()/100.0}>80 超买区域，LLM 信号看多，触发追高风险否决`;
    trail_entries.push(make_trail("R-401", "VETOED", reason));
}

// R-402: RSI<20 + LLM 看空 → 否决
if !vetoed && rsi_14 < 20.0 && llm_is_bearish {
    vetoed = true;
    rule_id = "R-402";
    reason = `RSI(14)=${(rsi_14*100.0).round()/100.0}<20 超卖区域，LLM 信号看空，触发恐慌杀跌否决`;
    trail_entries.push(make_trail("R-402", "VETOED", reason));
}

// R-403: MACD 顶背离 + LLM 看多 → 否决
if !vetoed && has_divergence_top && llm_is_bullish {
    vetoed = true;
    rule_id = "R-403";
    reason = "MACD 顶背离（价格新高但 MACD 柱下降），LLM 信号看多，触发背离否决";
    trail_entries.push(make_trail("R-403", "VETOED", reason));
}

// R-404: MACD 底背离 + LLM 看空 → 否决
if !vetoed && has_divergence_bottom && llm_is_bearish {
    vetoed = true;
    rule_id = "R-404";
    reason = "MACD 底背离（价格新低但 MACD 柱上升），LLM 信号看空，触发背离否决";
    trail_entries.push(make_trail("R-404", "VETOED", reason));
}

// R-405: RSI>80 + MACD 顶背离同时 → 否决（不管 LLM 方向）
if !vetoed && rsi_14 > 80.0 && has_divergence_top {
    vetoed = true;
    rule_id = "R-405";
    reason = `RSI>80 且 MACD 顶背离同时触发，技术面双重超买`;
    trail_entries.push(make_trail("R-405", "VETOED", reason));
}

// 全部通过 → PASS
if !vetoed {
    let passed_rules = [];
    // 记录所有检查通过的规则
    if rsi_14 <= 80.0 || !llm_is_bullish {
        trail_entries.push(make_trail("R-401", "PASS", `RSI(14)=${(rsi_14*100.0).round()/100.0}, 未触发否决条件`));
    }
    if rsi_14 >= 20.0 || !llm_is_bearish {
        trail_entries.push(make_trail("R-402", "PASS", `RSI(14)=${(rsi_14*100.0).round()/100.0}, 未触发否决条件`));
    }
    if !has_divergence_top || !llm_is_bullish {
        trail_entries.push(make_trail("R-403", "PASS", "未检测到 MACD 顶背离"));
    }
    if !has_divergence_bottom || !llm_is_bearish {
        trail_entries.push(make_trail("R-404", "PASS", "未检测到 MACD 底背离"));
    }
    trail_entries.push(make_trail("R-405", "PASS", "未触发 RSI+MACD 双重超买"));
    rule_id = "R-000";
    reason = "技术面硬约束检查全部通过";
}

// ── 输出 ──
#{
    "vetoed": vetoed,
    "rule_id": rule_id,
    "reason": reason,
    "decision_trail": trail_entries,
    "indicators": #{
        "rsi_14": (rsi_14 * 100.0).round() / 100.0,
        "rsi_6": (rsi_6 * 100.0).round() / 100.0,
        "macd_dif": (macd_dif * 100.0).round() / 100.0,
        "macd_dea": (macd_dea * 100.0).round() / 100.0,
        "macd_hist": (macd_hist * 100.0).round() / 100.0,
        "has_divergence_top": has_divergence_top,
        "has_divergence_bottom": has_divergence_bottom,
    },
}
