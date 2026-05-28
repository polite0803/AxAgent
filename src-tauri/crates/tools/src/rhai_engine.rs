//! Rhai 脚本引擎 —— 编译、缓存、执行 Rhai 脚本作为动态工具。
//!
//! 编译（工作流创建/更新时）→ 缓存 AST → 执行时注册为 tool_handler

use rhai::{AST, Engine, Scope};
use std::collections::HashMap;
use std::sync::Arc;

/// 编译结果缓存
pub type RhaiScriptCache = HashMap<String, Arc<AST>>;

/// 创建 Rhai 引擎并注入标准 API
pub fn create_rhai_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(100_000);
    engine.set_max_call_levels(16);
    engine.set_max_modules(0);
    engine
}

/// 编译一段 Rhai 脚本，成功返回 AST。
pub fn compile_script(engine: &Engine, script: &str) -> Result<AST, String> {
    engine
        .compile(script)
        .map_err(|e| format!("Rhai 编译失败: {e}"))
}

/// 对工作流中的 Code(rhai) 节点做批量编译，返回 (tool_name → AST) 映射。
pub fn compile_workflow_rhai_scripts(
    nodes: &[axagent_core::workflow_types::WorkflowNode],
) -> RhaiScriptCache {
    let engine = create_rhai_engine();
    let mut cache = HashMap::new();

    for node in nodes {
        if let axagent_core::workflow_types::WorkflowNode::Code(code_node) = node {
            if code_node.config.language != "rhai" || code_node.config.code.is_empty() {
                continue;
            }
            let tool_name = code_node
                .config
                .tool_name
                .clone()
                .unwrap_or_else(|| format!("code_{}", code_node.base.id));

            match compile_script(&engine, &code_node.config.code) {
                Ok(ast) => {
                    tracing::info!(
                        "[RhaiEngine] 编译成功: {tool_name} ({} 字节)",
                        code_node.config.code.len()
                    );
                    cache.insert(tool_name, Arc::new(ast));
                },
                Err(e) => {
                    tracing::warn!("[RhaiEngine] 编译失败 {tool_name}: {e} — 跳过此脚本");
                },
            }
        }
    }

    cache
}

/// 执行已编译的 Rhai AST，args 通过 scope 传入。
pub fn execute_rhai_ast(ast: &AST, args: serde_json::Value) -> Result<serde_json::Value, String> {
    let engine = create_rhai_engine();
    let mut scope = Scope::new();

    if let Some(obj) = args.as_object() {
        for (key, val) in obj {
            set_scope_value(&mut scope, key, val);
        }
    } else {
        scope.push("input", args);
    }

    let result: rhai::Dynamic = engine
        .eval_ast_with_scope(&mut scope, ast)
        .map_err(|e| format!("Rhai 执行失败: {e}"))?;

    Ok(dynamic_to_json(result))
}

fn set_scope_value(scope: &mut Scope, key: &str, value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => {
            scope.push(key, ());
        },
        serde_json::Value::Bool(b) => {
            scope.push(key, *b);
        },
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                scope.push(key, i);
            } else if let Some(f) = n.as_f64() {
                scope.push(key, f);
            } else {
                scope.push(key, n.to_string());
            }
        },
        serde_json::Value::String(s) => {
            scope.push(key, s.clone());
        },
        serde_json::Value::Array(arr) => {
            let list: Vec<rhai::Dynamic> = arr.iter().map(json_to_dynamic).collect();
            scope.push(key, list);
        },
        serde_json::Value::Object(obj) => {
            let mut map = rhai::Map::new();
            for (k, v) in obj {
                map.insert(k.as_str().into(), json_to_dynamic(v));
            }
            scope.push(key, map);
        },
    };
}

fn json_to_dynamic(value: &serde_json::Value) -> rhai::Dynamic {
    match value {
        serde_json::Value::Null => rhai::Dynamic::UNIT,
        serde_json::Value::Bool(b) => rhai::Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rhai::Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                rhai::Dynamic::from(f)
            } else {
                rhai::Dynamic::from(n.to_string())
            }
        },
        serde_json::Value::String(s) => rhai::Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let list: Vec<rhai::Dynamic> = arr.iter().map(json_to_dynamic).collect();
            rhai::Dynamic::from(list)
        },
        serde_json::Value::Object(obj) => {
            let mut map = rhai::Map::new();
            for (k, v) in obj {
                map.insert(k.as_str().into(), json_to_dynamic(v));
            }
            rhai::Dynamic::from(map)
        },
    }
}

fn dynamic_to_json(d: rhai::Dynamic) -> serde_json::Value {
    if d.is::<()>() || d.is_unit() {
        serde_json::Value::Null
    } else if d.is::<bool>() {
        serde_json::Value::Bool(d.as_bool().unwrap_or(false))
    } else if d.is::<i64>() {
        serde_json::Value::Number(d.as_int().unwrap_or(0).into())
    } else if d.is::<f64>() {
        let f = d.as_float().unwrap_or(0.0);
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::String(f.to_string()))
    } else if d.is::<String>() {
        serde_json::Value::String(d.into_string().unwrap_or_default())
    } else if d.is::<Vec<rhai::Dynamic>>() {
        let list: Vec<serde_json::Value> = d
            .into_typed_array::<rhai::Dynamic>()
            .unwrap_or_default()
            .into_iter()
            .map(dynamic_to_json)
            .collect();
        serde_json::Value::Array(list)
    } else {
        serde_json::Value::String(d.to_string())
    }
}
