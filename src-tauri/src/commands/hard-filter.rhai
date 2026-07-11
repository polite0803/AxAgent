// hard-filter.rhai — 瓶颈掘金：候选股硬裁决过滤层 (P2-2)
//
// 在 c-data-verifier 之后、a-decision-explainer 之前执行。
// 基于 serenity_* 参数对候选股清单做硬裁决过滤，确保不符合硬性约束的候选被剔除。
//
// 硬裁决规则（对应 S-200 系列规则码）：
//   S-200: stock_code 为空 → 剔除
//   S-201: exit_signals.overall_exit_urgency == "exit_now" → 剔除
//   S-202: PE(TTM) > serenity_max_pe（且无高增长豁免）→ 剔除
//   S-203: PB > serenity_max_pb → 剔除
//   S-204: 近3月涨幅 > serenity_max_3m_gain_pct → 剔除
//   S-205: 近12月涨幅 > serenity_max_12m_gain_pct → 剔除
//   S-206: 毛利率 < serenity_min_gross_margin 且 serenity_score < 80 → 剔除
//   S-207: 负债率 > serenity_max_debt_ratio 且 serenity_score < 85 → 剔除
//   S-208: 营收增速 < serenity_min_revenue_growth 且 serenity_score < 85 → 剔除
//
// 高增长豁免：营收增速 > serenity_growth_exempt_pct 且 PEG < 2 时，PE 超标可放行

fn is_unit(x) { type_of(x) == "()" }

// 安全取浮点数：支持 f64/i64/int/float，string 尝试 to_float，其他返回 default
fn safe_float(val, default) {
    if is_unit(val) { return default; }
    let t = type_of(val);
    if t == "f64" { return val; }
    if t == "i64" { return val.to_float(); }
    if t == "int" { return val.to_float(); }
    if t == "float" { return val; }
    default
}

fn get_nested(obj, key1, key2) {
    if is_unit(obj) { return (); }
    if type_of(obj) != "map" { return (); }
    let inner = obj[key1];
    if is_unit(inner) { return (); }
    if type_of(inner) != "map" { return (); }
    inner[key2]
}

fn main() {
    // 读取硬裁决参数（来自 workflow Variables，类型为 JSON 数字 → Rhai f64）
    let max_pe = safe_float(serenity_max_pe, 100.0);
    let max_pb = safe_float(serenity_max_pb, 10.0);
    let max_3m_gain = safe_float(serenity_max_3m_gain_pct, 150.0);
    let max_12m_gain = safe_float(serenity_max_12m_gain_pct, 300.0);
    let min_gross_margin = safe_float(serenity_min_gross_margin, 15.0);
    let max_debt_ratio = safe_float(serenity_max_debt_ratio, 85.0);
    let growth_exempt_pct = safe_float(serenity_growth_exempt_pct, 50.0);
    let min_revenue_growth = safe_float(serenity_min_revenue_growth, 0.0);

    // 解析 candidates（来自 c-data-verifier.result）
    let candidates_raw = if is_unit(candidates) {
        []
    } else if type_of(candidates) == "array" {
        candidates
    } else if type_of(candidates) == "map" {
        // c-data-verifier 输出可能是 { result: [...] } 或 { candidates: [...] }
        if candidates["result"] != () && type_of(candidates["result"]) == "array" {
            candidates["result"]
        } else if candidates["candidates"] != () && type_of(candidates["candidates"]) == "array" {
            candidates["candidates"]
        } else {
            []
        }
    } else {
        []
    };

    let filtered = [];
    let rejected = [];
    let total = candidates_raw.len();

    for c in candidates_raw {
        if type_of(c) != "map" {
            rejected.push(#{"stock_code": "", "stock_name": "", "reason": "候选非有效对象", "rule_code": "S-200"});
            continue;
        }

        let code = if c["stock_code"] != () { c["stock_code"].to_string() } else { "" };
        let name = if c["stock_name"] != () { c["stock_name"].to_string() } else { "" };

        // S-200: stock_code 为空
        if code == "" {
            rejected.push(#{"stock_code": "", "stock_name": name, "reason": "stock_code为空", "rule_code": "S-200"});
            continue;
        }

        // S-201: exit_now 信号
        let exit_urgency = get_nested(c, "exit_signals", "overall_exit_urgency");
        if !is_unit(exit_urgency) && exit_urgency.to_string() == "exit_now" {
            rejected.push(#{"stock_code": code, "stock_name": name, "reason": "退出信号触发(exit_now)", "rule_code": "S-201"});
            continue;
        }

        // V54 修复: 默认值 100（无数据时不触发过滤），避免缺失字段导致误杀
        let serenity_score = safe_float(c["serenity_score"], 100.0);

        // S-202: PE 超标（含高增长豁免）
        let pe = safe_float(c["pe_ttm"], safe_float(c["pe"], 0.0));
        let revenue_growth = safe_float(get_nested(c, "financial_snapshot", "revenue_growth_yoy"), safe_float(c["revenue_growth_yoy"], 0.0));
        if pe > max_pe && pe > 0.0 {
            // 高增长豁免：营收增速 > growth_exempt_pct 且 PEG < 2
            let peg = if revenue_growth > 0.0 { pe / revenue_growth } else { 999.0 };
            let exempted = revenue_growth > growth_exempt_pct && peg < 2.0;
            if !exempted {
                rejected.push(#{
                    "stock_code": code,
                    "stock_name": name,
                    "reason": "PE(" + pe.to_string() + ")超过上限(" + max_pe.to_string() + ")且无高增长豁免",
                    "rule_code": "S-202"
                });
                continue;
            }
        }

        // S-203: PB 超标
        let pb = safe_float(c["pb"], safe_float(c["pb_ttm"], 0.0));
        if pb > max_pb && pb > 0.0 {
            rejected.push(#{
                "stock_code": code,
                "stock_name": name,
                "reason": "PB(" + pb.to_string() + ")超过上限(" + max_pb.to_string() + ")",
                "rule_code": "S-203"
            });
            continue;
        }

        // S-204: 近3月涨幅超标
        let gain_3m = safe_float(c["gain_3m_pct"], safe_float(c["three_month_gain_pct"], 0.0));
        if gain_3m > max_3m_gain {
            rejected.push(#{
                "stock_code": code,
                "stock_name": name,
                "reason": "近3月涨幅(" + gain_3m.to_string() + "%)超过上限(" + max_3m_gain.to_string() + "%)",
                "rule_code": "S-204"
            });
            continue;
        }

        // S-205: 近12月涨幅超标
        let gain_12m = safe_float(c["gain_12m_pct"], safe_float(c["twelve_month_gain_pct"], 0.0));
        if gain_12m > max_12m_gain {
            rejected.push(#{
                "stock_code": code,
                "stock_name": name,
                "reason": "近12月涨幅(" + gain_12m.to_string() + "%)超过上限(" + max_12m_gain.to_string() + "%)",
                "rule_code": "S-205"
            });
            continue;
        }

        // S-206: 毛利率过低且评分不足
        let gross_margin = safe_float(get_nested(c, "financial_snapshot", "gross_margin"), safe_float(c["gross_margin"], 0.0));
        if gross_margin < min_gross_margin && serenity_score < 80.0 {
            rejected.push(#{
                "stock_code": code,
                "stock_name": name,
                "reason": "毛利率(" + gross_margin.to_string() + "%)低于下限(" + min_gross_margin.to_string() + "%)且评分不足80",
                "rule_code": "S-206"
            });
            continue;
        }

        // S-207: 负债率过高且评分不足
        let debt_ratio = safe_float(get_nested(c, "financial_snapshot", "debt_ratio"), safe_float(c["debt_ratio"], 0.0));
        if debt_ratio > max_debt_ratio && serenity_score < 85.0 {
            rejected.push(#{
                "stock_code": code,
                "stock_name": name,
                "reason": "负债率(" + debt_ratio.to_string() + "%)超过上限(" + max_debt_ratio.to_string() + "%)且评分不足85",
                "rule_code": "S-207"
            });
            continue;
        }

        // S-208: 营收增速过低且评分不足
        if revenue_growth < min_revenue_growth && serenity_score < 85.0 {
            rejected.push(#{
                "stock_code": code,
                "stock_name": name,
                "reason": "营收增速(" + revenue_growth.to_string() + "%)低于下限(" + min_revenue_growth.to_string() + "%)且评分不足85",
                "rule_code": "S-208"
            });
            continue;
        }

        // 通过所有硬裁决
        filtered.push(c);
    }

    #{
        "status": "ok",
        "filtered_candidates": filtered,
        "rejected_candidates": rejected,
        "filter_stats": #{
            "total_input": total,
            "total_filtered": filtered.len(),
            "total_rejected": rejected.len()
        }
    }
}
