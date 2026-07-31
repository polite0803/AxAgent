// bottleneck-calc.rhai v12 — 扩展证据类型 + 评分权重改为变量注入
// 不再依赖 7 个预定义行业基线，仅使用 chain-decomposer 产业链数据 + 行业排名
// 权重变量由 seed_serenity.rs 模板注入，默认 w_supply=0.35 w_demand=0.35 w_irreplace=0.30
// Rhai v1.25 不支持 let mut，全部 let

// ═══════════════════════════════════════════════
// 第一部分：函数定义
// ═══════════════════════════════════════════════

// 任意值 → 数组（防御 ToolNode 输出的多种形态）
fn to_array(v) {
    if type_of(v) == "array" { v }
    else if type_of(v) == "string" {
        let p = json_parse(v.to_string());
        if type_of(p) == "array" { p } else { [] }
    } else if type_of(v) == "map" {
        if v["content"] != () && type_of(v["content"]) == "string" {
            let p = json_parse(v["content"].to_string());
            if type_of(p) == "array" { p }
            else if type_of(p) == "map" && p["data"] != () && type_of(p["data"]) == "array" { p["data"] }
            else if type_of(p) == "map" && p["industries"] != () && type_of(p["industries"]) == "array" { p["industries"] }
            else { [] }
        } else if v["data"] != () && type_of(v["data"]) == "array" { v["data"] }
        else if v["industries"] != () && type_of(v["industries"]) == "array" { v["industries"] }
        else if v["items"] != () && type_of(v["items"]) == "array" { v["items"] }
        else { [] }
    } else { [] }
}

fn barrier_to_score(level) {
    if level == "high" { 85.0 } else if level == "medium" { 60.0 } else { 35.0 }
}

fn certainty_to_score(level) {
    if level == "high" { 80.0 } else if level == "medium" { 55.0 } else { 30.0 }
}

fn evidence_to_score(type_str) {
    // 已知证据类型按可信度评分，覆盖 LLM 常见输出变体
    if type_str == "有已公开长协/订单" { 90.0 }
    else if type_str == "已签长协" || type_str == "长协订单" || type_str == "合同负债增长" || type_str == "合同负债" { 75.0 }
    else if type_str == "产能预订" || type_str == "产能满载" || type_str == "产能利用率提升" || type_str == "订单饱满" || type_str == "订单增长" { 60.0 }
    else if type_str == "客户扩产" || type_str == "下游扩产" || type_str == "政策强制" || type_str == "政策驱动" { 55.0 }
    else if type_str == "无公开证据" || type_str == "LLM推测" || type_str == "合理推断" || type_str == "" { 30.0 }
    else {
        // 未知类型按中位数 50 分并打印 warning
        print("WARNING: evidence_to_score 遇到未知证据类型 '" + type_str + "'，按 50.0 评分");
        50.0
    }
}

fn calc_financial_comps_score(fin_data) {
    if fin_data == () { return #{"available": false}; };
    let gm = if fin_data["gross_margin"] != () { fin_data["gross_margin"] } else { -1.0 };
    let rg = if fin_data["revenue_growth_yoy"] != () { fin_data["revenue_growth_yoy"] } else { -1.0 };
    let dr = if fin_data["debt_ratio"] != () { fin_data["debt_ratio"] } else { -1.0 };
    let roe = if fin_data["roe"] != () { fin_data["roe"] } else { -1.0 };
    let rnd = if fin_data["rnd_ratio"] != () { fin_data["rnd_ratio"] } else { -1.0 };
    let cdr = if fin_data["capex_dep_ratio"] != () { fin_data["capex_dep_ratio"] } else { -1.0 };
    let gm_score = if gm >= 50.0 { 90.0 } else if gm >= 30.0 { 65.0 } else if gm >= 0.0 { 40.0 } else { -1.0 };
    let rg_score = if rg >= 20.0 { 85.0 } else if rg >= 10.0 { 60.0 } else if rg >= 0.0 { 35.0 } else { -1.0 };
    let dr_score = if dr >= 0.0 && dr < 40.0 { 80.0 } else if dr < 60.0 { 55.0 } else if dr >= 0.0 { 30.0 } else { -1.0 };
    let roe_score = if roe >= 15.0 { 85.0 } else if roe >= 8.0 { 60.0 } else if roe >= 0.0 { 35.0 } else { -1.0 };
    let rnd_score = if rnd >= 10.0 { 85.0 } else if rnd >= 5.0 { 60.0 } else if rnd >= 0.0 { 35.0 } else { -1.0 };
    let cdr_score = if cdr >= 2.0 { 80.0 } else if cdr >= 1.0 { 55.0 } else if cdr >= 0.0 { 30.0 } else { -1.0 };
    let total = 0.0; let cnt = 0.0;
    if gm_score >= 0.0 { total += gm_score; cnt += 1.0; };
    if rg_score >= 0.0 { total += rg_score; cnt += 1.0; };
    if dr_score >= 0.0 { total += dr_score; cnt += 1.0; };
    if roe_score >= 0.0 { total += roe_score; cnt += 1.0; };
    if rnd_score >= 0.0 { total += rnd_score; cnt += 1.0; };
    if cdr_score >= 0.0 { total += cdr_score; cnt += 1.0; };
    #{
        "available": true, "num_metrics": cnt,
        "gross_margin_score": gm_score, "revenue_growth_score": rg_score,
        "debt_ratio_score": dr_score, "roe_score": roe_score,
        "rnd_score": rnd_score, "capex_score": cdr_score,
        "financial_health": if cnt > 0.0 { clamp(total / cnt, 0.0, 100.0) } else { -1.0 },
        "raw_values": #{
            "gross_margin": gm, "revenue_growth": rg, "debt_ratio": dr,
            "roe": roe, "rnd_ratio": rnd, "capex_dep_ratio": cdr
        }
    }
}

// ═══════════════════════════════════════════════
// 第二部分：主函数
// ═══════════════════════════════════════════════

fn main() {
    // 解包 chain_analysis（来自链拆解 Agent 的 JSON 输出）
    let chain_nodes = [];
    if chain_analysis["chain_nodes"] != () {
        chain_nodes = to_array(chain_analysis["chain_nodes"]);
    } else if chain_analysis["arguments"] != () {
        let args = chain_analysis["arguments"];
        if args["chain_nodes"] != () {
            chain_nodes = to_array(args["chain_nodes"]);
        } else if args["content"] != () {
            let content_str = args["content"].to_string();
            let parsed = json_parse(content_str);
            if parsed != () && parsed["chain_nodes"] != () {
                chain_nodes = to_array(parsed["chain_nodes"]);
            }
        }
    }
    let num_nodes = chain_nodes.len();
    if num_nodes == 0 {
        return #{
            "status": "no_data", "computed_nodes": [],
            "summary": "无产业链数据输入",
            "overall_reliability": "insufficient_data"
        };
    }

    // 处理行业排名数据（用于行业动量评分，不影响瓶颈评分本身）
    let industry_map = #{};
    if industry_ranking != () {
        let ir = to_array(industry_ranking);
        if ir.len() > 0 {
            for ind in ir {
                let name = if ind["industryName"] != () { ind["industryName"].to_string() } else { "" };
                if name != "" {
                    industry_map[name] = #{
                        "changePct": if ind["changePct"] != () { ind["changePct"] } else { 0.0 },
                        "mainInflow": if ind["mainInflow"] != () { ind["mainInflow"] } else { 0.0 },
                        "turnover": if ind["turnover"] != () { ind["turnover"] } else { 0.0 },
                        "leaderName": if ind["leaderName"] != () { ind["leaderName"].to_string() } else { "" }
                    };
                }
            }
        }
    }

    // 遍历每个产业链环节，计算三力评分
    let computed_nodes = [];
    let max_composite = 0.0;
    let top_node_name = "";
    let any_has_financial = false;
    let any_has_industry = false;

    for node in chain_nodes {
        let node_name = if node["node_name"] != () { node["node_name"].to_string() } else { "未知环节" };
        let raw_supplier_count = if node["global_supplier_count"] != () { node["global_supplier_count"] } else { -1.0 };
        let raw_top3_share = if node["top3_market_share"] != () { node["top3_market_share"] } else { -1.0 };
        let raw_expansion = if node["expansion_cycle_months"] != () { node["expansion_cycle_months"] } else { -1.0 };
        let raw_barrier = if node["tech_barrier"] != () { node["tech_barrier"].to_string() } else { "low" };
        let demand_certainty = if node["demand_validation"] != () && node["demand_validation"]["demand_certainty"] != () { node["demand_validation"]["demand_certainty"].to_string() } else { "low" };
        let evidence_type = if node["demand_validation"] != () && node["demand_validation"]["order_visibility"] != () { node["demand_validation"]["order_visibility"].to_string() } else { "" };
        let financial_data = if node["financial_data"] != () { node["financial_data"] } else { () };
        let comps = calc_financial_comps_score(financial_data);
        let has_financial = comps["available"];
        if has_financial { any_has_financial = true; };

        // 行业动量：遍历 industry_ranking 尝试匹配（不强制，匹配不上则默认）
        let ind_change_pct = 0.0;
        let ind_main_inflow = 0.0;
        let matched_ind_name = "";
        for ind_key in industry_map.keys() {
            let kw = ind_key.to_string();
            if node_name.find(kw) != () || kw.find(node_name) != () {
                matched_ind_name = kw;
                if industry_map[kw] != () {
                    ind_change_pct = if industry_map[kw]["changePct"] != () { industry_map[kw]["changePct"] } else { 0.0 };
                    ind_main_inflow = if industry_map[kw]["mainInflow"] != () { industry_map[kw]["mainInflow"] } else { 0.0 };
                };
                break;
            }
        }
        if matched_ind_name != "" { any_has_industry = true; };
        let industry_momentum_score = if ind_change_pct > 5.0 && ind_main_inflow > 0.0 { 75.0 } else if ind_change_pct > 2.0 && ind_main_inflow > 0.0 { 60.0 } else if ind_change_pct > 0.0 { 45.0 } else { 30.0 };

        // 三力评分（权重由模板变量注入，默认等权重：0.35/0.35/0.30）
        // 用户可在前端 Serenity 设置Tab中调整 w_supply/w_demand/w_irreplace
        let ws = if w_supply != () { w_supply } else { 0.35 };
        let wd = if w_demand != () { w_demand } else { 0.35 };
        let wi = if w_irreplace != () { w_irreplace } else { 0.30 };
        let concentration_score = if raw_top3_share >= 70.0 { 85.0 } else if raw_top3_share >= 50.0 { 65.0 } else if raw_top3_share >= 0.0 { 40.0 } else { 50.0 };
        let barrier_score = barrier_to_score(raw_barrier);
        let cycle_score = if raw_expansion >= 24.0 { 80.0 } else if raw_expansion >= 12.0 { 60.0 } else if raw_expansion >= 0.0 { 40.0 } else { 50.0 };
        let financial_capex_score = if has_financial { comps["capex_score"] } else { -1.0 };
        let adjusted_cycle_score = if financial_capex_score >= 0.0 { (cycle_score + financial_capex_score) / 2.0 } else { cycle_score };
        let supply_rigidity_score = concentration_score * 0.30 + barrier_score * 0.40 + adjusted_cycle_score * 0.30;

        let evidence_score = evidence_to_score(evidence_type);
        let certainty_score = certainty_to_score(demand_certainty);
        let financial_revenue_score = if has_financial { comps["revenue_growth_score"] } else { -1.0 };
        let adjusted_evidence = if industry_momentum_score > 0.0 {
            let base = if financial_revenue_score >= 0.0 { (evidence_score + financial_revenue_score) / 2.0 } else { evidence_score };
            if ind_change_pct > 0.0 { (base + industry_momentum_score) / 2.0 } else if base > 50.0 { base - 10.0 } else { base }
        } else if financial_revenue_score >= 0.0 { (evidence_score + financial_revenue_score) / 2.0 } else { evidence_score };
        let demand_elasticity_score = adjusted_evidence * 0.60 + certainty_score * 0.40;

        let supplier_score = if raw_supplier_count <= 2.0 && raw_supplier_count >= 0.0 { 90.0 } else if raw_supplier_count <= 5.0 { 70.0 } else if raw_supplier_count <= 10.0 { 50.0 } else if raw_supplier_count > 10.0 { 30.0 } else { 50.0 };
        let financial_rnd_score = if has_financial { comps["rnd_score"] } else { -1.0 };
        let financial_roe_score = if has_financial { comps["roe_score"] } else { -1.0 };
        let tech_moat_score = if financial_rnd_score >= 0.0 && financial_roe_score >= 0.0 { financial_rnd_score * 0.5 + financial_roe_score * 0.5 } else { barrier_score };
        let irreplaceability_score = supplier_score * 0.30 + tech_moat_score * 0.40 + barrier_score * 0.30;

        let bottleneck_composite = supply_rigidity_score * ws + demand_elasticity_score * wd + irreplaceability_score * wi;
        let final_composite = clamp(bottleneck_composite, 0.0, 100.0);

        // 可靠度标签（不再依赖硬编码行业基线）
        let data_reliability = if has_financial { "partially_verified" } else if matched_ind_name != "" { "industry_inferred" } else if raw_supplier_count >= 0.0 || raw_top3_share >= 0.0 { "llm_estimated" } else { "insufficient" };
        if final_composite > max_composite { max_composite = final_composite; top_node_name = node_name; };

        computed_nodes.push(#{
            "node_name": node_name,
            "bottleneck_potential": if node["bottleneck_potential"] != () { node["bottleneck_potential"].to_string() } else { "unknown" },
            "bottleneck_type": if node["bottleneck_type"] != () { node["bottleneck_type"].to_string() } else { "unknown" },
            "supply_rigidity_score": clamp(supply_rigidity_score, 0.0, 100.0),
            "demand_elasticity_score": clamp(demand_elasticity_score, 0.0, 100.0),
            "irreplaceability_score": clamp(irreplaceability_score, 0.0, 100.0),
            "bottleneck_composite": final_composite,
            "data_reliability": data_reliability,
            "has_financial_data": has_financial,
            "matched_industry": matched_ind_name,
            "industry_momentum_score": industry_momentum_score,
            "industry_change_pct": ind_change_pct,
            "financial_comps": comps,
            "readiness_signal": if final_composite >= 75.0 { "strong_bottleneck" } else if final_composite >= 55.0 { "potential_bottleneck" } else if final_composite >= 35.0 { "weak_signal" } else { "no_signal" },
            "raw_inputs": #{
                "global_supplier_count": raw_supplier_count,
                "top3_market_share": raw_top3_share,
                "expansion_cycle_months": raw_expansion,
                "tech_barrier": raw_barrier
            }
        });
    }

    // 汇总
    let reliability_summary = if any_has_financial { "有真实个股财务数据支撑" } else if any_has_industry { "有行业排名数据交叉验证" } else { "纯 LLM 估算，无任何真实数据支撑" };
    let overall_reliability = if any_has_financial { "mixed_with_financial" } else if any_has_industry { "industry_corroborated" } else { "all_llm_estimated" };
    let summary = "共分析" + num_nodes.to_string() + "个环节, 最高瓶颈综合分: " + top_node_name + " = " + max_composite.to_string() + ", 可信度: " + reliability_summary;
    return #{
        "status": "ok",
        "num_nodes": num_nodes,
        "computed_nodes": computed_nodes,
        "has_any_financial_data": any_has_financial,
        "has_any_industry_match": any_has_industry,
        "top_bottleneck": #{ "node_name": top_node_name, "composite_score": max_composite },
        "overall_reliability": overall_reliability,
        "summary": summary,
        "industry_data_available": industry_ranking != ()
    };
}

// ═══════════════════════════════════════════════
// 第三部分：执行入口
// ═══════════════════════════════════════════════

main()
