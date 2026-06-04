//! 节点间数据传递的 Schema 校验工具
//!
//! 在工作流节点之间通过 serde JSON 传递数据时，
//! 提供严格的序列化/反序列化格式强制工具。

use serde_json::Value;

/// 节点输出 Schema 校验
///
/// 校验 `output` 是否匹配 `schema` 定义。
/// 返回 `Ok(())` 或 `Err`（包含所有错误信息列表）。
pub fn validate_output_against_schema(output: &Value, schema: &Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    validate_value("root", output, schema, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_value(path: &str, val: &Value, schema: &Value, errors: &mut Vec<String>) {
    // 如果 schema 是 true/false，对应 any/not-any
    if let Some(b) = schema.as_bool() {
        if !b {
            errors.push(format!("{path}: 不允许任何值"));
        }
        return;
    }

    // type 检查
    if let Some(expected_type) = schema.get("type").and_then(|t| t.as_str()) {
        match expected_type {
            "object" => {
                if !val.is_object() {
                    errors.push(format!("{path}: 期望 object，实际 {}", type_name(val)));
                    return;
                }
                if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
                    for (key, prop_schema) in properties {
                        let child_path = format!("{path}.{key}");
                        if let Some(child_val) = val.get(key) {
                            if !child_val.is_null() {
                                validate_value(&child_path, child_val, prop_schema, errors);
                            }
                        } else if is_required(prop_schema) {
                            errors.push(format!("{child_path}: 缺少必填字段"));
                        }
                    }
                }
                // additionalProperties 检查
                if let Some(additional) = schema.get("additionalProperties") {
                    if additional.as_bool() == Some(false) {
                        if let Some(obj) = val.as_object() {
                            if let Some(properties) =
                                schema.get("properties").and_then(|p| p.as_object())
                            {
                                for key in obj.keys() {
                                    if !properties.contains_key(key) {
                                        errors.push(format!("{path}.{key}: 未定义的字段"));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "array" => {
                if !val.is_array() {
                    errors.push(format!("{path}: 期望 array，实际 {}", type_name(val)));
                    return;
                }
                if let Some(items_schema) = schema.get("items") {
                    if let Some(arr) = val.as_array() {
                        for (i, item) in arr.iter().enumerate() {
                            validate_value(&format!("{path}[{i}]"), item, items_schema, errors);
                        }
                    }
                }
            }
            "string" => {
                if !val.is_string() {
                    errors.push(format!("{path}: 期望 string"));
                }
            }
            "number" | "integer" => {
                if !val.is_number() {
                    errors.push(format!("{path}: 期望 number"));
                }
            }
            "boolean" => {
                if !matches!(val, Value::Bool(_)) {
                    errors.push(format!("{path}: 期望 boolean"));
                }
            }
            _ => {}
        }
    }
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

fn is_required(schema: &Value) -> bool {
    // 没有 default 值的字段视为必需的
    !schema.get("default").is_some()
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
        assert!(validate_output_against_schema(&output, &schema).is_ok());
    }

    #[test]
    fn test_missing_required_field() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });
        let output = json!({});
        let result = validate_output_against_schema(&output, &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("缺少必填字段"));
    }

    #[test]
    fn test_type_mismatch() {
        let schema = json!({
            "type": "object",
            "properties": {
                "age": { "type": "integer" }
            }
        });
        let output = json!({ "age": "not_a_number" });
        let result = validate_output_against_schema(&output, &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("期望 number"));
    }

    #[test]
    fn test_additional_properties_blocked() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "additionalProperties": false
        });
        let output = json!({ "name": "Alice", "extra": "not allowed" });
        let result = validate_output_against_schema(&output, &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("未定义的字段"));
    }

    #[test]
    fn test_array_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        });
        let output = json!({ "items": ["a", "b", "c"] });
        assert!(validate_output_against_schema(&output, &schema).is_ok());

        let bad_output = json!({ "items": ["a", 42, "c"] });
        let result = validate_output_against_schema(&bad_output, &schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_boolean_schema_true() {
        // true schema allows anything
        let schema = json!(true);
        let output = json!("anything");
        assert!(validate_output_against_schema(&output, &schema).is_ok());
    }

    #[test]
    fn test_boolean_schema_false() {
        // false schema allows nothing
        let schema = json!(false);
        let output = json!("anything");
        let result = validate_output_against_schema(&output, &schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "meta": {
                    "type": "object",
                    "properties": {
                        "count": { "type": "integer" }
                    }
                }
            }
        });
        let output = json!({ "meta": { "count": 5 } });
        assert!(validate_output_against_schema(&output, &schema).is_ok());

        let bad_output = json!({ "meta": { "count": "five" } });
        let result = validate_output_against_schema(&bad_output, &schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_field_with_default_is_optional() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "default": "unknown" }
            }
        });
        let output = json!({});
        assert!(validate_output_against_schema(&output, &schema).is_ok());
    }
}
