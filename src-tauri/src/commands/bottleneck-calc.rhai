// bottleneck-calc.rhai v10 — v9 + Rhai v1.25 不支持 let mut，全部改为 let
// 彻底避免 Rhai 解析 fn 与语句交错的兼容性问题

// ═══════════════════════════════════════════════
// 第一部分：函数定义
// ═══════════════════════════════════════════════

fn is_baseline_zero(baseline) {
    if baseline == () { return true; };
    let cp = baseline["competitive_position"];
    if cp == () { return true; };
    let sector = if baseline["sector"] != () { baseline["sector"].to_string() } else { "" };
    let gm = if cp["gross_margin_pct"] != () { cp["gross_margin_pct"] } else { 0.0 };
    let roe = if cp["roe_pct"] != () { cp["roe_pct"] } else { 0.0 };
    let dr = if cp["debt_ratio_pct"] != () { cp["debt_ratio_pct"] } else { 0.0 };
    let rnd = if cp["rnd_intensity"] != () { cp["rnd_intensity"] } else { 0.0 };
    sector == "" && gm == 0.0 && roe == 0.0 && dr == 0.0 && rnd == 0.0
}

fn add_baseline(arr, name, data) {
    if data == () { return; };
    if data["competitive_position"] == () { return; };
    let cp = data["competitive_position"];
    let capex_signal = if data["capacity_indicators"] != () {
        data["capacity_indicators"]["signal"].to_string()
    } else { "" };
    let sector = if data["sector"] != () { data["sector"].to_string() } else { "" };
    arr.push(#{
        "sector_name": name,
        "gross_margin": cp["gross_margin_pct"],
        "roe": cp["roe_pct"],
        "debt_ratio": cp["debt_ratio_pct"],
        "rnd_intensity": cp["rnd_intensity"],
        "gm_rank": cp["gm_rank_in_peers"],
        "peer_count": cp["total_peer_count"],
        "capex_signal": capex_signal,
        "sector": sector
    });
}

fn barrier_to_score(level) {
    if level == "high" { 85.0 } else if level == "medium" { 60.0 } else { 35.0 }
}

fn certainty_to_score(level) {
    if level == "high" { 80.0 } else if level == "medium" { 55.0 } else { 30.0 }
}

fn evidence_to_score(type_str) {
    if type_str == "有已公开长协/订单" { 90.0 }
    else if type_str == "合同负债增长" { 75.0 }
    else if type_str == "产能预订" { 60.0 }
    else { 30.0 }
}

fn find_matching_industry(node_name, ind_keys) {
    for key in ind_keys {
        let ind_key = key.to_string();
        if node_name.find(ind_key) != () || ind_key.find(node_name) != () {
            return ind_key;
        }
    }
    ""
}

fn find_baseline_match(node_name, baselines_list) {
    for bl in baselines_list {
        if bl["sector"] == () { continue; };
        let api_sector = bl["sector"].to_string();
        if api_sector == "" { continue; };
        let parts = api_sector.split("/");
        for part in parts {
            let kw = part.trim();
            if kw == "" { continue; };
            if node_name.find(kw) != () { return bl; };
            if kw.len() > 2 {
                let sub_keywords = kw.split(" ");
                for sub in sub_keywords {
                    let s = sub.trim();
                    if s != "" && node_name.find(s) != () { return bl; }
                }
            }
        }
    }
    // 无匹配：返回全行业均值
    if len(baselines_list) == 0 { return; };
    let cnt = len(baselines_list).to_float();
    let sg = 0.0;
    let sr = 0.0;
    let sd = 0.0;
    let vc = 0.0;
    for bl in baselines_list {
        let gm = if bl["gross_margin"] != () { bl["gross_margin"] } else { -1.0 };
        if gm >= 0.0 { sg += gm; vc += 1.0; };
        if bl["roe"] != () { sr += bl["roe"]; };
        if bl["debt_ratio"] != () { sd += bl["debt_ratio"]; };
    }
    if vc > 0.0 {
        return #{
            "sector_name": "全行业均值(聚合)",
            "gross_margin": sg / vc,
            "roe": sr / cnt,
            "debt_ratio": sd / cnt,
            "rnd_intensity": 0.0,
            "peer_count": vc,
            "is_averaged": true
        };
    }
    return;
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
// 第二部分：主函数（全部可执行代码在此）
// ═══════════════════════════════════════════════

fn main() {
    // 零数据防御
    let all_zero = is_baseline_zero(baseline_semi) &&
        is_baseline_zero(baseline_battery) &&
        is_baseline_zero(baseline_chem) &&
        is_baseline_zero(baseline_med) &&
        is_baseline_zero(baseline_aero) &&
        is_baseline_zero(baseline_consumer_elec) &&
        is_baseline_zero(baseline_auto);
    if all_zero {
        return #{
            "status": "no_data",
            "computed_nodes": [],
            "summary": "所有行业基线数据为空（可能为非交易日或数据源故障），无法计算瓶颈指标",
            "overall_reliability": "insufficient_data",
            "all_baselines_zero": true
        };
    }

    // 解包 chain_analysis
    let chain_nodes = [];
    if chain_analysis["chain_nodes"] != () {
        chain_nodes = chain_analysis["chain_nodes"];
    } else if chain_analysis["arguments"] != () {
        let args = chain_analysis["arguments"];
        if args["chain_nodes"] != () {
            chain_nodes = args["chain_nodes"];
        } else if args["content"] != () {
            let content_str = args["content"].to_string();
            let parsed = json_parse(content_str);
            if parsed != () && parsed["chain_nodes"] != () {
                chain_nodes = parsed["chain_nodes"];
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

    // 处理行业排名数据
    let industry_map = #{};
    if industry_ranking != () {
        for ind in industry_ranking {
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

    // 处理行业基线
    let baselines = [];
    add_baseline(baselines, "半导体设备", baseline_semi);
    add_baseline(baselines, "电池/新能源", baseline_battery);
    add_baseline(baselines, "化工/材料", baseline_chem);
    add_baseline(baselines, "医药器械", baseline_med);
    add_baseline(baselines, "军工航天", baseline_aero);
    add_baseline(baselines, "消费电子", baseline_consumer_elec);
    add_baseline(baselines, "汽车零部件", baseline_auto);

    // 预取行业 map 的 key 列表
    let ind_keys = [];
    for key in industry_map.keys() { ind_keys.push(key); }

    // 遍历每个环节
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
        let matched_ind_name = find_matching_industry(node_name, ind_keys);
        let has_industry_match = matched_ind_name != "" && industry_map[matched_ind_name] != ();
        if has_industry_match { any_has_industry = true; };
        let industry_data = if has_industry_match { industry_map[matched_ind_name] } else { () };
        let ind_change_pct = if industry_data != () { industry_data["changePct"] } else { 0.0 };
        let ind_main_inflow = if industry_data != () { industry_data["mainInflow"] } else { 0.0 };
        let industry_momentum_score = if ind_change_pct > 5.0 && ind_main_inflow > 0.0 { 75.0 } else if ind_change_pct > 2.0 && ind_main_inflow > 0.0 { 60.0 } else if ind_change_pct > 0.0 { 45.0 } else { 30.0 };
        let matched_baseline = find_baseline_match(node_name, baselines);
        let has_baseline = matched_baseline != ();
        if has_baseline && !any_has_industry { any_has_industry = true; };
        let baseline_gm = if has_baseline { matched_baseline["gross_margin"] } else { -1.0 };
        let baseline_roe = if has_baseline { matched_baseline["roe"] } else { -1.0 };
        let baseline_dr = if has_baseline { matched_baseline["debt_ratio"] } else { -1.0 };
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
        let bottleneck_composite = supply_rigidity_score * 0.35 + demand_elasticity_score * 0.35 + irreplaceability_score * 0.30;
        let final_composite = clamp(bottleneck_composite, 0.0, 100.0);
        let data_reliability = if has_financial { "partially_verified" } else if has_baseline { "baseline_corroborated" } else if has_industry_match { "industry_inferred" } else if raw_supplier_count >= 0.0 || raw_top3_share >= 0.0 { "llm_estimated" } else { "insufficient" };
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
            "has_baseline_match": has_baseline,
            "baseline_gm": baseline_gm,
            "baseline_roe": baseline_roe,
            "matched_baseline_sector": if has_baseline { matched_baseline["sector_name"].to_string() } else { "" },
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
    let reliability_summary = if any_has_financial { "有真实个股财务数据支撑" } else if any_has_industry { "无个股数据，但有行业排名/基线数据交叉验证" } else { "纯 LLM 估算，无任何真实数据支撑" };
    let overall_reliability = if any_has_financial { "mixed_with_financial" } else if any_has_industry { "industry_corroborated" } else { "all_llm_estimated" };
    let b0 = if baselines.len() > 0 { baselines[0]["gross_margin"].to_string() } else { "无" };
    let b1 = if baselines.len() > 1 { baselines[1]["gross_margin"].to_string() } else { "无" };
    let b2 = if baselines.len() > 2 { baselines[2]["gross_margin"].to_string() } else { "无" };
    let summary = "共分析" + num_nodes.to_string() + "个环节, 最高瓶颈综合分: " + top_node_name + " = " + max_composite.to_string() + ", 可信度: " + reliability_summary + ", 基线毛利率: 半导体=" + b0 + "% 电池=" + b1 + "% 化工=" + b2 + "%";
    return #{
        "status": "ok",
        "num_nodes": num_nodes,
        "computed_nodes": computed_nodes,
        "has_any_financial_data": any_has_financial,
        "has_any_industry_match": any_has_industry,
        "baselines_loaded": baselines.len(),
        "baselines": baselines,
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
