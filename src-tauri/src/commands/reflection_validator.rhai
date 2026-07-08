// reflection_validator.rhai — 反思工作流：反思输出硬裁决验证层 (P2-1/P2-2)
//
// 在 reflection-agent 之后、store-ref 之前执行。
// 验证 reflection-agent 输出的 7 字段类型/枚举值/长度，
// 自动修正 verdict 枚举、截断 lesson_summary、补全 missed_signals 数组等。
//
// 硬裁决规则（对应 R-300 系列规则码）：
//   R-302: verdict 字段不在 correct/partial/wrong 三选一 → 默认 "partial"
//   R-303: lesson_summary 超过 200 字符 → 截断到 200 字符
//   R-304: missed_signals 不是数组 → 转为空数组
//   R-305: params_suggestion 不是对象 → 转为空对象
//
// 注意：本脚本不删除字段，仅修正类型/枚举值/长度，确保 store-ref 写入的数据完整可用。

fn is_unit(x) { type_of(x) == "()" }

// 安全解析 JSON 字符串
fn safe_parse(raw) {
    if is_unit(raw) { return (); }
    if type_of(raw) != "string" { return (); }
    json_parse(raw)
}

// 从 reflection_input 中提取反思 JSON 对象
// reflection_input 可能是：
//   1. 直接的 map（engine 已解析）
//   2. 字符串（LLM 原始输出，需要 json_parse）
//   3. 包装对象 {"content": "...", "params": {...}, ...}
fn extract_reflection(raw) {
    if is_unit(raw) { return #{}; }

    // 直接是 map
    if type_of(raw) == "map" {
        // 检查是否有 params 字段（agent_executor 包装格式）
        let params = raw["params"];
        if !is_unit(params) && type_of(params) == "map" {
            return params;
        }
        // 检查是否有 result/output 字段
        let result = raw["result"];
        if !is_unit(result) && type_of(result) == "map" {
            return result;
        }
        let output = raw["output"];
        if !is_unit(output) && type_of(output) == "map" {
            return output;
        }
        // 检查 content 字段（LLM 原始字符串输出）
        let content = raw["content"];
        if !is_unit(content) && type_of(content) == "string" {
            let parsed = safe_parse(content);
            if !is_unit(parsed) && type_of(parsed) == "map" {
                return parsed;
            }
        }
        // 直接就是反思对象本身
        return raw;
    }

    // 字符串形式
    if type_of(raw) == "string" {
        let parsed = safe_parse(raw);
        if !is_unit(parsed) && type_of(parsed) == "map" {
            return parsed;
        }
    }

    #{}
}

fn main() {
    let reflection = extract_reflection(reflection_input);

    // 提取 7 个字段，做硬裁决验证
    let verdict_raw = reflection["verdict"];
    let alpha_cited_raw = reflection["alpha_cited"];
    let lesson_raw = reflection["lesson_summary"];
    let wrong_raw = reflection["what_went_wrong"];
    let missed_raw = reflection["missed_signals"];
    let fix_raw = reflection["fix_for_future"];
    let params_raw = reflection["params_suggestion"];
    let rule_codes_raw = reflection["rule_codes"];

    // R-302: verdict 必须是 correct/partial/wrong 三选一
    let verdict = if !is_unit(verdict_raw) && type_of(verdict_raw) == "string" {
        let v = verdict_raw.to_string();
        if v == "correct" || v == "partial" || v == "wrong" {
            v
        } else {
            "partial"
        }
    } else {
        "partial"
    };

    // alpha_cited: 字符串兜底
    let alpha_cited = if !is_unit(alpha_cited_raw) && type_of(alpha_cited_raw) == "string" {
        alpha_cited_raw.to_string()
    } else {
        ""
    };

    // R-303: lesson_summary 截断到 200 字符
    let lesson_full = if !is_unit(lesson_raw) && type_of(lesson_raw) == "string" {
        lesson_raw.to_string()
    } else {
        ""
    };
    let lesson_chars = lesson_full.chars();
    let lesson = if lesson_chars.len() > 200 {
        // 取前 200 字符
        let truncated = "";
        let count = 0;
        for ch in lesson_chars {
            if count >= 200 { break; }
            truncated = truncated + ch.to_string();
            count = count + 1;
        }
        truncated
    } else {
        lesson_full
    };

    // what_went_wrong: 字符串兜底
    let what_went_wrong = if !is_unit(wrong_raw) && type_of(wrong_raw) == "string" {
        wrong_raw.to_string()
    } else {
        ""
    };

    // R-304: missed_signals 必须是数组
    let missed_signals = if !is_unit(missed_raw) && type_of(missed_raw) == "array" {
        missed_raw
    } else {
        []
    };

    // fix_for_future: 字符串兜底
    let fix_for_future = if !is_unit(fix_raw) && type_of(fix_raw) == "string" {
        fix_raw.to_string()
    } else {
        ""
    };

    // R-305: params_suggestion 必须是对象
    let params_suggestion = if !is_unit(params_raw) && type_of(params_raw) == "map" {
        params_raw
    } else {
        #{}
    };

    // rule_codes: 必须是数组
    let rule_codes = if !is_unit(rule_codes_raw) && type_of(rule_codes_raw) == "array" {
        rule_codes_raw
    } else {
        ["R-306"]
    };

    // 检查是否触发 R-303 截断
    let final_rule_codes = if lesson_chars.len() > 200 {
        let has_r303 = false;
        for rc in rule_codes {
            if rc.to_string() == "R-303" {
                has_r303 = true;
                break;
            }
        }
        if !has_r303 {
            rule_codes.push("R-303");
        }
        rule_codes
    } else {
        rule_codes
    };

    // 构建验证后的反思对象
    #{
        "status": "ok",
        "verdict": verdict,
        "alpha_cited": alpha_cited,
        "lesson_summary": lesson,
        "what_went_wrong": what_went_wrong,
        "missed_signals": missed_signals,
        "fix_for_future": fix_for_future,
        "params_suggestion": params_suggestion,
        "rule_codes": final_rule_codes
    }
}
