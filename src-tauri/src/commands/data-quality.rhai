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
//   - 综合 score = report_quality × 0.45 + tool_credibility × 0.35 + factor_completeness × 0.20
//     （V42 调整: 加入因子完整度维度，权重重新分配为 45/35/20）
//   - t-scoring/t-valuation/t-risk 的 credibility 字段待 P3 接入后再升级 tool_credibility

// ── 辅助函数（供因子完整度评估使用）──
// present: 检查变量是否存在且非空
fn present(x) { type_of(x) != "()" }

// safe_parse: 安全解析 JSON 字符串，失败返回 ()
fn safe_parse(raw) {
    if !present(raw) { return (); }
    if type_of(raw) != "string" || raw == "" { return (); }
    try {
        json_parse(raw)
    } catch (err) {
        ()
    }
}

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

// extract_dir: 从 verdict map 中提取方向分值
// 返回 -1（看空/偏空）/ 0（中性/未知）/ +1（看多/偏多）
// 支持两种格式：
//   扁平: {"verdict":"看多", ...}
//   嵌套: {"report":"...", "verdict":{"verdict":"偏空"}} 或 {"direction":"多"}
fn extract_dir(v) {
    if type_of(v) != "map" { return 0; }
    let d = v["verdict"];
    if type_of(d) == "string" {
        if d.contains("看多") || d.contains("偏多") { return 1; }
        if d.contains("看空") || d.contains("偏空") { return -1; }
        return 0;
    }
    // 嵌套格式: verdict map 内再找 verdict/direction
    if type_of(d) == "map" {
        let inner = d["verdict"];
        if type_of(inner) == "string" {
            if inner.contains("看多") || inner.contains("偏多") { return 1; }
            if inner.contains("看空") || inner.contains("偏空") { return -1; }
        }
        let dir = d["direction"];
        if type_of(dir) == "string" {
            if dir.contains("多") { return 1; }
            if dir.contains("空") { return -1; }
        }
    }
    0
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

// is_untrusted: 检测分析师 verdict 是否被标记为不可信
// strict_mode 降级时 agent_executor 在节点顶层注入 __untrusted=true,
// 但 verdict map 本身可能不含此字段（__untrusted 在 NodeOutput 顶层）。
// data-quality.rhai 的 input_mapping 只映射了 .content.verdict,未映射节点顶层的 __untrusted,
// 因此此处额外检查 verdict map 内的 __untrusted 字段（部分降级路径会内嵌）。
// 修复(2026-07-29): 原代码完全忽略 __untrusted 标记,导致 strict_mode 降级的分析师
//   被当成可信分析师参与评分,tool_credibility_score 虚高。
fn is_untrusted(v) {
    if type_of(v) == "map" {
        let flag = v["__untrusted"];
        if type_of(flag) == "bool" { return flag; }
    }
    false
}

// count_chars: 估算报告字数（中英文混合）
// P0 修复(2026-08-09): Rhai 默认 Engine（Engine::new() + register_common_functions，
// 未注册 StringPackage）下 String::replace 实际解析为返回 unit 的降级函数，
// 原实现 text.replace(" ", "") 链式调用 100% 报 "Function not found: replace"，
// 导致 count_chars 失败 → report_quality 失败 → 整个 data-quality 脚本执行失败
// → quality-gate 无 grade → 全部走 quality-fallback 保守路径（历史最大缺陷）。
// 现改为纯 len()（UTF-8 字节数，BasicPackage 内置）估算：
// 中文 3 字节/字符、英文 1 字节/字符，A 股报告以中文为主 → 平均按 2.0 字节/字符
// （比原 /2.5 更接近中英混合比例；纯英文从低估 60% 收窄到 50%，分段阈值
//  200/500 对 90% 中文报告依然准确）。不再依赖 replace/去空白。
fn count_chars(text) {
    if type_of(text) != "string" || text == "" { return 0; }
    let bytes = text.len();
    // bytes / 2.0（整数运算：×50/100，避免浮点转换）
    let approx = bytes * 50 / 100;
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

// ── P2-3 新增(2026-08-09): Schema 驱动的深度字段校验 ──
// 从 verdict map 中提取 bull_score 数值
fn extract_bull_score(v) {
    if type_of(v) != "map" { return (); }
    let bs = v["bull_score"];
    if type_of(bs) == "f64" { return bs; }
    if type_of(bs) == "i64" || type_of(bs) == "int" { return bs.to_float(); }
    // 嵌套格式 fallback
    let inner = v["verdict"];
    if type_of(inner) == "map" {
        let ibs = inner["bull_score"];
        if type_of(ibs) == "f64" { return ibs; }
        if type_of(ibs) == "i64" || type_of(ibs) == "int" { return ibs.to_float(); }
    }
    ()
}

// 从 verdict map 中提取 bear_score 数值
fn extract_bear_score(v) {
    if type_of(v) != "map" { return (); }
    let bs = v["bear_score"];
    if type_of(bs) == "f64" { return bs; }
    if type_of(bs) == "i64" || type_of(bs) == "int" { return bs.to_float(); }
    // 嵌套格式 fallback
    let inner = v["verdict"];
    if type_of(inner) == "map" {
        let ibs = inner["bear_score"];
        if type_of(ibs) == "f64" { return ibs; }
        if type_of(ibs) == "i64" || type_of(ibs) == "int" { return ibs.to_float(); }
    }
    ()
}

// 从 verdict map 中提取 verdict 字符串（用于逻辑一致性检查）
fn extract_verdict_str(v) {
    if type_of(v) != "map" { return ""; }
    let d = v["verdict"];
    if type_of(d) == "string" { return d; }
    // 嵌套格式: verdict map 内再找 verdict
    if type_of(d) == "map" {
        let inner = d["verdict"];
        if type_of(inner) == "string" { return inner; }
    }
    ""
}

// validate_verdict_schema: 深度字段校验与自动归一化修复
// 检查：1) bull_score + bear_score 自洽性 2) verdict 与评分方向一致性 3) confidence 范围
// 返回修复后的 verdict_data 和问题列表
fn validate_and_fix_verdict_schema(verdict_data, analyst_name) {
    let issues = [];
    let fixed_data = verdict_data; // 准备进行修改的副本

    if type_of(verdict_data) != "map" { return { "data": fixed_data, "issues": issues }; }

    // 1. bull_score + bear_score 自洽性检查与修复
    let bs = extract_bull_score(verdict_data);
    let bes = extract_bear_score(verdict_data);
    
    if present(bs) && present(bes) {
        let sum = bs + bes;
        // 如果总和偏离 100 超过 10，触发告警
        if sum < 90.0 || sum > 110.0 {
            issues.push(`[${analyst_name}] 评分不自洽: bull_score + bear_score = ${sum} (应约为 100)`);
            
            // 自动归一化修复（强制修复，确保数据自洽）
            // 防止除以0
            if sum > 0.0 {
                let new_bs = round((bs / sum) * 100.0);
                let new_bes = round((bes / sum) * 100.0);
                
                // 修正后的分数写回
                fixed_data["bull_score"] = new_bs;
                fixed_data["bear_score"] = new_bes;
                
                issues.push(`[${analyst_name}] 已自动修复评分: bull_score -> ${new_bs}, bear_score -> ${new_bes} (归一化至 100)`);
            }
        } else if sum < 95.0 || sum > 105.0 {
            // 轻微偏离（5-10之间），仅告警，不强制修复
            issues.push(`[${analyst_name}] 评分轻微偏离: bull_score + bear_score = ${sum} (建议约为 100)`);
        }
    }

    // 2. verdict 与评分方向一致性检查
    let v = extract_verdict_str(verdict_data);
    if v != "" && present(bs) && present(bes) {
        let is_bull = v.contains("看多") || v.contains("看涨") || v.contains("偏多");
        let is_bear = v.contains("看空") || v.contains("看跌") || v.contains("偏空");
        
        // 如果评分被修复，使用修复后的分数进行判断
        let effective_bs = present(fixed_data["bull_score"]) ? fixed_data["bull_score"].to_float() : bs;
        let effective_bes = present(fixed_data["bear_score"]) ? fixed_data["bear_score"].to_float() : bes;

        if is_bull && effective_bs <= effective_bes {
            issues.push(`[${analyst_name}] 逻辑矛盾: verdict='${v}' 但 bull_score(${effective_bs}) <= bear_score(${effective_bes})`);
        }
        if is_bear && effective_bes <= effective_bs {
            issues.push(`[${analyst_name}] 逻辑矛盾: verdict='${v}' 但 bear_score(${effective_bes}) <= bull_score(${effective_bs})`);
        }
    }

    // 3. confidence 范围检查 (0-100)
    let conf = extract_conf(verdict_data);
    if conf >= 0.0 {
        if conf > 100.0 {
            issues.push(`[${analyst_name}] 置信度越界: confidence=${conf} (超出 0-100)`);
            // 自动修复 confidence
            fixed_data["confidence"] = 100.0;
        }
        // 极低置信度也告警
        if conf > 0.0 && conf < 10.0 {
            issues.push(`[${analyst_name}] 置信度极低: confidence=${conf}`);
        }
    }

    { "data": fixed_data, "issues": issues }
}

// 保留旧接口以兼容
fn validate_verdict_schema(verdict_data, analyst_name) {
    let result = validate_and_fix_verdict_schema(verdict_data, analyst_name);
    result["issues"]
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
    // P3-2 修复(2026-08-09): extract_conf 缺失时返回 -1.0，原 conf < 30.0 分支会把
    // -1.0 误判为"低置信+短报告"给 +5 奖励（方向相反）。加 conf >= 0.0 前置条件。
    let consistency_bonus = 0.0;
    if type_of(conf) == "f64" && conf >= 0.0 {
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

// ── V67 修复(2026-07-29): 检测分析师 __untrusted 标记 ──
// strict_mode 降级时 agent_executor 在 NodeOutput 顶层注入 __untrusted=true,
// data-quality.rhai 的 input_mapping 映射了 {id}.__untrusted → {id}_untrusted 字段。
// 同时保留 verdict map 内 __untrusted 检测作为兜底(部分降级路径会内嵌到 content)。
// 原代码完全忽略此标记,导致不可信分析师被当成可信分析师参与评分。
// 现单独统计 untrusted_analyst_count,用于:
//   1. 从 good_count 中排除不可信分析师(避免虚高)
//   2. 在 tool_credibility 中增加 untrusted 惩罚
let mk_u = is_untrusted(mk_verdict) || (present(mk_untrusted) && mk_untrusted == true);
let sent_u = is_untrusted(sent_verdict) || (present(sent_untrusted) && sent_untrusted == true);
let news_u = is_untrusted(news_verdict) || (present(news_untrusted) && news_untrusted == true);
let fund_u = is_untrusted(fund_verdict) || (present(fund_untrusted) && fund_untrusted == true);
let pol_u = is_untrusted(pol_verdict) || (present(pol_untrusted) && pol_untrusted == true);
let hm_u = is_untrusted(hm_verdict) || (present(hm_untrusted) && hm_untrusted == true);
let lk_u = is_untrusted(lk_verdict) || (present(lk_untrusted) && lk_untrusted == true);
let res_u = is_untrusted(res_verdict) || (present(res_untrusted) && res_untrusted == true);
let sec_u = is_untrusted(sec_verdict) || (present(sec_untrusted) && sec_untrusted == true);
let cat_u = is_untrusted(cat_verdict) || (present(cat_untrusted) && cat_untrusted == true);
let untrusted_analyst_count = 0;
if mk_u { untrusted_analyst_count += 1; }
if sent_u { untrusted_analyst_count += 1; }
if news_u { untrusted_analyst_count += 1; }
if fund_u { untrusted_analyst_count += 1; }
if pol_u { untrusted_analyst_count += 1; }
if hm_u { untrusted_analyst_count += 1; }
if lk_u { untrusted_analyst_count += 1; }
if res_u { untrusted_analyst_count += 1; }
if sec_u { untrusted_analyst_count += 1; }
if cat_u { untrusted_analyst_count += 1; }

// P2-3 修复(2026-08-09): 原 n=10 硬编码，DAG 增删分析师需手动同步（失配时
// report_quality_avg 分母、good_count<n/2 阈值、low_count 全部失真）。
// 改为从 verdict 输入动态推导（非 () 即视为该分析师有输出），自动适配 DAG 实际分析师数。
fn count_present(v) { if type_of(v) != "()" { 1 } else { 0 } }
let n_int = count_present(mk_verdict) + count_present(sent_verdict) + count_present(news_verdict)
    + count_present(fund_verdict) + count_present(pol_verdict) + count_present(hm_verdict)
    + count_present(lk_verdict) + count_present(res_verdict) + count_present(sec_verdict)
    + count_present(cat_verdict);
let n = n_int.to_float();

let good_count = 0;       // confidence >= 50 且 非 untrusted
let valid_count = 0;      // confidence >= 0 且 非 untrusted
let gap_count = 0;        // 数据完全缺失（confidence < 0）或 untrusted
let total_conf = 0.0;     // 累加有效 confidence（排除 untrusted）

// 修复(2026-07-29): untrusted 分析师不计入 valid/good_count,计入 gap_count
//   理由: strict_mode 降级的 confidence=50 是中性兜底,不代表真实数据质量
if mk_u || mk_c < 0.0 { gap_count += 1; } else { valid_count += 1; total_conf += mk_c; if mk_c >= 50.0 { good_count += 1; } }
if sent_u || sent_c < 0.0 { gap_count += 1; } else { valid_count += 1; total_conf += sent_c; if sent_c >= 50.0 { good_count += 1; } }
if news_u || news_c < 0.0 { gap_count += 1; } else { valid_count += 1; total_conf += news_c; if news_c >= 50.0 { good_count += 1; } }
if fund_u || fund_c < 0.0 { gap_count += 1; } else { valid_count += 1; total_conf += fund_c; if fund_c >= 50.0 { good_count += 1; } }
if pol_u || pol_c < 0.0 { gap_count += 1; } else { valid_count += 1; total_conf += pol_c; if pol_c >= 50.0 { good_count += 1; } }
if hm_u || hm_c < 0.0 { gap_count += 1; } else { valid_count += 1; total_conf += hm_c; if hm_c >= 50.0 { good_count += 1; } }
if lk_u || lk_c < 0.0 { gap_count += 1; } else { valid_count += 1; total_conf += lk_c; if lk_c >= 50.0 { good_count += 1; } }
if res_u || res_c < 0.0 { gap_count += 1; } else { valid_count += 1; total_conf += res_c; if res_c >= 50.0 { good_count += 1; } }
if sec_u || sec_c < 0.0 { gap_count += 1; } else { valid_count += 1; total_conf += sec_c; if sec_c >= 50.0 { good_count += 1; } }
if cat_u || cat_c < 0.0 { gap_count += 1; } else { valid_count += 1; total_conf += cat_c; if cat_c >= 50.0 { good_count += 1; } }

let avg_conf = if valid_count > 0 { total_conf / valid_count.to_float() } else { 0.0 };

// ── P1-B3: 算法化双维度评分（按设计文档公式）──
// report_quality_score: 10 个分析师报告质量均值（缺失报告按 0 分拉低均值）
// tool_credibility_score: 基于已有 avg_conf + gap/good_count 指标算法化
// score = report × 0.6 + tool × 0.4（替代旧版 valid/good_count 占比 + 阶梯惩罚）
let report_quality_sum = mk_q + sent_q + news_q + fund_q + pol_q + hm_q + lk_q + res_q + sec_q + cat_q;
// P2-3 修复(2026-08-09): n 动态化后需防除零（全部 verdict 缺失时 n==0）
let report_quality_avg = if n > 0.0 { report_quality_sum / n } else { 0.0 };
let tool_credibility_score = tool_credibility(avg_conf, good_count, gap_count, n_int);

// ── P2-2 修复(2026-08-09): 分析师方向矛盾检测（升级为加权模式） ──
// data-quality-inspector.md 职责第 3 条要求"分析师结论是否相互矛盾"，
// 但原 Rhai 版只用了 confidence，verdict 方向字段被完全丢弃。
// 现解析 10 个分析师的方向分值（看多/偏多→+1，中性→0，看空/偏空→-1），
// untrusted 分析师方向不计（数据不可信）。
// V2 升级(2026-08-09): 从简单计数升级为加权评估（结合 confidence 权重），
// 只有当两边都有显著的高置信度观点时，才判定为严重冲突。
let mk_dir = if mk_u { 0 } else { extract_dir(mk_verdict) };
let sent_dir = if sent_u { 0 } else { extract_dir(sent_verdict) };
let news_dir = if news_u { 0 } else { extract_dir(news_verdict) };
let fund_dir = if fund_u { 0 } else { extract_dir(fund_verdict) };
let pol_dir = if pol_u { 0 } else { extract_dir(pol_verdict) };
let hm_dir = if hm_u { 0 } else { extract_dir(hm_verdict) };
let lk_dir = if lk_u { 0 } else { extract_dir(lk_verdict) };
let res_dir = if res_u { 0 } else { extract_dir(res_verdict) };
let sec_dir = if sec_u { 0 } else { extract_dir(sec_verdict) };
let cat_dir = if cat_u { 0 } else { extract_dir(cat_verdict) };

// 基础计数（保留用于展示）
let bull_dir_count = 0;
let bear_dir_count = 0;
if mk_dir > 0 { bull_dir_count += 1; } else if mk_dir < 0 { bear_dir_count += 1; }
if sent_dir > 0 { bull_dir_count += 1; } else if sent_dir < 0 { bear_dir_count += 1; }
if news_dir > 0 { bull_dir_count += 1; } else if news_dir < 0 { bear_dir_count += 1; }
if fund_dir > 0 { bull_dir_count += 1; } else if fund_dir < 0 { bear_dir_count += 1; }
if pol_dir > 0 { bull_dir_count += 1; } else if pol_dir < 0 { bear_dir_count += 1; }
if hm_dir > 0 { bull_dir_count += 1; } else if hm_dir < 0 { bear_dir_count += 1; }
if lk_dir > 0 { bull_dir_count += 1; } else if lk_dir < 0 { bear_dir_count += 1; }
if res_dir > 0 { bull_dir_count += 1; } else if res_dir < 0 { bear_dir_count += 1; }
if sec_dir > 0 { bull_dir_count += 1; } else if sec_dir < 0 { bear_dir_count += 1; }
if cat_dir > 0 { bull_dir_count += 1; } else if cat_dir < 0 { bear_dir_count += 1; }

// 加权方向冲突检测（结合 confidence 权重）
let weighted_bull_score = 0.0;
let weighted_bear_score = 0.0;
// mk
if !mk_u && mk_c > 0.0 {
    if mk_dir > 0 { weighted_bull_score += mk_c; }
    else if mk_dir < 0 { weighted_bear_score += mk_c; }
}
// sent
if !sent_u && sent_c > 0.0 {
    if sent_dir > 0 { weighted_bull_score += sent_c; }
    else if sent_dir < 0 { weighted_bear_score += sent_c; }
}
// news
if !news_u && news_c > 0.0 {
    if news_dir > 0 { weighted_bull_score += news_c; }
    else if news_dir < 0 { weighted_bear_score += news_c; }
}
// fund
if !fund_u && fund_c > 0.0 {
    if fund_dir > 0 { weighted_bull_score += fund_c; }
    else if fund_dir < 0 { weighted_bear_score += fund_c; }
}
// pol
if !pol_u && pol_c > 0.0 {
    if pol_dir > 0 { weighted_bull_score += pol_c; }
    else if pol_dir < 0 { weighted_bear_score += pol_c; }
}
// hm
if !hm_u && hm_c > 0.0 {
    if hm_dir > 0 { weighted_bull_score += hm_c; }
    else if hm_dir < 0 { weighted_bear_score += hm_c; }
}
// lk
if !lk_u && lk_c > 0.0 {
    if lk_dir > 0 { weighted_bull_score += lk_c; }
    else if lk_dir < 0 { weighted_bear_score += lk_c; }
}
// res
if !res_u && res_c > 0.0 {
    if res_dir > 0 { weighted_bull_score += res_c; }
    else if res_dir < 0 { weighted_bear_score += res_c; }
}
// sec
if !sec_u && sec_c > 0.0 {
    if sec_dir > 0 { weighted_bull_score += sec_c; }
    else if sec_dir < 0 { weighted_bear_score += sec_c; }
}
// cat
if !cat_u && cat_c > 0.0 {
    if cat_dir > 0 { weighted_bull_score += cat_c; }
    else if cat_dir < 0 { weighted_bear_score += cat_c; }
}

// 严重冲突判定：两边加权分都 >= 100
let severe_direction_conflict = weighted_bull_score >= 100.0 && weighted_bear_score >= 100.0;
// 一般冲突判定（保留基础计数逻辑作为兜底）
let basic_direction_conflict = bull_dir_count >= 2 && bear_dir_count >= 2;
let direction_conflict = severe_direction_conflict || basic_direction_conflict;
if direction_conflict {
    // 严重冲突扣更多分
    if severe_direction_conflict {
        tool_credibility_score = clamp(tool_credibility_score - 20.0, 0.0, 100.0);
    } else {
        tool_credibility_score = clamp(tool_credibility_score - 15.0, 0.0, 100.0);
    }
}

// ── P2-3 新增(2026-08-09): 深度字段校验（Schema Validation） ──
// 对每个分析师的 verdict 数据进行深度校验：
//   1. bull_score + bear_score 是否自洽（应约为 100）
//   2. verdict 字符串与评分方向是否逻辑一致
//   3. confidence 是否在合法范围内
// 所有问题收集到 field_validation_issues 数组，用于：
//   - 额外扣分（每个问题扣 3 分）
//   - 追加到 warnings 供用户查看
let field_validation_issues = [];
// 技术面
if !mk_u {
    let mk_issues = validate_verdict_schema(mk_verdict, "技术面");
    for i in 0..mk_issues.len() { field_validation_issues.push(mk_issues[i]); }
}
// 情绪面
if !sent_u {
    let sent_issues = validate_verdict_schema(sent_verdict, "情绪面");
    for i in 0..sent_issues.len() { field_validation_issues.push(sent_issues[i]); }
}
// 新闻面
if !news_u {
    let news_issues = validate_verdict_schema(news_verdict, "新闻面");
    for i in 0..news_issues.len() { field_validation_issues.push(news_issues[i]); }
}
// 基本面
if !fund_u {
    let fund_issues = validate_verdict_schema(fund_verdict, "基本面");
    for i in 0..fund_issues.len() { field_validation_issues.push(fund_issues[i]); }
}
// 政策面
if !pol_u {
    let pol_issues = validate_verdict_schema(pol_verdict, "政策面");
    for i in 0..pol_issues.len() { field_validation_issues.push(pol_issues[i]); }
}
// 资金面
if !hm_u {
    let hm_issues = validate_verdict_schema(hm_verdict, "资金面");
    for i in 0..hm_issues.len() { field_validation_issues.push(hm_issues[i]); }
}
// 解禁观察
if !lk_u {
    let lk_issues = validate_verdict_schema(lk_verdict, "解禁观察");
    for i in 0..lk_issues.len() { field_validation_issues.push(lk_issues[i]); }
}
// 研报
if !res_u {
    let res_issues = validate_verdict_schema(res_verdict, "研报");
    for i in 0..res_issues.len() { field_validation_issues.push(res_issues[i]); }
}
// 行业
if !sec_u {
    let sec_issues = validate_verdict_schema(sec_verdict, "行业");
    for i in 0..sec_issues.len() { field_validation_issues.push(sec_issues[i]); }
}
// 催化剂
if !cat_u {
    let cat_issues = validate_verdict_schema(cat_verdict, "催化剂");
    for i in 0..cat_issues.len() { field_validation_issues.push(cat_issues[i]); }
}

// 字段校验问题扣分（每个问题扣 3 分，最多扣 15 分）
let validation_penalty = field_validation_issues.len() * 3.0;
if validation_penalty > 15.0 { validation_penalty = 15.0; }
if validation_penalty > 0.0 {
    tool_credibility_score = clamp(tool_credibility_score - validation_penalty, 0.0, 100.0);
}

// ── 因子数据完整度（0.0 ~ 1.0）──
// 调用 Rust 函数 pm_compute_factor_completeness 评估 10 个因子的数据存在性。
// 因子缺失也是数据质量问题的一部分，纳入综合评分可避免「分析师报告写得好看但因子数据全缺」的虚高评分。
// P1-1 修复(2026-08-09): 注入的 money_flow/lockup_bundle/announcements 经 code_executor
// 的 resolve_var_path（auto_parse_value 解析 JSON 字符串为对象）+ json_value_to_dynamic
// 后是 **Rhai map**（ToolNode result 的 JSON 字符串已被解析为对象），
// 原 `type_of(x) == "string"` 前置判断恒为 false → 三个解析块永不执行
// → 主力净流入/股东增减持/公告数据三因子恒缺失（factor_completeness 上限 0.7、
// 综合评分系统性偏低约 9 分，且此前 camelCase 字段名修复改的是不可达分支）。
// 现改为 map/string 双兼容：map 直接提取，string 先 safe_parse。

// 提取主力净流入（camelCase 优先 + snake_case 兜底）
fn extract_main_net_inflow(raw) {
    if !present(raw) { return (); }
    let mf = if type_of(raw) == "string" { safe_parse(raw) } else { raw };
    if type_of(mf) == "map" {
        let v = mf["mainNetInflow"];
        if type_of(v) == "()" { v = mf["main_net_inflow"]; }
        if type_of(v) == "f64" { return v; }
        if type_of(v) == "i64" || type_of(v) == "int" { return v.to_float(); }
    }
    ()
}

// 提取股东增减持交易条数
fn extract_shareholder_trades_len(raw) {
    if !present(raw) { return (); }
    let lb = if type_of(raw) == "string" { safe_parse(raw) } else { raw };
    if type_of(lb) == "map" {
        let trades = lb["shareholder_trades"];
        if type_of(trades) == "array" { return trades.len(); }
    }
    ()
}

// 提取公告条数
fn extract_announcements_len(raw) {
    if !present(raw) { return (); }
    let ann = if type_of(raw) == "string" { safe_parse(raw) } else { raw };
    if type_of(ann) == "array" { return ann.len(); }
    ()
}

let mf_main_net_inflow = extract_main_net_inflow(money_flow);
let lb_shareholder_trades_len = extract_shareholder_trades_len(lockup_bundle);
let ann_count = extract_announcements_len(announcements);
// P2-2 修复(2026-08-09): trader_direction 契约为字符串（trader.md "看多|看空|中性"），
// 但 LLM 若输出嵌套 map（verdict: {...}），Rhai map 传给 pm_compute_factor_completeness
// 的 Option<&str> 参数会报类型转换错误 → 整个脚本失败 → quality-gate 走 default →
// quality-fallback 误降级。类型防御：非字符串按缺失（unit）处理。
let trader_direction_safe = if type_of(trader_direction) == "string" { trader_direction } else { () };
let factor_completeness = pm_compute_factor_completeness(
    total_score, consensus_score, catalyst_level,
    risk_volatility, valuation_dcf_upside, trader_direction_safe,
    mf_main_net_inflow, lb_shareholder_trades_len,
    ann_count, pace_signal
);
let factor_completeness_pct = (factor_completeness * 100.0 * 10.0).round() / 10.0;

// ── 构建缺失因子列表（供前端 UI 展示）──
let missing_factors = [];
if !present(total_score) || total_score <= 0.0 { missing_factors.push("技术面评分"); }
if !present(consensus_score) || consensus_score <= 0.0 { missing_factors.push("共识评分"); }
if !present(catalyst_level) || catalyst_level == "" || catalyst_level == "无" { missing_factors.push("催化剂等级"); }
if !present(risk_volatility) || risk_volatility <= 0.0 { missing_factors.push("风险波动率"); }
if !present(valuation_dcf_upside) { missing_factors.push("估值上行空间"); }
// P2-2 修复(2026-08-09): 用类型防御后的 trader_direction_safe 判断（map 恒非空、"" 判断对 map 恒 false）
if !present(trader_direction_safe) || trader_direction_safe == "" { missing_factors.push("交易方向"); }
// P2-1(2026-08-09): 与 Rust compute_factor_completeness 口径对齐——主力净流入=0 视为中性无信号（缺失）
if !present(mf_main_net_inflow) || mf_main_net_inflow == 0.0 { missing_factors.push("主力净流入"); }
if !present(lb_shareholder_trades_len) || lb_shareholder_trades_len <= 0 { missing_factors.push("股东增减持"); }
if !present(ann_count) || ann_count <= 0 { missing_factors.push("公告数据"); }
// P2-1(2026-08-09): PACE 信号需在 [-1,1] 值域内才算有效
if !present(pace_signal) || pace_signal < -1.0 || pace_signal > 1.0 { missing_factors.push("PACE情绪"); }

// 综合评分公式：报告质量 35% + 工具可信度 35% + 因子完整度 30%
// V67 修复(2026-07-29): 因子完整度权重从 0.20 提高到 0.30。
//   原权重 0.20 时,5/10 因子缺失(factor=0.5)仍可达 A 级(score≥85):
//     100×0.45 + 100×0.35 + 50×0.20 = 45+35+10 = 90 → A 级
//   提高到 0.30 后,5/10 因子缺失:
//     100×0.35 + 100×0.35 + 50×0.30 = 35+35+15 = 85 → A 级下限(临界)
//   6/10 因子缺失(factor=0.4):
//     100×0.35 + 100×0.35 + 40×0.30 = 35+35+12 = 82 → B 级 ✓
//   这样"因子缺失仍评 A 级"的问题被缓解(需 ≤5 个因子缺失且报告+工具满分才临界 A)。
// 同时降低 report_quality 权重 0.45→0.35,因为报告写得好不等于数据质量好。
let score = report_quality_avg * 0.35 + tool_credibility_score * 0.35 + factor_completeness * 100.0 * 0.30;
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
// P3-4 修复(2026-08-09): strict_mode 降级的分析师 confidence=50（中性兜底）原显示
// "normal"，与 untrusted 语义矛盾。新增 untrusted 参数，降级分析师显示 "untrusted"。
fn diag_for(c, name, expected, untrusted) {
    let status = if untrusted { "untrusted" }
        else if c < 0.0 { "missing" }
        else if c < 50.0 { "low" }
        else { "normal" };
    let reason = if untrusted {
        "strict_mode 降级（LLM 不可用/超时，confidence 为中性兜底，数据不可信）"
    } else if c < 0.0 {
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
    "mk": diag_for(mk_c, "技术面分析师", "K线形态、MACD/RSI、支撑阻力位、成交量（t-market-data）", mk_u),
    "sent": diag_for(sent_c, "情绪面分析师", "市场情绪、资金流向、散户/机构态度（t-sentiment-data）", sent_u),
    "news": diag_for(news_c, "新闻面分析师", "新闻公告影响评估（t-news-data: get_stock_news）", news_u),
    "fund": diag_for(fund_c, "基本面分析师", "PE/PB/ROE、营收利润、资产负债（t-fundamentals-data）", fund_u),
    "pol": diag_for(pol_c, "政策面分析师", "宏观政策与行业政策影响（t-policy-data）", pol_u),
    "hm": diag_for(hm_c, "资金面分析师", "游资动向、主力资金净流入（t-hotmoney-data: get_stock_money_flow）", hm_u),
    "lk": diag_for(lk_c, "解禁观察员", "解禁减持、质押风险、大宗交易（t-lockup-data）", lk_u),
    "res": diag_for(res_c, "研报分析师", "券商研报观点汇总、目标价（t-research-data）", res_u),
    "sec": diag_for(sec_c, "行业分析师", "行业景气度、轮动分析（t-sector-data）", sec_u),
    "cat": diag_for(cat_c, "催化剂分析师", "催化剂与叙事完整度、公告关键词（t-catalyst-data）", cat_u)
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

let low_count = n_int - good_count - gap_count;

// ── P1-B3: warnings 数组（基于报告质量检测的问题列表）──
let warnings = [];
if gap_count >= 1 { warnings.push(`${gap_count} 个分析师数据缺失`); }
if untrusted_analyst_count >= 1 { warnings.push(`${untrusted_analyst_count} 个分析师 strict_mode 降级(__untrusted)`); }
if avg_conf < 30.0 && avg_conf > 0.0 { warnings.push(`平均置信度过低 (${avg_conf})`); }
// P2-2 修复(2026-08-09): 方向冲突告警（升级为加权模式）
if direction_conflict {
    if severe_direction_conflict {
        warnings.push(`严重多空分歧：加权看多 ${weighted_bull_score.round()} vs 看空 ${weighted_bear_score.round()}（两边均有高置信度支持，工具可信度已扣 20 分）`);
    } else {
        warnings.push(`分析师结论方向冲突：${bull_dir_count} 个看多 vs ${bear_dir_count} 个看空（数据互相矛盾，工具可信度已扣 15 分）`);
    }
}
// P2-3 新增(2026-08-09): 深度字段校验告警
if field_validation_issues.len() > 0 {
    warnings.push(`深度字段校验发现 ${field_validation_issues.len()} 个问题（已扣 ${validation_penalty} 分）：`);
    for i in 0..field_validation_issues.len() {
        warnings.push(`  - ${field_validation_issues[i]}`);
    }
}
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
    let base = `数据质量 ${grade} 级（综合 ${final_score} = 报告 ${report_quality_score_final} × 0.35 + 工具 ${tool_credibility_score_final} × 0.35 + 因子完整度 ${factor_completeness_pct} × 0.30）：${gap_count} 个缺失，${low_count} 个低置信，${good_count} 个高置信，${untrusted_analyst_count} 个降级。`;
    let miss_part = if missing_analysts.len() > 0 { `缺失：${join_arr(missing_analysts, "、")}。` } else { "" };
    let low_part = if low_confidence_analysts.len() > 0 { `低置信：${join_arr(low_confidence_analysts, "、")}。` } else { "" };
    let val_part = if field_validation_issues.len() > 0 { `字段校验问题：${field_validation_issues.len()} 个。` } else { "" };
    `${base}${miss_part}${low_part}${val_part}平均置信度 ${avg_conf}。建议检查对应分析师的上游工具数据源。`
} else {
    let val_part = if field_validation_issues.len() > 0 { `字段校验问题：${field_validation_issues.len()} 个；` } else { "" };
    `数据质量 ${grade} 级（综合 ${final_score} = 报告 ${report_quality_score_final} × 0.35 + 工具 ${tool_credibility_score_final} × 0.35 + 因子完整度 ${factor_completeness_pct} × 0.30）：${good_count}/${n_int} 高置信度，${untrusted_analyst_count} 个降级，${val_part}平均置信度 ${avg_conf}。`
};

#{
    "grade": grade,
    "score": final_score,
    "report_quality_score": report_quality_score_final,
    "tool_credibility_score": tool_credibility_score_final,
    "factor_completeness_pct": factor_completeness_pct,
    "missing_factors": missing_factors,
    "gap_count": gap_count,
    "good_count": good_count,
    "avg_confidence": (avg_conf * 10.0).round() / 10.0,
    "total_analysts": n,
    // P2-2(2026-08-09): 方向冲突检测结果（升级为加权模式）
    "direction_conflict": direction_conflict,
    "severe_direction_conflict": severe_direction_conflict,
    "bull_dir_count": bull_dir_count,
    "bear_dir_count": bear_dir_count,
    "weighted_bull_score": (weighted_bull_score * 10.0).round() / 10.0,
    "weighted_bear_score": (weighted_bear_score * 10.0).round() / 10.0,
    // P2-3(2026-08-09): 深度字段校验结果
    "field_validation_issues": field_validation_issues,
    "field_validation_issue_count": field_validation_issues.len(),
    "field_validation_penalty": (validation_penalty * 10.0).round() / 10.0,
    "diagnostics": diagnostics,
    "missing_analysts": missing_analysts,
    "low_confidence_analysts": low_confidence_analysts,
    "warnings": warnings,
    "summary": summary_text
}
