// data-quality.rhai — 数据质量确定性评分（替代原 LLM Agent，P1-4 修复）
//
// 输入：每个 stock-analyst 的 verdict map（来自 content.verdict）
// 输出：{grade: "A"|"B"|"C"|"D"|"F", score: 0-100, summary: String}
//
// 分析师缩写对照（当前 DAG 10 个分析师）:
//   mk   → a-market-analyst (market-analyst)
//   sent → a-sentiment (sentiment-analyst)
//   news → a-news (news-analyst)
//   fund → a-fundamentals (fundamentals-analyst)
//   pol  → a-policy (policy-analyst)
//   hm   → a-hot-money (hot-money-tracker)
//   lk   → a-lockup (lockup-watcher)
//   res  → a-research (research-analyst)
//   sec  → a-sector (sector-analyst)
//   cat  → a-catalyst (catalyst-analyst)
//
// 生命周期维护:
//   - DAG 中新增 stock-analyst → 在此添加一行 {abbr}_verdict
//     并同步更新下方的 n 值和分析师列表。
//
// P1 修复（2026-07-23）:
//   - input_mapping 从 {id}.content.verdict.confidence 改为 {id}.content.verdict
//   - 新增 extract_conf() 从 verdict map 中提取 confidence

// extract_conf: 从 verdict map 中提取 confidence 数值
// 支持两种格式：
//   扁平: {"verdict":"看多", "confidence":70, ...}
//   嵌套: {"report":"...", "verdict":{"confidence":85, ...}}
// Rhai 1.25 64-bit: type_of(i64) == "i64"，不是 "int"
fn extract_conf(v) {
    if type_of(v) == "map" {
        let c = v["confidence"];
        if type_of(c) == "f64" { return c; }
        if type_of(c) == "i64" || type_of(c) == "int" { return c.to_float(); }
        // 嵌套格式 fallback: v.verdict.confidence
        let inner = v["verdict"];
        if type_of(inner) == "map" {
            let ic = inner["confidence"];
            if type_of(ic) == "f64" { return ic; }
            if type_of(ic) == "i64" || type_of(ic) == "int" { return ic.to_float(); }
        }
        -1.0
    } else { -1.0 }
}

// ── 分析师 confidence 解析 ──
let mk_c = extract_conf(mk_verdict);
let sent_c = extract_conf(sent_verdict);
let news_c = extract_conf(news_verdict);
let fund_c = extract_conf(fund_verdict);
let pol_c = extract_conf(pol_verdict);
let hm_c = extract_conf(hm_verdict);
let lk_c = extract_conf(lk_verdict);
let res_c = extract_conf(res_verdict);
let sec_c = extract_conf(sec_verdict);
let cat_c = extract_conf(cat_verdict);

// gap 推断：confidence < 0（字段缺失/节点失败）才算真正的数据缺口
let mk_g = mk_c < 0.0;
let sent_g = sent_c < 0.0;
let news_g = news_c < 0.0;
let fund_g = fund_c < 0.0;
let pol_g = pol_c < 0.0;
let hm_g = hm_c < 0.0;
let lk_g = lk_c < 0.0;
let res_g = res_c < 0.0;
let sec_g = sec_c < 0.0;
let cat_g = cat_c < 0.0;

// 分母 = 当前分析师数量
let n = 10.0;

let good_count = 0;       // confidence >= 50
let valid_count = 0;      // confidence >= 0
let gap_count = 0;        // 数据完全缺失
let total_conf = 0.0;     // 累加有效 confidence

if mk_c >= 0.0 { valid_count += 1; total_conf += mk_c; if mk_c >= 50.0 { good_count += 1; } } else { gap_count += 1; }
if sent_c >= 0.0 { valid_count += 1; total_conf += sent_c; if sent_c >= 50.0 { good_count += 1; } } else { gap_count += 1; }
if news_c >= 0.0 { valid_count += 1; total_conf += news_c; if news_c >= 50.0 { good_count += 1; } } else { gap_count += 1; }
if fund_c >= 0.0 { valid_count += 1; total_conf += fund_c; if fund_c >= 50.0 { good_count += 1; } } else { gap_count += 1; }
if pol_c >= 0.0 { valid_count += 1; total_conf += pol_c; if pol_c >= 50.0 { good_count += 1; } } else { gap_count += 1; }
if hm_c >= 0.0 { valid_count += 1; total_conf += hm_c; if hm_c >= 50.0 { good_count += 1; } } else { gap_count += 1; }
if lk_c >= 0.0 { valid_count += 1; total_conf += lk_c; if lk_c >= 50.0 { good_count += 1; } } else { gap_count += 1; }
if res_c >= 0.0 { valid_count += 1; total_conf += res_c; if res_c >= 50.0 { good_count += 1; } } else { gap_count += 1; }
if sec_c >= 0.0 { valid_count += 1; total_conf += sec_c; if sec_c >= 50.0 { good_count += 1; } } else { gap_count += 1; }
if cat_c >= 0.0 { valid_count += 1; total_conf += cat_c; if cat_c >= 50.0 { good_count += 1; } } else { gap_count += 1; }

let avg_conf = if valid_count > 0 { total_conf / valid_count.to_float() } else { 0.0 };

// 评分：valid_count 占比 × 50 + good_count 占比 × 50
// (P2 修复 2026-07-24: 之前为 60/40，单个分析师缺失时 valid_count=9 仍拿 54 分，
// 再扣 4 分 gap 惩罚后还有 86+ → A 级，掩盖了节点失败的事实。改为 50/50 均衡权重)
let score = (valid_count.to_float() / n) * 50.0 + (good_count.to_float() / n) * 50.0;

// gap 惩罚（P2 修复: 大幅提升惩罚力度，单个缺口就能显著拉低评分）
if gap_count >= 4 { score = score - 30.0; }
else if gap_count >= 3 { score = score - 25.0; }
else if gap_count >= 2 { score = score - 18.0; }
else if gap_count >= 1 { score = score - 12.0; }

// 信心惩罚
if good_count == 0 { score = score - 30.0; }
else if good_count <= 2 { score = score - 15.0; }

// 平均置信度修正
if avg_conf < 20.0 { score = score - 10.0; }
else if avg_conf > 75.0 { score = score + 5.0; }

score = clamp(score, 0.0, 100.0);
let final_score = (score * 10.0).round() / 10.0;

let grade = if final_score >= 85.0 { "A" }
    else if final_score >= 65.0 { "B" }
    else if final_score >= 45.0 { "C" }
    else if final_score >= 25.0 { "D" }
    else { "F" };

// ── 诊断条目生成 ──
fn diag_for(c, name, expected) {
    let status = if c < 0.0 { "missing" } else if c < 50.0 { "low" } else { "normal" };
    let reason = if c < 0.0 {
        "节点未输出 VERDICT 或 confidence 字段缺失（可能原因：LLM 超时/截断/解析失败/strict_mode 降级未注入 confidence）"
    } else if c < 50.0 {
        `置信度自评 ${c}（可能原因：上游工具数据不完整 / strict_mode 降级兜底 / 分析师对数据覆盖度不满意）`
    } else { "" };
    #{
        "name": name,
        "expected_data": expected,
        "confidence": c,
        "status": status,
        "gap_reason": reason
    }
}

fn join_arr(arr, sep) {
    let s = "";
    for i in 0..arr.len() {
        if i > 0 { s += sep; }
        s += arr[i];
    }
    s
}

let diagnostics = #{
    "mk": diag_for(mk_c, "技术面分析师", "K线形态、MACD/RSI、支撑阻力位、成交量（t-market-data）"),
    "sent": diag_for(sent_c, "情绪面分析师", "市场情绪、资金流向、散户/机构态度（t-sentiment-data）"),
    "news": diag_for(news_c, "新闻面分析师", "新闻公告影响评估（t-news-data: get_stock_news）"),
    "fund": diag_for(fund_c, "基本面分析师", "PE/PB/ROE、营收利润、资产负债（t-fundamentals-data）"),
    "pol": diag_for(pol_c, "政策面分析师", "宏观政策与行业政策影响（t-policy-data）"),
    "hm": diag_for(hm_c, "资金面分析师", "游资动向、主力资金净流入（t-hotmoney-data: get_stock_money_flow）"),
    "lk": diag_for(lk_c, "解禁观察员", "解禁减持、质押风险、大宗交易（t-lockup-data）"),
    "res": diag_for(res_c, "研报分析师", "券商研报观点汇总、目标价（t-research-data）"),
    "sec": diag_for(sec_c, "行业分析师", "行业景气度、轮动分析（t-sector-data）"),
    "cat": diag_for(cat_c, "催化剂分析师", "催化剂与叙事完整度、公告关键词（t-catalyst-data）")
};

let missing_analysts = [];
let low_confidence_analysts = [];
if mk_c < 0.0 { missing_analysts.push("技术面"); } else if mk_c < 50.0 { low_confidence_analysts.push("技术面"); }
if sent_c < 0.0 { missing_analysts.push("情绪面"); } else if sent_c < 50.0 { low_confidence_analysts.push("情绪面"); }
if news_c < 0.0 { missing_analysts.push("新闻面"); } else if news_c < 50.0 { low_confidence_analysts.push("新闻面"); }
if fund_c < 0.0 { missing_analysts.push("基本面"); } else if fund_c < 50.0 { low_confidence_analysts.push("基本面"); }
if pol_c < 0.0 { missing_analysts.push("政策面"); } else if pol_c < 50.0 { low_confidence_analysts.push("政策面"); }
if hm_c < 0.0 { missing_analysts.push("资金面"); } else if hm_c < 50.0 { low_confidence_analysts.push("资金面"); }
if lk_c < 0.0 { missing_analysts.push("解禁观察"); } else if lk_c < 50.0 { low_confidence_analysts.push("解禁观察"); }
if res_c < 0.0 { missing_analysts.push("研报"); } else if res_c < 50.0 { low_confidence_analysts.push("研报"); }
if sec_c < 0.0 { missing_analysts.push("行业"); } else if sec_c < 50.0 { low_confidence_analysts.push("行业"); }
if cat_c < 0.0 { missing_analysts.push("催化剂"); } else if cat_c < 50.0 { low_confidence_analysts.push("催化剂"); }

let low_count = 10 - good_count - gap_count;
let summary_text = if grade == "F" || grade == "D" {
    let base = `数据质量 ${grade} 级（得分 ${final_score}）：${gap_count} 个分析师数据缺失，${low_count} 个低置信度，仅 ${good_count} 个高置信度。`;
    let miss_part = if missing_analysts.len() > 0 { `缺失：${join_arr(missing_analysts, "、")}。` } else { "" };
    let low_part = if low_confidence_analysts.len() > 0 { `低置信：${join_arr(low_confidence_analysts, "、")}。` } else { "" };
    `${base}${miss_part}${low_part}平均置信度 ${avg_conf}。建议检查对应分析师的上游工具数据源。`
} else {
    `数据质量 ${grade} 级（得分 ${final_score}）：${good_count}/10 高置信度，平均置信度 ${avg_conf}。`
};

#{
    "grade": grade,
    "score": final_score,
    "gap_count": gap_count,
    "good_count": good_count,
    "avg_confidence": (avg_conf * 10.0).round() / 10.0,
    "total_analysts": n,
    "diagnostics": diagnostics,
    "missing_analysts": missing_analysts,
    "low_confidence_analysts": low_confidence_analysts,
    "summary": summary_text
}
