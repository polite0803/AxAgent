// SPDX-License-Identifier: AGPL-3.0-only

//! JSON Schema 校验 —— 跨 crate 共享的权威实现
//!
//! 合并了 `kit::schema_validator` 和 `harness::serialization` 的功能：
//! - type / required / properties / items / enum / minLength / maxLength / minimum / maximum
//! - additionalProperties / boolean schemas (true=任何, false=禁止)
//!
//! `kit::schema_validator` 现为薄封装重导出层。

use serde_json::Value;

/// JSON Schema 校验入口。
///
/// 返回 `(全部通过, 错误消息列表)`。
/// 与旧 `kit::validate_against_schema` 签名兼容。
pub fn validate_against_schema(value: &Value, schema: &Value) -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let valid = validate_recursive(value, schema, "", &mut errors);
    (valid, errors)
}

/// Result 风格的快捷入口 —— 与旧 `harness::validate_output_against_schema` 签名兼容。
pub fn validate_ok(value: &Value, schema: &Value) -> Result<(), Vec<String>> {
    let (valid, errors) = validate_against_schema(value, schema);
    if valid { Ok(()) } else { Err(errors) }
}

/// 递归 Schema 校验（支持路径追踪）
pub fn validate_recursive(
    value: &Value,
    schema: &Value,
    path: &str,
    errors: &mut Vec<String>,
) -> bool {
    let mut valid = true;

    // boolean schema: true = 任何值, false = 拒绝一切
    if let Some(b) = schema.as_bool() {
        if !b {
            errors.push(format!("{path}: 不允许任何值"));
            return false;
        }
        return true;
    }

    // type 关键字
    if let Some(schema_type) = schema.get("type").and_then(|t| t.as_str()) {
        let type_match = match schema_type {
            "object" => {
                if !value.is_object() {
                    errors.push(format!("{path}: 期望 object，实际 {}", type_name(value)));
                    false
                } else {
                    true
                }
            },
            "array" => {
                if !value.is_array() {
                    errors.push(format!("{path}: 期望 array，实际 {}", type_name(value)));
                    false
                } else {
                    true
                }
            },
            "string" => {
                if !value.is_string() {
                    errors.push(format!("{path}: 期望 string，实际 {}", type_name(value)));
                    false
                } else {
                    true
                }
            },
            "number" | "integer" => {
                if !value.is_number() {
                    errors.push(format!("{path}: 期望 number，实际 {}", type_name(value)));
                    false
                } else if schema_type == "integer"
                    && !value.as_f64().is_some_and(|f| f.fract() == 0.0)
                {
                    errors.push(format!("{path}: 期望整数"));
                    false
                } else {
                    true
                }
            },
            "boolean" => {
                if !matches!(value, Value::Bool(_)) {
                    errors.push(format!("{path}: 期望 boolean，实际 {}", type_name(value)));
                    false
                } else {
                    true
                }
            },
            "null" => {
                if !value.is_null() {
                    errors.push(format!("{path}: 期望 null，实际 {}", type_name(value)));
                    false
                } else {
                    true
                }
            },
            _ => true,
        };
        if !type_match {
            valid = false;
            return valid;
        }
    }

    // 对象专属：properties / required / additionalProperties
    if let Some(obj) = value.as_object() {
        if let Some(required_fields) = schema.get("required").and_then(|r| r.as_array()) {
            for field in required_fields {
                let key = field.as_str().unwrap_or("");
                if !obj.contains_key(key) || obj.get(key) == Some(&Value::Null) {
                    errors.push(format!("{path}.{key}: 缺少必填字段"));
                    valid = false;
                }
            }
        }

        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            for (key, prop_schema) in properties {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                if let Some(child_val) = obj.get(key) {
                    if !child_val.is_null() {
                        if !validate_recursive(child_val, prop_schema, &child_path, errors) {
                            valid = false;
                        }
                    }
                }
            }
        }

        // additionalProperties
        if let Some(additional) = schema.get("additionalProperties") {
            if additional.as_bool() == Some(false) {
                if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
                    for key in obj.keys() {
                        if !properties.contains_key(key) {
                            errors.push(format!("{path}.{key}: 未定义的字段"));
                            valid = false;
                        }
                    }
                }
            }
        }
    }

    // 数组专属：items
    if let Some(arr) = value.as_array() {
        if let Some(items_schema) = schema.get("items") {
            for (i, item) in arr.iter().enumerate() {
                let child_path = format!("{path}[{i}]");
                if !validate_recursive(item, items_schema, &child_path, errors) {
                    valid = false;
                }
            }
        }
    }

    // 通用约束：enum / minLength / maxLength / minimum / maximum
    if let Some(enum_vals) = schema.get("enum").and_then(|e| e.as_array()) {
        if !enum_vals.contains(value) {
            errors.push(format!("{path}: 值不在允许范围内: {:?}", enum_vals));
            valid = false;
        }
    }

    if let Some(min_len) = schema.get("minLength").and_then(|m| m.as_u64()) {
        if let Some(s) = value.as_str() {
            if (s.len() as u64) < min_len {
                errors.push(format!("{path}: 长度不能少于 {min_len}"));
                valid = false;
            }
        }
    }

    if let Some(max_len) = schema.get("maxLength").and_then(|m| m.as_u64()) {
        if let Some(s) = value.as_str() {
            if (s.len() as u64) > max_len {
                errors.push(format!("{path}: 长度不能超过 {max_len}"));
                valid = false;
            }
        }
    }

    if let Some(min) = schema.get("minimum").and_then(|m| m.as_f64()) {
        if let Some(n) = value.as_f64() {
            if n < min {
                errors.push(format!("{path}: 不能小于 {min}"));
                valid = false;
            }
        }
    }

    if let Some(max) = schema.get("maximum").and_then(|m| m.as_f64()) {
        if let Some(n) = value.as_f64() {
            if n > max {
                errors.push(format!("{path}: 不能大于 {max}"));
                valid = false;
            }
        }
    }

    valid
}

fn type_name(val: &Value) -> &'static str {
    match val {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            }
        });
        let output = json!({ "name": "Alice", "age": 30 });
        let (ok, _) = validate_against_schema(&output, &schema);
        assert!(ok);
    }

    #[test]
    fn test_missing_required_field() {
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": { "type": "string" }
            }
        });
        let output = json!({});
        let (ok, errors) = validate_against_schema(&output, &schema);
        assert!(!ok);
        assert!(errors.iter().any(|e| e.contains("缺少必填字段")));
    }

    #[test]
    fn test_type_mismatch() {
        let schema = json!({ "type": "integer" });
        let (ok, errors) = validate_against_schema(&json!("not a number"), &schema);
        assert!(!ok);
        assert!(errors.iter().any(|e| e.contains("期望 number")));
    }

    #[test]
    fn test_enum_validation() {
        let schema = json!({
            "type": "string",
            "enum": ["red", "green", "blue"]
        });
        let (ok, _) = validate_against_schema(&json!("red"), &schema);
        assert!(ok);
        let (ok, errors) = validate_against_schema(&json!("yellow"), &schema);
        assert!(!ok);
        assert!(errors.iter().any(|e| e.contains("允许范围")));
    }

    #[test]
    fn test_additional_properties_rejected() {
        let schema = json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "additionalProperties": false
        });
        let (ok, errors) =
            validate_against_schema(&json!({ "name": "Alice", "extra": true }), &schema);
        assert!(!ok);
        assert!(errors.iter().any(|e| e.contains("未定义")));
    }

    #[test]
    fn test_boolean_schema_true() {
        let (ok, _) = validate_against_schema(&json!("anything"), &json!(true));
        assert!(ok);
    }

    #[test]
    fn test_boolean_schema_false() {
        let (ok, _) = validate_against_schema(&json!("anything"), &json!(false));
        assert!(!ok);
    }

    #[test]
    fn test_array_items_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "tags": { "type": "array", "items": { "type": "string" } }
            }
        });
        let (ok, _) = validate_against_schema(&json!({ "tags": ["a", "b"] }), &schema);
        assert!(ok);

        let (ok, errors) = validate_against_schema(&json!({ "tags": [1, 2] }), &schema);
        assert!(!ok);
        assert!(errors.iter().any(|e| e.contains("期望 string")));
    }

    #[test]
    fn test_min_max_length() {
        let schema = json!({ "type": "string", "minLength": 3, "maxLength": 5 });
        let (ok, _) = validate_against_schema(&json!("abc"), &schema);
        assert!(ok);
        let (ok, errors) = validate_against_schema(&json!("ab"), &schema);
        assert!(!ok);
        assert!(errors.iter().any(|e| e.contains("不能少于")));
        let (ok, errors) = validate_against_schema(&json!("abcdef"), &schema);
        assert!(!ok);
        assert!(errors.iter().any(|e| e.contains("不能超过")));
    }

    #[test]
    fn test_validate_ok_variant() {
        let schema = json!({ "type": "string" });
        assert!(validate_ok(&json!("hello"), &schema).is_ok());
        assert!(validate_ok(&json!(42), &schema).is_err());
    }
}
