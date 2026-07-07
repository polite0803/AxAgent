// SPDX-License-Identifier: AGPL-3.0-only

//! JSON Schema 校验工具 —— 从 kit 提升至 harness 的基础校验能力

/// 对 JSON 值执行 JSON Schema 校验。
pub fn validate_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
) -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let valid = validate_recursive(value, schema, "", &mut errors);
    (valid, errors)
}

/// 递归 Schema 校验（带路径追踪）
pub fn validate_recursive(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    path: &str,
    errors: &mut Vec<String>,
) -> bool {
    let mut valid = true;
    if let Some(expected_type) = schema.get("type").and_then(|v| v.as_str()) {
        let type_match = match expected_type {
            "string" => value.is_string(),
            "integer" => value.is_i64() || value.is_u64(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            "null" => value.is_null(),
            _ => true,
        };
        if !type_match {
            errors.push(format!("{path}: expected type '{expected_type}', got {value}"));
            valid = false;
        }
    }
    if let Some(required) = schema.get("required").and_then(|v| v.as_array())
        && let Some(obj) = value.as_object()
    {
        for req_field in required {
            if let Some(field_name) = req_field.as_str()
                && !obj.contains_key(field_name)
            {
                errors.push(format!("{path}: missing required field '{field_name}'"));
                valid = false;
            }
        }
    }
    if let Some(properties) = schema.get("properties").and_then(|v| v.as_object())
        && let Some(obj) = value.as_object()
    {
        for (prop_name, prop_schema) in properties {
            let child_path = if path.is_empty() {
                prop_name.clone()
            } else {
                format!("{path}.{prop_name}")
            };
            if let Some(child_value) = obj.get(prop_name)
                && !validate_recursive(child_value, prop_schema, &child_path, errors)
            {
                valid = false;
            }
        }
    }
    if let Some(items_schema) = schema.get("items")
        && let Some(arr) = value.as_array()
    {
        for (i, item) in arr.iter().enumerate() {
            if !validate_recursive(item, items_schema, &format!("{path}[{i}]"), errors) {
                valid = false;
            }
        }
    }
    if let Some(min_len) = schema.get("minLength").and_then(|v| v.as_u64())
        && let Some(s) = value.as_str()
        && (s.len() as u64) < min_len
    {
        errors.push(format!("{path}: minLength={min_len}, got {}", s.len()));
        valid = false;
    }
    if let Some(max_len) = schema.get("maxLength").and_then(|v| v.as_u64())
        && let Some(s) = value.as_str()
        && (s.len() as u64) > max_len
    {
        errors.push(format!("{path}: maxLength={max_len}, got {}", s.len()));
        valid = false;
    }
    if let Some(allowed) = schema.get("enum").and_then(|v| v.as_array())
        && !allowed.contains(value)
    {
        errors.push(format!("{path}: value '{value}' not in allowed enum"));
        valid = false;
    }
    valid
}
