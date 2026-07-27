// SPDX-License-Identifier: AGPL-3.0-only

//! Rhai 脚本引擎适配器契约 + 通用函数注册。
//!
//! 提供 Rhai 脚本的编译和执行能力，用于工作流中的动态脚本节点。
//! 同时提供 `register_common_functions` 等纯函数，供所有执行 Rhai 脚本的 Engine 实例复用，
//! 避免分散注册导致遗漏（历史上 rt-workflow / quant / market-sim 各自维护一份导致 bug）。

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use rhai::Engine;
use serde_json::Value as JsonValue;

/// Rhai 脚本引擎适配器契约
///
/// 封装 Rhai 脚本的批量编译和按名执行能力。
/// 实现方（`axagent-tools::rhai_engine`）管理内部脚本缓存。
pub trait RhaiEngineAdapter: fmt::Debug + Send + Sync {
    /// 批量注册并编译脚本（在工作流初始化时调用）
    ///
    /// `scripts`：脚本定义数组，每个元素为 `{ "tool_name": "...", "code": "..." }`
    fn register_scripts(&self, scripts: &[JsonValue]);

    /// 执行已注册的指定脚本
    ///
    /// - `script_name`：要执行的脚本名称（与注册时的 `tool_name` 对应）
    /// - `args`：输入参数
    /// - `tool_fns`：可被脚本调用的工具函数映射，key=工具名，value= `(name, args) -> Result`
    fn execute_script(
        &self,
        script_name: &str,
        args: JsonValue,
        tool_fns: &HashMap<String, RhaiToolFn>,
    ) -> Result<JsonValue, String>;
}

/// Rhai 可调用工具函数
///
/// 签名：`(工具名, JSON参数) -> Result<JSON结果, 错误信息>`
pub type RhaiToolFn = Arc<dyn Fn(String, JsonValue) -> Result<JsonValue, String> + Send + Sync>;

// ────────────────────────────────────────────────────────────────────────────
// 通用 Rhai 函数注册（下沉自 rt-workflow::code_executor，消除 quant 同义重复定义）
// ────────────────────────────────────────────────────────────────────────────

/// 注册通用 Rhai 函数（clamp / join / json_parse）。
///
/// 所有执行 Rhai 脚本的 Engine 实例都应调用此函数，确保脚本可用的
/// 自定义函数集一致，避免分散注册导致遗漏。
///
/// 历史背景：原 `register_common_functions` 定义在 `rt-workflow::code_executor`，
/// 但 `quant` / `market-sim` 等 consumer crate 按依赖铁律不能依赖 rt-workflow（hybrid 层），
/// 导致各自维护一份 `json_value_to_rhai` 同义实现（违反铁律 4「禁止重复定义」）。
/// 下沉到 harness（foundation 层）后，所有 crate 通过 `pub use` 复用同一份实现。
///
/// 参考：portfolio-mgr.rhai / consistency-check.rhai / bottleneck-calc.rhai 等脚本
/// 均依赖 `json_parse`；`clamp` 用于信号夹紧；`join` 用于数组拼接。
pub fn register_common_functions(engine: &mut Engine) {
    engine.register_fn("clamp", |value: f64, min: f64, max: f64| -> f64 {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    });
    engine.register_fn("join", |arr: rhai::Array, sep: &str| -> String {
        arr.iter().map(|item| item.to_string()).collect::<Vec<_>>().join(sep)
    });
    engine.register_fn("json_parse", |s: &str| -> rhai::Dynamic {
        match serde_json::from_str::<serde_json::Value>(s) {
            Ok(v) => json_value_to_dynamic(&v),
            Err(e) => {
                tracing::warn!("[harness::rhai_engine] json_parse 失败: {e}");
                rhai::Dynamic::UNIT
            },
        }
    });
}

/// 将 `serde_json::Value` 转换为 Rhai `Dynamic`。
///
/// 整数语义：JSON Number 优先尝试 `as_i64()` 保持整数类型，避免整数精度丢失
/// （原 rt-workflow 版本优先 `as_f64` 会把 JSON 整数 5 静默变成 Rhai float 5.0）。
/// 采纳自 `quant::script::json_value_to_rhai` 的更正确实现。
pub fn json_value_to_dynamic(v: &serde_json::Value) -> rhai::Dynamic {
    match v {
        serde_json::Value::Null => rhai::Dynamic::UNIT,
        serde_json::Value::Bool(b) => rhai::Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rhai::Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                rhai::Dynamic::from(f)
            } else {
                rhai::Dynamic::UNIT
            }
        },
        serde_json::Value::String(s) => rhai::Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let items: rhai::Array = arr.iter().map(json_value_to_dynamic).collect();
            rhai::Dynamic::from_array(items)
        },
        serde_json::Value::Object(obj) => {
            let mut map = rhai::Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_value_to_dynamic(v));
            }
            map.into()
        },
    }
}

/// 将 Rhai `Dynamic` 转换回 `serde_json::Value`。
///
/// 与 [`json_value_to_dynamic`] 互为反函数。注意 Rhai 的整数和浮点数
/// 会分别映射到 JSON Number 的 i64 / f64 表示。
pub fn dynamic_to_json_value(v: &rhai::Dynamic) -> serde_json::Value {
    if v.is_unit() {
        return serde_json::Value::Null;
    }
    if v.is_bool() {
        return serde_json::Value::Bool(v.as_bool().unwrap_or(false));
    }
    if let Ok(s) = v.clone().into_string() {
        return serde_json::Value::String(s);
    }
    if let Ok(i) = v.as_int() {
        return serde_json::Value::Number(serde_json::Number::from(i));
    }
    if let Ok(f) = v.as_float() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return serde_json::Value::Number(n);
        }
        return serde_json::Value::Number(serde_json::Number::from(0));
    }
    // Array
    if let Some(arr) = v.clone().try_cast::<rhai::Array>() {
        return serde_json::Value::Array(
            arr.into_iter().map(|item| dynamic_to_json_value(&item)).collect(),
        );
    }
    // Map
    if let Some(map) = v.clone().try_cast::<rhai::Map>() {
        let mut obj = serde_json::Map::new();
        for (k, val) in &map {
            obj.insert(format!("{k}"), dynamic_to_json_value(val));
        }
        return serde_json::Value::Object(obj);
    }
    serde_json::Value::String(format!("{v}"))
}

// 空实现 — 总是失败（Rhai 引擎未配置）
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::NoopRhaiEngineAdapter;

    #[test]
    fn noop_never_succeeds_on_execute() {
        let adapter = NoopRhaiEngineAdapter;
        adapter.register_scripts(&[]);
        let result = adapter.execute_script("test", JsonValue::Null, &HashMap::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not configured"));
    }

    #[test]
    fn register_common_functions_provides_clamp_join_json_parse() {
        let mut engine = Engine::new();
        register_common_functions(&mut engine);
        // clamp
        let r: f64 = engine.eval_expression("clamp(15.0, 0.0, 10.0)").unwrap();
        assert_eq!(r, 10.0);
        // json_parse: eval_expression 只能解析表达式，不能含 let 语句，
        // 改用 eval 解析整段脚本。json_parse 返回 Map，通过 ["x"] 索引访问。
        let r: i64 = engine.eval::<i64>("let obj = json_parse(`{\"x\": 5}`); obj[\"x\"]").unwrap();
        assert_eq!(r, 5);
        // join
        let r: String = engine.eval_expression::<String>("join([1, 2, 3], \", \")").unwrap();
        assert_eq!(r, "1, 2, 3");
    }

    #[test]
    fn json_value_to_dynamic_preserves_integer() {
        let v = serde_json::json!({"count": 42, "price": 3.15});
        let d = json_value_to_dynamic(&v);
        let map = d.try_cast::<rhai::Map>().unwrap();
        // 整数保持 i64，不被静默转 f64
        let count = map.get("count").unwrap();
        assert_eq!(count.as_int().unwrap(), 42);
        // 浮点数保持 f64
        let price = map.get("price").unwrap();
        assert!((price.as_float().unwrap() - 3.15).abs() < 1e-9);
    }

    #[test]
    fn dynamic_to_json_value_roundtrip() {
        let original = serde_json::json!({
            "name": "test",
            "count": 42,
            "active": true,
            "items": [1, 2, 3],
            "nested": {"key": "value"}
        });
        let dynamic = json_value_to_dynamic(&original);
        let back = dynamic_to_json_value(&dynamic);
        assert_eq!(back, original);
    }
}
