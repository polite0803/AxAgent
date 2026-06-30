// SPDX-License-Identifier: AGPL-3.0-only

//! Rhai 脚本引擎 —— 编译、缓存、执行 Rhai 脚本作为动态工具。
//!
//! 编译（工作流创建时）→ 缓存 AST → 执行时注册为 tool_handler
//! 脚本中可通过 `tool("name", args_map)` 调用已注册的工具

use rhai::{AST, Engine, Scope};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::{LazyLock, Mutex};

static SHARED_RHAI_RUNTIME: LazyLock<std::sync::Arc<tokio::runtime::Runtime>> =
    LazyLock::new(|| {
        std::sync::Arc::new(tokio::runtime::Runtime::new().expect("failed to create Rhai runtime"))
    });

pub type RhaiScriptCache = HashMap<String, Arc<AST>>;

/// 创建编译期用的 Rhai 引擎（不含工具，仅语法检查）
/// 安全配置：默认沙箱（无文件系统/网络 API）、操作数限制、调用深度限制、
/// 字符串/数组大小限制，防止 DoS 攻击。
pub fn create_rhai_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(100_000);
    engine.set_max_call_levels(16);
    engine.set_max_modules(0);
    // DoS 防护：限制字符串和数组大小
    engine.set_max_string_size(2_000_000);  // 2MB
    engine.set_max_array_size(50_000);       // 5 万元素上限
    engine
}

/// 编译一段 Rhai 脚本
pub fn compile_script(engine: &Engine, script: &str) -> Result<AST, String> {
    engine
        .compile(script)
        .map_err(|e| format!("Rhai 编译失败: {e}"))
}

/// 从模板 tool_defs 批量编译 Rhai 工具（非 DAG 节点方式）
pub fn compile_from_tool_defs(
    tool_defs: &[axagent_core::workflow_types::RhaiToolDef],
) -> RhaiScriptCache {
    let engine = create_rhai_engine();
    let mut cache = HashMap::new();
    for td in tool_defs {
        if td.code.is_empty() {
            continue;
        }
        match compile_script(&engine, &td.code) {
            Ok(ast) => {
                cache.insert(td.tool_name.clone(), Arc::new(ast));
            },
            Err(e) => tracing::warn!("[RhaiEngine] 编译失败 {}: {e}", td.tool_name),
        }
    }
    cache
}

/// 执行 Rhai AST，支持通过 `tool("name", args)` 调用注册的工具
pub type ToolFn = Arc<
    dyn Fn(
            String,
            serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>,
        > + Send
        + Sync,
>;

pub fn execute_rhai_ast(
    ast: &AST,
    args: serde_json::Value,
    tools: Option<&HashMap<String, ToolFn>>,
) -> Result<serde_json::Value, String> {
    let mut engine = create_rhai_engine();
    let mut scope = Scope::new();

    // 注入 tool() 函数 —— 共享一个独立 Runtime，避免每次调用都创建线程池
    let rt = if tools.is_some() {
        Some(SHARED_RHAI_RUNTIME.clone())
    } else {
        None
    };

    if let (Some(tool_map), Some(rt)) = (tools, rt.clone()) {
        let tool_map = tool_map.clone();
        engine.register_fn("tool", move |name: &str, args: rhai::Map| {
            let tool_map = tool_map.clone();
            let tool_name = name.to_string();
            let json_args = rhai_map_to_json(args);
            let rt = rt.clone(); // Arc 复用，零开销
            let result = rt.block_on(async {
                if let Some(h) = tool_map.get(&tool_name) {
                    h(tool_name, json_args).await
                } else {
                    Err(format!("工具 '{tool_name}' 未注册"))
                }
            });
            match result {
                Ok(v) => json_to_dynamic(&v),
                Err(e) => rhai::Dynamic::from(format!("Error: {e}")),
            }
        });
    }

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

fn rhai_map_to_json(map: rhai::Map) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (k, v) in map {
        obj.insert(k.to_string(), dynamic_to_json(v));
    }
    serde_json::Value::Object(obj)
}

// ── JSON ↔ Rhai Dynamic 互转 ──

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
            rhai::Dynamic::from(arr.iter().map(json_to_dynamic).collect::<Vec<_>>())
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
        serde_json::Value::Array(
            d.into_typed_array::<rhai::Dynamic>()
                .unwrap_or_default()
                .into_iter()
                .map(dynamic_to_json)
                .collect(),
        )
    } else {
        serde_json::Value::String(d.to_string())
    }
}

// ── Harness RhaiEngineAdapter trait 实现 ──

use axagent_harness::RhaiEngineAdapter as HarnessRhaiEngineAdapter;
use axagent_harness::RhaiToolFn;

/// 状态化的 Rhai 引擎适配器，内部管理脚本缓存。
#[derive(Debug)]
pub struct RhaiEngine {
    cache: Mutex<RhaiScriptCache>,
}

impl RhaiEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for RhaiEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessRhaiEngineAdapter for RhaiEngine {
    fn register_scripts(&self, scripts: &[JsonValue]) {
        let engine = create_rhai_engine();
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        for script in scripts {
            let tool_name = script
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let code = script.get("code").and_then(|v| v.as_str()).unwrap_or("");
            if code.is_empty() {
                continue;
            }
            match compile_script(&engine, code) {
                Ok(ast) => {
                    cache.insert(tool_name.to_string(), Arc::new(ast));
                },
                Err(e) => {
                    tracing::warn!("[RhaiEngineAdapter] 编译失败 {tool_name}: {e}");
                },
            }
        }
    }

    fn execute_script(
        &self,
        script_name: &str,
        args: JsonValue,
        tool_fns: &HashMap<String, RhaiToolFn>,
    ) -> Result<JsonValue, String> {
        let ast = {
            let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            cache
                .get(script_name)
                .cloned()
                .ok_or_else(|| format!("Rhai 脚本 '{script_name}' 未找到"))?
        };

        // 将 tool_fns 转换为本地 ToolFn 格式
        let rhai_tool_fns: Option<HashMap<String, ToolFn>> = if tool_fns.is_empty() {
            None
        } else {
            let mut map: HashMap<String, ToolFn> = HashMap::new();
            for (name, handler) in tool_fns {
                let handler = handler.clone();
                let name = name.clone();
                map.insert(
                    name.clone(),
                    Arc::new(move |tool_name: String, args: JsonValue| {
                        let handler = handler.clone();
                        let n = tool_name.clone();
                        // RhaiToolFn 是同步的，用 futures::future::ready 包装为 async
                        Box::pin(futures::future::ready(handler(n, args)))
                    }),
                );
            }
            Some(map)
        };

        execute_rhai_ast(&ast, args, rhai_tool_fns.as_ref())
    }
}
