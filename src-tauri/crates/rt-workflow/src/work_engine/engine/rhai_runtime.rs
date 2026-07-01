// SPDX-License-Identifier: AGPL-3.0-only

//! Rhai scripting runtime helpers: JSON conversion, script cache types.

use std::collections::HashMap;
use std::sync::Arc;

use rhai::{AST, EvalAltResult, Position};

/// Convert Rhai dynamic map to JSON value
pub(crate) fn rhai_map_to_json(map: rhai::Map) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (k, v) in map {
        let val: serde_json::Value = if v.is_int() {
            v.as_int()
                .map(|n| serde_json::Value::Number(n.into()))
                .unwrap_or(serde_json::Value::Null)
        } else if v.is_string() {
            v.try_cast::<String>()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null)
        } else if v.is_float() {
            match v.as_float() {
                Ok(f) => serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),
                Err(_) => serde_json::Value::Null,
            }
        } else if v.is_bool() {
            v.as_bool()
                .map(serde_json::Value::Bool)
                .unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        };
        obj.insert(k.to_string(), val);
    }
    serde_json::Value::Object(obj)
}

/// Rhai 脚本缓存类型：workflow_id -> (tool_name -> compiled AST)
pub(crate) type RhaiScriptCache = HashMap<String, Arc<AST>>;

/// Rhai 脚本工具回调类型
pub(crate) type LocalRhaiToolFn = Arc<
    dyn Fn(
            String,
            serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn futures::Future<Output = Result<serde_json::Value, String>> + Send>,
        > + Send
        + Sync,
>;
