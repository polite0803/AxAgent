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
        let obj: serde_json::Map<String, Value> = map
            .into_iter()
            .map(|(k, v)| (k.to_string(), dynamic_to_value(v)))
            .collect();
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
    let vars_map = ctx
        .variables
        .iter()
        .map(|(k, v)| (k.clone().into(), value_to_dynamic(v)))
        .collect::<Map>();
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
    let env_map = ctx
        .env
        .iter()
        .map(|(k, v)| (k.clone().into(), Dynamic::from(v.clone())))
        .collect::<Map>();
    scope.push_dynamic("$env", Dynamic::from_map(env_map));

    // 编译并执行
    let ast = engine
        .compile_expression(expr)
        .map_err(|e| RhaiEvalError::CompileError(e.to_string()))?;

    let result = engine
        .eval_ast_with_scope::<Dynamic>(&mut scope, &ast)
        .map_err(|e| RhaiEvalError::RuntimeError(e.to_string()))?;

    Ok(dynamic_to_value(result))
}
