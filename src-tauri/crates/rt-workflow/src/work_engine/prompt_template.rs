//! 轻量模板引擎 —— 编译时提取 {{path}} 占位符，执行时用变量表填充。
//!
//! 零外部依赖，纯 std 实现。两阶段：
//!   1. compile_prompt() — 扫描模板字符串，拆分为 Static / Slot 段
//!   2. render_prompt()  — 用 ExecutionState.variables 填充 Slot，返回最终 prompt

use std::collections::HashMap;

use serde_json::Value;

/// 模板中的一段：要么是静态文本，要么是需要填充的变量占位符。
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateSegment {
    /// 不需要替换的静态文本
    Static(String),
    /// 变量占位符，存点号分隔路径，如 "node_id.output.field"
    Slot(String),
}

/// 编译后的模板，仅存内存，不序列化到 DB。
#[derive(Debug, Clone)]
pub struct CompiledPrompt {
    pub segments: Vec<TemplateSegment>,
    /// 所有被引用的变量路径（展平去重），用于依赖分析和编辑器预览
    pub variable_refs: Vec<String>,
}

/// 模板渲染失败的错误。
#[derive(Debug, thiserror::Error)]
pub enum TemplateRenderError {
    #[error("模板变量未找到: {path}")]
    VariableNotFound { path: String },
    #[error("无法访问路径 '{path}' 中的 '{segment}': 中间值不是对象")]
    PathTraversalError { path: String, segment: String },
}

impl TemplateRenderError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::VariableNotFound { .. } => "TEMPLATE_VAR_NOT_FOUND",
            Self::PathTraversalError { .. } => "TEMPLATE_PATH_TRAVERSAL",
        }
    }
}

// ── 阶段一：编译 ──

/// 解析 system_prompt 模板，提取 `{{path}}` 占位符。
///
/// 不含 `{{` 的文本归一到单个 Static 段（向后兼容）。
pub fn compile_prompt(template: &str) -> CompiledPrompt {
    let mut segments: Vec<TemplateSegment> = Vec::new();
    let mut var_refs: Vec<String> = Vec::new();
    let mut current_text = String::new();
    let mut slot_buffer = String::new();

    enum State {
        Normal,
        SawOpen,
        InSlot,
        SawClose,
    }

    let mut state = State::Normal;
    let chars: Vec<char> = template.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        match state {
            State::Normal => {
                if ch == '{' {
                    state = State::SawOpen;
                } else {
                    current_text.push(ch);
                }
            },
            State::SawOpen => {
                if ch == '{' {
                    // 确认是 {{，刷新静态缓冲
                    if !current_text.is_empty() {
                        segments.push(TemplateSegment::Static(std::mem::take(&mut current_text)));
                    }
                    slot_buffer.clear();
                    state = State::InSlot;
                } else {
                    // 单花括号，当普通文本
                    current_text.push('{');
                    current_text.push(ch);
                    state = State::Normal;
                }
            },
            State::InSlot => {
                if ch == '}' {
                    state = State::SawClose;
                } else {
                    slot_buffer.push(ch);
                }
            },
            State::SawClose => {
                if ch == '}' {
                    // 确认是 }}，产出 Slot 段
                    let path = slot_buffer.trim().to_string();
                    if !var_refs.contains(&path) {
                        var_refs.push(path.clone());
                    }
                    segments.push(TemplateSegment::Slot(path));
                    slot_buffer.clear();
                    state = State::Normal;
                } else {
                    // 单右花括号，放回 slot 缓冲
                    slot_buffer.push('}');
                    slot_buffer.push(ch);
                    state = State::InSlot;
                }
            },
        }
        i += 1;
    }

    // EOF 时未闭合的 {{ → 当作静态文本放回
    match state {
        State::Normal => {},
        State::SawOpen => {
            current_text.push('{');
        },
        State::InSlot | State::SawClose => {
            current_text.push('{');
            current_text.push('{');
            current_text.push_str(&slot_buffer);
            if matches!(state, State::SawClose) {
                current_text.push('}');
            }
        },
    }

    if !current_text.is_empty() {
        segments.push(TemplateSegment::Static(current_text));
    }

    // 合并相邻的 Static 段（避免解析过程产生碎片）
    let mut segments = merge_adjacent_statics(segments);

    // 纯文本无 slot 的情况，归一到单个 Static 段
    if segments.is_empty() {
        segments.push(TemplateSegment::Static(String::new()));
    }

    CompiledPrompt {
        segments,
        variable_refs: var_refs,
    }
}

/// 合并相邻的 Static 段，减少碎片。
fn merge_adjacent_statics(segments: Vec<TemplateSegment>) -> Vec<TemplateSegment> {
    let mut merged: Vec<TemplateSegment> = Vec::new();
    for seg in segments {
        match seg {
            TemplateSegment::Static(s) => {
                if let Some(TemplateSegment::Static(last)) = merged.last_mut() {
                    last.push_str(&s);
                } else {
                    merged.push(TemplateSegment::Static(s));
                }
            },
            other => merged.push(other),
        }
    }
    merged
}

// ── 阶段二：渲染 ──

/// 用变量表填充编译后的模板，返回最终 prompt 字符串。
pub fn render_prompt(
    compiled: &CompiledPrompt,
    variables: &HashMap<String, Value>,
) -> Result<String, TemplateRenderError> {
    let mut result = String::new();
    for segment in &compiled.segments {
        match segment {
            TemplateSegment::Static(s) => result.push_str(s),
            TemplateSegment::Slot(path) => {
                let value = resolve_dot_path(path, variables)?;
                result.push_str(&value_to_string(value));
            },
        }
    }
    Ok(result)
}

/// 按点号路径从变量表中查找值。
///
/// 首段是 `variables` 的顶层 key，后续段递归 `get(segment)` 进入 JSON 嵌套。
fn resolve_dot_path<'a>(
    path: &str,
    variables: &'a HashMap<String, Value>,
) -> Result<&'a Value, TemplateRenderError> {
    let mut parts = path.splitn(2, '.');
    let root_key = parts.next().unwrap_or("");
    let remainder = parts.next();

    let mut current =
        variables
            .get(root_key)
            .ok_or_else(|| TemplateRenderError::VariableNotFound {
                path: path.to_string(),
            })?;
    if let Some(rest) = remainder {
        for segment in rest.split('.') {
            if !current.is_object() {
                return Err(TemplateRenderError::PathTraversalError {
                    path: path.to_string(),
                    segment: segment.to_string(),
                });
            }
            current =
                current
                    .get(segment)
                    .ok_or_else(|| TemplateRenderError::VariableNotFound {
                        path: path.to_string(),
                    })?;
        }
    }
    Ok(current)
}

/// 将 JSON Value 转为适合嵌入 prompt 的字符串。
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(map: &[(&str, &str)]) -> HashMap<String, Value> {
        map.iter()
            .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
            .collect()
    }

    // ── compile ──

    #[test]
    fn compile_empty() {
        let c = compile_prompt("");
        assert_eq!(c.segments, vec![TemplateSegment::Static(String::new())]);
        assert!(c.variable_refs.is_empty());
    }

    #[test]
    fn compile_no_braces() {
        let c = compile_prompt("Hello world");
        assert_eq!(c.segments, vec![TemplateSegment::Static("Hello world".to_string())]);
        assert!(c.variable_refs.is_empty());
    }

    #[test]
    fn compile_single_brace_ignored() {
        let c = compile_prompt("x {not_a_slot} y");
        assert_eq!(c.segments, vec![TemplateSegment::Static("x {not_a_slot} y".to_string())]);
    }

    #[test]
    fn compile_simple_slot() {
        let c = compile_prompt("Hello {{name}}!");
        assert_eq!(
            c.segments,
            vec![
                TemplateSegment::Static("Hello ".to_string()),
                TemplateSegment::Slot("name".to_string()),
                TemplateSegment::Static("!".to_string()),
            ]
        );
        assert_eq!(c.variable_refs, vec!["name"]);
    }

    #[test]
    fn compile_consecutive_slots() {
        let c = compile_prompt("{{a}}{{b}}");
        assert_eq!(
            c.segments,
            vec![
                TemplateSegment::Slot("a".to_string()),
                TemplateSegment::Slot("b".to_string()),
            ]
        );
    }

    #[test]
    fn compile_nested_path() {
        let c = compile_prompt("{{node.output.field}}");
        assert_eq!(c.segments, vec![TemplateSegment::Slot("node.output.field".to_string())]);
        assert_eq!(c.variable_refs, vec!["node.output.field"]);
    }

    #[test]
    fn compile_unmatched_open() {
        let c = compile_prompt("text {{unclosed");
        assert_eq!(c.segments, vec![TemplateSegment::Static("text {{unclosed".to_string())]);
    }

    #[test]
    fn compile_unmatched_close_in_slot() {
        let c = compile_prompt("{{a}} extra } text");
        assert_eq!(
            c.segments,
            vec![
                TemplateSegment::Slot("a".to_string()),
                TemplateSegment::Static(" extra } text".to_string()),
            ]
        );
    }

    #[test]
    fn compile_multiline() {
        let c = compile_prompt("第一行\n{{var}}\n第三行");
        assert_eq!(c.segments.len(), 3);
    }

    #[test]
    fn compile_slot_with_spaces_trimmed() {
        let c = compile_prompt("{{  name  }}");
        assert_eq!(c.segments, vec![TemplateSegment::Slot("name".to_string())]);
    }

    // ── render ──

    #[test]
    fn render_simple_substitution() {
        let c = compile_prompt("Hello {{name}}!");
        let v = vars(&[("name", "World")]);
        assert_eq!(render_prompt(&c, &v).unwrap(), "Hello World!");
    }

    #[test]
    fn render_missing_variable() {
        let c = compile_prompt("{{missing}}");
        let v = vars(&[]);
        let err = render_prompt(&c, &v).unwrap_err();
        assert!(matches!(err, TemplateRenderError::VariableNotFound { .. }));
    }

    #[test]
    fn render_nested_path() {
        let c = compile_prompt("{{n.output.text}}");
        let mut v = HashMap::new();
        v.insert("n".to_string(), serde_json::json!({"output": {"text": "hello"}}));
        assert_eq!(render_prompt(&c, &v).unwrap(), "hello");
    }

    #[test]
    fn render_nested_path_missing_key() {
        let c = compile_prompt("{{n.output.zzz}}");
        let mut v = HashMap::new();
        v.insert("n".to_string(), serde_json::json!({"output": {"text": "hello"}}));
        let err = render_prompt(&c, &v).unwrap_err();
        assert!(matches!(err, TemplateRenderError::VariableNotFound { .. }));
    }

    #[test]
    fn render_nested_path_non_object() {
        let c = compile_prompt("{{n.output}}");
        let mut v = HashMap::new();
        v.insert("n".to_string(), Value::String("not_an_object".to_string()));
        let err = render_prompt(&c, &v).unwrap_err();
        assert!(matches!(err, TemplateRenderError::PathTraversalError { .. }));
    }

    #[test]
    fn render_number_and_bool() {
        let c = compile_prompt("n={{num}}, b={{flag}}");
        let mut v = HashMap::new();
        v.insert("num".to_string(), serde_json::json!(42));
        v.insert("flag".to_string(), Value::Bool(true));
        assert_eq!(render_prompt(&c, &v).unwrap(), "n=42, b=true");
    }

    #[test]
    fn render_null_becomes_empty() {
        let c = compile_prompt("{{x}}");
        let mut v = HashMap::new();
        v.insert("x".to_string(), Value::Null);
        assert_eq!(render_prompt(&c, &v).unwrap(), "");
    }

    #[test]
    fn render_roundtrip_no_template() {
        let input = "你是研究员。\n请分析数据并给出结论。";
        let c = compile_prompt(input);
        let v = HashMap::new();
        assert_eq!(render_prompt(&c, &v).unwrap(), input);
    }

    #[test]
    fn render_roundtrip_complex() {
        let input =
            "你是 {{role}}。\n请根据 {{research_node.content}} 撰写报告。\n重点关注：{{focus}}";
        let c = compile_prompt(input);
        let mut v = HashMap::new();
        v.insert("role".to_string(), Value::String("分析师".to_string()));
        v.insert(
            "research_node".to_string(),
            serde_json::json!({"content": "市场调研结果：Q2增长15%"}),
        );
        v.insert("focus".to_string(), Value::String("成本控制".to_string()));
        let result = render_prompt(&c, &v).unwrap();
        assert_eq!(
            result,
            "你是 分析师。\n请根据 市场调研结果：Q2增长15% 撰写报告。\n重点关注：成本控制"
        );
    }

    #[test]
    fn render_composite_value_to_json_string() {
        let c = compile_prompt("{{arr}}");
        let mut v = HashMap::new();
        v.insert("arr".to_string(), serde_json::json!([1, 2, 3]));
        assert_eq!(render_prompt(&c, &v).unwrap(), "[1,2,3]");
    }

    #[test]
    fn compile_brace_at_end_of_slot() {
        // }} immediately followed by a single } should parse the slot
        let c = compile_prompt("{{x}}}");
        assert_eq!(
            c.segments,
            vec![
                TemplateSegment::Slot("x".to_string()),
                TemplateSegment::Static("}".to_string()),
            ]
        );
    }
}
