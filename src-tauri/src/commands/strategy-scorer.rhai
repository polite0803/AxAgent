// strategy-scorer.rhai v1 — 统一多策略评分
// 根据 trend_strategy 变量路由到对应策略的评分逻辑
// 所有策略输出统一格式的 computed_nodes，供 candidate-mapper 消费
// Rhai v1.25 不支持 let mut，全部 let

// ═══════════════════════════════════════════════
// 通用函数（所有策略共享）
// ═══════════════════════════════════════════════

fn clamp_score(v) { if v < 0.0 { 0.0 } else if v > 100.0 { 100.0 } else { v } }

// 从模板变量读取权重（兜底默认值）
// 注意：参数名不能用 default，它是 Rhai 保留关键字
fn read_weight(name, fallback) {
    if name == "w_supply" { if w_supply != () { w_supply } else { fallback } }
    else if name == "w_demand" { if w_demand != () { w_demand } else { fallback } }
    else if name == "w_irreplace" { if w_irreplace != () { w_irreplace } else { fallback } }
    else { fallback }
}

fn readiness_signal(score) {
    if score >= 75.0 { "strong_signal" }
    else if score >= 55.0 { "potential_signal" }
    else if score >= 35.0 { "weak_signal" }
    else { "no_signal" }
}

// ═══════════════════════════════════════════════
// bottleneck 策略评分（复用 bottleneck-calc 精简版）
// ═══════════════════════════════════════════════

fn score_bottleneck() {
    // 解包 chain_analysis
    let chain_nodes = [];
    if chain_analysis["chain_nodes"] != () {
        chain_nodes = chain_analysis["chain_nodes"];
    } else if chain_analysis["arguments"] != () {
        let args = chain_analysis["arguments"];
        chain_nodes = if args["chain_nodes"] != () { args["chain_nodes"] }
        else if args["content"] != () {
            let parsed = json_parse(args["content"].to_string());
            if parsed != () && parsed["chain_nodes"] != () { parsed["chain_nodes"] } else { [] }
        } else { [] };
    }
    if chain_nodes.len() == 0 {
        return #{"status": "no_data", "computed_nodes": [], "summary": "无产业链数据"};
    }

    // 行业排名数据处理
    let ind_map = #{};
    if industry_ranking != () {
        for ind in industry_ranking {
            let name = if ind["industryName"] != () { ind["industryName"].to_string() } else { "" };
            if name != "" {
                ind_map[name] = #{
                    "changePct": if ind["changePct"] != () { ind["changePct"] } else { 0.0 },
                    "mainInflow": if ind["mainInflow"] != () { ind["mainInflow"] } else { 0.0 },
                };
            }
        }
    }

    let computed = [];
    let max_score = 0.0; let top_name = "";
    let has_fin = false; let has_ind = false;

    for node in chain_nodes {
        let nn = if node["node_name"] != () { node["node_name"].to_string() } else { "未知" };
        let sc = if node["global_supplier_count"] != () { node["global_supplier_count"] } else { -1.0 };
        let t3 = if node["top3_market_share"] != () { node["top3_market_share"] } else { -1.0 };
        let exp = if node["expansion_cycle_months"] != () { node["expansion_cycle_months"] } else { -1.0 };
        let bar = if node["tech_barrier"] != () { node["tech_barrier"].to_string() } else { "low" };
        let dc = if node["demand_validation"] != () && node["demand_validation"]["demand_certainty"] != () { node["demand_validation"]["demand_certainty"].to_string() } else { "low" };
        let ev = if node["demand_validation"] != () && node["demand_validation"]["order_visibility"] != () { node["demand_validation"]["order_visibility"].to_string() } else { "" };
        let fd = if node["financial_data"] != () { node["financial_data"] } else { () };

        // 财务评分
        let fin_ok = false; let fin_health = -1.0; let capex_s = -1.0; let rev_s = -1.0; let rnd_s = -1.0; let roe_s = -1.0;
        if fd != () {
            let gm = if fd["gross_margin"] != () { fd["gross_margin"] } else { -1.0 };
            let rg = if fd["revenue_growth_yoy"] != () { fd["revenue_growth_yoy"] } else { -1.0 };
            let dr = if fd["debt_ratio"] != () { fd["debt_ratio"] } else { -1.0 };
            let roe = if fd["roe"] != () { fd["roe"] } else { -1.0 };
            let rnd = if fd["rnd_ratio"] != () { fd["rnd_ratio"] } else { -1.0 };
            let cdr = if fd["capex_dep_ratio"] != () { fd["capex_dep_ratio"] } else { -1.0 };
            let gms = if gm >= 50.0 { 90.0 } else if gm >= 30.0 { 65.0 } else if gm >= 0.0 { 40.0 } else { -1.0 };
            let rgs = if rg >= 20.0 { 85.0 } else if rg >= 10.0 { 60.0 } else if rg >= 0.0 { 35.0 } else { -1.0 };
            let drs = if dr >= 0.0 && dr < 40.0 { 80.0 } else if dr < 60.0 { 55.0 } else if dr >= 0.0 { 30.0 } else { -1.0 };
            let roes = if roe >= 15.0 { 85.0 } else if roe >= 8.0 { 60.0 } else if roe >= 0.0 { 35.0 } else { -1.0 };
            let rnds = if rnd >= 10.0 { 85.0 } else if rnd >= 5.0 { 60.0 } else if rnd >= 0.0 { 35.0 } else { -1.0 };
            let cdrs = if cdr >= 2.0 { 80.0 } else if cdr >= 1.0 { 55.0 } else if cdr >= 0.0 { 30.0 } else { -1.0 };
            let t = 0.0; let c = 0.0;
            if gms >= 0.0 { t += gms; c += 1.0; };
            if rgs >= 0.0 { t += rgs; c += 1.0; };
            if drs >= 0.0 { t += drs; c += 1.0; };
            if roes >= 0.0 { t += roes; c += 1.0; };
            if rnds >= 0.0 { t += rnds; c += 1.0; };
            if cdrs >= 0.0 { t += cdrs; c += 1.0; };
            fin_ok = true; has_fin = true;
            fin_health = if c > 0.0 { clamp_score(t / c) } else { -1.0 };
            capex_s = cdrs; rev_s = rgs; rnd_s = rnds; roe_s = roes;
        };

        // 行业动量
        let icp = 0.0; let iif = 0.0; let min = "";
        for ik in ind_map.keys() {
            let kw = ik.to_string();
            if nn.find(kw) != () || kw.find(nn) != () {
                min = kw;
                if ind_map[kw] != () {
                    icp = if ind_map[kw]["changePct"] != () { ind_map[kw]["changePct"] } else { 0.0 };
                    iif = if ind_map[kw]["mainInflow"] != () { ind_map[kw]["mainInflow"] } else { 0.0 };
                };
                break;
            }
        }
        if min != "" { has_ind = true; };
        let ims = if icp > 5.0 && iif > 0.0 { 75.0 } else if icp > 2.0 && iif > 0.0 { 60.0 } else if icp > 0.0 { 45.0 } else { 30.0 };

        // 三力评分
        let ws = if w_supply != () { w_supply } else { 0.35 };
        let wd = if w_demand != () { w_demand } else { 0.35 };
        let wi = if w_irreplace != () { w_irreplace } else { 0.30 };
        let cs = if t3 >= 70.0 { 85.0 } else if t3 >= 50.0 { 65.0 } else if t3 >= 0.0 { 40.0 } else { 50.0 };
        let bs = if bar == "high" { 85.0 } else if bar == "medium" { 60.0 } else { 35.0 };
        let cyc = if exp >= 24.0 { 80.0 } else if exp >= 12.0 { 60.0 } else if exp >= 0.0 { 40.0 } else { 50.0 };
        let acyc = if capex_s >= 0.0 { (cyc + capex_s) / 2.0 } else { cyc };
        let srs = cs * 0.30 + bs * 0.40 + acyc * 0.30;

        let es = if ev == "有已公开长协/订单" || ev == "已签长协" || ev == "长协订单" { 90.0 }
            else if ev == "合同负债增长" || ev == "合同负债" || ev == "产能预订" || ev == "产能满载" || ev == "订单饱满" || ev == "订单增长" { 60.0 }
            else if ev == "客户扩产" || ev == "下游扩产" || ev == "政策强制" || ev == "政策驱动" { 55.0 }
            else if ev == "无公开证据" || ev == "" { 30.0 }
            else { 50.0 };
        let cers = if dc == "high" { 80.0 } else if dc == "medium" { 55.0 } else { 30.0 };
        let ae = if ims > 0.0 {
            let base = if rev_s >= 0.0 { (es + rev_s) / 2.0 } else { es };
            if icp > 0.0 { (base + ims) / 2.0 } else if base > 50.0 { base - 10.0 } else { base }
        } else if rev_s >= 0.0 { (es + rev_s) / 2.0 } else { es };
        let des = ae * 0.60 + cers * 0.40;

        let ss = if sc <= 2.0 && sc >= 0.0 { 90.0 } else if sc <= 5.0 { 70.0 } else if sc <= 10.0 { 50.0 } else if sc > 10.0 { 30.0 } else { 50.0 };
        let tms = if rnd_s >= 0.0 && roe_s >= 0.0 { rnd_s * 0.5 + roe_s * 0.5 } else { bs };
        let irs = ss * 0.30 + tms * 0.40 + bs * 0.30;

        let composite = clamp_score(srs * ws + des * wd + irs * wi);
        let rel = if fin_ok { "partially_verified" } else if min != "" { "industry_inferred" } else if sc >= 0.0 || t3 >= 0.0 { "llm_estimated" } else { "insufficient" };
        if composite > max_score { max_score = composite; top_name = nn; };
        computed.push(#{
            "node_name": nn,
            "strategy_type": "bottleneck",
            "stock_code": if node["representative_stocks"] != () && node["representative_stocks"].len() > 0 { node["representative_stocks"][0]["code"].to_string() } else { "" },
            "stock_name": if node["representative_stocks"] != () && node["representative_stocks"].len() > 0 { node["representative_stocks"][0]["name"].to_string() } else { "" },
            "bottleneck_composite": composite,
            "supply_rigidity_score": clamp_score(srs),
            "demand_elasticity_score": clamp_score(des),
            "irreplaceability_score": clamp_score(irs),
            "data_reliability": rel,
            "readiness_signal": readiness_signal(composite),
            "industry_momentum_score": ims,
            "matched_industry": min,
        });
    }
    let rel_summary = if has_fin { "有真实个股财务数据支撑" } else if has_ind { "有行业排名数据交叉验证" } else { "纯 LLM 估算" };
    let overall = if has_fin { "mixed_with_financial" } else if has_ind { "industry_corroborated" } else { "all_llm_estimated" };
    let summary = "共分析 " + computed.len().to_string() + " 个环节, 最高分: " + top_name + " = " + max_score.to_string() + ", 可信度: " + rel_summary;
    #{"status": "ok", "num_nodes": computed.len(), "computed_nodes": computed,
      "top": #{ "name": top_name, "score": max_score },
      "overall_reliability": overall, "summary": summary}
}

// ═══════════════════════════════════════════════
// policy 策略评分
// ═══════════════════════════════════════════════

fn score_policy() {
    // note: weights are hardcoded per-strategy; future: inject via template variables
    let analysis = chain_analysis;  // Agent 输出的分析结果
    let beneficiaries = if analysis["beneficiaries"] != () { analysis["beneficiaries"] } else { [] };
    if beneficiaries.len() == 0 {
        return #{"status": "no_data", "computed_nodes": [], "summary": "无政策受益数据"};
    }
    let policy_impact = if analysis["policy_impact_score"] != () { analysis["policy_impact_score"] } else { 50.0 };
    let computed = [];
    for sector in beneficiaries {
        let sector_name = if sector["sector"] != () { sector["sector"].to_string() } else { "未知行业" };
        let stocks = if sector["stocks"] != () { sector["stocks"] } else { [] };
        for stk in stocks {
            let code = if stk["code"] != () { stk["code"].to_string() } else { "" };
            let name = if stk["name"] != () { stk["name"].to_string() } else { "" };
            let rationale = if stk["rationale"] != () { stk["rationale"].to_string() } else { "" };
            // policy 评分：政策力度 × 行业匹配度 × 龙头溢价
            let sector_match = 70.0; // 默认中高匹配
            let composite = clamp_score(policy_impact * 0.40 + sector_match * 0.35);
            computed.push(#{
                "node_name": name,
                "strategy_type": "policy",
                "stock_code": code,
                "stock_name": name,
                "sector": sector_name,
                "bottleneck_composite": composite,
                "policy_impact": policy_impact,
                "rationale": rationale,
                "data_reliability": "llm_estimated",
                "readiness_signal": readiness_signal(composite),
            });
        }
    }
    let summary = "政策分析: " + beneficiaries.len().to_string() + " 个受益行业, " + computed.len().to_string() + " 只候选";
    #{"status": "ok", "num_nodes": computed.len(), "computed_nodes": computed, "summary": summary,
      "overall_reliability": "llm_estimated"}
}

// ═══════════════════════════════════════════════
// earnings 策略评分
// ═══════════════════════════════════════════════

fn score_earnings() {
    let analysis = chain_analysis;
    let candidates = if analysis["candidates"] != () { analysis["candidates"] } else { [] };
    if candidates.len() == 0 {
        return #{"status": "no_data", "computed_nodes": [], "summary": "无业绩驱动数据"};
    }
    let computed = [];
    for c in candidates {
        let code = if c["code"] != () { c["code"].to_string() } else { "" };
        let name = if c["name"] != () { c["name"].to_string() } else { "" };
        let beat = if c["expected_beat_pct"] != () { c["expected_beat_pct"] } else { 0.0 };
        let reason = if c["reason"] != () { c["reason"].to_string() } else { "" };
        // earnings 评分：预期差幅度(40%) + 过往准确性(30%) + 催化剂时效(30%)
        let beat_score = if beat >= 30.0 { 90.0 } else if beat >= 15.0 { 70.0 } else if beat >= 5.0 { 50.0 } else { 25.0 };
        // 过往准确性：基于 reason 中的明确度估算（Agent 不输出时默认 50）
        let track_record = if reason.find("数据") != () || reason.find("调研") != () || reason.find("动销") != () { 70.0 } else { 50.0 };
        // 催化剂时效：短中期利好加分
        let timeliness = if c["timeframe"] != () {
            let tf = c["timeframe"].to_string();
            if tf.find("short") != () { 80.0 } else if tf.find("mid") != () { 60.0 } else { 40.0 }
        } else { 55.0 };
        let composite = clamp_score(beat_score * 0.40 + track_record * 0.30 + timeliness * 0.30);
        computed.push(#{
            "node_name": name,
            "strategy_type": "earnings",
            "stock_code": code,
            "stock_name": name,
            "bottleneck_composite": composite,
            "expected_beat_pct": beat,
            "track_record_score": track_record,
            "timeliness_score": timeliness,
            "rationale": reason,
            "data_reliability": "partially_verified",
            "readiness_signal": readiness_signal(composite),
        });
    }
    let summary = "业绩分析: " + computed.len().to_string() + " 只候选";
    #{"status": "ok", "num_nodes": computed.len(), "computed_nodes": computed, "summary": summary,
      "overall_reliability": "partially_verified"}
}

// ═══════════════════════════════════════════════
// capital 策略评分
// ═══════════════════════════════════════════════

fn score_capital() {
    let analysis = chain_analysis;
    let sectors_analysis = if analysis["sectors"] != () { analysis["sectors"] } else { [] };
    if sectors_analysis.len() == 0 {
        return #{"status": "no_data", "computed_nodes": [], "summary": "无资金面数据"};
    }
    let computed = [];
    for sec in sectors_analysis {
        let sec_name = if sec["name"] != () { sec["name"].to_string() } else { "未知板块" };
        let inflow = if sec["net_inflow"] != () { sec["net_inflow"] } else { 0.0 };
        let sustainability = if sec["days"] != () { sec["days"] } else { 1.0 };
        let stocks = if sec["stocks"] != () { sec["stocks"] } else { [] };
        for stk in stocks {
            let code = if stk["code"] != () { stk["code"].to_string() } else { "" };
            let name = if stk["name"] != () { stk["name"].to_string() } else { "" };
            // capital 评分：流入强度(40%) + 持续性(35%) + 成交量配合(25%)
            let inflow_score = if inflow > 20.0 { 85.0 } else if inflow > 10.0 { 65.0 } else if inflow > 5.0 { 45.0 } else { 25.0 };
            let sustain_score = if sustainability >= 5.0 { 85.0 } else if sustainability >= 3.0 { 65.0 } else { 40.0 };
            let volume_score = if sec["volume_surge"] != () {
                let vs = sec["volume_surge"];
                if vs > 1.5 { 80.0 } else if vs > 1.2 { 60.0 } else { 40.0 }
            } else { 50.0 };
            let composite = clamp_score(inflow_score * 0.40 + sustain_score * 0.35 + volume_score * 0.25);
            computed.push(#{
                "node_name": name,
                "strategy_type": "capital",
                "stock_code": code,
                "stock_name": name,
                "bottleneck_composite": composite,
                "net_inflow": inflow,
                "sustainability_days": sustainability,
                "volume_surge": if sec["volume_surge"] != () { sec["volume_surge"] } else { () },
                "sector": sec_name,
                "data_reliability": "industry_inferred",
                "readiness_signal": readiness_signal(composite),
            });
        }
    }
    let summary = "资金面分析: " + computed.len().to_string() + " 只候选";
    #{"status": "ok", "num_nodes": computed.len(), "computed_nodes": computed, "summary": summary,
      "overall_reliability": "industry_inferred"}
}

// ═══════════════════════════════════════════════
// 主函数：按策略类型路由
// ═══════════════════════════════════════════════

// ═══════════════════════════════════════════════
// event 策略评分（事件驱动）
// ═══════════════════════════════════════════════

fn score_event() {
    let analysis = chain_analysis;
    let events = if analysis["events"] != () { analysis["events"] } else { [] };
    if events.len() == 0 {
        return #{"status": "no_data", "computed_nodes": [], "summary": "无事件驱动数据"};
    }
    let computed = [];
    for ev in events {
        let code = if ev["code"] != () { ev["code"].to_string() } else { "" };
        let name = if ev["name"] != () { ev["name"].to_string() } else { "" };
        let ev_type = if ev["event_type"] != () { ev["event_type"].to_string() } else { "" };
        let desc = if ev["description"] != () { ev["description"].to_string() } else { "" };
        // 事件评分：事件确定性(40%) + 时效性(30%) + 价格位置(30%)
        let certainty = if ev_type == "contract" || ev_type == "订单" { 85.0 }
            else if ev_type == "policy" || ev_type == "政策" { 70.0 }
            else if ev_type == "earnings" || ev_type == "财报" { 60.0 }
            else if ev_type == "supply_shock" || ev_type == "供给冲击" { 75.0 }
            else if ev_type == "capacity_release" || ev_type == "产能释放" { 65.0 }
            else if ev_type == "rumor" || ev_type == "传闻" { 30.0 }
            else { 50.0 };
        let timeliness = if ev["timeframe"] != () {
            let tf = ev["timeframe"].to_string();
            if tf.find("short") != () { 80.0 } else if tf.find("mid") != () { 60.0 } else { 40.0 }
        } else { 50.0 };
        let price_position = if ev["price_position"] != () {
            let pp = ev["price_position"].to_string();
            if pp == "not_run" { 80.0 } else if pp == "slight_rise" { 65.0 } else if pp == "significant_rise" { 40.0 } else { 25.0 }
        } else { 50.0 };
        let composite = clamp_score(certainty * 0.40 + timeliness * 0.30 + price_position * 0.30);
        computed.push(#{
            "node_name": name,
            "strategy_type": "event",
            "stock_code": code,
            "stock_name": name,
            "bottleneck_composite": composite,
            "event_type": ev_type,
            "event_certainty": certainty,
            "event_timeliness": timeliness,
            "price_position_score": price_position,
            "description": desc,
            "data_reliability": "partially_verified",
            "readiness_signal": readiness_signal(composite),
        });
    }
    let summary = "事件分析: " + computed.len().to_string() + " 只候选";
    #{"status": "ok", "num_nodes": computed.len(), "computed_nodes": computed, "summary": summary,
      "overall_reliability": "partially_verified"}
}

// ═══════════════════════════════════════════════
// technical 策略评分（技术面驱动）
// ═══════════════════════════════════════════════

fn score_technical() {
    let analysis = chain_analysis;
    let signals = if analysis["signals"] != () { analysis["signals"] } else { [] };
    if signals.len() == 0 {
        return #{"status": "no_data", "computed_nodes": [], "summary": "无技术面信号"};
    }
    let computed = [];
    for sig in signals {
        let code = if sig["code"] != () { sig["code"].to_string() } else { "" };
        let name = if sig["name"] != () { sig["name"].to_string() } else { "" };
        let pattern = if sig["pattern"] != () { sig["pattern"].to_string() } else { "" };
        let reason = if sig["reason"] != () { sig["reason"].to_string() } else { "" };
        // 技术评分：形态强度(35%) + 成交量确认(35%) + 板块共振(30%)
        let pattern_score = if pattern.find("突破") != () || pattern.find("breakout") != () { 85.0 }
            else if pattern.find("金叉") != () || pattern.find("golden") != () { 75.0 }
            else if pattern.find("底背离") != () || pattern.find("divergence") != () { 65.0 }
            else if pattern.find("支撑") != () || pattern.find("反弹") != () { 55.0 }
            else { 45.0 };
        let volume_confirm = if sig["volume_surge"] != () {
            let vs = sig["volume_surge"];
            if vs > 1.5 { 85.0 } else if vs > 1.2 { 65.0 } else { 45.0 }
        } else { 50.0 };
        let sector_alignment = if sig["sector_aligned"] != () && sig["sector_aligned"] { 80.0 } else { 50.0 };
        let composite = clamp_score(pattern_score * 0.35 + volume_confirm * 0.35 + sector_alignment * 0.30);
        computed.push(#{
            "node_name": name,
            "strategy_type": "technical",
            "stock_code": code,
            "stock_name": name,
            "bottleneck_composite": composite,
            "pattern": pattern,
            "pattern_score": pattern_score,
            "volume_confirm_score": volume_confirm,
            "sector_alignment_score": sector_alignment,
            "rationale": reason,
            "data_reliability": "industry_inferred",
            "readiness_signal": readiness_signal(composite),
        });
    }
    let summary = "技术分析: " + computed.len().to_string() + " 只候选";
    #{"status": "ok", "num_nodes": computed.len(), "computed_nodes": computed, "summary": summary,
      "overall_reliability": "industry_inferred"}
}

fn main() {
    let strategy = if trend_strategy != () { trend_strategy.to_string() } else { "bottleneck" };
    if strategy == "bottleneck" { return score_bottleneck(); }
    if strategy == "policy" { return score_policy(); }
    if strategy == "earnings" { return score_earnings(); }
    if strategy == "capital" { return score_capital(); }
    if strategy == "event" { return score_event(); }
    if strategy == "technical" { return score_technical(); }
    #{"status": "unknown_strategy", "computed_nodes": [], "summary": "未知策略类型: " + strategy}
}

main()
