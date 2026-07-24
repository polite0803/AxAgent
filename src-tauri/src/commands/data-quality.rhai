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
//
// P1-B3 升级（2026-07-24）:
//   - 算法化 report_quality_score（60% 程序化，基于报告字数+关键词覆盖+占位符检测）
//   - 算法化 tool_credibility_score（基于已有 avg_conf + gap_count + good_count）
//   - 综合 score = report_quality_avg × 0.6 + tool_credibility × 0.4（按设计文档公式）
//   - t-scoring/t-valuation/t-risk 的 credibility 字段待 P3 接入后再升级 tool_credibility

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

// extract_report: 从 *_report 字段提取字符串正文
// 输入可能是：string（直接字符串）、map（含 report 字段）、()（缺失）
// 返回 "" 表示无正文
fn extract_report(v) {
    if type_of(v) == "string" { return v; }
    if type_of(v) == "map" {
        let r = v["report"];
        if type_of(r) == "string" { return r; }
    }
    ""
}

// count_chars: 估算报告字数（中英文混合）
// 简单实现：len() 返回 UTF-8 字节数，中文 3 字节，英文 1 字节
// 经验值：A 股分析师报告 90% 中文，平均 1 字符 ≈ 2.5 字节
fn count_chars(text) {
    if type_of(text) != "string" || text == "" { return 0; }
    let trimmed = text.replace(" ", "").replace("\n", "").replace("\r", "").replace("\t", "");
    let bytes = trimmed.len();
    // UTF-8 字节数 → 估算字符数（中文 3 字节，英文 1 字节，平均 2.5）
    // 整数运算：bytes × 40 / 100 ≈ bytes / 2.5（避免 Rhai 类型转换语法限制）
    let approx = bytes * 40 / 100;
    if approx < 0 { 0 } else { approx }
}

// has_any: 检查 text 是否包含 keywords 数组中任一关键词
fn has_any(text, keywords) {
    if type_of(text) != "string" || text == "" { return false; }
    for i in 0..keywords.len() {
        if text.contains(keywords[i]) { return true; }
    }
    false
}

// count_keyword_groups: 统计报告覆盖的关键词组数（最多 5 组）
// 五组：趋势/估值/风险/建议/数据，每组命中任一关键词即 +1
fn count_keyword_groups(text) {
    if type_of(text) != "string" || text == "" { return 0; }
    let groups = 0;
    if has_any(text, ["趋势", "走势", "MACD", "RSI", "K线", "均线", "支撑", "阻力"]) { groups += 1; }
    if has_any(text, ["估值", "PE", "PB", "ROE", "市值", "NAV", "EPS"]) { groups += 1; }
    if has_any(text, ["风险", "下行", "回撤", "波动", "止损", "利空"]) { groups += 1; }
    if has_any(text, ["建议", "操作", "仓位", "买入", "卖出", "持有", "增持", "减持"]) { groups += 1; }
    if has_any(text, ["数据", "指标", "显示", "根据", "统计", "查询"]) { groups += 1; }
    groups
}

// has_placeholder: 检测报告是否含占位符/失败标记
fn has_placeholder(text) {
    if type_of(text) != "string" || text == "" { return false; }
    let markers = ["占位报告", "占位", "placeholder", "Placeholder", "PLACEHOLDER",
                   "无法获取", "数据缺失", "未注入", "agentrunner 未注入",
                   "DRY RUN", "TODO", "TBD"];
    has_any(text, markers)
}

// report_quality: 算法化报告质量分（0-100）
// 维度：字数(40) + 关键词覆盖(30) + 占位符扣分(-15) + 一致性奖励(10)
// 缺失报告（空字符串）返回 0
fn report_quality(text, conf) {
    if type_of(text) != "string" || text == "" { return 0.0; }

    let chars = count_chars(text);
    // 字数评分（40 分）
    let length_score = if chars >= 500 { 40.0 }
        else if chars >= 200 { 25.0 }
        else if chars >= 50 { 10.0 }
        else { 0.0 };

    // 关键词覆盖评分（30 分，5 组 × 6 分）
    let kw_groups = count_keyword_groups(text);
    let kw_score = kw_groups.to_float() * 6.0;

    // 占位符扣分（-15）
    let placeholder_penalty = if has_placeholder(text) { -15.0 } else { 0.0 };

    // confidence 一致性奖励（10 分）
    let consistency_bonus = 0.0;
    if type_of(conf) == "f64" {
        if conf >= 70.0 && chars >= 200 { consistency_bonus = 10.0; }
        else if conf < 30.0 && chars < 50 { consistency_bonus = 5.0; }
    }

    let raw = length_score + kw_score + placeholder_penalty + consistency_bonus;
    clamp(raw, 0.0, 100.0)
}

// tool_credibility: 算法化工具可信度分（0-100）
// 当前指标：avg_conf 基础 + gap 惩罚 + good_count 惩罚
// P3 升级方向：接入 t-scoring/t-valuation/t-risk 的 credibility 字段后替换此实现
fn tool_credibility(avg_conf, good_count, gap_count, n) {
    let base = if avg_conf > 0.0 { avg_conf } else { 30.0 };
    let penalty = 0.0;
    // gap 惩罚（按 gap_count 阶梯扣分，对齐原 V1 严格性）
    if gap_count >= 4 { penalty += 40.0; }
    else if gap_count >= 3 { penalty += 30.0; }
    else if gap_count >= 2 { penalty += 20.0; }
    else if gap_count >= 1 { penalty += 10.0; }
    // good_count 惩罚（高置信度分析师比例）
    if good_count == 0 { penalty += 20.0; }
    else if good_count < n / 2 { penalty += 15.0; }
    clamp(base - penalty, 0.0, 100.0)
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

// ── P1-B3: 分析师报告正文解析（用于算法化 report_quality_score）──
let mk_r = extract_report(mk_report);
let sent_r = extract_report(sent_report);
let news_r = extract_report(news_report);
let fund_r = extract_report(fund_report);
let pol_r = extract_report(pol_report);
let hm_r = extract_report(hm_report);
let lk_r = extract_report(lk_report);
let res_r = extract_report(res_report);
let sec_r = extract_report(sec_report);
let cat_r = extract_report(cat_report);

// 单个分析师报告质量分（缺失报告 → 0 分）
let mk_q = report_quality(mk_r, mk_c);
let sent_q = report_quality(sent_r, sent_c);
let news_q = report_quality(news_r, news_c);
let fund_q = report_quality(fund_r, fund_c);
let pol_q = report_quality(pol_r, pol_c);
let hm_q = report_quality(hm_r, hm_c);
let lk_q = report_quality(lk_r, lk_c);
let res_q = report_quality(res_r, res_c);
let sec_q = report_quality(sec_r, sec_c);
let cat_q = report_quality(cat_r, cat_c);

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
let n_int = 10;

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

// ── P1-B3: 算法化双维度评分（按设计文档公式）──
// report_quality_score: 10 个分析师报告质量均值（缺失报告按 0 分拉低均值）
// tool_credibility_score: 基于已有 avg_conf + gap/good_count 指标算法化
// score = report × 0.6 + tool × 0.4（替代旧版 valid/good_count 占比 + 阶梯惩罚）
let report_quality_sum = mk_q + sent_q + news_q + fund_q + pol_q + hm_q + lk_q + res_q + sec_q + cat_q;
let report_quality_avg = report_quality_sum / n;
let tool_credibility_score = tool_credibility(avg_conf, good_count, gap_count, n_int);

let score = report_quality_avg * 0.6 + tool_credibility_score * 0.4;
score = clamp(score, 0.0, 100.0);
let final_score = (score * 10.0).round() / 10.0;
let report_quality_score_final = (report_quality_avg * 10.0).round() / 10.0;
let tool_credibility_score_final = (tool_credibility_score * 10.0).round() / 10.0;

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

// ── P1-B3: warnings 数组（基于报告质量检测的问题列表）──
let warnings = [];
if gap_count >= 1 { warnings.push(`${gap_count} 个分析师数据缺失`); }
if avg_conf < 30.0 && avg_conf > 0.0 { warnings.push(`平均置信度过低 (${avg_conf})`); }
// 字数过低的报告（< 200 字符）
if mk_r != "" && count_chars(mk_r) < 200 { warnings.push("技术面报告字数不足 200"); }
if sent_r != "" && count_chars(sent_r) < 200 { warnings.push("情绪面报告字数不足 200"); }
if news_r != "" && count_chars(news_r) < 200 { warnings.push("新闻面报告字数不足 200"); }
if fund_r != "" && count_chars(fund_r) < 200 { warnings.push("基本面报告字数不足 200"); }
if pol_r != "" && count_chars(pol_r) < 200 { warnings.push("政策面报告字数不足 200"); }
if hm_r != "" && count_chars(hm_r) < 200 { warnings.push("资金面报告字数不足 200"); }
if lk_r != "" && count_chars(lk_r) < 200 { warnings.push("解禁观察报告字数不足 200"); }
if res_r != "" && count_chars(res_r) < 200 { warnings.push("研报报告字数不足 200"); }
if sec_r != "" && count_chars(sec_r) < 200 { warnings.push("行业报告字数不足 200"); }
if cat_r != "" && count_chars(cat_r) < 200 { warnings.push("催化剂报告字数不足 200"); }
// 占位符检测
if has_placeholder(mk_r) || has_placeholder(sent_r) || has_placeholder(news_r)
    || has_placeholder(fund_r) || has_placeholder(pol_r) || has_placeholder(hm_r)
    || has_placeholder(lk_r) || has_placeholder(res_r) || has_placeholder(sec_r)
    || has_placeholder(cat_r) {
    warnings.push("检测到占位符/失败标记（占位报告/placeholder/无法获取等）");
}

let summary_text = if grade == "F" || grade == "D" {
    let base = `数据质量 ${grade} 级（综合 ${final_score} = 报告质量 ${report_quality_score_final} × 0.6 + 工具可信度 ${tool_credibility_score_final} × 0.4）：${gap_count} 个缺失，${low_count} 个低置信，${good_count} 个高置信。`;
    let miss_part = if missing_analysts.len() > 0 { `缺失：${join_arr(missing_analysts, "、")}。` } else { "" };
    let low_part = if low_confidence_analysts.len() > 0 { `低置信：${join_arr(low_confidence_analysts, "、")}。` } else { "" };
    `${base}${miss_part}${low_part}平均置信度 ${avg_conf}。建议检查对应分析师的上游工具数据源。`
} else {
    `数据质量 ${grade} 级（综合 ${final_score} = 报告质量 ${report_quality_score_final} × 0.6 + 工具可信度 ${tool_credibility_score_final} × 0.4）：${good_count}/10 高置信度，平均置信度 ${avg_conf}。`
};

#{
    "grade": grade,
    "score": final_score,
    "report_quality_score": report_quality_score_final,
    "tool_credibility_score": tool_credibility_score_final,
    "gap_count": gap_count,
    "good_count": good_count,
    "avg_confidence": (avg_conf * 10.0).round() / 10.0,
    "total_analysts": n,
    "diagnostics": diagnostics,
    "missing_analysts": missing_analysts,
    "low_confidence_analysts": low_confidence_analysts,
    "warnings": warnings,
    "summary": summary_text
}
