// reflection_untrusted_aggregator.rhai — 反思工作流：上游可信度聚合器 (P0-2)
//
// 在 sub-analysis 之后、reflection-agent 之前执行。
// 检测 sub-analysis 输出是否为空 + 提取子工作流结果中的 __untrusted 标记，
// 输出聚合后的 untrusted_count / untrusted_sources / sub_analysis_empty。
//
// 工作原理：
//   1. sub-analysis 是 SubWorkflowNode，其输出是子工作流 results HashMap 的序列化 JSON。
//   2. 子工作流内部 9 个分析 Agent 节点（a-fundamentals / a-technicals / a-catalyst 等）
//      若触发 hallucination_guard 失败，agent_executor 会注入 __untrusted=true。
//   3. 本脚本扫描子工作流 results，统计 __untrusted=true 的节点数。
//   4. 若 sub-analysis 输出为空或非有效 JSON，标记 sub_analysis_empty=true。

fn is_unit(x) { type_of(x) == "()" }

// 安全解析 JSON 字符串
fn safe_parse(raw) {
    if is_unit(raw) { return (); }
    if type_of(raw) != "string" { return (); }
    json_parse(raw)
}

// 递归扫描 map 中的 __untrusted 字段
fn scan_untrusted(obj, sources) {
    if is_unit(obj) { return; }
    if type_of(obj) != "map" { return; }

    // 检查当前层是否有 __untrusted=true
    let flag = obj["__untrusted"];
    if !is_unit(flag) && type_of(flag) == "bool" && flag == true {
        // 尝试获取节点 ID
        let node_id = if !is_unit(obj["node_id"]) {
            obj["node_id"].to_string()
        } else if !is_unit(obj["id"]) {
            obj["id"].to_string()
        } else {
            "unknown"
        };
        sources.push(node_id);
    }

    // 递归扫描所有 value
    for key in obj.keys() {
        let val = obj[key];
        if type_of(val) == "map" {
            scan_untrusted(val, sources);
        } else if type_of(val) == "array" {
            for item in val {
                if type_of(item) == "map" {
                    scan_untrusted(item, sources);
                }
            }
        }
    }
}

fn main() {
    let sources = [];
    let empty = false;

    // sub_analysis_output 来自 input_mapping: "sub-analysis" 节点输出
    // SubWorkflowNode 输出通常是子工作流 results 的 JSON 序列化
    if is_unit(sub_analysis_output) {
        empty = true;
    } else if type_of(sub_analysis_output) == "string" {
        // 字符串形式：尝试解析为 JSON
        let parsed = safe_parse(sub_analysis_output);
        if is_unit(parsed) {
            empty = true;
        } else if type_of(parsed) == "map" {
            scan_untrusted(parsed, sources);
        } else if type_of(parsed) == "array" {
            for item in parsed {
                if type_of(item) == "map" {
                    scan_untrusted(item, sources);
                }
            }
        }
    } else if type_of(sub_analysis_output) == "map" {
        // 已经是 map（可能 engine 自动解析了）
        scan_untrusted(sub_analysis_output, sources);
        // 检查是否为空 map
        let key_count = 0;
        for _ in sub_analysis_output.keys() {
            key_count = key_count + 1;
        }
        if key_count == 0 {
            empty = true;
        }
    } else if type_of(sub_analysis_output) == "array" {
        // 数组形式
        if sub_analysis_output.len() == 0 {
            empty = true;
        } else {
            for item in sub_analysis_output {
                if type_of(item) == "map" {
                    scan_untrusted(item, sources);
                }
            }
        }
    } else {
        empty = true;
    }

    // 去重 sources
    let unique_sources = [];
    for s in sources {
        let found = false;
        for u in unique_sources {
            if u == s {
                found = true;
                break;
            }
        }
        if !found {
            unique_sources.push(s);
        }
    }

    #{
        "status": "ok",
        "untrusted_count": unique_sources.len(),
        "untrusted_sources": unique_sources,
        "sub_analysis_empty": empty
    }
}
