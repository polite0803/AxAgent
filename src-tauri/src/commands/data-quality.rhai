// data-quality.rhai — 数据质量确定性评分（替代原 LLM Agent，P1-4 修复）
//
// 输入：每个 stock-analyst 的 confidence（来自 verdict.confidence）
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
//   - DAG 中新增 stock-analyst → 在此添加一行 {abbr}_confidence
//   - 同步更新下方的 n 值（当前 DAG 有 10 个分析师 = 下文的 10 个字段）
//   - gap_count 阈值（4/3/2/1）是经验值，随分析师数量增大可等比调整
//
// P0 修复（2026-07-23）:
//   - confidence 路径从 content.confidence 改为 content.verdict.confidence
//     （agent_executor 把 VERDICT tag 重构为 {"report":..., "verdict":{...}}）
//   - data_gaps 不再从上游读取（分析师不输出 if_data_gaps 字段），
//     改为用 confidence < 50 推断数据缺口，覆盖三种情况：
//     ① 节点失败 → confidence=0 ② strict_mode 降级 → confidence=0 ③ 分析师自评低置信度

// safe_conf: 缺失字段（unit/null）返回 0.0（零置信度），避免把"数据缺失"误判为"数据正常"
fn safe_conf(v) { if type_of(v) == "f64" { v } else if type_of(v) == "int" { v.to_float() } else { 0.0 } }

// ── 分析师 confidence 解析（每新增一个分析师，在此加一行 {abbr}_confidence）──
let mk_c = safe_conf(mk_confidence);
let sent_c = safe_conf(sent_confidence);
let news_c = safe_conf(news_confidence);
let fund_c = safe_conf(fund_confidence);
let pol_c = safe_conf(pol_confidence);
let hm_c = safe_conf(hm_confidence);
let lk_c = safe_conf(lk_confidence);
let res_c = safe_conf(res_confidence);
let sec_c = safe_conf(sec_confidence);
let cat_c = safe_conf(cat_confidence);

// gap 推断：confidence < 50 视为有数据缺口
let mk_g = mk_c < 50.0;
let sent_g = sent_c < 50.0;
let news_g = news_c < 50.0;
let fund_g = fund_c < 50.0;
let pol_g = pol_c < 50.0;
let hm_g = hm_c < 50.0;
let lk_g = lk_c < 50.0;
let res_g = res_c < 50.0;
let sec_g = sec_c < 50.0;
let cat_g = cat_c < 50.0;

// 分母 = 当前分析师数量。新增分析师时同步更新此值。
// 注意: 此值必须与上方 {abbr}_confidence 字段数量一致。
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
