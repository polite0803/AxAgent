// SPDX-License-Identifier: AGPL-3.0-only

//! 节点间数据传递的 Schema 校验工具
//!
//! 在工作流节点之间通过 serde JSON 传递数据时，
//! 提供严格的序列化/反序列化格式强制工具。

use serde_json::Value;

/// 节点输出 Schema 校验
///
/// 校验 `output` 是否匹配 `schema` 定义。
/// 返回 `Ok(())` 或 `Err`（包含所有错误信息列表）。
///
/// 实际逻辑已委托给 `crate::json_schema::validate_ok`，
/// 本函数保留以维持旧 API 兼容性。
pub fn validate_output_against_schema(output: &Value, schema: &Value) -> Result<(), Vec<String>> {
    crate::json_schema::validate_ok(output, schema)
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
            "required": ["name"],
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
