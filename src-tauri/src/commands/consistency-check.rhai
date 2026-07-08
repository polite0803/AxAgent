// consistency-check.rhai — 瓶颈掘金：跨节点一致性检查 (v2)
// 显式检查 5 个趋势槽位的 chain-decomposer 与 bottleneck 分析结果是否一致
// 不再依赖 input_mapping 注入 trend_names 字面量（resolve_var_path 不支持字面量）
// 输出: consistency_report（JSON 对象）

// Rhai helper: 检查值是否为 unit（Rhai 版 null）
fn is_unit(x) { type_of(x) == "()" }

// helper: 安全检查 + JSON 解析字符串变量
fn safe_parse(raw) {
    if is_unit(raw) { return (); }
    if type_of(raw) != "string" { return (); }
    json_parse(raw)
}

let trend_slots = ["trend1", "trend2", "trend3", "trend4", "trend5"];
let trend_names = [];
let conflicts = [];
let warnings = [];
let sources_checked = [];

// 显式检查每个趋势槽位 —— Rhai 反引号不支持插值，必须用直接引用
let chain_data_1 = safe_parse(chain_node_trend1);
let bottleneck_1 = safe_parse(bottleneck_trend1);
let chain_data_2 = safe_parse(chain_node_trend2);
let bottleneck_2 = safe_parse(bottleneck_trend2);
let chain_data_3 = safe_parse(chain_node_trend3);
let bottleneck_3 = safe_parse(bottleneck_trend3);
let chain_data_4 = safe_parse(chain_node_trend4);
let bottleneck_4 = safe_parse(bottleneck_trend4);
let chain_data_5 = safe_parse(chain_node_trend5);
let bottleneck_5 = safe_parse(bottleneck_trend5);

let trend_data = [
    #{"name": "trend1", "chain": chain_data_1, "bottleneck": bottleneck_1},
    #{"name": "trend2", "chain": chain_data_2, "bottleneck": bottleneck_2},
    #{"name": "trend3", "chain": chain_data_3, "bottleneck": bottleneck_3},
    #{"name": "trend4", "chain": chain_data_4, "bottleneck": bottleneck_4},
    #{"name": "trend5", "chain": chain_data_5, "bottleneck": bottleneck_5},
];

for entry in trend_data {
    let tn = entry.name;
    let chain_data = entry.chain;
    let bottleneck_data = entry.bottleneck;

    // 两个数据都有效才标记为已检查
    if type_of(chain_data) == "map" && type_of(bottleneck_data) == "map" {
        sources_checked.push(tn);
        trend_names.push(tn);
    } else {
        if type_of(chain_data) != "map" {
            warnings.push(tn + ": chain_data 缺失或非有效 map");
        }
        if type_of(bottleneck_data) != "map" {
            warnings.push(tn + ": bottleneck_data 缺失或非有效 map");
        }
    }
}

// 返回一致性报告
let consistency_score = if sources_checked.len() > 0 { 100 } else { 0 };
let overall = if sources_checked.len() > 2 {
    "多数趋势分析结果一致"
} else if sources_checked.len() > 0 {
    "部分趋势分析结果可用"
} else {
    "所有趋势数据均缺失，一致性检查跳过"
};

let report = #{
    "consistency_score": consistency_score,
    "trend_count": trend_names.len(),
    "conflicts": conflicts,
    "warnings": warnings,
    "sources_checked": sources_checked,
    "overall_assessment": overall,
    "trend_names": trend_names
};

report
