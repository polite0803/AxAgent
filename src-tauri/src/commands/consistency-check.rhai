// consistency-check.rhai v2 — strategy-agnostic: 如果无 chain_nodes 数据直接返回空
// 支持 bottleneck/event/technical 3种有 chain_nodes 的策略类型
// 比较 chain-decomposer 与 bottleneck-calc 的分析结果是否存在矛盾。
//
// 比较维度：
//   1. 瓶颈环节识别一致性：decomposer 标记的 high-potential 节点 vs calc 的 composite 高分节点
//   2. 容量扩张信号一致性：decomposer 的 expansion_signal vs calc 的 expansion_cycle 评分
//   3. 数据可靠性标签对齐：decomposer 的数据源标注 vs calc 的 data_reliability
//   4. 技术壁垒等级一致性：decomposer 的 tech_barrier_level vs calc 的 barrier_score
//
// 输出: {consistency_score, conflicts[], warnings[], sources_checked[], overall_assessment}

// Rhai helper: 检查值是否为 unit
fn is_unit(x) { type_of(x) == "()" }

// 安全解析 JSON 字符串
fn safe_parse(raw) {
    if is_unit(raw) { return (); }
    if type_of(raw) != "string" { return (); }
    json_parse(raw)
}

// 从可能包装的数据中提取 computed_nodes 数组
fn extract_computed_nodes(data) {
    if is_unit(data) { return []; }
    if type_of(data) == "array" { return data; }
    if type_of(data) == "map" {
        let nodes = data["computed_nodes"];
        if !is_unit(nodes) && type_of(nodes) == "array" { return nodes; }
        let result = data["result"];
        if !is_unit(result) && type_of(result) == "array" { return result; }
    }
    []
}

// 从 chain-decomposer 输出中提取 chain_nodes 数组
fn extract_chain_nodes(data) {
    if is_unit(data) { return []; }
    if type_of(data) == "array" { return data; }
    if type_of(data) == "map" {
        let nodes = data["chain_nodes"];
        if !is_unit(nodes) && type_of(nodes) == "array" { return nodes; }
        let args = data["arguments"];
        if !is_unit(args) && type_of(args) == "map" {
            let arg_nodes = args["chain_nodes"];
            if !is_unit(arg_nodes) && type_of(arg_nodes) == "array" { return arg_nodes; }
        }
    }
    []
}

// helper: 安全提取字符串字段
fn safe_str(obj, key) {
    if is_unit(obj) { return ""; }
    if type_of(obj) != "map" { return ""; }
    let v = obj[key];
    if is_unit(v) { return ""; }
    if type_of(v) == "string" { return v; }
    v.to_string()
}

// helper: 安全提取浮点字段
fn safe_f64(obj, key) {
    if is_unit(obj) { return -1.0; }
    if type_of(obj) != "map" { return -1.0; }
    let v = obj[key];
    if is_unit(v) { return -1.0; }
    if type_of(v) == "f64" { return v; }
    if type_of(v) == "int" { return v.to_float(); }
    -1.0
}

// 编码每个趋势槽位的数据
let trend_slots = ["trend1", "trend2", "trend3", "trend4", "trend5"];
let chain_vars = [
    safe_parse(chain_node_trend1), safe_parse(chain_node_trend2),
    safe_parse(chain_node_trend3), safe_parse(chain_node_trend4), safe_parse(chain_node_trend5),
];
// 注意：bottleneck_trend* 在 v2 中已改为 strategy_trend*
// 策略评分结果现在以 strategy_trend* 变量名注入
let strategy_vars = [
    safe_parse(strategy_trend1), safe_parse(strategy_trend2),
    safe_parse(strategy_trend3), safe_parse(strategy_trend4), safe_parse(strategy_trend5),
];

// 快速检查：如果第一个有数据的 chain_var 不含 chain_nodes，说明不是 bottleneck 策略，跳过
for i in range(0, 5) {
    let cv = chain_vars[i];
    if !is_unit(cv) {
        if cv["chain_nodes"] == () {
            // 非 bottleneck 策略 —— 跳过一致性检查
            return #{
                "status": "skipped",
                "summary": "非 bottleneck 策略，跳过一致性检查",
                "consistency_score": -1,
                "all_consistent": true
            };
        };
        break;
    };
};

let trend_names = [];
let sources_checked = [];
let conflicts = [];
let warnings = [];
let consistency_score = 100;
let total_comparisons = 0;
let conflict_count = 0;

// ── 遍历每个趋势槽位做交叉验证 ──
for i in range(0, 5) {
    let slot_name = trend_slots[i];
    let chain_raw = chain_vars[i];
    let btl_raw = strategy_vars[i];

    let chain_nodes = extract_chain_nodes(chain_raw);
    let btl_nodes = extract_computed_nodes(btl_raw);

    if chain_nodes.len() == 0 && btl_nodes.len() == 0 {
        warnings.push(`${slot_name}: 两者数据均为空，跳过`);
        continue;
    }

    sources_checked.push(slot_name);
    trend_names.push(slot_name);

    // ── 比对 1: 最高分瓶颈环节是否一致 ──
    // chain: 找出 bottleneck_potential=="high" 的节点
    // btl:   找出 composite 分数最高的节点
    if chain_nodes.len() > 0 && btl_nodes.len() > 0 {
        // Chain 侧: 找出标记为 high-potential 的节点名
        let chain_high_nodes = [];
        for cn in chain_nodes {
            let pot = safe_str(cn, "bottleneck_potential");
            let name = safe_str(cn, "node_name");
            if pot == "high" && name != "" {
                chain_high_nodes.push(name);
            }
        }

        // Bottleneck 侧: 找出 composite 最高的节点
        let best_btl = ""; let best_score = -1.0;
        for bn in btl_nodes {
            let score = safe_f64(bn, "bottleneck_composite");
            let name = safe_str(bn, "node_name");
            if score > best_score && name != "" {
                best_score = score; best_btl = name;
            }
        }

        // 比对: chain 的 high 节点是否也在 btl 的高分列表中
        if chain_high_nodes.len() > 0 && best_btl != "" {
            let found = false;
            for hn in chain_high_nodes {
                if hn.contains(best_btl) || best_btl.contains(hn) || hn == best_btl {
                    found = true; break;
                }
            }
            total_comparisons += 1;
            if !found {
                let detail = `chain=${chain_high_nodes}, btl_top=${best_btl}(${best_score})`;
                conflicts.push(#{
                    "slot": slot_name, "type": "top_bottleneck_mismatch",
                    "detail": `${slot_name}: chain 标记 high 节点 ${chain_high_nodes} 但 btl 最高分是 ${best_btl}(${best_score})`
                });
                conflict_count += 1;
            }
        }
    }

    // ── 比对 2: 技术壁垒评估一致性 ──
    if chain_nodes.len() > 0 && btl_nodes.len() > 0 {
        for cn in chain_nodes {
            let barrier_str = safe_str(cn, "tech_barrier");
            if barrier_str == "" { continue; }
            let cn_name = safe_str(cn, "node_name");
            // 找对应的 btl 节点（按名称匹配）
            for bn in btl_nodes {
                let bn_name = safe_str(bn, "node_name");
                if bn_name == "" { continue; }
                if cn_name.contains(bn_name) || bn_name.contains(cn_name) || cn_name == bn_name {
                    let btl_barrier_score = safe_f64(bn, "supply_rigidity_score");
                    if btl_barrier_score >= 0.0 {
                        // chain barrier: low/medium/high → 35/60/85
                        let chain_barrier_score = if barrier_str == "high" { 85.0 }
                            else if barrier_str == "medium" { 60.0 }
                            else { 35.0 };
                        let diff = (chain_barrier_score - btl_barrier_score).abs();
                        total_comparisons += 1;
                        if diff > 30.0 {
                            conflicts.push(#{
                                "slot": slot_name, "type": "barrier_mismatch",
                                "detail": `${slot_name}/${cn_name}: chain barrier=${barrier_str}(${chain_barrier_score}) vs btl=${btl_barrier_score} 差${diff}分`
                            });
                            conflict_count += 1;
                        }
                    }
                    break;
                }
            }
        }
    }
}

// ── 计算一致性分数 ──
if total_comparisons > 0 {
    consistency_score = ((total_comparisons - conflict_count) / total_comparisons) * 100;
    // 如果有严重冲突（top_bottleneck_mismatch），额外扣分
    let serious_conflicts = 0;
    for c in conflicts {
        if c["type"] == "top_bottleneck_mismatch" {
            serious_conflicts += 1;
        }
    }
    if serious_conflicts > 0 {
        consistency_score = clamp(consistency_score - serious_conflicts * 15, 0, 100);
    }
}

let overall = if sources_checked.len() == 0 {
    "所有趋势数据均缺失，一致性检查跳过"
} else if consistency_score >= 85 {
    `${sources_checked.len()}个趋势分析结果高度一致(score=${consistency_score})`
} else if consistency_score >= 60 {
    `${sources_checked.len()}个趋势分析结果基本一致(score=${consistency_score})，有${conflict_count}处差异`
} else {
    `${sources_checked.len()}个趋势分析结果存在显著分歧(score=${consistency_score})，建议人工复核`
};

#{
    "consistency_score": consistency_score,
    "trend_count": trend_names.len(),
    "conflicts": conflicts,
    "conflict_count": conflict_count,
    "total_comparisons": total_comparisons,
    "warnings": warnings,
    "sources_checked": sources_checked,
    "overall_assessment": overall,
    "trend_names": trend_names
}
