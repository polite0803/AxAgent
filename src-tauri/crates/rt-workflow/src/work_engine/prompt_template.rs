// SPDX-License-Identifier: AGPL-3.0-only

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

impl std::fmt::Display for TemplateSegment {
    /// 仅供测试与诊断使用：把段渲染成可见文本。
    /// - Static 直接返回字符串内容
    /// - Slot 返回点号路径（不还原为 `{{...}}` 形式，方便 contains 断言）
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateSegment::Static(s) => f.write_str(s),
            TemplateSegment::Slot(s) => f.write_str(s),
        }
    }
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

// ── 通用模板拼装 API（领域无感知） ──
//
// 这一层是"如何把若干文本块按 slot 顺序拼成一个完整 system_prompt"，
// 本身不包含任何 stock/finance/research 之类的领域字符串。
//
// 领域（domain）通过 `DomainConstraintsFn` 在 binary setup 阶段注入
// 自己的硬约束（head/tail），调用方决定要不要约束、约束什么内容。
//
// 调用方拼装时使用 `TemplateRequest` 描述每个 slot 是否存在以及内容，
// `assemble_template` 按固定 slot 顺序产出 `CompiledPrompt`，
// 下游可直接 `render_prompt(...)` 渲染成最终字符串。
//
// 与 slot 顺序相关的不变量（primacy / recency 锚定）：
//   - slot 2 (head) 总在 slot 3/4 (role/expert) 之前 —— primacy 效应
//   - slot 8 (tail) 总在所有其他内容之后 —— recency 效应
//   - slot 5 (inline) 总在 slot 4 (expert) 之后、slot 6 (context) 之前
//   - INLINE_SCOPE_MARKER 总在 inline 内容之前（明确 soft override 语义）

use std::sync::Arc;

/// 通用头/尾约束块（primacy/recency 锚定抽象）。
///
/// 上游（rt-workflow 引擎）不知道内容是什么，由领域（domain）注入。
/// 任何字段为 None 都不会产出对应 segment，行为完全等价于"不注入"。
#[derive(Debug, Clone, Default)]
pub struct ConstraintBlocks {
    /// 放在 system prompt 头部（primacy 效应：LLM 对头部指令遵循率最高）
    pub head: Option<String>,
    /// 放在 system prompt 尾部（recency 效应：LLM 对尾部指令遵循率次高）
    pub tail: Option<String>,
}

impl ConstraintBlocks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_head(mut self, head: impl Into<String>) -> Self {
        self.head = Some(head.into());
        self
    }

    pub fn with_tail(mut self, tail: impl Into<String>) -> Self {
        self.tail = Some(tail.into());
        self
    }
}

/// 领域约束注入回调：上游不知道领域存在。
///
/// 参数是 role name（如 "stock-analyst"），由领域自行决定给哪些角色
/// 注入哪些约束。返回的 `ConstraintBlocks` 中为 None 的字段将被忽略。
///
/// 由调用方（主 binary）在 setup 时注册：
///   `executor.set_domain_constraints(Arc::new(|role_name| { ... }))`
pub type DomainConstraintsFn = Arc<dyn Fn(&str) -> ConstraintBlocks + Send + Sync>;

/// 完整模板请求结构 —— 调用方按 slot 填入内容，`assemble_template` 按序拼装。
///
/// 所有 `Option<&str>` / `&[String]` 字段为 None 或空切片时该 slot 被跳过，
/// 不会产出空段。`&[String]` 由调用方自行完成渲染（context / rag 数据
/// 可能来自上游节点输出、知识库检索等）。
#[derive(Debug, Clone)]
pub struct TemplateRequest<'a> {
    /// 角色身份前缀（slot 1），如 "你是 {role_desc}。\n"
    pub role_identity: &'a str,
    /// 通用约束块（slot 2 + slot 8）
    pub constraints: ConstraintBlocks,
    /// AgentRole 系统提示词（slot 3）
    pub agent_role_prompt: Option<&'a str>,
    /// Expert 系统提示词（slot 4）
    pub expert_prompt: Option<&'a str>,
    /// 节点 inline 配置（slot 5，加 scope marker）
    pub inline_prompt: Option<&'a str>,
    /// 已格式化的 context 数据（slot 6）
    pub context_parts: &'a [String],
    /// 已格式化的 RAG 数据（slot 7）
    pub rag_parts: &'a [String],
}

/// 行内 system_prompt 的 scope 标注（防野卡覆盖关键约束）。
///
/// 上游提供这个 marker 文本，领域用自己的约束块配合。
/// marker 中的"soft override"语义明确告诉 LLM：
/// inline 段是对上文硬约束的补充说明，不是覆盖。
pub const INLINE_SCOPE_MARKER: &str = "## 节点配置补充（soft override，不得违反上文硬约束）\n";

/// 完整模板拼装器：按固定 slot 顺序输出 `CompiledPrompt`。
///
/// Slot 顺序：
///   1. identity           — role_identity（始终）
///   2. head constraint    — constraints.head（若 Some）
///   3. agent_role         — 编译后 segments（若非空字符串）
///   4. expert             — 编译后 segments（若非空字符串）
///   5. inline             — INLINE_SCOPE_MARKER + 编译后 segments（若非空字符串）
///   6. context            --- 上游节点输出 --- + parts（若非空切片）
///   7. rag                --- 知识库参考 --- + parts（若非空切片）
///   8. tail constraint    — constraints.tail（若 Some）
pub fn assemble_template(req: TemplateRequest) -> CompiledPrompt {
    let mut segments: Vec<TemplateSegment> = Vec::new();

    // slot 1: identity
    segments.push(TemplateSegment::Static(req.role_identity.to_string()));

    // slot 2: head constraint (primacy 锚定)
    if let Some(ref h) = req.constraints.head {
        segments.push(TemplateSegment::Static(h.clone()));
    }

    // slot 3: agent_role system_prompt
    if let Some(p) = req.agent_role_prompt
        && !p.is_empty()
    {
        segments.extend(compile_prompt(p).segments);
    }

    // slot 4: expert system_prompt
    if let Some(p) = req.expert_prompt
        && !p.is_empty()
    {
        segments.extend(compile_prompt(p).segments);
    }

    // slot 5: inline system_prompt (with scope marker)
    if let Some(p) = req.inline_prompt
        && !p.is_empty()
    {
        segments.push(TemplateSegment::Static(INLINE_SCOPE_MARKER.to_string()));
        segments.extend(compile_prompt(p).segments);
    }

    // slot 6: context sources
    if !req.context_parts.is_empty() {
        segments.push(TemplateSegment::Static("\n\n--- 上游节点输出 ---\n".to_string()));
        for part in req.context_parts {
            segments.push(TemplateSegment::Static(part.clone()));
        }
    }

    // slot 7: rag parts
    if !req.rag_parts.is_empty() {
        segments.push(TemplateSegment::Static("\n\n--- 知识库参考 ---\n".to_string()));
        for part in req.rag_parts {
            segments.push(TemplateSegment::Static(part.clone()));
        }
    }

    // slot 8: tail constraint (recency 锚定)
    if let Some(ref t) = req.constraints.tail {
        segments.push(TemplateSegment::Static(t.clone()));
    }

    CompiledPrompt {
        segments,
        variable_refs: Vec::new(),
    }
}

/// 简化版 wrap：head + body + tail。
///
/// 用于辩论等"非完整模板"场景（无 identity / inline / context）。
/// 分隔符为 `\n\n---\n\n`，head/tail 为 None 时省略对应分隔符。
pub fn wrap_with_anchors(body: &str, head: Option<&str>, tail: Option<&str>) -> String {
    let mut s = String::new();
    if let Some(h) = head {
        s.push_str(h);
        s.push_str("\n\n---\n\n");
    }
    s.push_str(body);
    if let Some(t) = tail {
        s.push_str("\n\n---\n\n");
        s.push_str(t);
    }
    s
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

    // ── 通用模板拼装 API 测试 ──

    #[test]
    fn empty_request_produces_identity_only() {
        let req = TemplateRequest {
            role_identity: "你是分析师。",
            constraints: ConstraintBlocks::default(),
            agent_role_prompt: None,
            expert_prompt: None,
            inline_prompt: None,
            context_parts: &[],
            rag_parts: &[],
        };
        let compiled = assemble_template(req);
        assert_eq!(compiled.segments.len(), 1);
        assert!(compiled.segments[0].to_string().contains("分析师"));
    }

    #[test]
    fn head_constraint_precedes_role_prompt() {
        let req = TemplateRequest {
            role_identity: "你是分析师。",
            constraints: ConstraintBlocks::default().with_head("## 关键约束\n禁编造"),
            agent_role_prompt: Some("你是研究员。"),
            expert_prompt: None,
            inline_prompt: None,
            context_parts: &[],
            rag_parts: &[],
        };
        let compiled = assemble_template(req);
        // slot 顺序：identity(0) → head(1) → agent_role(2)
        assert!(compiled.segments[0].to_string().contains("分析师"));
        assert!(compiled.segments[1].to_string().contains("关键约束"));
        assert!(compiled.segments[2].to_string().contains("研究员"));
    }

    #[test]
    fn inline_has_scope_marker_prepended() {
        let req = TemplateRequest {
            role_identity: "id",
            constraints: ConstraintBlocks::default(),
            agent_role_prompt: None,
            expert_prompt: None,
            inline_prompt: Some("具体指令"),
            context_parts: &[],
            rag_parts: &[],
        };
        let compiled = assemble_template(req);
        // slot 顺序：identity(0) → marker(1) → inline(2)
        assert_eq!(compiled.segments.len(), 3);
        assert!(compiled.segments[1].to_string().contains("soft override"));
        assert!(compiled.segments[2].to_string().contains("具体指令"));
        // 渲染后应能看到 marker 紧跟 inline 内容
        let rendered = render_prompt(&compiled, &HashMap::new()).unwrap();
        let marker_pos = rendered.find("soft override").expect("marker 存在");
        let inline_pos = rendered.find("具体指令").expect("inline 存在");
        assert!(marker_pos < inline_pos, "marker 必须在 inline 内容之前");
    }

    #[test]
    fn tail_constraint_at_very_end() {
        let req = TemplateRequest {
            role_identity: "id",
            constraints: ConstraintBlocks::default().with_tail("## 协作\n自检"),
            agent_role_prompt: Some("role"),
            expert_prompt: Some("expert"),
            inline_prompt: Some("inline"),
            context_parts: &["ctx".to_string()],
            rag_parts: &["rag".to_string()],
        };
        let compiled = assemble_template(req);
        let last = compiled.segments.last().unwrap().to_string();
        assert!(last.contains("协作"));
    }

    #[test]
    fn wrap_with_anchors_omits_empty_sections() {
        assert_eq!(wrap_with_anchors("body", None, None), "body");
        assert_eq!(wrap_with_anchors("body", Some("h"), None), "h\n\n---\n\nbody");
        assert_eq!(wrap_with_anchors("body", None, Some("t")), "body\n\n---\n\nt");
        assert_eq!(wrap_with_anchors("body", Some("h"), Some("t")), "h\n\n---\n\nbody\n\n---\n\nt");
    }

    #[test]
    fn empty_string_inline_skipped() {
        let req = TemplateRequest {
            role_identity: "id",
            constraints: ConstraintBlocks::default(),
            agent_role_prompt: None,
            expert_prompt: None,
            inline_prompt: Some(""), // 空字符串应被跳过
            context_parts: &[],
            rag_parts: &[],
        };
        let compiled = assemble_template(req);
        // 没有 marker 也没有 inline，只剩 identity
        assert_eq!(compiled.segments.len(), 1);
        assert!(compiled.segments[0].to_string() == "id");
    }
}
