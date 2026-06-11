// SPDX-License-Identifier: AGPL-3.0-only

//! JSON Schema 校验工具 —— 跨 crate 共享的基础校验能力
//!
//! 从 axagent-agent::self_verifier 提取至 core crate，供 trajectory、
//! rt-workflow、agent 等多个 crate 共用。

/// 对 JSON 值执行 JSON Schema 校验。
///
/// 返回 (全部通过, 错误消息列表)。
/// 支持的 Schema 关键字：type, required, properties, items, minLength, maxLength, enum。
pub fn validate_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
) -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let valid = validate_recursive(value, schema, "", &mut errors);
    (valid, errors)
}

/// 递归 Schema 校验（带路径追踪，供 self_verifier 等模块直接调用）
pub fn validate_recursive(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    path: &str,
    errors: &mut Vec<String>,
) -> bool {
    let mut valid = true;

    // type 关键字
    if let Some(schema_type) = schema.get("type").and_then(|t| t.as_str()) {
        let type_match = match schema_type {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.is_i64() || value.is_u64(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => true,
        };
        if !type_match {
            let loc = if path.is_empty() { "root" } else { path };
            errors.push(format!("类型不匹配于 '{}': 期望 '{}'", loc, schema_type));
            valid = false;
        }
    }

    // required 关键字
    if let Some(required) = schema.get("required").and_then(|r| r.as_array())
        && let Some(obj) = value.as_object()
    {
        for field in required {
            if let Some(field_name) = field.as_str()
                && !obj.contains_key(field_name)
            {
                let loc = if path.is_empty() { "root" } else { path };
                errors.push(format!("缺少必填字段 '{}' 于 '{}'", field_name, loc));
                valid = false;
            }
        }
    }

    // properties 关键字 —— 递归校验子属性
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object())
        && let Some(obj) = value.as_object()
    {
        for (key, prop_schema) in properties {
            if let Some(child_value) = obj.get(key) {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                if !validate_recursive(child_value, prop_schema, &child_path, errors) {
                    valid = false;
                }
            }
        }
    }

    // items 关键字 —— 递归校验数组元素
    if let Some(items_schema) = schema.get("items")
        && let Some(arr) = value.as_array()
    {
        for (i, item) in arr.iter().enumerate() {
            let item_path = format!("{}[{}]", if path.is_empty() { "root" } else { path }, i);
            if !validate_recursive(item, items_schema, &item_path, errors) {
                valid = false;
            }
        }
    }

    // minLength 关键字
    if let Some(min) = schema.get("minLength").and_then(|v| v.as_u64())
        && let Some(s) = value.as_str()
        && (s.len() as u64) < min
    {
        let loc = if path.is_empty() { "root" } else { path };
        errors.push(format!("字符串 '{}' 过短（最小长度: {}）", loc, min));
        valid = false;
    }

    // maxLength 关键字
    if let Some(max) = schema.get("maxLength").and_then(|v| v.as_u64())
        && let Some(s) = value.as_str()
        && (s.len() as u64) > max
    {
        let loc = if path.is_empty() { "root" } else { path };
        errors.push(format!("字符串 '{}' 过长（最大长度: {}）", loc, max));
        valid = false;
    }

    // enum 关键字
    if let Some(enum_values) = schema.get("enum").and_then(|v| v.as_array())
        && !enum_values.iter().any(|e| e == value)
    {
        let loc = if path.is_empty() { "root" } else { path };
        errors.push(format!("值 '{}' 不在允许的枚举值范围内", loc));
        valid = false;
    }

    valid
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_type_mismatch() {
        let value = json!("hello");
        let schema = json!({"type": "number"});
        let (valid, errors) = validate_against_schema(&value, &schema);
        assert!(!valid);
        assert!(errors.iter().any(|e| e.contains("类型不匹配")));
    }

    #[test]
    fn test_type_match() {
        let value = json!("hello");
        let schema = json!({"type": "string"});
        let (valid, _) = validate_against_schema(&value, &schema);
        assert!(valid);
    }

    #[test]
    fn test_required_present() {
        let value = json!({"name": "test", "age": 30});
        let schema = json!({"type": "object", "required": ["name"]});
        let (valid, _) = validate_against_schema(&value, &schema);
        assert!(valid);
    }

    #[test]
    fn test_required_missing() {
        let value = json!({"age": 30});
        let schema = json!({"type": "object", "required": ["name"]});
        let (valid, errors) = validate_against_schema(&value, &schema);
        assert!(!valid);
        assert!(errors.iter().any(|e| e.contains("缺少必填字段")));
    }

    #[test]
    fn test_properties_recursive() {
        let value = json!({"user": {"name": "Alice"}});
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "required": ["name"],
                    "properties": {"name": {"type": "string"}}
                }
            }
        });
        let (valid, _) = validate_against_schema(&value, &schema);
        assert!(valid);
    }

    #[test]
    fn test_nested_required_missing() {
        let value = json!({"user": {"age": 30}});
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "required": ["name"]
                }
            }
        });
        let (valid, errors) = validate_against_schema(&value, &schema);
        assert!(!valid);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("缺少必填字段") && e.contains("name"))
        );
    }

    #[test]
    fn test_min_length() {
        let value = json!("ab");
        let schema = json!({"type": "string", "minLength": 3});
        let (valid, errors) = validate_against_schema(&value, &schema);
        assert!(!valid);
        assert!(errors.iter().any(|e| e.contains("过短")));
    }

    #[test]
    fn test_max_length() {
        let value = json!("abcdef");
        let schema = json!({"type": "string", "maxLength": 3});
        let (valid, errors) = validate_against_schema(&value, &schema);
        assert!(!valid);
        assert!(errors.iter().any(|e| e.contains("过长")));
    }

    #[test]
    fn test_enum_valid() {
        let value = json!("apple");
        let schema = json!({"enum": ["apple", "banana", "cherry"]});
        let (valid, _) = validate_against_schema(&value, &schema);
        assert!(valid);
    }

    #[test]
    fn test_enum_invalid() {
        let value = json!("grape");
        let schema = json!({"enum": ["apple", "banana", "cherry"]});
        let (valid, errors) = validate_against_schema(&value, &schema);
        assert!(!valid);
        assert!(errors.iter().any(|e| e.contains("枚举值")));
    }

    #[test]
    fn test_items_schema() {
        let value = json!([1, 2, 3]);
        let schema = json!({"type": "array", "items": {"type": "integer"}});
        let (valid, _) = validate_against_schema(&value, &schema);
        assert!(valid);
    }

    #[test]
    fn test_items_schema_fail() {
        let value = json!([1, "two", 3]);
        let schema = json!({"type": "array", "items": {"type": "integer"}});
        let (valid, errors) = validate_against_schema(&value, &schema);
        assert!(!valid);
        assert!(errors.iter().any(|e| e.contains("类型不匹配")));
    }

    #[test]
    fn test_empty_value_empty_schema() {
        let value = json!({});
        let schema = json!({});
        let (valid, _) = validate_against_schema(&value, &schema);
        assert!(valid);
    }
}
