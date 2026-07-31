// FIX-02 + P0-2: 财务数据交叉验证 CodeNode (Rhai)
// 输入:
//   candidates       — 来自 a-candidate-mapper 的候选股列表
//   tool_calls_made  — 来自 a-candidate-mapper 的工具调用记录（可选）
// 功能:
//   1. 对每个候选股的 financial_snapshot 做基本合理性验证（范围检查）
//   2. 交叉验证：检查 LLM 是否调用了 get_stock_financials，
//      若有调用，比对 LLM 输出的财务数据与工具返回的真实数据是否一致
//   3. 标记数据来源可信度
// 输出: 同结构 candidates 数组，每项增加 data_verification 字段

fn present(x) { type_of(x) != "()" }

let verified_candidates = [];

// S-M2 修复: candidates 路径可能因 LLM 输出格式不同而解析失败
// 路径1 (tool_call格式): candidates = a-candidate-mapper.content.arguments.candidates
// 路径2 (直接JSON格式): candidates_direct = a-candidate-mapper.content.candidates
// 路径3 (content整体, 2026-07-31 23:05 实锤): agent_executor 拆包 tool_json 后
//   final_content = args.to_string()，即 content 是 arguments 对象文本字符串
//   （{"candidates":[...], "summary":...}）或裸候选数组文本。此时路径1的
//   content.arguments 层不存在、路径2对字符串索引 candidates 失败 → 全 null →
//   data-verifier early return 空数组 → serenity 优先取 c-data-verifier 吞掉真候选。
//   直接对 content 整体 json_parse 兜底。
let candidates_from_raw = if present(candidates_raw) && type_of(candidates_raw) == "string" {
    let p = json_parse(candidates_raw);
    if type_of(p) == "array" { p }
    else if type_of(p) == "map" && p["candidates"] != () && type_of(p["candidates"]) == "array" {
        p["candidates"]
    } else { [] }
} else { [] };
// 选择第一个有效的数组
let effective_candidates = if present(candidates) && type_of(candidates) == "array" && candidates.len() > 0 {
    candidates
} else if present(candidates_direct) && type_of(candidates_direct) == "array" && candidates_direct.len() > 0 {
    print("[data-verifier] candidates 主路径为空，使用 candidates_direct fallback");
    candidates_direct
} else if candidates_from_raw.len() > 0 {
    print("[data-verifier] candidates 主路径为空，使用 content 整体 fallback");
    candidates_from_raw
} else {
    print("[data-verifier] candidates 和 candidates_direct 均为空");
    return verified_candidates;
};

// 非 bottleneck 策略跳过详细的财务交叉验证
let first_candidate = if effective_candidates.len() > 0 { effective_candidates[0] } else { () };
let strategy_type = if present(first_candidate) && first_candidate["strategy_type"] != () {
    first_candidate["strategy_type"].to_string()
} else { "bottleneck" };
if strategy_type != "bottleneck" {
    // 非 bottleneck 策略只做基本的数据存在性验证
    // 注意：Rhai 无 map.get() 方法（v44 修复，原 c.get("code") 会 Function not found），
    // 键存在性用 in 操作符，取值用 [] 索引
    for c in effective_candidates {
        verified_candidates.push(#{
            "stock_code": if c["stock_code"] != () { c["stock_code"] }
                          else if "code" in c { c["code"] } else { () },
            "stock_name": if c["stock_name"] != () { c["stock_name"] }
                          else if "name" in c { c["name"] } else { () },
            "data_verification": #{
                "strategy_type": strategy_type,
                "verified": true,
                "note": "非 bottleneck 策略，跳过财务数据交叉验证"
            }
        });
    }
    return verified_candidates;
};

// 防御: candidates 可能为空、非数组
if type_of(effective_candidates) != "array" { return verified_candidates; }
if effective_candidates.len() == 0 { return verified_candidates; }

// ── 构建 tool_calls 索引：按 stock_code → 真实财务数据 ──
// tool_calls_made 是 a-candidate-mapper 的 AgentNode 输出字段，
// 结构为 [{tool: "get_stock_financials", arguments: {stock_code: "300285"}, result: "<json_string>", is_error: false}, ...]
let real_financials = #{};
let has_tool_calls = present(tool_calls_made) && type_of(tool_calls_made) == "array";

if has_tool_calls {
    for tc in tool_calls_made {
        if tc.tool == "get_stock_financials" && !tc.is_error {
            let code = tc.arguments.stock_code;
            if present(code) {
                let code_str = code.to_string();
                let result_str = tc.result;
                if present(result_str) && type_of(result_str) == "string" {
                    let parsed = json_parse(result_str);
                    if present(parsed) {
                        real_financials[code_str] = parsed;
                    }
                }
            }
        }
    }
}

// ── 辅助函数：从真实财务数据数组中提取字段 ──
// get_stock_financials 返回 Vec<FinancialReport> → JSON 数组
// 取第一条（最新报告期）的字段
fn get_real_field(real_data, field) {
    if !present(real_data) { return (); }
    if type_of(real_data) != "array" { return (); }
    if real_data.len() == 0 { return (); }
    let first = real_data[0];
    if type_of(first) != "map" { return (); }
    let v = first[field];
    if !present(v) { return (); }
    v
}

// ── 辅助函数：比较 LLM 输出值与真实值，允许容差百分比 ──
fn cross_check(llm_val, real_val, field_name, tolerance_pct) {
    if !present(llm_val) { return #{ "match": true, "note": "" }; }
    if !present(real_val) { return #{ "match": true, "note": field_name + "无真实数据可比" }; }

    let lv = if type_of(llm_val) == "f64" { llm_val } else { llm_val.to_float() };
    let rv = if type_of(real_val) == "f64" { real_val } else { real_val.to_float() };

    if rv.abs() < 0.01 {
        // 真实值接近零，用绝对差
        let diff = (lv - rv).abs();
        if diff > 5.0 {
            return #{ "match": false, "note": field_name + ": LLM=" + lv + " vs 真实=" + rv + " (偏差" + diff + ")" };
        }
    } else {
        let diff_pct = ((lv - rv).abs() / rv.abs()) * 100.0;
        if diff_pct > tolerance_pct {
            return #{ "match": false, "note": field_name + ": LLM=" + lv + " vs 真实=" + rv + " (偏差" + diff_pct.to_int() + "%)" };
        }
    }
    #{ "match": true, "note": "" }
}

for candidate in effective_candidates {
    let verification = #{
        "status": "not_checked",
        "issues": [],
        "confidence": 100,
        "cross_validation": #{ "performed": false, "mismatches": [], "source": "none" }
    };

    let stock_code = candidate.stock_code;
    let stock_code_str = if present(stock_code) { stock_code.to_string() } else { "" };

    let fin = candidate.financial_snapshot;
    let has_fin = present(fin);

    if has_fin {
        let issues = [];
        let confidence = 100;

        // ── 第一阶段：合理性范围检查（原有逻辑） ──

        // 1. 毛利率合理性: 正常制造业 5-95%, 过高(>95%)或负数都是可疑的
        if present(fin.gross_margin) {
            let gm = if type_of(fin.gross_margin) == "f64" { fin.gross_margin } else { fin.gross_margin.to_float() };
            if gm < 0.0 || gm > 95.0 {
                issues.push("毛利率超出合理范围(0-95%)");
                confidence -= 30;
            }
            if gm > 85.0 {
                issues.push("毛利率偏高(>85%)，需确认是否为极高壁垒企业");
                confidence -= 10;
            }
        } else {
            issues.push("毛利率数据缺失");
            confidence -= 10;
        }

        // 2. ROE 合理性: 正常 -50% 到 60%, 异常值需标记
        if present(fin.roe) {
            let roe_v = if type_of(fin.roe) == "f64" { fin.roe } else { fin.roe.to_float() };
            if roe_v < -50.0 || roe_v > 60.0 {
                issues.push("ROE超出合理范围(-50%~60%)");
                confidence -= 30;
            }
            if roe_v < 0.0 {
                issues.push("ROE为负，处于亏损状态");
                confidence -= 10;
            }
        } else {
            issues.push("ROE数据缺失");
            confidence -= 10;
        }

        // 3. 负债率合理性: 0-100%
        if present(fin.debt_ratio) {
            let dr = if type_of(fin.debt_ratio) == "f64" { fin.debt_ratio } else { fin.debt_ratio.to_float() };
            if dr < 0.0 || dr > 100.0 {
                issues.push("负债率超出合理范围(0-100%)");
                confidence -= 30;
            }
            // 瓶颈企业扩张期适度高负债(60-80%)正常
            if dr > 85.0 {
                issues.push("负债率过高(>85%)");
                confidence -= 15;
            }
        } else {
            issues.push("负债率数据缺失");
            confidence -= 10;
        }

        // 4. 营收增速合理性
        if present(fin.revenue_growth_3y_cagr) {
            let rg = if type_of(fin.revenue_growth_3y_cagr) == "f64" { fin.revenue_growth_3y_cagr } else { fin.revenue_growth_3y_cagr.to_float() };
            if rg < -50.0 || rg > 200.0 {
                issues.push("营收增速超出合理范围(-50%~200%)");
                confidence -= 25;
            }
        } else {
            issues.push("营收增速数据缺失");
            confidence -= 10;
        }

        // 5. Capex/折旧比合理性
        if present(fin.capex_depreciation_ratio) {
            let cd = if type_of(fin.capex_depreciation_ratio) == "f64" { fin.capex_depreciation_ratio } else { fin.capex_depreciation_ratio.to_float() };
            if cd < 0.0 || cd > 20.0 {
                issues.push("Capex/折旧比超出合理范围(0~20)");
                confidence -= 20;
            }
        } else {
            issues.push("Capex/折旧比数据缺失");
            confidence -= 10;
        }

        // 6. 研发费用率合理性
        if present(fin.rnd_ratio) {
            let rr = if type_of(fin.rnd_ratio) == "f64" { fin.rnd_ratio } else { fin.rnd_ratio.to_float() };
            if rr < 0.0 || rr > 80.0 {
                issues.push("研发费用率超出合理范围(0-80%)");
                confidence -= 20;
            }
        } else {
            issues.push("研发费用率数据缺失");
            confidence -= 10;
        }

        // ── P0-3 新增: PE 行业分位检查 ──
        // 从 Rust 端注入 industry_pe_percentile（来自 get_industry_pe_percentile 数据源）
        if present(fin.pe_ttm) && present(industry_pe_mean) && present(industry_pe_std) {
            let pe = if type_of(fin.pe_ttm) == "f64" { fin.pe_ttm } else { fin.pe_ttm.to_float() };
            let pe_mean = if type_of(industry_pe_mean) == "f64" { industry_pe_mean } else { industry_pe_mean.to_float() };
            let pe_std = if type_of(industry_pe_std) == "f64" { industry_pe_std } else { industry_pe_std.to_float() };

            if pe_std > 0.01 {
                let z_score = (pe - pe_mean) / pe_std;
                // ±2σ 范围外 → 标记高估/低估风险
                if z_score > 2.0 {
                    issues.push(`PE=${(pe*100.0).round()/100.0} 高于行业均值+2σ(均值=${(pe_mean*100.0).round()/100.0}, σ=${(pe_std*100.0).round()/100.0})，高估风险`);
                    confidence -= 20;
                } else if z_score < -2.0 {
                    issues.push(`PE=${(pe*100.0).round()/100.0} 低于行业均值-2σ(均值=${(pe_mean*100.0).round()/100.0}, σ=${(pe_std*100.0).round()/100.0})，低估信号`);
                    // 低估不扣 confidence
                }
            }
        }

        // ── P0-3 新增: 审计意见检查 ──
        if present(fin.audit_opinion) {
            let opinion = if type_of(fin.audit_opinion) == "string" { fin.audit_opinion } else { fin.audit_opinion.to_string() };
            if opinion != "标准无保留意见" && opinion != "标准无保留" && opinion != "无保留意见" {
                issues.push(`审计意见异常: "${opinion}"，非标准无保留意见触发 Insufficient 标记`);
                confidence = 0;  // 直接置零，强制 Insufficient
            }
        }

        // ── 第二阶段：交叉验证（P0-2 新增） ──
        // 检查 LLM 是否调用了 get_stock_financials 获取该股票的真实数据
        // 注意：Rhai 无内置 map.has()，用 in 操作符检查键
        let real_data = if stock_code_str != "" && (stock_code_str in real_financials) {
            real_financials[stock_code_str]
        } else { () };

        let cross_mismatches = [];
        let cross_performed = present(real_data);
        let cross_source = if cross_performed { "get_stock_financials" } else if has_tool_calls { "tool_called_but_not_this_stock" } else { "no_tool_call" };

        if cross_performed {
            // 比对 LLM 输出的 financial_snapshot 与真实工具返回数据
            // 容差 15%：允许 LLM 四舍五入或使用不同统计口径

            // 毛利率
            if present(fin.gross_margin) {
                let real_gm = get_real_field(real_data, "gross_margin");
                if present(real_gm) {
                    let check = cross_check(fin.gross_margin, real_gm, "毛利率", 15.0);
                    if !check["match"] {
                        cross_mismatches.push(check.note);
                        confidence -= 20;
                    }
                }
            }

            // ROE
            if present(fin.roe) {
                let real_roe = get_real_field(real_data, "roe");
                if present(real_roe) {
                    let check = cross_check(fin.roe, real_roe, "ROE", 15.0);
                    if !check["match"] {
                        cross_mismatches.push(check.note);
                        confidence -= 20;
                    }
                }
            }

            // 负债率
            if present(fin.debt_ratio) {
                let real_dr = get_real_field(real_data, "debt_ratio");
                if present(real_dr) {
                    let check = cross_check(fin.debt_ratio, real_dr, "负债率", 15.0);
                    if !check["match"] {
                        cross_mismatches.push(check.note);
                        confidence -= 20;
                    }
                }
            }

            // 营收增速
            if present(fin.revenue_growth_3y_cagr) {
                // get_stock_financials 返回的是 revenue_yoy（同比），而非 3年CAGR
                let real_rg = get_real_field(real_data, "revenue_yoy");
                if present(real_rg) {
                    // 3年CAGR 与单年 YoY 差异可能较大，用 30% 容差
                    let check = cross_check(fin.revenue_growth_3y_cagr, real_rg, "营收增速", 30.0);
                    if !check["match"] {
                        cross_mismatches.push(check.note);
                        confidence -= 10;  // CAGR vs YoY 差异正常，降低扣分
                    }
                }
            }
        } else if cross_source == "no_tool_call" {
            // LLM 未调用 get_stock_financials，财务数据可能来自训练数据
            issues.push("LLM未调用get_stock_financials——财务数据可能来自训练记忆而非实时数据");
            confidence -= 25;
        }

        verification.cross_validation = #{
            "performed": cross_performed,
            "mismatches": cross_mismatches,
            "source": cross_source
        };

        // 计算最终验证状态
        let status = if issues.len() == 0 && cross_mismatches.len() == 0 {
            "verified"
        } else if confidence < 40 {
            "conflict"
        } else if confidence < 70 {
            "partial"
        } else {
            "verified"
        };

        verification.status = status;
        verification.issues = issues;
        verification.confidence = confidence;
    } else {
        // 无 financial_snapshot: LLM 未输出财务数据
        verification.status = "no_data";
        verification.issues = ["financial_snapshot 缺失——LLM 可能未调用 get_stock_financials"];
        verification.confidence = 0;
        verification.cross_validation = #{
            "performed": false,
            "mismatches": [],
            "source": if has_tool_calls { "tool_called_but_no_financial_snapshot" } else { "no_tool_call" }
        };
    }

    // 保留原始所有字段，增加 data_verification
    let verified = #{
        // 从原始候选复制字段（按已知 schema 逐个复制，保留扩展字段）
        "stock_code": candidate.stock_code,
        "stock_name": candidate.stock_name,
        "relevance": candidate.relevance,
        "serenity_score": candidate.serenity_score,
        "confidence": candidate.confidence,
        "bottleneck_product": candidate.bottleneck_product,
        "primary_risk": candidate.primary_risk,
        "catalysts": candidate.catalysts,
        "exit_signals": candidate.exit_signals,
        "attention_metrics": candidate.attention_metrics,
        "financial_snapshot": candidate.financial_snapshot,
        "data_verification": verification
    };

    verified_candidates += verified;
}

// 输出: 验证后的候选列表
verified_candidates
