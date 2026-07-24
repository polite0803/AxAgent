// SPDX-License-Identifier: AGPL-3.0-only

//! 代码执行器 —— 执行 CodeNode 中的代码片段。
//!
//! 支持两种模式：
//! - `execute_directly = false`（默认）：Rhai 脚本注册为工具，由 Agent/LLM 调用
//! - `execute_directly = true`：在 DAG 中直接执行 Rhai 代码，通过 input_mapping
//!   从 context.variables 读取结构化参数，输出 JSON 结果

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;
use rhai::Engine;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

// 复用 harness 下沉的通用 Rhai 函数与转换工具，避免重复定义（铁律 4）。
// 历史上 rt-workflow 与 quant 各自维护一份 json_value_to_dynamic/json_value_to_rhai，
// 且 rt-workflow 版本把 JSON 整数静默转 f64，存在 subtle bug。下沉后统一采纳 quant 版本语义。
// P1-D10: 引入 AST 缓存，避免批量分析时重复编译 portfolio-mgr.rhai（1373 行）等静态脚本。
use axagent_harness::{
    dynamic_to_json_value, get_or_compile_ast, json_value_to_dynamic, register_common_functions,
};

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    NodeError, NodeExecutorTrait, NodeOutput, error_code,
};

pub struct CodeExecutor;

impl CodeExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CodeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 共享 Rhai Engine 单例（池化 + 复用），避免每次执行重复分配与初始化。
fn shared_rhai_engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut engine = Engine::new();
        // SECURITY (C4): Rhai 沙箱限制 — 防 DoS
        engine.set_max_operations(200_000);
        engine.set_max_call_levels(32);
        engine.set_max_modules(0);
        engine.set_max_string_size(2_000_000);
        engine.set_max_array_size(50_000);
        engine.set_max_expr_depths(1024, 1024);
        register_common_functions(&mut engine);
        // P1-D10 修复: 调用 wiring 层注册的额外初始化函数（如 pm_* 函数）。
        // rt-workflow 是 hybrid 层，不能依赖 stock-analysis（AxInvest 专属 implementor），
        // 但 wiring 层（src/init/）可以同时依赖两者，通过此回调在 Engine 初始化时
        // 注入 portfolio-mgr.rhai 依赖的 pm_evidence_scale / pm_kelly_position 等函数。
        // 修复前：DAG 主路径未注册 pm_*，导致 portfolio-mgr.rhai 的 pm_* 调用失败
        // 被 try/catch 吞掉，决策永远走保守兜底路径（action="观望", confidence=0）。
        if let Some(init) = EXTRA_INITIALIZER.get() {
            init(&mut engine);
        }
        engine
    })
}

/// 额外的 Engine 初始化回调类型。
/// wiring 层通过 [`register_shared_engine_initializer`] 注册，
/// 在 `shared_rhai_engine()` 首次创建时调用。
type EngineInitializer = Box<dyn Fn(&mut Engine) + Send + Sync>;

static EXTRA_INITIALIZER: OnceLock<EngineInitializer> = OnceLock::new();

/// 注册额外的 Rhai Engine 初始化函数。
///
/// 必须在 `shared_rhai_engine()` 首次调用前注册（通常在应用启动时 `src/init/` 调用）。
/// 后续注册不会生效（`OnceLock::set` 在已初始化后返回 Err）。
///
/// # 用途
///
/// rt-workflow（hybrid 层）不能依赖 AxInvest 专属 crate（如 stock-analysis），
/// 但 portfolio-mgr.rhai 调用了 `pm_evidence_scale` 等 5 个由 stock-analysis
/// 提供的 Rust 函数。wiring 层通过此回调在 Engine 初始化时注册这些函数，
/// 使 DAG 主路径执行 portfolio-mgr.rhai 时 pm_* 调用能正确解析。
pub fn register_shared_engine_initializer(init: EngineInitializer) {
    if EXTRA_INITIALIZER.set(init).is_err() {
        tracing::warn!(
            "[code_executor] register_shared_engine_initializer 调用过晚：\
             shared_rhai_engine 已初始化，额外注册将被忽略"
        );
    }
}

/// 执行 Rhai 脚本的 in-process 引擎。
/// 通过 `input_mapping` 从 context.variables 读取注入值为数字/字符串，
/// 并通过 Rhai 的 `Scope` 传递给脚本，执行后收集结果构造 JSON 输出。
///
/// Phase 5: 返回 (script_result, input_params_snapshot) 二元组。
/// input_params_snapshot 是所有 input_mapping 解析值的快照，
/// 用于 What-If 回测 UI 读取原始参数值。
///
/// P1-D10: 通过 `cache_key` 复用全局 AST 缓存，避免批量分析时重复编译。
/// `cache_key` 通常传 node_id（如 "portfolio-mgr"），仅用于日志诊断。
async fn execute_rhai_directly(
    cache_key: &str,
    code: &str,
    input_mapping: &std::collections::HashMap<String, String>,
    context: &ExecutionState,
) -> Result<(serde_json::Value, serde_json::Value), NodeError> {
    let mut input_params_snapshot = serde_json::Map::new();

    // V49 诊断：input_mapping 是否为空（空则所有变量丢失）
    tracing::debug!(
        "[code_executor V49] input_mapping entries={}, keys={:?}",
        input_mapping.len(),
        input_mapping.keys().collect::<Vec<_>>()
    );

    // 将 input_mapping 的值注入 Rhai scope
    let mut scope_vars: HashMap<String, rhai::Dynamic> = HashMap::new();
    for (target_key, source_key) in input_mapping {
        let value = super::resolve_var_path(source_key, &context.variables);
        // 记录解析值的快照（Phase 5: What-If 回测参数持久化）
        let snapshot_value = value.clone().unwrap_or(Value::Null);
        input_params_snapshot.insert(target_key.clone(), snapshot_value);
        // V49: 统一转为 Dynamic 再 push_constant，避免 push_dynamic 在 v1.25 中静默失败
        let dyn_val = match &value {
            Some(Value::Null) | None => rhai::Dynamic::UNIT,
            Some(Value::Bool(b)) => rhai::Dynamic::from(*b),
            Some(Value::Number(n)) => {
                if let Some(f) = n.as_f64() {
                    rhai::Dynamic::from(f)
                } else if let Some(i) = n.as_i64() {
                    rhai::Dynamic::from(i as f64)
                } else if let Some(u) = n.as_u64() {
                    rhai::Dynamic::from(u as f64)
                } else {
                    rhai::Dynamic::from(0.0_f64)
                }
            },
            Some(Value::String(s)) => rhai::Dynamic::from(s.clone()),
            Some(v) => json_value_to_dynamic(v),
        };
        scope_vars.insert(target_key.clone(), dyn_val);
    }
    // V29 诊断：记录所有 input_mapping resolve 结果，精确定位哪个变量解析失败
    tracing::warn!(
        "[code_executor] input_mapping snapshot: {}",
        serde_json::to_string(&input_params_snapshot).unwrap_or_default()
    );

    // V57: 扫描脚本中 `present(xxx)` / `present(xxx.yyy)` 调用，对未在 input_mapping
    // 中提供的变量自动补 unit 默认值，避免 Rhai 在求值参数阶段直接抛
    // `Variable not found: xxx`（present 函数体根本进不去）。
    // 这一次性根治所有 Rhai 脚本对未注入变量的安全访问问题，
    // 无需每次新增 input_mapping 占位都重新生成工作流模板。
    //
    // V57-fix: 排除函数形参（`fn name(param)`）和脚本内 `let` 局部变量，
    // 避免误报（如 portfolio-mgr.rhai 中 fn present(x) 的形参 x、
    // fn safe_parse(raw) 的形参 raw、let f7_signal = ... 的局部变量 f7_signal）。
    let local_vars = extract_local_vars(code);
    let missing_vars = extract_present_vars(code)
        .into_iter()
        .filter(|name| !scope_vars.contains_key(name))
        .filter(|name| !local_vars.contains(name))
        .collect::<Vec<_>>();
    if !missing_vars.is_empty() {
        tracing::warn!("[code_executor] V57 自动补默认 unit 的未注入变量: {:?}", missing_vars);
        for name in &missing_vars {
            scope_vars.insert(name.clone(), rhai::Dynamic::UNIT);
        }
    }

    // 执行脚本，期望返回一个 map
    // scope_vars 已从 input_mapping 直接构建为 HashMap，避免 Rhai Scope 的 Send 限制。
    // 使用 `eval_ast_with_scope` 而非 `eval`：`eval` 不接受 scope，会导致 input_mapping
    // 注入的所有变量（llm_events / money_flow_net 等）根本无法被脚本访问，
    // 触发 `Variable not found: xxx` 错误。这是 V57 修复的真正根因。
    // 使用 `eval_ast_with_scope` 而非 `eval_expression_with_scope`：.rhai 脚本含 `fn` 定义、
    // `let` 语句等，`eval_expression_*` 仅支持单个表达式，遇到 `fn`/`let` 会报 "Unexpected 'fn'"。
    //
    // P1-D10: 先通过全局 AST 缓存获取编译后的 AST（首次编译，后续命中缓存），
    // 再用 `eval_ast_with_scope` 执行。避免批量分析时重复编译 1373 行的 portfolio-mgr.rhai。
    // AST 与 Engine 解耦：AST 只包含语法树，函数在 eval 时按 Engine 查找，
    // 因此缓存的 AST 可被任意 Engine（含不同函数注册集）执行。
    let code_owned = code.to_string();
    let cache_key_owned = cache_key.to_string();
    let join = tokio::task::spawn_blocking(move || {
        let engine = shared_rhai_engine();
        // P1-D10: 获取或编译 AST（全局缓存，首次编译后永久命中）
        let ast = get_or_compile_ast(&cache_key_owned, &code_owned, engine).map_err(|e| {
            tracing::error!(error = %e, "Rhai AST 编译失败");
            e
        })?;
        let mut scope = rhai::Scope::new();
        for (k, v) in scope_vars {
            scope.push_constant(k, v);
        }
        engine.eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast).map_err(|e| e.to_string())
    });
    let result: rhai::Dynamic = match tokio::time::timeout(
        std::time::Duration::from_secs(30), // P2-18: 30s 硬上限
        join,
    )
    .await
    {
        Ok(Ok(Ok(v))) => v,
        Ok(Ok(Err(e))) => {
            tracing::error!(error = %e, "Rhai 执行失败");
            return Err(NodeError::exec_failed(
                error_code::VALIDATION_FAILED,
                format!("Rhai execution failed: {e}"),
            ));
        },
        Ok(Err(join_err)) => {
            tracing::error!(error = %join_err, "Rhai 任务被取消");
            return Err(NodeError::exec_failed(
                error_code::TIMEOUT,
                "Rhai task cancelled".to_string(),
            ));
        },
        Err(_elapsed) => {
            tracing::error!("Rhai 执行超时（30s）—— 强制终止");
            return Err(NodeError::exec_failed(
                error_code::TIMEOUT,
                "Rhai execution exceeded 30s timeout".to_string(),
            ));
        },
    };

    // 将 Rhai 结果转换回 JSON
    Ok((dynamic_to_json_value(&result), Value::Object(input_params_snapshot)))
}

/// 从 Rhai 脚本源码中提取所有 `present(<var>)` / `present(<var>.field)` 调用
/// 中引用的顶层变量名，用于在 scope 中预先补上默认 unit 值，避免
/// `Variable not found` 错误（Rhai 在求值参数阶段就会失败，函数体进不去）。
///
/// 仅识别合法 Rhai 标识符（`[_a-zA-Z][_a-zA-Z0-9]*`），跳过 `_present(` / `fpresent(`
/// 这类更长标识符的子串匹配。去重后返回。
fn extract_present_vars(code: &str) -> Vec<String> {
    use std::collections::HashSet;

    let mut result: HashSet<String> = HashSet::new();
    let bytes = code.as_bytes();
    let n = bytes.len();
    let needle = b"present(";

    let mut i = 0usize;
    while i + needle.len() <= n {
        if &bytes[i..i + needle.len()] != needle {
            i += 1;
            continue;
        }
        // 检查前一个字符是否为标识符字符，若是则跳过（避免匹配 _present( / fpresent(）
        // 使用 let-chain 风格合并 if 以满足 clippy::collapsible_if
        if i > 0 && {
            let prev = bytes[i - 1];
            prev.is_ascii_alphanumeric() || prev == b'_'
        } {
            i += 1;
            continue;
        }
        // 跳过 `present(` 后的前导空白
        let mut k = i + needle.len();
        while k < n && (bytes[k] == b' ' || bytes[k] == b'\t') {
            k += 1;
        }
        // 读取标识符
        if k < n && (bytes[k].is_ascii_alphabetic() || bytes[k] == b'_') {
            let id_start = k;
            while k < n && (bytes[k].is_ascii_alphanumeric() || bytes[k] == b'_') {
                k += 1;
            }
            if let Ok(name) = std::str::from_utf8(&bytes[id_start..k])
                && !name.is_empty()
            {
                result.insert(name.to_string());
            }
        }
        i += needle.len();
    }

    let mut v: Vec<String> = result.into_iter().collect();
    v.sort();
    v
}

/// 扫描 Rhai 脚本中的局部变量定义，返回需要排除的变量名集合。
///
/// 包括两类：
/// 1. 函数形参：`fn name(param1, param2)` → param1, param2
/// 2. `let` 局部变量：`let var = ...` → var
///
/// 这些变量在脚本内部定义，不需要从 input_mapping 注入，
/// 因此 V57 自动补默认值时应跳过它们，避免误报。
fn extract_local_vars(code: &str) -> std::collections::HashSet<String> {
    use std::collections::HashSet;
    let mut result: HashSet<String> = HashSet::new();

    // 1. 匹配函数形参：fn <name>(<params>)
    //    形参用逗号分隔，可能有空白
    for line in code.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("fn ") {
            if let Some(close) = rest.find('(') {
                let after_paren = &rest[close + 1..];
                if let Some(close_paren) = after_paren.find(')') {
                    let params_str = &after_paren[..close_paren];
                    for param in params_str.split(',') {
                        let p = param.trim();
                        // 跳过 `&` 引用标记和类型注解
                        let p = p.strip_prefix('&').unwrap_or(p).trim();
                        let p = p.split(':').next().unwrap_or(p).trim();
                        if !p.is_empty()
                            && (p.as_bytes()[0].is_ascii_alphabetic() || p.as_bytes()[0] == b'_')
                        {
                            result.insert(p.to_string());
                        }
                    }
                }
            }
        }
    }

    // 2. 匹配 let 局部变量：let <name> = ...
    //    仅匹配顶层 let，不匹配 for 循环中的 let（虽然 Rhai 中 let 在任何位置都定义局部变量）
    for line in code.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("let ") {
            // 读取变量名（到空格、=、分号为止）
            let mut name_end = 0;
            for (i, c) in rest.char_indices() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    name_end = i + c.len_utf8();
                } else {
                    break;
                }
            }
            if name_end > 0 {
                if let Ok(name) = std::str::from_utf8(&rest.as_bytes()[..name_end])
                    && !name.is_empty()
                {
                    result.insert(name.to_string());
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod extract_present_vars_tests {
    use super::{extract_local_vars, extract_present_vars};

    #[test]
    fn extracts_simple_present_calls() {
        let code = r#"
            if present(llm_events) && llm_events != "" { }
            if present(announcement_events) { }
        "#;
        let mut v = extract_present_vars(code);
        v.sort();
        assert_eq!(v, vec!["announcement_events".to_string(), "llm_events".to_string()]);
    }

    #[test]
    fn skips_substring_matches() {
        // 不应匹配 _present( 或 fpresent(
        let code = "let x = _present(foo);";
        assert!(extract_present_vars(code).is_empty());
        let code2 = "let y = fpresent(bar);";
        assert!(extract_present_vars(code2).is_empty());
    }

    #[test]
    fn handles_present_with_member_access() {
        // present(obj.field) 应提取 "obj"
        let code = "if present(obj.field) { }";
        let v = extract_present_vars(code);
        assert_eq!(v, vec!["obj".to_string()]);
    }

    #[test]
    fn dedupes_repeated_vars() {
        let code = r#"
            if present(x) { }
            if present(x) { }
            if present(y) { }
        "#;
        let mut v = extract_present_vars(code);
        v.sort();
        assert_eq!(v, vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn handles_tight_whitespace() {
        let code = "if present(   spaced   ) { }";
        let v = extract_present_vars(code);
        assert_eq!(v, vec!["spaced".to_string()]);
    }

    #[test]
    fn extract_local_vars_fn_params() {
        let code = r#"
            fn present(x) { type_of(x) != "()" }
            fn safe_parse(raw) { if !present(raw) { return (); } }
        "#;
        let vars = extract_local_vars(code);
        assert!(vars.contains("x"));
        assert!(vars.contains("raw"));
    }

    #[test]
    fn extract_local_vars_let_bindings() {
        let code = r#"
            let f7_signal = if present(trader_direction) { 0.5 } else { 0.0 };
            let score = 100.0;
        "#;
        let vars = extract_local_vars(code);
        assert!(vars.contains("f7_signal"));
        assert!(vars.contains("score"));
    }

    #[test]
    fn extract_local_vars_portfolio_mgr_scenario() {
        // 模拟 portfolio-mgr.rhai 中的场景：
        // fn present(x) / fn safe_parse(raw) / let f7_signal = ...
        // V57 不应误报这三个变量
        let code = r#"
            fn present(x) { type_of(x) != "()" }
            fn safe_parse(raw) { if !present(raw) { return (); } }
            let f7_signal = if present(trader_direction) { 0.5 } else { 0.0 };
            if present(f7_signal) && f7_weight > 0.0 { }
        "#;
        let local_vars = extract_local_vars(code);
        let present_vars = extract_present_vars(code);
        // present_vars 应包含 f7_signal 和 trader_direction
        assert!(present_vars.contains(&"f7_signal".to_string()));
        assert!(present_vars.contains(&"trader_direction".to_string()));
        // local_vars 应包含 x, raw, f7_signal
        assert!(local_vars.contains("x"));
        assert!(local_vars.contains("raw"));
        assert!(local_vars.contains("f7_signal"));
        // 过滤后应只剩 trader_direction（真正需要从外部注入的变量）
        let filtered: Vec<String> =
            present_vars.into_iter().filter(|name| !local_vars.contains(name)).collect();
        assert_eq!(filtered, vec!["trader_direction".to_string()]);
    }
}

#[async_trait]
impl NodeExecutorTrait for CodeExecutor {
    fn node_type(&self) -> &'static str {
        "code"
    }

    async fn execute(
        &self,
        node: &WorkflowNode,
        context: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Code(code_node) = node else {
            return Err(NodeError::type_mismatch(
                "code".to_string(),
                super::node_type_name(node).to_string(),
            ));
        };

        // ── 直接执行模式（execute_directly=true）──
        // Rhai 脚本在 DAG 中直接执行，通过 input_mapping 消费上游结构化参数。
        if code_node.config.execute_directly && code_node.config.language == "rhai" {
            tracing::warn!(
                "[code_executor] Rhai execution: node_id={}, input_mapping keys={:?}, variables keys count={}, has_t_scoring={}, has_debate_convergence={}, has_a_catalyst={}, has_raw_data={}, sample_keys={:?}, totalScore resolve={:?}, consensusScore resolve={:?}, catalyst_level resolve={:?}",
                code_node.base.id,
                code_node.config.input_mapping.keys().collect::<Vec<_>>(),
                context.variables.keys().count(),
                context.variables.contains_key("t-scoring"),
                context.variables.contains_key("debate-convergence"),
                context.variables.contains_key("a-catalyst"),
                context.variables.contains_key("raw-data"),
                context.variables.keys().take(10).collect::<Vec<_>>(),
                super::resolve_var_path("t-scoring.result.totalScore", &context.variables),
                super::resolve_var_path(
                    "debate-convergence.content.consensus_score",
                    &context.variables
                ),
                super::resolve_var_path("a-catalyst.content.catalyst_level", &context.variables),
            );
            let (result, input_params) = execute_rhai_directly(
                &code_node.base.id,
                &code_node.config.code,
                &code_node.config.input_mapping,
                context,
            )
            .await?;
            // Phase 5: 将 input_mapping 解析值快照嵌入 output.input_params，
            // 确保 What-If 回测 UI 可直接读取原始参数值，无需从上游节点重建。
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "status": "executed",
                    "language": "rhai",
                    "result": result,
                    "input_params": input_params,
                    "node_id": node.base_id(),
                    // 将 result 中的关键决策字段提升到 params 层，供下游 resolve_var_path 消费
                    "params": result,
                }),
                output_var: Some(code_node.config.output_var.clone()),
                control: None,
            });
        }

        // ── 工具注册模式（向后兼容）──
        // Rhai 脚本已在预处理阶段编译并注册为工具，DAG 中无需执行
        if code_node.config.language == "rhai" {
            let tool_name = code_node
                .config
                .tool_name
                .clone()
                .unwrap_or_else(|| format!("code_{}", code_node.base.id));
            return Ok(NodeOutput {
                output: serde_json::json!({
                    "status": "tool_registered",
                    "tool_name": tool_name,
                    "note": "Rhai 脚本已注册为工具，由 Agent/LLM 调用，无需 DAG 执行",
                    "node_id": node.base_id(),
                }),
                output_var: Some(code_node.config.output_var.clone()),
                control: None,
            });
        }

        // 非 Rhai 语言：返回代码摘要供 LLM 或下游节点使用
        let code_lines = code_node.config.code.lines().count();
        Ok(NodeOutput {
            output: serde_json::json!({
                "status": "code_ready",
                "language": code_node.config.language,
                "code_lines": code_lines,
                // V37 修复: 按 char 边界取前缀，避免 .len().min(500) 落在多字节 UTF-8
                // 字符中间导致 panic
                "code_preview": code_node.config.code.chars().take(500).collect::<String>(),
                "node_id": node.base_id(),
            }),
            output_var: Some(code_node.config.output_var.clone()),
            control: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 pace-calc.rhai 能被 Rhai 引擎编译通过（不含 fn/let 语法错误）。
    /// 历史上 `eval_expression` 不支持 `fn`/`let` 语句，改用 `eval` 后需确认所有脚本都能编译。
    ///
    /// V57: 同时验证 `extract_present_vars` 自动补 unit 机制 + `eval_with_scope`
    /// 能根治 `Variable not found: llm_events` 错误。
    #[test]
    fn pace_calc_rhai_compiles_with_eval() {
        let code = include_str!("../../../../../src/commands/pace-calc.rhai");
        let mut engine = Engine::new();
        register_common_functions(&mut engine);
        let mut scope = rhai::Scope::new();
        // V57: 模拟 execute_rhai_directly 的自动补默认 unit 逻辑
        for name in extract_present_vars(code) {
            scope.push_constant(name, rhai::Dynamic::UNIT);
        }
        let result = engine.eval_with_scope::<rhai::Dynamic>(&mut scope, code);
        if let Err(e) = result {
            panic!("pace-calc.rhai 编译失败: {e}");
        }
    }
}
