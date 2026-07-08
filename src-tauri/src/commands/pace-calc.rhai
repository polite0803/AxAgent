// pace-calc.rhai — PACE 四维情绪向量计算
//
// 理论依据：
//   P (Polarity 极性)：净得分 = Σ(事件基础分 × 来源权重 × 时效衰减)
//   A (Actuality 时效性)：24h 半衰期指数衰减，>72h 强制归零
//   C (Credibility 可信度)：来源权重硬编码，LLM 自评 <0.4 自动丢弃
//   E (Expectation 预期差)：30日 P 值移动平均的 Z-score
//
// 输出 pace_vector + 背离修正 + 协同增强 → pace_factor（供 portfolio-mgr 作为 f11）

fn present(x) { type_of(x) != "()" }

// ── 硬编码基础分符号表 ──
let base_score_map = #{
    // 利好事件 → 正分
    "增持": 2.0,
    "回购": 2.5,
    "中标": 1.5,
    "重大合同": 2.0,
    "业绩预增": 2.5,
    "扭亏为盈": 3.0,
    "股权激励": 1.5,
    "分红": 1.0,
    "送转": 0.8,
    "新产品发布": 1.2,
    "技术突破": 1.8,
    "政策利好": 1.5,
    "行业景气": 1.2,
    "评级上调": 1.5,
    "纳入指数": 1.0,
    "并购重组": 2.0,
    "业绩超预期": 3.0,
    // 利空事件 → 负分
    "减持": -3.0,
    "业绩预亏": -3.0,
    "亏损": -2.5,
    "立案调查": -4.0,
    "监管函": -3.0,
    "警示函": -2.5,
    "退市": -5.0,
    "ST": -4.0,
    "*ST": -4.5,
    "风险提示": -2.0,
    "异常波动": -1.5,
    "强制平仓": -4.0,
    "违约": -3.5,
    "评级下调": -2.0,
    "商誉减值": -3.5,
    "诉讼": -2.5,
    "处罚": -3.0,
    "被动减持": -3.0,
    "股东减持": -2.5,
    // 中性
    "股东大会": 0.0,
    "定期报告": 0.0,
    "人事变动": -0.2,
    "停牌": -0.5,
    "复牌": 0.0,
};

// ── 来源权重硬编码 ──
let source_weight_map = #{
    "official": 1.0,       // 官方公告/交易所披露
    "regulatory": 1.0,     // 监管机构
    "exchange": 0.9,       // 交易所
    "media_major": 0.7,    // 主流财经媒体
    "media": 0.6,          // 一般媒体
    "research": 0.7,       // 券商研报
    "self_media": 0.2,     // 自媒体
    "social": 0.15,        // 社交媒体
    "unknown": 0.3,        // 未知来源
};

// ── 时效衰减函数（24h 半衰期）──
fn time_decay(hours_elapsed) {
    if hours_elapsed < 0.0 { return 1.0; }
    if hours_elapsed > 72.0 { return 0.0; }  // >3 工作日强制归零
    // exp(-ln(2) * hours / 24) = 0.5^(hours/24)
    let ln2 = 0.69314718056;
    (-ln2 * hours_elapsed / 24.0).exp()
}

// ── 分类器：从 event_type 文本中匹配基础分 ──
fn classify_event(event_type) {
    if !present(event_type) || type_of(event_type) != "string" { return 0.0; }
    let et = event_type;
    // 按 specificity 从高到低匹配
    let ordered_keys = [
        "*ST", "ST",
        "立案调查", "强制平仓", "退市", "违约", "商誉减值",
        "业绩超预期", "扭亏为盈", "业绩预增", "业绩预亏",
        "监管函", "警示函", "处罚", "诉讼",
        "增持", "减持", "回购", "股东减持", "被动减持",
        "中标", "重大合同", "股权激励", "评级上调", "评级下调",
        "并购重组", "技术突破", "政策利好", "新产品发布",
        "行业景气", "纳入指数", "分红", "送转",
        "风险提示", "异常波动", "亏损",
        "停牌", "复牌", "人事变动",
        "股东大会", "定期报告",
    ];
    for key in ordered_keys {
        if et.contains(key) { return base_score_map[key]; }
    }
    // 正向/负向兜底
    if et.contains("利好") || et.contains("正面") || et.contains("增长") { return 0.5; }
    if et.contains("利空") || et.contains("负面") || et.contains("下降") || et.contains("风险") { return -0.5; }
    0.0
}

// ── 分类器：来源权重 ──
fn classify_source(source_type) {
    if !present(source_type) || type_of(source_type) != "string" { return 0.3; }
    let st = source_type.to_lower();
    for key in source_weight_map.keys() {
        if st.contains(key) { return source_weight_map[key]; }
    }
    0.3  // unknown
}

// ── 主计算 ──
let events = [];
let now_hours = if present(now_timestamp) { now_timestamp } else { 0.0 };

// 解析输入事件（来自 LLM a-catalyst / a-news 节点）
if present(llm_events) && type_of(llm_events) == "string" && llm_events != "" {
    let parsed = json_parse(llm_events);
    if type_of(parsed) == "array" {
        for item in parsed {
            if type_of(item) == "map" {
                events.push(item);
            }
        }
    } else if type_of(parsed) == "map" {
        // 单个事件
        events.push(parsed);
    }
}

// 如果无 LLM 事件但有 announcement events，回退使用公告数据
if events.len() == 0 && present(announcement_events) && type_of(announcement_events) == "string" && announcement_events != "" {
    let ann_parsed = json_parse(announcement_events);
    if type_of(ann_parsed) == "array" {
        for item in ann_parsed {
            if type_of(item) == "map" {
                // 构建 LLM 事件兼容格式
                let event_type = if type_of(item["event_type"]) == "string" { item["event_type"] } else {
                    if type_of(item["ann_type"]) == "string" { item["ann_type"] } else { "" }
                };
                let title_text = if type_of(item["title"]) == "string" { item["title"] } else { "" };
                let combined_type = if event_type != "" { event_type } else { title_text };
                events.push(#{
                    "event_type": combined_type,
                    "source_type": "official",
                    "confidence_score": 0.9,
                    "hours_elapsed": if type_of(item["hours_elapsed"]) == "f64" { item["hours_elapsed"] } else { 24.0 },
                    "target_entity": if type_of(item["stock_code"]) == "string" { item["stock_code"] } else { "" },
                });
            }
        }
    }
}

// ── 计算 P（Polarity 极性）──
let p_score = 0.0;
let valid_event_count = 0;
let discarded_events = [];

for evt in events {
    let llm_confidence = if type_of(evt["confidence_score"]) == "f64" { evt["confidence_score"] }
        else if type_of(evt["confidence"]) == "f64" { evt["confidence"] }
        else { 0.5 };

    // C 维度：LLM 自评 <0.4 自动丢弃
    if llm_confidence < 0.4 {
        discarded_events.push(#{
            "reason": "LLM自评可信度<0.4",
            "event": if type_of(evt["event_type"]) == "string" { evt["event_type"] } else { "unknown" },
            "confidence": llm_confidence,
        });
        continue;
    }

    let event_type = if type_of(evt["event_type"]) == "string" { evt["event_type"] }
        else if type_of(evt["impact_direction"]) == "string" { evt["impact_direction"] }
        else { "" };

    let source_type = if type_of(evt["source_type"]) == "string" { evt["source_type"] }
        else if type_of(evt["source"]) == "string" { evt["source"] }
        else { "unknown" };

    let hours_elapsed = if type_of(evt["hours_elapsed"]) == "f64" { evt["hours_elapsed"] }
        else if type_of(evt["age_hours"]) == "f64" { evt["age_hours"] }
        else { 24.0 };

    let base_score = classify_event(event_type);
    let source_weight = classify_source(source_type);
    let decay = time_decay(hours_elapsed);

    if decay <= 0.0 { continue; }  // 过期事件跳过

    let contribution = base_score * source_weight * decay * llm_confidence;
    p_score += contribution;
    valid_event_count += 1;
}

// ── 计算 A（Actuality 时效性）──
// 取最近一条事件距今的小时数
let min_hours = 999.0;
for evt in events {
    let h = if type_of(evt["hours_elapsed"]) == "f64" { evt["hours_elapsed"] }
        else if type_of(evt["age_hours"]) == "f64" { evt["age_hours"] }
        else { 24.0 };
    if h < min_hours { min_hours = h; }
}
let a_score = if min_hours < 999.0 { time_decay(min_hours) } else { 0.0 };

// ── 计算 C（Credibility 可信度）──
// 取所有事件来源权重的加权平均
let c_score = if valid_event_count > 0 {
    let total_cred = 0.0;
    let cred_count = 0;
    for evt in events {
        let llm_conf = if type_of(evt["confidence_score"]) == "f64" { evt["confidence_score"] }
            else if type_of(evt["confidence"]) == "f64" { evt["confidence"] }
            else { 0.5 };
        if llm_conf < 0.4 { continue; }
        let source_type = if type_of(evt["source_type"]) == "string" { evt["source_type"] }
            else if type_of(evt["source"]) == "string" { evt["source"] }
            else { "unknown" };
        total_cred += classify_source(source_type) * llm_conf;
        cred_count += 1;
    }
    if cred_count > 0 { clamp(total_cred / cred_count, 0.0, 1.0) } else { 0.0 }
} else { 0.0 };

// ── 计算 E（Expectation 预期差）──
// 需要 30 日历史 P 值。如果上游注入了 p_history 数组则计算 Z-score
let e_score = 0.0;
let e_anomaly = false;
if present(p_history) && type_of(p_history) == "array" && p_history.len() >= 5 {
    let n = p_history.len();
    let sum = 0.0;
    for val in p_history {
        if type_of(val) == "f64" { sum += val; }
        else if type_of(val) == "int" { sum += val; }
    }
    let mean = sum / n;
    let sq_sum = 0.0;
    for val in p_history {
        if type_of(val) == "f64" {
            let diff = val - mean;
            sq_sum += diff * diff;
        } else if type_of(val) == "int" {
            let diff = val - mean;
            sq_sum += diff * diff;
        }
    }
    let std = (sq_sum / n).sqrt();
    if std > 0.001 {
        e_score = (p_score - mean) / std;
        if e_score.abs() > 2.0 { e_anomaly = true; }
    }
}

// ── 背离修正（§6.1）──
let divergence_flag = false;
let divergence_reason = "";
let pace_factor = 1.0;

// 资金流向检查
if present(money_flow_net) && present(daily_avg_volume) && daily_avg_volume > 0.0 {
    let net_outflow_ratio = (-money_flow_net) / daily_avg_volume;
    if p_score > 0.0 && net_outflow_ratio > 0.10 {
        // Polarity 为正（看多）但资金净流出 > 10% 日均成交额 → 对冲衰减 ×0.5
        divergence_flag = true;
        divergence_reason = `情绪与资金背离:P=${(p_score*100.0).round()/100.0}(正) 但资金净流出占比${(net_outflow_ratio*100.0*10.0).round()/10.0}%`;
        pace_factor *= 0.5;
    }
}

// ── 协同增强（§6.2）──
let synergy_flag = false;
let synergy_reason = "";
if present(sector_etf_direction) && type_of(sector_etf_direction) == "string" && sector_etf_direction != "" {
    let etf_dir = sector_etf_direction.to_lower();
    let event_dir = if p_score > 0.5 { "inflow" } else if p_score < -0.5 { "outflow" } else { "neutral" };

    if (event_dir == "inflow" && etf_dir.contains("inflow")) || (event_dir == "outflow" && etf_dir.contains("outflow")) {
        synergy_flag = true;
        synergy_reason = `事件方向与板块ETF资金流一致→协同增强×1.3`;
        pace_factor *= 1.3;
    } else if (event_dir == "inflow" && etf_dir.contains("outflow")) || (event_dir == "outflow" && etf_dir.contains("inflow")) {
        synergy_flag = true;
        synergy_reason = `事件方向与板块ETF资金流背离→协同衰减×0.7`;
        pace_factor *= 0.7;
    }
}

// ── 情绪异动标记 ──
let sentiment_anomaly = e_anomaly;
let anomaly_reason = if e_anomaly {
    `P值Z-score=${(e_score*100.0).round()/100.0} 超过±2σ，情绪异动`
} else { "" };

// ── PACE 信号归一化为 [-1, 1] ──
// P: 基于所有有效事件贡献，用 tanh 归一化
let p_normalized = if valid_event_count > 0 {
    // 使用 tanh 做 soft clamp，单个事件 ±3.0 贡献约映射到 ±0.995
    let tanh_input = p_score / (valid_event_count as f64).max(1.0) / 1.5;
    let tanh_val = (tanh_input.exp() - (-tanh_input).exp()) / (tanh_input.exp() + (-tanh_input).exp());
    clamp(tanh_val * p_score.sign(), -1.0, 1.0)
} else { 0.0 };

// ── 输出 ──
#{
    "pace_vector": #{
        "P": (p_normalized * 100.0).round() / 100.0,
        "A": (a_score * 100.0).round() / 100.0,
        "C": (c_score * 100.0).round() / 100.0,
        "E": (e_score * 100.0).round() / 100.0,
    },
    "p_raw": (p_score * 100.0).round() / 100.0,
    "valid_events": valid_event_count,
    "discarded_events": discarded_events,
    "sentiment_anomaly": sentiment_anomaly,
    "anomaly_reason": anomaly_reason,
    "divergence_flag": divergence_flag,
    "divergence_reason": divergence_reason,
    "synergy_flag": synergy_flag,
    "synergy_reason": synergy_reason,
    "pace_factor": (pace_factor * 100.0).round() / 100.0,
    "pace_signal": (p_normalized * pace_factor * 100.0).round() / 100.0,
}
