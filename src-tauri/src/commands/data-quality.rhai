// data-quality.rhai — 数据质量确定性评分（替代原 LLM Agent，P1-4 修复）
//
// 输入：每个 stock-analyst 的 confidence + data_gaps 布尔值
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
//   - DAG 中新增 stock-analyst → 在此添加 {abbr}_confidence + {abbr}_data_gaps 两行
//   - 同步更新下方的 n 值（当前 DAG 有 10 个分析师 = 下文的 10 对字段）
//   - gap_count 阈值（4/3/2/1）是经验值，随分析师数量增大可等比调整

fn present(x) { type_of(x) != "()" }
fn safe_conf(v) { if type_of(v) == "f64" { v } else if type_of(v) == "int" { v.to_float() } else { 50.0 } }
fn safe_gap(v) { if type_of(v) == "bool" { v } else { false } }

// ── 分析师字段解析（每新增一个分析师，在此加一对 {abbr}_confidence + {abbr}_data_gaps）──
let mk_c = safe_conf(mk_confidence); let mk_g = safe_gap(mk_data_gaps);
let sent_c = safe_conf(sent_confidence); let sent_g = safe_gap(sent_data_gaps);
let news_c = safe_conf(news_confidence); let news_g = safe_gap(news_data_gaps);
let fund_c = safe_conf(fund_confidence); let fund_g = safe_gap(fund_data_gaps);
let pol_c = safe_conf(pol_confidence); let pol_g = safe_gap(pol_data_gaps);
let hm_c = safe_conf(hm_confidence); let hm_g = safe_gap(hm_data_gaps);
let lk_c = safe_conf(lk_confidence); let lk_g = safe_gap(lk_data_gaps);
let res_c = safe_conf(res_confidence); let res_g = safe_gap(res_data_gaps);
let sec_c = safe_conf(sec_confidence); let sec_g = safe_gap(sec_data_gaps);
let cat_c = safe_conf(cat_confidence); let cat_g = safe_gap(cat_data_gaps);

// 分母 = 当前分析师数量。新增分析师时同步更新此值。
// 注意: 此值必须与上方 {abbr}_confidence 字段对数量一致。
let n = 10.0;

let good_count = 0;
let gap_count = 0;
let total_conf = 0.0;

// 手动聚合以避开 for 循环的潜在问题
total_conf = total_conf + mk_c; if mk_c >= 50.0 { good_count += 1; } if mk_g { gap_count += 1; }
total_conf = total_conf + sent_c; if sent_c >= 50.0 { good_count += 1; } if sent_g { gap_count += 1; }
total_conf = total_conf + news_c; if news_c >= 50.0 { good_count += 1; } if news_g { gap_count += 1; }
total_conf = total_conf + fund_c; if fund_c >= 50.0 { good_count += 1; } if fund_g { gap_count += 1; }
total_conf = total_conf + pol_c; if pol_c >= 50.0 { good_count += 1; } if pol_g { gap_count += 1; }
total_conf = total_conf + hm_c; if hm_c >= 50.0 { good_count += 1; } if hm_g { gap_count += 1; }
total_conf = total_conf + lk_c; if lk_c >= 50.0 { good_count += 1; } if lk_g { gap_count += 1; }
total_conf = total_conf + res_c; if res_c >= 50.0 { good_count += 1; } if res_g { gap_count += 1; }
total_conf = total_conf + sec_c; if sec_c >= 50.0 { good_count += 1; } if sec_g { gap_count += 1; }
total_conf = total_conf + cat_c; if cat_c >= 50.0 { good_count += 1; } if cat_g { gap_count += 1; }

let avg_conf = total_conf / n;

// 基础分：good_count 占比 x 100
let score = (good_count.to_float() / n) * 100.0;

// gap 惩罚（阈值基于 n=10 调优，n 增大时可等比调整）
if gap_count >= 4 { score = score - 25.0; }
else if gap_count >= 3 { score = score - 18.0; }
else if gap_count >= 2 { score = score - 10.0; }
else if gap_count >= 1 { score = score - 4.0; }

// 信心惩罚
if good_count <= 1 { score = score - 40.0; }
else if good_count <= 3 { score = score - 20.0; }

// 平均置信度修正
if avg_conf < 30.0 { score = score - 10.0; }
else if avg_conf > 75.0 { score = score + 5.0; }

score = clamp(score, 0.0, 100.0);
let final_score = (score * 10.0).round() / 10.0;

// 等级映射
let grade = if final_score >= 85.0 { "A" }
    else if final_score >= 65.0 { "B" }
    else if final_score >= 45.0 { "C" }
    else if final_score >= 25.0 { "D" }
    else { "F" };

// JSON 输出（与 AgentNode 输出格式一致：grade + score）
#{
    "grade": grade,
    "score": final_score,
    "gap_count": gap_count,
    "good_count": good_count,
    "avg_confidence": (avg_conf * 10.0).round() / 10.0,
    "total_analysts": n
}
