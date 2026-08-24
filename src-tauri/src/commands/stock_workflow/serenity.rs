use super::decision::{load_and_inject_template, parse_asof_param, resolve_runtime_options};
use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_agent_macro::agent_command;
use axagent_astock_data::as_of::{self};
use axagent_entities::reco_picks;
use axagent_harness::response_normalizer::ResponseNormalizer;
use axagent_harness::types::{ChatResponse, ContentBlock};
use axagent_rt_workflow::work_engine::{ProgressCallback, RunOptions, StepProgressEvent};
use axagent_runtime_core::DefaultResponseNormalizer;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::sync::Arc;
use tauri::{Emitter, State};

/// 从 Agent 节点输出中提取结构化 JSON。
///
/// 优先顺序：
///   1) 顶层 `params` 字段
///   2) 顶层 `output` / `result` / `data` / `candidates` / `trends` 字段
///   3) 顶层 `content` 字符串：直接用 `axagent_kit::utils::extract_json_from_llm_response`
///      解析（不经过 ResponseNormalizer——它针对工具调用场景，会将 ````json` 块
///      误识别为 ToolUse）
///   4) 原始包装对象（兜底）
pub(crate) async fn extract_agent_output(raw: serde_json::Value) -> serde_json::Value {
    let obj = match raw.as_object() {
        Some(o) => o,
        None => return raw,
    };
    // 1) 顶层 params
    if let Some(params) = obj.get("params") {
        return params.clone();
    }
    // 2) 顶层常见容器字段
    for key in ["output", "result", "data", "candidates", "trends"] {
        if let Some(v) = obj.get(key) {
            return v.clone();
        }
    }
    // 3) 直接从 content 提取 JSON：找到第一个 { 或 [，找匹配闭合，解析。
    //    不依赖 extract_json_from_llm_response 的 fence 剥离（在复杂嵌套场景可能失效）。
    if let Some(content) = obj.get("content").and_then(|c| c.as_str()) {
        let candidate = axagent_kit::utils::extract_json_from_llm_response(content);
        // 诊断：打印 candidate 前后各 200 字符
        let preview: String = candidate.chars().take(200).collect();
        let tail: String =
            candidate.chars().rev().take(200).collect::<String>().chars().rev().collect();
        tracing::info!("[serenity] 提取文本 前200: {} / 后200: {}", preview, tail);
        // A: 精确解析（fence 剥离后的文本）
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&candidate) {
            if parsed.is_object() || parsed.is_array() {
                // 拆包 tool_json 格式: {"name": "...", "arguments": {...}} → arguments
                // 有些 LLM 用 "input" 代替 "arguments"
                if let Some(args) = parsed
                    .as_object()
                    .and_then(|o| o.get("arguments"))
                    .or_else(|| parsed.as_object().and_then(|o| o.get("input")))
                {
                    return args.clone();
                }
                return parsed;
            }
        }
        // B: 裸括号提取 candidates/trends 数组（免疫未转义引号）
        if let Some(parsed) = extract_named_arrays(&candidate) {
            return parsed;
        }
        if let Some(parsed) = extract_named_arrays(content) {
            return parsed;
        }
        // C: 修复后重试
        let repaired = repair_json(&candidate);
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&repaired) {
            if parsed.is_object() || parsed.is_array() {
                return parsed;
            }
        }
        // D: extract_outer_json（多起点 + in_string 追踪）
        if let Some(parsed) = extract_outer_json(content) {
            return parsed;
        }
        // E: 检测 LLM 自然语言拒绝（短文本非 JSON），防御性降级
        let content_len = content.chars().count();
        if content_len < 30 {
            tracing::warn!(
                "[serenity] LLM 内容为短自然语言（长度={}），返回空值防御性降级: {}",
                content_len,
                content.chars().take(50).collect::<String>()
            );
            return serde_json::Value::Null;
        }
        let head: String = content.chars().take(300).collect();
        let tail_start = content.chars().count().saturating_sub(200);
        let tail: String = content.chars().skip(tail_start).collect();
        let c_head: String = candidate.chars().take(1000).collect();
        let c_tail: String =
            candidate.chars().rev().take(200).collect::<String>().chars().rev().collect();
        tracing::warn!(
            "[serenity] content JSON 提取失败，总长度 {}, 前300: {} / 后200: {}",
            content.chars().count(),
            head,
            tail
        );
        tracing::warn!("[serenity] 预处理文本 前1000: {} / 后200: {}", c_head, c_tail);
    }
    // 4) 兜底
    raw
}

/// 通过 `ResponseNormalizer` 把 `content` 字符串规范化为 IR 块，再从 IR 中
/// 提取结构化 JSON。优先取 `ContentBlock::ToolUse.input`（通常是 JSON 串），
/// 文本块拼接后走 `axagent_kit::utils::extract_json_from_llm_response` 兜底。
///
/// 注意：`extract_agent_output` 不再调用此函数（改用 `extract_json_from_llm_response` 直接提取）。
/// 此函数保留供测试和未来工具调用场景复用。
#[allow(dead_code)]
async fn extract_via_normalizer(content: &str) -> Option<serde_json::Value> {
    if content.trim().is_empty() {
        return None;
    }
    let response = ChatResponse {
        id: String::new(),
        model: String::new(),
        content: content.to_string(),
        thinking: None,
        usage: Default::default(),
        tool_calls: None,
    };
    let normalizer = DefaultResponseNormalizer;
    let blocks: Vec<ContentBlock> = normalizer.normalize(&response).await;

    // 优先：ToolUse 块的 input（项目里工具参数就是 JSON 串）
    for block in &blocks {
        if let ContentBlock::ToolUse { input, .. } = block
            && let Some(parsed) = parse_loose_json(input)
        {
            return Some(parsed);
        }
    }
    // 兜底：拼接所有 Text 块，用项目统一的 LLM JSON 提取函数
    let joined: String = blocks
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !joined.trim().is_empty() {
        let candidate = axagent_kit::utils::extract_json_from_llm_response(&joined);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(candidate) {
            return Some(v);
        }
    }
    None
}

/// 轻量 JSON 修复：处理 LLM 偶发的括号不匹配和引号未闭合。
///
/// 只做两种统计级修复（不解析语义）：
/// 1. **括号平衡** — 跳过字符串内部，统计 `{`/`[` vs `}`/`]`，补/删尾部括号
/// 2. **引号闭合** — 奇数个未转义 `"` 时末尾补一个
///
/// 对合法 JSON 零开销（不改变原文）；只在 `serde_json::from_str` 已失败后调用。
fn repair_json(s: &str) -> String {
    let mut result = s.to_string();

    // LLM 高频手滑："nulll"→"null"
    result = result.replace("nulll", "null");

    // LLM 尾逗号：,"→"、,}→}、,]→]
    // 只在可能有尾逗号的上下文中处理（简单字符串替换，低风险）
    result = result.replace(",]", "]");
    result = result.replace(",}", "}");

    let bytes = result.as_bytes();
    let len = bytes.len();
    if len == 0 {
        return result;
    }

    // 第一遍：统计括号和引号，跳过字符串内部
    let mut open_curly = 0i32;
    let mut open_bracket = 0i32;
    let mut in_string = false;

    let mut i = 0;
    while i < len {
        let b = bytes[i];
        match in_string {
            false => match b {
                b'{' => open_curly += 1,
                b'}' => open_curly -= 1,
                b'[' => open_bracket += 1,
                b']' => open_bracket -= 1,
                b'"' => in_string = true,
                _ => {},
            },
            true => {
                // 在字符串内部：只关心 \" 和 字符串结束 "
                if b == b'\\' {
                    i += 1; // 跳过下一个字符（转义序列）
                } else if b == b'"' {
                    in_string = false; // 字符串结束
                }
            },
        }
        i += 1;
    }

    // 第二遍：从尾部修复 — 只处理末尾多余的闭合括号
    // 复用第一遍已经过 nulll→null 修复的 result，不重新从 s 构建
    // (这行是故意 blank 的以使用前面的 result 变量)

    // 先处理括号不平衡：补缺失的闭合括号
    let needs_curly = open_curly.max(0) as usize;
    let needs_bracket = open_bracket.max(0) as usize;

    // 如果有缺失闭合，在尾部补上
    for _ in 0..needs_curly {
        result.push('}');
    }
    for _ in 0..needs_bracket {
        result.push(']');
    }

    // 引号修复：如果正在字符串中（奇数个引号），末尾补 "
    if in_string {
        result.push('"');
    }

    // 处理尾部多余闭合（open 为负数 → 多了闭合括号）
    // S-H3 修复：旧逻辑用 rposition 全局找最后一个 }，可能删除合法的闭合括号。
    // 新逻辑：只从字符串末尾向前删除连续的 } 或 ]，避免破坏合法 JSON 结构。
    let mut extra_curly = (-open_curly).max(0) as usize;
    while extra_curly > 0 {
        let bytes = result.as_bytes();
        let last = *bytes.last().unwrap_or(&0);
        // 只删除末尾的 }（跳过空白）
        if last == b'}' {
            result.pop();
            extra_curly -= 1;
        } else if last == b' ' || last == b'\n' || last == b'\r' || last == b'\t' {
            result.pop();
        } else {
            break;
        }
    }
    let mut extra_bracket = (-open_bracket).max(0) as usize;
    while extra_bracket > 0 {
        let bytes = result.as_bytes();
        let last = *bytes.last().unwrap_or(&0);
        if last == b']' {
            result.pop();
            extra_bracket -= 1;
        } else if last == b' ' || last == b'\n' || last == b'\r' || last == b'\t' {
            result.pop();
        } else {
            break;
        }
    }

    result
}

/// 用裸括号追踪从文本中提取指定 key 的 JSON 数组（容忍引号错乱）。
///
/// LLM 常在字符串值中使用未转义双引号（如：他说"这是关键"），
/// 导致 `serde_json` 全量解析失败。此函数绕过引号状态追踪，
/// 直接匹配 `"key": [` 找到数组起始，然后裸计 `[`/`]` 深度找到闭合，
/// 只对这一小段 `[...]` 调用 `serde_json::from_str`。
///
/// 返回 `{"candidates": [...], "trends": [...]}`（只含成功解析的 key）。
fn extract_named_arrays(text: &str) -> Option<serde_json::Value> {
    let keys = ["candidates", "trends"];
    let mut result = serde_json::Map::new();

    for key in &keys {
        let pattern = format!("\"{}\":", key);
        // 找所有匹配位置（可能有多个同名 key，取最后一个）
        let mut pos = 0;
        let mut last_match = None;
        while let Some(mut p) = text[pos..].find(&pattern) {
            p += pos;
            last_match = Some(p + pattern.len());
            pos = p + 1;
        }
        let Some(after_key) = last_match else { continue };

        // 在 after_key.. 中找第一个 [
        let remaining = &text[after_key..];
        let bracket_start = remaining.find('[')?;
        let array_slice = &remaining[bracket_start..];

        // 裸 `[`/`]` 深度追踪：不处理引号，只数括号
        let mut depth = 0u32;
        let mut end = 0;
        for (i, b) in array_slice.bytes().enumerate() {
            if b == b'[' {
                depth += 1;
            } else if b == b']' {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
        }
        if depth != 0 {
            continue;
        } // 数组未闭合

        // 尝试解析这个数组片段
        let array_text = &array_slice[..=end];
        let parsed = match serde_json::from_str::<serde_json::Value>(array_text) {
            Ok(v) => Some(v),
            Err(_) => {
                // 数组内部可能有尾逗号等小语法问题，尝试 repair_json 修复后重试
                let repaired = repair_json(array_text);
                serde_json::from_str::<serde_json::Value>(&repaired).ok()
            },
        };
        if let Some(v) = parsed {
            result.insert(key.to_string(), v);
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(result))
    }
}

/// 从文本中提取最外层 JSON 对象或数组。
/// 跳过开头的空白和非 JSON 字符，找到第一个 `{` 或 `[`，
/// 追踪括号平衡（带 in_string 追踪）找到匹配闭合，返回解析结果。
/// 如果第一个起点解析失败，尝试下一个起点。
fn extract_outer_json(text: &str) -> Option<serde_json::Value> {
    let bytes = text.as_bytes();
    let len = bytes.len();

    // 收集所有 { 和 [ 的位置
    let start_positions: Vec<usize> = bytes
        .iter()
        .enumerate()
        .filter_map(|(i, &b)| {
            if b == b'{' || b == b'[' {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    for &start in &start_positions {
        let open = bytes[start];
        let close: u8 = if open == b'{' { b'}' } else { b']' };

        let mut depth: i32 = 0;
        let mut in_string = false;
        let mut escaped = false;
        let mut found = false;
        let mut end = 0;

        for (idx, &b) in bytes[start..len].iter().enumerate() {
            let i = start + idx;
            if escaped {
                escaped = false;
                continue;
            }
            if b == b'\\' && in_string {
                escaped = true;
                continue;
            }
            if b == b'"' {
                in_string = !in_string;
                continue;
            }
            if !in_string {
                if b == open {
                    depth += 1;
                } else if b == close {
                    depth -= 1;
                    if depth == 0 {
                        found = true;
                        end = i;
                        break;
                    }
                }
            }
        }
        if !found {
            continue;
        }

        let snippet = &text[start..=end];
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(snippet) {
            return Some(v);
        }
    }
    None
}

/// 宽松 JSON 解析：处理模型在 `input` 字段里偶尔出现的轻微格式问题。
#[allow(dead_code)]
fn parse_loose_json(s: &str) -> Option<serde_json::Value> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return Some(v);
    }
    // 兼容：input 有时是单引号 / 带尾逗号 / 缺外层花括号，这里走 IR 文本块的抽取
    let candidate = axagent_kit::utils::extract_json_from_llm_response(trimmed);
    serde_json::from_str(candidate).ok()
}

/// 深度搜索：从任意嵌套的 JSON 对象中找到含 stock_code 的候选数组
/// 用于兜底提取，当正常路径（params → candidates）失败时
fn find_candidates_deep(value: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut results = Vec::new();
    match value {
        serde_json::Value::Array(arr) => {
            // 检查数组元素是否像候选对象（有 stock_code）
            for item in arr {
                if item.get("stock_code").is_some()
                    && item
                        .get("stock_code")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| !s.is_empty())
                {
                    results.push(item.clone());
                } else if item.is_object() || item.is_array() {
                    // 递归搜索
                    results.extend(find_candidates_deep(item));
                }
            }
        },
        serde_json::Value::Object(map) => {
            // 优先找 candidates/stocks 等容器字段
            for key in ["candidates", "stocks", "list", "data", "items"] {
                if let Some(v) = map.get(key) {
                    if v.is_array() {
                        for item in v.as_array().unwrap() {
                            if item.get("stock_code").is_some() {
                                results.push(item.clone());
                            }
                        }
                    }
                }
            }
            // 没找到则递归搜索所有值
            if results.is_empty() {
                for v in map.values() {
                    results.extend(find_candidates_deep(v));
                }
            }
        },
        _ => {},
    }
    results
}

/// 逐个提取候选对象：在 candidates 数组内对每个顶层 `{...}` 独立尝试解析。
/// 当某个候选对象内部有语法错误（如字符串中未转义的 `"`）时，不影响其他候选的提取。
fn extract_candidates_one_by_one(text: &str) -> Option<Vec<serde_json::Value>> {
    // 1. 定位 candidates 数组起始
    let arr_start = {
        let key_pos = text.find("\"candidates\"")?;
        let after_key = &text[key_pos + 12..];
        let bracket = after_key.find('[')?;
        key_pos + 12 + bracket + 1
    };
    let content = &text[arr_start..];
    // 2. 逐个扫描顶层对象：正确追踪 in_string
    let mut depth: i32 = 0;
    let mut obj_start: Option<usize> = None;
    let mut in_string = false;
    let mut escaped = false;
    let mut results = Vec::new();
    for (i, &b) in content.as_bytes().iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if b == b'\\' && in_string {
            escaped = true;
            continue;
        }
        if b == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if b == b'{' {
            depth += 1;
            if depth == 1 {
                obj_start = Some(i);
            }
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                if let Some(os) = obj_start.take() {
                    let slice = &content[os..=i];
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(slice) {
                        results.push(v);
                    } else {
                        // 单个候选内部有语法错误 → repair_json 后重试
                        let repaired = repair_json(slice);
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&repaired) {
                            results.push(v);
                        }
                    }
                }
            }
        } else if b == b']' && depth == 0 {
            break;
        }
    }
    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

/// 从文本内容中尝试启发式提取候选列表
/// 用于最终兜底：当所有结构化提取都失败时，直接从 LLM 文本输出中挖
/// 返回 (candidates 数组, 是否包含 summary 字段)
fn try_extract_candidates_from_text(text: &str) -> Option<(Vec<serde_json::Value>, bool)> {
    // 尝试1: 找 "candidates": [ ... ] 块，逐个提取
    // 逐个提取相比于全量解析更稳健——即使某个候选对象内部有语法错误，
    // 其他候选仍能被回收。（LLM 高频问题：字符串值中未转义的引号）
    if let Some(found) = extract_candidates_one_by_one(text) {
        let summary_pos = text.find("\"summary\"").map(|p| p.saturating_sub(500));
        let has_summary = summary_pos.is_some_and(|sp| {
            let region = &text[sp..sp.saturating_add(200)];
            region.contains(": \"") || region.contains(":\"")
        });
        return Some((found, has_summary));
    }

    // 尝试2: 搜索 "stock_code": "XXXXXX" 模式，提取周围的对象
    let mut found = Vec::new();
    let mut search_start = 0;
    while let Some(pos) = text[search_start..].find("\"stock_code\"") {
        let abs_pos = search_start + pos;
        // 验证后面跟着 : "6位数字"
        let after_key = &text[abs_pos + 12..];
        if after_key.starts_with("\": \"") {
            let code_start = abs_pos + 15;
            if code_start + 6 <= text.len() {
                let code = &text[code_start..code_start + 6];
                if code.chars().all(|c| c.is_ascii_digit()) {
                    // 向前找 { 向后找 } 来包围这个对象
                    let region_start = abs_pos.saturating_sub(300);
                    let region_end = (abs_pos + 500).min(text.len());
                    let region = &text[region_start..region_end];
                    let obj_offset = abs_pos - region_start;
                    if let Some(obj_s) = region[..obj_offset].rfind('{') {
                        if let Some(obj_e) = region[obj_s..].find('}') {
                            let candidate_str = &region[obj_s..obj_s + obj_e + 1];
                            if let Ok(obj) =
                                serde_json::from_str::<serde_json::Value>(candidate_str)
                            {
                                if obj.get("stock_code").is_some()
                                    && obj.get("stock_name").is_some()
                                {
                                    found.push(obj);
                                }
                            }
                        }
                    }
                }
            }
        }
        search_start = abs_pos + 13; // 跳过已搜索部分
    }

    if found.is_empty() {
        None
    } else {
        Some((found, false))
    }
}

/// 从节点原始输出中直接提取 candidates 数组
/// 与通用 extract_agent_output 不同，此函数直接导航已知 JSON 路径：
///   {"content": "...```json\n{\"name\": \"...\", \"arguments\": {\"candidates\": [...]}\n```..."}
/// 返回 {"candidates": [...], "summary": "..."} 或 null。
/// `summary` 取自 arguments.summary（当上游趋势/瓶颈数据缺失时，LLM 通常会
/// 在此字段给出"为什么没有候选"的解释，前端需要在空候选时把它展示给用户）。
fn serenity_extract_from_node(raw: &serde_json::Value) -> serde_json::Value {
    let content = match raw.get("content").and_then(|c| c.as_str()) {
        Some(c) => c,
        None => {
            tracing::warn!("[serenity] 节点输出无 content 字段");
            return serde_json::Value::Null;
        },
    };
    let extracted = axagent_kit::utils::extract_json_from_llm_response(content);
    let parsed: serde_json::Value = match serde_json::from_str(extracted) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[serenity] JSON 解析失败: {e}, 尝试修复链");
            // 第一层：repair_json 修复括号/引号 → 重新解析
            let repaired = repair_json(extracted);
            if let Ok(v) = serde_json::from_str(&repaired) {
                tracing::info!("[serenity] repair_json 成功");
                return v;
            }
            // 第二层：extract_named_arrays 从裁剪后文本提取
            if let Some(named) = extract_named_arrays(extracted) {
                tracing::info!("[serenity] extract_named_arrays(extracted) 成功");
                return named;
            }
            // 第三层：extract_named_arrays 从原始 content 提取
            // 裁剪后的 extracted 可能被 trim_after_json 截断，
            // 原始 content 包含完整 JSON，免疫截断问题
            if let Some(named) = extract_named_arrays(content) {
                tracing::info!("[serenity] extract_named_arrays(content) 成功");
                return named;
            }
            // 第四层：文本启发式兜底
            tracing::warn!("[serenity] 修复链前三层均失败，尝试文本兜底提取");
            if let Some((found, has_summary)) = try_extract_candidates_from_text(content) {
                if has_summary {
                    tracing::info!("[serenity] 文本兜底提取成功，0 个候选 + summary 字段");
                    return serde_json::json!({"candidates": [], "summary": "上游数据不足，无法识别有效候选标的"});
                }
                return serde_json::json!({"candidates": found});
            }
            return serde_json::Value::Null;
        },
    };
    // 🚨 2026-07-31 修复：agent_executor 拆包 tool_json 时，若 LLM 输出的 arguments 是裸数组
    // （{"name":"submit_candidates","arguments":[{candidate},...]}，GLM 偶发形态），
    // content 就是 candidates 数组本身。此前只处理"对象含 candidates 字段"，裸数组
    // 一路走到 None → 返回 Null → 种子永不注入（21:47 轮 candidate-mapper 4 候选全被丢弃）。
    if parsed.is_array() {
        let count = parsed.as_array().map(|a| a.len()).unwrap_or(0);
        tracing::info!(
            "[serenity] parsed 为裸候选数组（arguments 直出形态），直接作为 candidates: {} 个",
            count
        );
        return serde_json::json!({"candidates": parsed});
    }
    // 导航到 arguments/input → candidates
    let args = parsed
        .as_object()
        .and_then(|o| o.get("arguments"))
        .or_else(|| parsed.as_object().and_then(|o| o.get("input")));
    let candidates = match args {
        Some(a) => a.get("candidates"),
        None => parsed.as_object().and_then(|o| o.get("candidates")),
    };
    // summary 同样在 arguments.summary（或顶层 summary），用来在 candidates 为空时
    // 告知前端"为什么没有候选"（如：上游 data_gaps=true、模型反幻觉拒绝编造等）
    let summary = args
        .and_then(|a| a.get("summary"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            parsed
                .as_object()
                .and_then(|o| o.get("summary"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
        });
    match candidates {
        Some(arr) if arr.is_array() => {
            let count = arr.as_array().map(|a| a.len()).unwrap_or(0);
            tracing::info!("[serenity] 直接提取成功，找到 {} 个候选", count);
            if let Some(s) = summary {
                serde_json::json!({"candidates": arr, "summary": s})
            } else {
                serde_json::json!({"candidates": arr})
            }
        },
        Some(_) => {
            tracing::warn!(
                "[serenity] candidates 不是数组，keys={:?}",
                candidates
                    .and_then(|c| c.as_object())
                    .map(|o| o.keys().cloned().collect::<Vec<_>>())
            );
            // 即使 candidates 字段格式异常，summary 仍可能有用
            if let Some(s) = summary {
                serde_json::json!({"candidates": [], "summary": s})
            } else {
                serde_json::Value::Null
            }
        },
        None => {
            // 最后的兜底：parsed 本身可能是裸候选对象
            if parsed.get("stock_code").is_some() {
                let mut out = serde_json::json!({"candidates": [parsed]});
                if let Some(s) = summary {
                    out.as_object_mut()
                        .map(|o| o.insert("summary".to_string(), serde_json::Value::String(s)));
                }
                out
            } else if let Some(s) = summary {
                // 找不到 candidates 字段但有 summary：典型场景是 LLM 拒绝编造
                tracing::info!("[serenity] 未找到 candidates 字段但有 summary，空候选 + 原因");
                serde_json::json!({"candidates": [], "summary": s})
            } else {
                tracing::warn!(
                    "[serenity] 无法找到 candidates 字段, parsed keys={:?}",
                    parsed.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>())
                );
                serde_json::Value::Null
            }
        },
    }
}

/// 运行 Serenity 瓶颈筛选工作流（serenity-screening 模板）。
///
/// 与 run_stock_workflow 不同：
///   - 不需要 stock_code 输入（自驱动，从市场数据发现趋势）
///   - 不写 stock_analyses 表
///   - 返回候选股清单（而非单只股票的分析结论）
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description =  "运行Serenity瓶颈筛选工作流")]
#[tauri::command]
pub async fn run_serenity_screening(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    as_of_date: Option<String>,
    themes: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let engine = Arc::clone(&state.work_engine);

    // 解析 as_of_date（支持回放模式）
    let as_of_ctx = parse_asof_param(as_of_date.clone())?;

    // 运行前确保模板为最新版（幂等：版本已是最新则跳过）。
    // 修复：启动时 seed 在异步任务中执行（lib.rs:446），失败被吞掉（mod.rs:471 只 log）。
    // 若数据库停留在旧版本（如 v2 缺少 baseline_* input_mapping），
    // Rhai 脚本会报 "Variable not found: baseline_semi"。此处兜底重新 seed。
    crate::commands::stock_analysis_setup::ensure_stock_analysis_experts_seeded(state.harness.db())
        .await
        .map_err(|e| format!("重新种子化失败: {e}"))?;

    // 1. 加载 serenity-screening 模板
    let loaded = load_and_inject_template(state.harness.db(), "", "", "serenity-screening").await?;

    // 注入 vendor 启用状态过滤器（与 stock-analysis 主工作流一致）
    super::decision::inject_vendor_state(&state.astock_client, loaded.variables.as_ref());

    // v47: 注入用户主题到 variables（对话式主题荐股）
    let mut variables = loaded.variables;
    if let Some(ref theme_list) = themes {
        if !theme_list.is_empty() {
            let themes_value = serde_json::json!(theme_list);
            if let Some(ref mut vars) = variables {
                if let Some(var) = vars.iter_mut().find(|v| v.name == "user_themes") {
                    var.value = themes_value;
                } else {
                    vars.push(axagent_harness::workflow_types::Variable {
                        name: "user_themes".into(),
                        var_type: "json".into(),
                        value: themes_value,
                        description: Some("用户指定主题词列表".into()),
                        is_secret: false,
                    });
                }
            } else {
                variables = Some(vec![axagent_harness::workflow_types::Variable {
                    name: "user_themes".into(),
                    var_type: "json".into(),
                    value: themes_value,
                    description: Some("用户指定主题词列表".into()),
                    is_secret: false,
                }]);
            }
        }
    }

    let (max_concurrent, step_timeout, _total_timeout) =
        resolve_runtime_options(variables.as_deref());

    // 2. 创建 Workflow
    let wf_name = format!("serenity-screening-{}", chrono::Utc::now().timestamp_millis());
    let workflow =
        engine.create_workflow(&wf_name, loaded.nodes, loaded.edges).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("创建工作流失败: {e}"))
        })?;
    let wf_id = workflow.id.clone();
    let wf_id_ret = wf_id.clone();
    let app_h = app.clone();

    // 3. 进度回调
    let progress_app = app.clone();
    let progress_wf_id = wf_id.clone();
    let progress_cb: ProgressCallback = Arc::new(move |event: StepProgressEvent| {
        let app = progress_app.clone();
        let wf_id = progress_wf_id.clone();
        Box::pin(async move {
            let payload = serde_json::json!({
                "workflowId": wf_id,
                "type": "serenity-screening",
                "nodeId": event.node_id,
                "status": event.status,
                "totalNodes": event.total_nodes,
                "completedNodes": event.completed_nodes,
                // 修复：透传节点真实输出与错误信息（与 stock_workflow/core.rs 的
                // workflow-step-done 事件对齐）。此前只发 nodeId/status/counts，
                // 前端 SerenityScreeningPanel 执行日志永远显示"执行完成，无输出内容"，
                // 用户无法判断节点是否拿到真实数据。
                "output": event.output,
                "error": event.error,
            });
            let _ = app.emit("serenity-screening-step", payload);
        })
    });

    // 4. 运行（支持 as-of 时间截断）
    // 注入模板变量（来自 UI 可编辑的 Variables）。
    // 🚨 2026-07-31 修复：原过滤只认 ref_*/serenity_* 前缀，policy_news_keywords（t-policy-news
    // 的搜索关键词变量）被滤掉 → 运行时 resolve_var_path 查不到 → search_news keyword 恒空
    // （连续 8 轮日志"keyword="）。v17 已删除 ref_*_code，前缀白名单过时。
    // 更彻底的做法：模板 variables 全部注入（均为可编辑参数，无敏感字段）。
    // v47: 使用已注入用户主题的 variables（而非原始 loaded.variables）
    let serenity_vars: Option<Vec<axagent_harness::workflow_types::Variable>> = variables;
    let opts = RunOptions {
        max_concurrent,
        step_timeout,
        progress_callback: Some(progress_cb),
        input: None,
        input_schema: loaded.input_schema.clone(),
        output_schema: loaded.output_schema.clone(),
        variables: serenity_vars,
        dry_run: false,
        ..Default::default()
    };

    let exec = async { engine.run_workflow(&wf_id, opts).await };

    let result = as_of::AS_OF.scope(as_of_ctx, exec).await;

    match result {
        Ok(wf_result) => {
            tracing::info!(
                "[serenity] wf_result.results 所有键: {:?}",
                wf_result.results.keys().cloned().collect::<Vec<_>>(),
            );

            let candidates_raw = wf_result
                .results
                .get("c-data-verifier")  // FIX-02: 优先使用 data-verifier 的输出（含验证状态）
                .map(|v| {
                    // CodeNode 输出: {"status": "executed", "result": [...], "params": [...]}
                    // 从 result 中提取候选列表，兼容 AgentNode 的 content 格式
                    if v.get("result").is_some() {
                        serde_json::json!({"content": serde_json::to_string(&v["result"]).unwrap_or_default()})
                    } else {
                        v.clone()
                    }
                })
                // v44 修复（2026-07-31 23:05）：data-verifier 因 input_mapping 路径失效
                // （content 为 arguments 文本字符串）early return 空数组时，不能吞掉真候选，
                // 回退到 a-candidate-mapper 原始输出重新提取。
                .filter(|v| {
                    let is_empty_array = v
                        .get("content")
                        .and_then(|c| c.as_str())
                        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                        .and_then(|p| p.as_array().map(|a| a.is_empty()))
                        .unwrap_or(false);
                    !is_empty_array
                })
                .or_else(|| wf_result.results.get("a-candidate-mapper").cloned())
                .unwrap_or(serde_json::Value::Null);
            // 诊断：打印原始节点输出
            {
                let preview = serde_json::to_string(&candidates_raw)
                    .map(|s| s.chars().take(500).collect::<String>())
                    .unwrap_or_default();
                tracing::info!("[serenity] a-candidate-mapper 原始输出 (前500字符): {}", preview);
            }
            // 使用专属提取函数直接从已知 JSON 路径 (content → arguments.candidates) 提取，
            // 绕过 extract_agent_output 的复杂 fallback 逻辑（该函数在 tool_json 格式下可能返回首条候选而非完整数组）
            let candidates_raw_fallback = candidates_raw.clone();
            let candidates = serenity_extract_from_node(&candidates_raw);
            // 诊断：serenity_extract_from_node 的返回值
            {
                let preview = serde_json::to_string(&candidates)
                    .map(|s| s.chars().take(400).collect::<String>())
                    .unwrap_or_default();
                tracing::info!(
                    "[serenity] serenity_extract 返回类型={}  前400字符: {}",
                    if candidates.is_array() {
                        "数组".to_string()
                    } else if candidates.is_object() {
                        let keys = candidates
                            .as_object()
                            .map(|o| o.keys().cloned().collect::<Vec<_>>())
                            .unwrap_or_default();
                        format!("对象 keys=[{}]", keys.join(","))
                    } else if candidates.is_null() {
                        "null".to_string()
                    } else {
                        "其他".to_string()
                    },
                    preview,
                );
            }
            // 规范化：如果 extract_agent_output 返回裸候选对象（有 stock_code 但无 candidates 包装键），
            // 包装成 {"candidates": [obj]}，使下游 .get("candidates") 能正常工作。
            let candidates = if candidates.is_object()
                && !candidates.as_object().is_some_and(|o| o.contains_key("candidates"))
                && candidates.get("stock_code").is_some()
            {
                serde_json::json!({"candidates": [candidates]})
            } else {
                candidates
            };
            // 提取 candidates 数组（各种包装格式统一为平级数组，直接供前端消费）
            let raw_candidate_array = if candidates.is_array() {
                candidates.clone()
            } else if let Some(obj) = candidates.as_object() {
                obj.get("candidates").cloned().unwrap_or(serde_json::Value::Array(vec![]))
            } else {
                serde_json::Value::Array(vec![])
            };
            // 校验：过滤缺少 stock_code 的残缺候选，避免前端渲染空白卡片
            let mut candidate_array: Vec<serde_json::Value> = Vec::new();
            let mut dropped_count = 0;
            let mut exit_now_count = 0;
            if let Some(arr) = raw_candidate_array.as_array() {
                for c in arr {
                    let has_code =
                        c.get("stock_code").and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty());
                    if !has_code {
                        dropped_count += 1;
                        tracing::warn!(
                            "[serenity] 丢弃残缺候选（无 stock_code）: {}",
                            serde_json::to_string(c).unwrap_or_default()
                        );
                        continue;
                    }
                    // 自动剔除 exit_now 候选（LLM 可能违反 prompt 规则仍然输出）
                    let is_exit_now = c
                        .get("exit_signals")
                        .and_then(|es| es.get("overall_exit_urgency"))
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| s == "exit_now");
                    if is_exit_now {
                        exit_now_count += 1;
                        tracing::warn!(
                            "[serenity] 自动剔除 exit_now 候选（{}）: {}",
                            c["stock_code"].as_str().unwrap_or("?"),
                            c["exit_signals"]["overall_exit_urgency"]
                                .as_str()
                                .unwrap_or("exit_now"),
                        );
                        continue;
                    }
                    candidate_array.push(c.clone());
                }
            }
            if dropped_count > 0 || exit_now_count > 0 || candidate_array.is_empty() {
                tracing::warn!(
                    "[serenity] 候选校验: 总量={}, 有效={}, 丢弃(无stock_code)={}, 剔除(exit_now)={}, candidates原始keys={:?}",
                    raw_candidate_array.as_array().map_or(0, |a| a.len()),
                    candidate_array.len(),
                    dropped_count,
                    exit_now_count,
                    raw_candidate_array
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|c| c.as_object())
                        .map(|o| o.keys().cloned().collect::<Vec<_>>()),
                );
            }
            let mut candidate_array = serde_json::Value::Array(candidate_array);

            // 兜底：如果正常提取路径得到空数组，尝试从 candidates（extract_agent_output 结果）中深度搜索
            if candidate_array.as_array().is_none_or(|a| a.is_empty()) {
                tracing::warn!(
                    "[serenity] ⚠️ 候选数组为空，尝试兜底提取... candidates类型={} keys={:?}",
                    if candidates.is_array() {
                        "array"
                    } else if candidates.is_object() {
                        "object"
                    } else if candidates.is_null() {
                        "null"
                    } else {
                        "other"
                    },
                    candidates.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()),
                );
                // 兜底策略1: 从 candidates 对象的任意嵌套层搜索含 stock_code 的数组
                let fallback = find_candidates_deep(&candidates);
                if !fallback.is_empty() {
                    tracing::info!("[serenity] 兜底提取成功，找到 {} 个候选", fallback.len());
                    candidate_array = serde_json::json!(fallback);
                }
                // 兜底策略2: 从原始节点输出的 content 字段中提取
                if candidate_array.as_array().is_none_or(|a| a.is_empty()) {
                    if let Some(content) =
                        candidates_raw_fallback.get("content").and_then(|c| c.as_str())
                    {
                        if let Some((found, _)) = try_extract_candidates_from_text(content) {
                            if !found.is_empty() {
                                tracing::info!(
                                    "[serenity] 文本兜底提取成功，找到 {} 个候选",
                                    found.len()
                                );
                                candidate_array = serde_json::json!(found);
                            }
                        }
                    }
                }
            }

            // 提取趋势扫描结果（a-trend-scanner 节点输出）
            let trends_raw = wf_result
                .results
                .get("a-trend-scanner")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let trends = extract_agent_output(trends_raw).await;
            // 规范化：如果返回裸 trend 对象（有 trend_name 但无 trends 包装键），
            // 包装成 {"trends": [obj]}
            let trends = if trends.is_object()
                && !trends.as_object().is_some_and(|o| o.contains_key("trends"))
                && trends.get("trend_name").is_some()
            {
                serde_json::json!({"trends": [trends]})
            } else {
                trends
            };
            // trends 可能是 { trends: [...] } 对象，也可能是原始数组
            let trends_list =
                trends.as_object().and_then(|obj| obj.get("trends")).cloned().unwrap_or(trends);

            tracing::info!(
                "[serenity] candidates 提取后类型: {}, keys: {:?}; trends 提取后类型: {}",
                if candidates.is_array() {
                    "数组"
                } else if candidates
                    .as_object()
                    .map(|o| o.contains_key("candidates"))
                    .unwrap_or(false)
                {
                    "含 candidates 字段"
                } else if candidates.is_null() {
                    "null"
                } else {
                    "其他"
                },
                candidates.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()),
                if trends_list.is_array() {
                    "数组"
                } else {
                    "对象"
                },
            );

            // 提取"为什么没有候选"的原因：a-candidate-mapper 的 arguments.summary
            // 当上游三个瓶颈节点均返回 data_gaps=true 时，LLM 会拒绝编造候选
            // 并在 summary 字段说明原因；前端在 candidates 为空时展示给用户。
            let empty_reason = candidates
                .as_object()
                .and_then(|o| o.get("summary"))
                .and_then(|s| s.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            // ── 持久化 Serenity 候选到 reco_picks 表（style="serenity"）──
            // 先持久化再 emit 事件，确保数据一致性
            let mut persistence_success = true;
            let mut persistence_detail = String::new();
            // best-effort：失败只记日志，不影响返回结果
            {
                let db = state.harness.db();
                // 统一 generated_at 格式：与 recommend_stocks 一致(ISO 8601 带毫秒)
                let now_str = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();
                let ts_ms = chrono::Utc::now().timestamp_millis();
                // candidates 可能是 { candidates: [...] } 对象、{name, arguments: {candidates: [...]}} 格式、
                // 也可能是原始数组
                // 优先使用已经过校验(过滤缺 stock_code + exit_now)的 candidate_array
                let candidate_list: Vec<&serde_json::Value> =
                    if let Some(arr) = candidate_array.as_array() {
                        if !arr.is_empty() {
                            arr.iter().collect()
                        } else {
                            // 兜底：从 candidates 中提取
                            candidates
                                .as_object()
                                .and_then(|obj| {
                                    obj.get("candidates")
                                        .or_else(|| {
                                            obj.get("arguments").and_then(|a| a.get("candidates"))
                                        })
                                        .and_then(|v| v.as_array())
                                })
                                .or_else(|| candidates.as_array())
                                .map(|arr| arr.iter().collect())
                                .unwrap_or_default()
                        }
                    } else {
                        Vec::new()
                    };
                let mut detail_cache: std::collections::HashMap<String, serde_json::Value> =
                    std::collections::HashMap::new();
                let mut serenity_seed: Vec<(String, String, Option<String>)> = Vec::new();
                // 去重：同一 stock_code 只保留第一个（置信度最高的）候选
                let mut seen_codes: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for (i, c) in candidate_list.iter().enumerate() {
                    let code = c["stock_code"].as_str().unwrap_or("");
                    let name = c["stock_name"].as_str().unwrap_or("");
                    let conf = c["confidence"].as_i64().unwrap_or(50) as i32;
                    if code.is_empty() {
                        continue;
                    }
                    // 去重：跳过已处理的 code
                    if !seen_codes.insert(code.to_string()) {
                        tracing::debug!("[serenity] 跳过重复候选（{}）: 保留首次出现", code,);
                        continue;
                    }
                    // 构造完整 RecoPick JSON（与 types.rs 中 camelCase 一致）
                    // 候选数据不保证有价格/入场/止损等字段,缺失时填 0 或默认值
                    let pick_data_val = serde_json::json!({
                        "stockCode": code,
                        "stockName": name,
                        "style": "serenity",
                        "strategy_type": c.get("strategy_type").and_then(|v| v.as_str()).unwrap_or("bottleneck"),
                        "period": "mid",
                        "price": c.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "entryLow": c.get("entryLow").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "entryHigh": c.get("entryHigh").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "stopLoss": c.get("stopLoss").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "targetPrice": c.get("targetPrice").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "positionPct": c.get("positionPct").and_then(|v| v.as_f64()).unwrap_or(5.0),
                        "holdingDays": c.get("holdingDays").and_then(|v| v.as_i64()).unwrap_or(20),
                        "confidence": conf,
                        "reasons": c.get("reasons").and_then(|v| v.as_array()).map(|a| {
                            a.iter().filter_map(|v| v.as_str().map(|s| s.to_owned())).collect::<Vec<_>>()
                        }).unwrap_or_default(),
                        "riskNotes": [],
                        "secondaryStyles": [],
                        "synthetic": false,
                    });
                    // 持久化到 reco_picks
                    // 修复：style 统一设置为 "serenity"，便于前端查询历史时过滤
                    // 策略类型信息（bottleneck/policy/earnings 等）已存储在 pick_data.strategy_type 中
                    let pick_id = format!("serenity-{ts_ms}-{i}-{code}");
                    let pick = reco_picks::ActiveModel {
                        id: Set(pick_id),
                        generated_at: Set(now_str.clone()),
                        period: Set("mid".to_string()),
                        stock_code: Set(code.to_string()),
                        stock_name: Set(name.to_string()),
                        style: Set("serenity".to_string()),
                        confidence: Set(conf),
                        synthetic: Set(0),
                        seed_pool_json: Set(Some(serde_json::to_string(c).unwrap_or_default())),
                        strategy_weights_json: Set(None),
                        pick_data: Set(Some(
                            serde_json::to_string(&pick_data_val).unwrap_or_default(),
                        )),
                        created_at: Set(now_str.clone()),
                    };
                    if let Err(e) = pick.insert(db).await {
                        tracing::warn!("[serenity] 写入 reco_picks 失败 ({}): {e}", code);
                        persistence_success = false;
                        persistence_detail = format!("写入 {} 失败: {}", code, e);
                    }
                    // 构建全量数据缓存
                    detail_cache.insert(
                        code.to_string(),
                        serde_json::json!({
                            "serenity_score": c["serenity_score"],
                            "strategy_type": c.get("strategy_type").and_then(|v| v.as_str()).unwrap_or("bottleneck"),
                            "catalysts": c["catalysts"],
                            "exit_signals": c["exit_signals"],
                            "attention_metrics": c["attention_metrics"],
                            "bottleneck_product": c["bottleneck_product"],
                            "primary_risk": c["primary_risk"],
                            "relevance": c["relevance"],
                            "confidence": conf,
                        }),
                    );
                    // 构建种子列表
                    serenity_seed.push((code.to_string(), name.to_string(), None));
                }
                // 同步到全局种子 + 全量数据缓存
                if !serenity_seed.is_empty() {
                    axagent_analysis_engine::recommender::set_serenity_seed(serenity_seed);
                    axagent_analysis_engine::recommender::set_serenity_candidate_cache(
                        detail_cache,
                    );
                }
            }

            // 持久化完成后 emit completed 事件
            let persistence_status = if persistence_success {
                "completed"
            } else {
                "partial_failure"
            };
            if !persistence_success {
                tracing::warn!("[serenity] 持久化部分失败: {}", persistence_detail,);
            }
            // v47: 根据是否有用户主题判断 source
            let source = if themes.as_ref().is_some_and(|t| !t.is_empty()) {
                "user"
            } else {
                "auto"
            };
            let _ = app_h.emit(
                "serenity-screening-completed",
                serde_json::json!({
                    "workflowId": wf_id_ret,
                    "status": persistence_status,
                    "result": candidates,
                    "candidates": candidate_array,
                    "trends": trends_list,
                    "emptyReason": empty_reason,
                    "source": source,
                    "persistenceError": if persistence_success {
                        serde_json::Value::Null
                    } else {
                        serde_json::json!(persistence_detail)
                    },
                }),
            );

            // wrap array candidates for frontend
            let result_val = if candidates.is_array() {
                serde_json::json!({
                    "candidates": candidates,
                })
            } else {
                candidates
            };
            Ok(serde_json::json!({
                "status": "completed",
                "candidates": result_val["candidates"].clone(),
                "trends": trends_list,
                "emptyReason": empty_reason,
                "source": source,
            }))
        },
        Err(e) => {
            let err_msg = format!("Serenity 筛选工作流失败: {e}");
            let _ = app_h.emit(
                "serenity-screening-completed",
                serde_json::json!({
                    "workflowId": wf_id_ret,
                    "status": "failed",
                    "error": err_msg,
                }),
            );
            Err(err_msg)
        },
    }
}

/// 刷新 Serenity 候选的退出信号（Phase 3 持续监控）
/// 加载最近一次 Serenity 筛选的候选列表，逐个检查退出条件
/// 支持 as_of_date 参数用于回放模式
#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description =  "刷新Serenity退出信号")]
#[tauri::command]
pub async fn refresh_serenity_exit_signals(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<serde_json::Value, String> {
    // 如果指定了 as_of_date，在 as-of 作用域内执行
    if let Some(ref date_str) = as_of_date {
        let as_of_ctx = parse_asof_param(Some(date_str.clone())).map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("解析 as_of_date 失败: {e}"))
        })?;
        let exec = async { do_refresh_exit_signals(&state).await };
        return as_of::AS_OF.scope(as_of_ctx, exec).await;
    }
    do_refresh_exit_signals(&state).await
}

async fn do_refresh_exit_signals(state: &State<'_, AppState>) -> Result<serde_json::Value, String> {
    use axagent_entities::reco_picks;

    let db = state.harness.db();
    // 加载最近 50 条 Serenity 候选（按 created_at 降序）
    let picks = reco_picks::Entity::find()
        .filter(reco_picks::Column::Style.eq("serenity"))
        .order_by_desc(reco_picks::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询 Serenity 候选失败: {e}"))
        })?;
    // 只取最近 50 条
    let picks: Vec<_> = picks.into_iter().take(50).collect();

    let client = &state.astock_client;
    let mut results = Vec::new();

    for pick in &picks {
        let stop_loss = pick.seed_pool_json.as_ref().and_then(|seed_json| {
            serde_json::from_str::<serde_json::Value>(seed_json)
                .ok()
                .and_then(|v| v["stop_loss"].as_f64())
        });

        // 获取当前行情
        let quote = client.get_quote(&pick.stock_code).await.ok();
        let price = quote.as_ref().map(|q| q.price).unwrap_or(0.0);

        // 搜索退出相关新闻
        let news = client
            .search_news(&format!("{} 技术替代 产能过剩", pick.stock_code), 5)
            .await
            .unwrap_or_default();
        let has_disruption_news = news.len() >= 2;

        // 检查毛利率趋势
        let margin_declining = client
            .get_financials(&pick.stock_code)
            .await
            .ok()
            .and_then(|f| {
                if f.len() >= 2 {
                    let curr = f[0].gross_margin.unwrap_or(0.0);
                    let prev = f[1].gross_margin.unwrap_or(0.0);
                    Some(prev > 0.0 && curr < prev * 0.85)
                } else {
                    None
                }
            })
            .unwrap_or(false);

        // 判断退出紧迫度
        let stop_loss_hit = stop_loss.map(|sl| price < sl).unwrap_or(false);
        let urgency = if stop_loss_hit || (has_disruption_news && margin_declining) {
            "exit_now"
        } else if has_disruption_news || margin_declining {
            "caution"
        } else {
            "no_urgency"
        };

        results.push(serde_json::json!({
            "stock_code": pick.stock_code,
            "stock_name": pick.stock_name,
            "current_price": price,
            "stop_loss_hit": stop_loss_hit,
            "has_disruption_news": has_disruption_news,
            "margin_declining": margin_declining,
            "exit_urgency": urgency,
            "confidence": pick.confidence,
        }));
    }

    Ok(serde_json::json!({
        "status": "completed",
        "checked_count": results.len(),
        "exit_now_count": results.iter().filter(|r| r["exit_urgency"] == "exit_now").count(),
        "caution_count": results.iter().filter(|r| r["exit_urgency"] == "caution").count(),
        "candidates": results,
    }))
}

/// 刷新 Serenity 回馈闭环：跟踪推荐表现、验证催化剂、调优权重
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description =  "刷新Serenity回馈闭环")]
#[tauri::command]
pub async fn refresh_serenity_feedback(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
) -> Result<serde_json::Value, String> {
    // 如果指定了 as_of_date，在 as-of 作用域内执行
    if let Some(ref date_str) = as_of_date {
        let as_of_ctx = parse_asof_param(Some(date_str.clone())).map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("解析 as_of_date 失败: {e}"))
        })?;
        let exec = async { do_feedback_loop(&state).await };
        return as_of::AS_OF.scope(as_of_ctx, exec).await;
    }
    do_feedback_loop(&state).await
}

async fn do_feedback_loop(state: &State<'_, AppState>) -> Result<serde_json::Value, String> {
    use axagent_entities::reco_picks;

    let db = state.harness.db();
    // 固定取过去 30 天的 Serenity 候选，避免新工作流产出的记录不断顶替旧样本
    let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
    let cutoff = thirty_days_ago.to_rfc3339();
    let picks = reco_picks::Entity::find()
        .filter(reco_picks::Column::Style.eq("serenity"))
        .filter(reco_picks::Column::CreatedAt.gte(cutoff))
        .order_by_desc(reco_picks::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询 Serenity 候选失败: {e}"))
        })?;

    let client = &state.astock_client;
    let mut performances = Vec::new();

    for (idx, pick) in picks.iter().enumerate() {
        // 提取推荐日期（从 created_at 取前 10 字符 = YYYY-MM-DD）
        let rec_date = pick.created_at.as_str().get(..10).unwrap_or("2025-01-01");

        // 提取候选全量数据（seed_pool_json 存储的是候选 JSON 对象）
        let detail = pick
            .seed_pool_json
            .as_ref()
            .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok());

        if detail.is_none() {
            tracing::info!(
                "[serenity-feedback] pick={} seed_pool_json=None，跳过（历史数据）",
                pick.stock_code
            );
            // 历史数据：seed_pool_json 为 None，无法计算催化剂，跳过
            continue;
        }

        // 限流：每处理 1 条记录后延迟 500ms，避免触发东方财富 API 限流
        if idx > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // 计算表现：获取推荐日至今的 K 线
        tracing::info!("[serenity-feedback] pick={} 获取 K 线", pick.stock_code);
        let entry_kline = match client.get_klines(&pick.stock_code, "daily", 120).await {
            Ok(k) => {
                tracing::info!(
                    "[serenity-feedback] pick={} K 线成功, {} 条",
                    pick.stock_code,
                    k.len()
                );
                Some(k)
            },
            Err(e) => {
                tracing::warn!("[serenity-feedback] pick={} K 线失败: {e:?}", pick.stock_code);
                None
            },
        };
        let (entry_price, used_fallback) = entry_kline
            .as_ref()
            .and_then(|k| {
                // 优先找推荐日当天的 K 线
                k.iter().find(|k| k.date.starts_with(rec_date)).map(|k| (k.close, false))
                // 找不到则用倒数第二根（推荐日 K 线不在时避免与 current_price 撞车）
                .or_else(|| {
                    if k.len() >= 2 {
                        Some((k[k.len()-2].close, true))
                    } else {
                        k.last().map(|k| (k.close, true))
                    }
                })
            })
            .unwrap_or((0.0, false));
        if entry_price <= 0.0 {
            tracing::warn!(
                "[serenity-feedback] pick={} entry_price=0 (rec_date={}, kline_count={})",
                pick.stock_code,
                rec_date,
                entry_kline.as_ref().map(|k| k.len()).unwrap_or(0)
            );
        } else {
            tracing::info!(
                "[serenity-feedback] pick={} entry_price={} (rec_date={}){}",
                pick.stock_code,
                entry_price,
                rec_date,
                if used_fallback {
                    " [参考值:推荐日K线未收盘，取前一日]"
                } else {
                    ""
                },
            );
        }

        let current_quote = match client.get_quote(&pick.stock_code).await {
            Ok(q) => {
                tracing::info!(
                    "[serenity-feedback] pick={} get_quote 成功: price={}",
                    pick.stock_code,
                    q.price
                );
                Some(q)
            },
            Err(e) => {
                // 打印完整错误链，帮助定位 error sending request 的根因
                tracing::warn!(
                    "[serenity-feedback] pick={} get_quote 失败: {e:#?}",
                    pick.stock_code
                );
                // 同时打印 source chain
                let mut src: Option<&dyn std::error::Error> = std::error::Error::source(&e);
                while let Some(s) = src {
                    tracing::warn!("[serenity-feedback]   Caused by: {s:#?}");
                    src = s.source();
                }
                None
            },
        };
        let current_price = current_quote.as_ref().map(|q| q.price).unwrap_or(0.0);
        let return_pct = if entry_price > 0.0 && current_price > 0.0 {
            (current_price - entry_price) / entry_price * 100.0
        } else {
            0.0
        };

        // 验证催化剂：从 detail 中提取（兼容多种字段名）
        let catalysts_info = detail
            .as_ref()
            .map(|d| {
                // 尝试多种可能的字段名
                let arr = d
                    .get("catalysts")
                    .or_else(|| d.get("catalyst"))
                    .or_else(|| d.get("catalyst_list"));
                arr.and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0)
            })
            .unwrap_or(0);
        // 同时尝试嵌套路径：有些工作流输出把 catalysts 放在 params.catalysts
        let catalysts_info = if catalysts_info == 0 {
            detail
                .as_ref()
                .and_then(|d| {
                    d.get("params")
                        .and_then(|p| p.get("catalysts"))
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                })
                .unwrap_or(0)
        } else {
            catalysts_info
        };

        // 搜索该股相关新闻作为催化剂验证的 proxy
        let catalyst_news =
            match client.search_news(&format!("{} 财报 量产 订单", pick.stock_code), 5).await
            {
                Ok(news) => {
                    tracing::info!(
                        "[serenity-feedback] pick={} search_news 成功: {} 条",
                        pick.stock_code,
                        news.len()
                    );
                    news
                },
                Err(e) => {
                    tracing::warn!(
                        "[serenity-feedback] pick={} search_news 失败: {e:#?}",
                        pick.stock_code
                    );
                    Vec::new()
                },
            };
        let catalysts_verified_count = catalyst_news.len().min(catalysts_info);

        performances.push(serde_json::json!({
            "id": pick.id,
            "stock_code": pick.stock_code,
            "stock_name": pick.stock_name,
            "confidence": pick.confidence,
            "recommend_date": rec_date,
            "entry_price": entry_price,
            "current_price": current_price,
            "return_pct": (return_pct * 100.0).round() / 100.0,
            "is_profitable": return_pct > 0.0,
            "return_pending": used_fallback,
            "catalysts": serde_json::json!({
                "total": catalysts_info,
                "verified": catalysts_verified_count,
            }),
        }));
    }

    // 计算汇总指标
    let profitable =
        performances.iter().filter(|p| p["is_profitable"].as_bool().unwrap_or(false)).count();
    let total = performances.len();
    let avg_return = if total > 0 {
        performances.iter().map(|p| p["return_pct"].as_f64().unwrap_or(0.0)).sum::<f64>()
            / total as f64
    } else {
        0.0
    };

    Ok(serde_json::json!({
        "status": "completed",
        "total": total,
        "profitable_count": profitable,
        "win_rate": if total > 0 { (profitable as f64 / total as f64 * 100.0).round() / 100.0 } else { 0.0 },
        "avg_return_pct": (avg_return * 100.0).round() / 100.0,
        "performances": performances,
    }))
}

#[cfg(test)]
mod serenity_extract_tests {
    use super::*;
    use axagent_harness::types::{ChatResponse, ContentBlock};
    use axagent_runtime_core::DefaultResponseNormalizer;

    // ── helper：把字符串送进 IR Normalizer 拿到 ContentBlock 列表 ──
    async fn normalize(content: &str) -> Vec<ContentBlock> {
        let resp = ChatResponse {
            id: String::new(),
            model: String::new(),
            content: content.to_string(),
            thinking: None,
            usage: Default::default(),
            tool_calls: None,
        };
        let normalizer = DefaultResponseNormalizer;
        normalizer.normalize(&resp).await
    }

    // ── 1) 标准 tool_json 块：name=submit_candidates，arguments 是数据 ──
    #[tokio::test]
    async fn tool_json_block_extracts_candidates() {
        let content = r#"```tool_json
{"name": "submit_candidates", "arguments": {"candidates": [{"stock_code": "300285", "stock_name": "国瓷材料", "serenity_score": 75}], "summary": "ok"}}
```"#;
        let v = extract_via_normalizer(content).await;
        let v = v.expect("IR 提取应成功");
        let arr = v.get("candidates").and_then(|x| x.as_array()).expect("candidates 应为数组");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["stock_code"], "300285");
    }

    // ── 2) 普通 json 块（无 name 字段）→ IR 当文本块保留 → extract_json_from_llm_response 兜底 ──
    #[tokio::test]
    async fn plain_json_block_falls_back_to_text_extraction() {
        let content = r#"```json
{"trends": [{"trend_name": "AI 算力散热", "confidence": 80}]}
```"#;
        let v = extract_via_normalizer(content).await;
        let v = v.expect("纯 json 块应能解析");
        let arr = v.get("trends").and_then(|x| x.as_array()).expect("trends 应为数组");
        assert_eq!(arr[0]["trend_name"], "AI 算力散热");
    }

    // ── 3) 截断 JSON（用户日志里 "market_cap_level 混文字" 场景）──
    //     LLM 把思考文字夹进了字符串值；我们的策略是 IR + 文本块内部 JSON 解析，
    //     若破损则返回 None，让上层走降级。
    #[tokio::test]
    async fn truncated_json_returns_none_or_partial() {
        // 模拟用户日志中的破损输出：缺右括号、字符串值被截断
        let content = r#"```json
{
  "candidates": [
    {
      "stock_code": "300285",
      "stock_name": "国瓷材料",
      "market_cap_level": "中盘",
      "serenity_score": 75
    }
  ]
"#;
        // 不抛 panic，要么成功（拿到部分有效 JSON）要么返回 None
        let result = extract_via_normalizer(content).await;
        if let Some(v) = result {
            // 如果能解析，至少应该能拿到 candidates 字段
            assert!(v.get("candidates").is_some() || v.get("stock_code").is_some());
        }
        // None 也是可接受的——上层会走降级
    }

    // ── 4) IR Normalizer 自身：tool_json 块 → ContentBlock::ToolUse ──
    #[tokio::test]
    async fn normalizer_emits_tool_use_for_tool_json() {
        let blocks = normalize(
            r#"```tool_json
{"name": "submit_chain", "arguments": {"trend_name": "AI 算力"}}
```"#,
        )
        .await;
        assert!(
            blocks.iter().any(|b| matches!(b, ContentBlock::ToolUse { .. })),
            "tool_json 块应被 Normalizer 解析为 ToolUse，实际：{:?}",
            blocks
        );
    }

    // ── 5) IR Normalizer：纯文本无代码块 → ContentBlock::Text ──
    #[tokio::test]
    async fn normalizer_passes_plain_text_through() {
        let blocks = normalize("hello world").await;
        assert!(matches!(blocks.as_slice(), [ContentBlock::Text { .. }]));
    }

    // ── 6) extract_agent_output 顶层字段优先级：params > output > content ──
    #[tokio::test]
    pub(crate) async fn extract_agent_output_prefers_top_level_params() {
        let raw = serde_json::json!({
            "content": "ignored",
            "params": {"candidates": [{"stock_code": "1"}]},
            "output": {"should_not": "appear"},
        });
        let v = extract_agent_output(raw).await;
        assert_eq!(v["candidates"][0]["stock_code"], "1");
    }

    // ── 7) extract_agent_output 顶层 candidates 字段直通 ──
    #[tokio::test]
    pub(crate) async fn extract_agent_output_passes_top_level_candidates() {
        let raw = serde_json::json!({
            "candidates": [{"stock_code": "600519"}],
            "content": "ignored",
        });
        let v = extract_agent_output(raw).await;
        let arr = v.as_array().expect("candidates 应直返为数组");
        assert_eq!(arr[0]["stock_code"], "600519");
    }

    // ── 8) 兜底：content 是破损 JSON（无 code fence），返回 None（不走原始对象）──
    //     extract_via_normalizer 内部：直接尝试 `serde_json::from_str(content)` → 失败
    //     所以会从内容中找 ```json``` 失败，最终返回 None。
    #[tokio::test]
    async fn extract_via_normalizer_handles_garbage_input() {
        let v = extract_via_normalizer("not a json at all").await;
        assert!(v.is_none());
    }

    // ── 9) parse_loose_json：标准 JSON 直通 ──
    #[test]
    fn parse_loose_json_accepts_valid() {
        let v = parse_loose_json(r#"{"k": 1}"#);
        assert_eq!(v.expect("应能解析")["k"], 1);
    }

    // ── 10) parse_loose_json：空字符串 → None ──
    #[test]
    fn parse_loose_json_empty_string() {
        assert!(parse_loose_json("").is_none());
        assert!(parse_loose_json("   ").is_none());
    }
}
