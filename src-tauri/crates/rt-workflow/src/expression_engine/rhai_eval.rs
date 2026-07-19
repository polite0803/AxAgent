// SPDX-License-Identifier: AGPL-3.0-only

use super::ExpressionContext;
use chrono::{Datelike, Timelike, Utc};
use rhai::{Dynamic, Engine, Map, Scope};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum RhaiEvalError {
    #[error("表达式编译失败: {0}")]
    CompileError(String),
    #[error("表达式执行失败: {0}")]
    RuntimeError(String),
    #[error("不支持的运算")]
    UnsupportedOperation,
    #[error("变量未找到: {0}")]
    VariableNotFound(String),
}

/// 将 serde_json::Value 转换为 Rhai Dynamic
fn value_to_dynamic(value: &Value) -> Dynamic {
    match value {
        Value::Null => Dynamic::UNIT,
        Value::Bool(b) => Dynamic::from_bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from_int(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from_float(f)
            } else {
                Dynamic::from(n.to_string())
            }
        },
        Value::String(s) => Dynamic::from(s.clone()),
        Value::Array(arr) => {
            let items: Vec<Dynamic> = arr.iter().map(value_to_dynamic).collect();
            Dynamic::from_array(items)
        },
        Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), value_to_dynamic(v));
            }
            Dynamic::from_map(map)
        },
    }
}

/// 将 Rhai Dynamic 转换回 serde_json::Value
fn dynamic_to_value(d: Dynamic) -> Value {
    if d.is::<i64>() {
        let n: i64 = d.try_cast().unwrap_or(0);
        Value::Number(n.into())
    } else if d.is::<f64>() {
        let f: f64 = d.try_cast().unwrap_or(0.0);
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Value::Number(n);
        }
        Value::Null
    } else if d.is::<bool>() {
        let b: bool = d.try_cast().unwrap_or(false);
        Value::Bool(b)
    } else if d.is::<String>() {
        let s: String = d.try_cast().unwrap_or_default();
        Value::String(s)
    } else if d.is::<Vec<Dynamic>>() {
        let arr: Vec<Value> = d
            .try_cast::<Vec<Dynamic>>()
            .unwrap_or_default()
            .into_iter()
            .map(dynamic_to_value)
            .collect();
        Value::Array(arr)
    } else if d.is::<Map>() {
        let map: Map = d.try_cast::<Map>().unwrap_or_default();
        let obj: serde_json::Map<String, Value> =
            map.into_iter().map(|(k, v)| (k.to_string(), dynamic_to_value(v))).collect();
        Value::Object(obj)
    } else {
        Value::Null
    }
}

/// 解析单个表达式（非模板，纯表达式）
/// 例如: "$vars.price * $vars.qty" → 数字
///       "$node[\"http1\"].status" → 字符串
pub fn resolve_expression(expr: &str, ctx: &ExpressionContext) -> Result<Value, RhaiEvalError> {
    let engine = Engine::new();
    let mut scope = Scope::new();

    // 注入 $vars：全局变量
    let vars_map =
        ctx.variables.iter().map(|(k, v)| (k.clone().into(), value_to_dynamic(v))).collect::<Map>();
    scope.push_dynamic("$vars", Dynamic::from_map(vars_map));

    // 注入 $node：节点输出（按名称索引）
    let node_map = ctx
        .node_outputs
        .iter()
        .map(|(k, v)| (k.clone().into(), value_to_dynamic(v)))
        .collect::<Map>();
    scope.push_dynamic("$node", Dynamic::from_map(node_map));

    // 注入 $input：当前节点输入
    scope.push_dynamic("$input", value_to_dynamic(&ctx.input_params));

    // 注入 $now：当前时间
    let now = Utc::now();
    let mut now_map = Map::new();
    now_map.insert("timestamp".into(), Dynamic::from_int(now.timestamp()));
    now_map.insert("iso".into(), Dynamic::from(now.to_rfc3339()));
    now_map.insert("year".into(), Dynamic::from_int(now.year() as i64));
    now_map.insert("month".into(), Dynamic::from_int(now.month() as i64));
    now_map.insert("day".into(), Dynamic::from_int(now.day() as i64));
    now_map.insert("hour".into(), Dynamic::from_int(now.hour() as i64));
    now_map.insert("minute".into(), Dynamic::from_int(now.minute() as i64));
    scope.push_dynamic("$now", Dynamic::from_map(now_map));

    // 注入 $env
    let env_map =
        ctx.env.iter().map(|(k, v)| (k.clone().into(), Dynamic::from(v.clone()))).collect::<Map>();
    scope.push_dynamic("$env", Dynamic::from_map(env_map));

    // 编译并执行
    let ast =
        engine.compile_expression(expr).map_err(|e| RhaiEvalError::CompileError(e.to_string()))?;

    let result = engine
        .eval_ast_with_scope::<Dynamic>(&mut scope, &ast)
        .map_err(|e| RhaiEvalError::RuntimeError(e.to_string()))?;

    Ok(dynamic_to_value(result))
}

/// 4.2 P3:while/until 条件表达式求值(支持 Rhai 任意表达式)
///
/// 与 `resolve_expression` 不同,本函数额外注入 Loop 上下文变量:
/// - `iter_index`: 当前迭代序号(i64)
/// - `partial`: 已完成的迭代输出数组(可通过 `partial.len()` / `partial[0]` 访问)
///
/// 变量命名约定(无 `$` 前缀;Rhai 1.x 中 `$` 是保留符号):
/// - `vars`        — 全局变量(ExecutionState.variables)
/// - `node`        — 节点输出(ExecutionState.node_outputs)
/// - `input`       — 当前节点输入参数
/// - `env`         — 环境变量
/// - `now`         — 当前时间(map: timestamp/iso/year/month/day/hour/minute)
/// - `iter_index`  — 当前迭代序号
/// - `partial`     — 已完成迭代输出数组
///
/// 表达式返回值按以下规则转换为 bool:
/// - `bool` → 原值
/// - `i64` / `f64` → 非 0 即 true
/// - `String` → 非空即 true
/// - 其他 / 错误 → false(避免错误条件触发死循环)
///
/// 示例:
/// - `iter_index < 10`
/// - `vars.threshold > 0 && partial.len() < vars.threshold`
/// - `node["check_result"].ok`
pub fn resolve_loop_condition(
    cond: &str,
    ctx: &ExpressionContext,
    iter_index: u32,
    partial: &[Value],
) -> Result<bool, RhaiEvalError> {
    let engine = Engine::new();
    let mut scope = Scope::new();

    // 复用 resolve_expression 的注入逻辑(但用不带 `$` 的标识符)
    let vars_map =
        ctx.variables.iter().map(|(k, v)| (k.clone().into(), value_to_dynamic(v))).collect::<Map>();
    scope.push_dynamic("vars", Dynamic::from_map(vars_map));

    let node_map = ctx
        .node_outputs
        .iter()
        .map(|(k, v)| (k.clone().into(), value_to_dynamic(v)))
        .collect::<Map>();
    scope.push_dynamic("node", Dynamic::from_map(node_map));

    scope.push_dynamic("input", value_to_dynamic(&ctx.input_params));

    let now = Utc::now();
    let mut now_map = Map::new();
    now_map.insert("timestamp".into(), Dynamic::from_int(now.timestamp()));
    now_map.insert("iso".into(), Dynamic::from(now.to_rfc3339()));
    now_map.insert("year".into(), Dynamic::from_int(now.year() as i64));
    now_map.insert("month".into(), Dynamic::from_int(now.month() as i64));
    now_map.insert("day".into(), Dynamic::from_int(now.day() as i64));
    now_map.insert("hour".into(), Dynamic::from_int(now.hour() as i64));
    now_map.insert("minute".into(), Dynamic::from_int(now.minute() as i64));
    scope.push_dynamic("now", Dynamic::from_map(now_map));

    let env_map =
        ctx.env.iter().map(|(k, v)| (k.clone().into(), Dynamic::from(v.clone()))).collect::<Map>();
    scope.push_dynamic("env", Dynamic::from_map(env_map));

    // Loop 专属变量
    scope.push("iter_index", Dynamic::from_int(iter_index as i64));
    let partial_arr: Vec<Dynamic> = partial.iter().map(value_to_dynamic).collect();
    scope.push_dynamic("partial", Dynamic::from_array(partial_arr));

    let ast =
        engine.compile_expression(cond).map_err(|e| RhaiEvalError::CompileError(e.to_string()))?;

    let result = engine
        .eval_ast_with_scope::<Dynamic>(&mut scope, &ast)
        .map_err(|e| RhaiEvalError::RuntimeError(e.to_string()))?;

    Ok(dynamic_to_bool(result))
}

/// 将 Rhai Dynamic 转换为 bool(条件表达式专用)
///
/// 规则:`bool` 原值;数字非零为 true;字符串非空为 true;其他类型 false。
fn dynamic_to_bool(d: Dynamic) -> bool {
    if d.is::<bool>() {
        d.try_cast().unwrap_or(false)
    } else if d.is::<i64>() {
        d.try_cast::<i64>().unwrap_or(0) != 0
    } else if d.is::<f64>() {
        d.try_cast::<f64>().unwrap_or(0.0) != 0.0
    } else if d.is::<String>() {
        !d.try_cast::<String>().unwrap_or_default().is_empty()
    } else {
        false
    }
}
