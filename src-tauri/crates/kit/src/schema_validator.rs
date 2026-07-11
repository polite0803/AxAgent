// SPDX-License-Identifier: AGPL-3.0-only

//! JSON Schema 校验工具 —— 薄封装重导出层
//!
//! 权威实现已移至 `axagent_harness::json_schema`。
//! 本模块保留以维持旧 API 兼容性。

pub use axagent_harness::json_schema::{validate_against_schema, validate_recursive};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_type_validation_passes() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            }
        });
        let value = json!({ "name": "Alice", "age": 30 });
        let (ok, errors) = validate_against_schema(&value, &schema);
        assert!(ok, "Expected success, got: {:?}", errors);
    }

    #[test]
    fn test_required_field_missing() {
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": { "name": { "type": "string" } }
        });
        let value = json!({});
        let (ok, errors) = validate_against_schema(&value, &schema);
        assert!(!ok);
        assert!(!errors.is_empty());
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
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_array_items_validation() {
        let schema = json!({
            "type": "array",
            "items": { "type": "integer" }
        });
        let (ok, _) = validate_against_schema(&json!([1, 2, 3]), &schema);
        assert!(ok);

        let (ok, errors) = validate_against_schema(&json!([1, "bad", 3]), &schema);
        assert!(!ok);
        assert!(!errors.is_empty());
    }
}
