//! Schema 序列化回归测试
//!
//! 防止上游 `JsonSchema` 字段因缺 `#[serde(skip_serializing_if = "Option::is_none")]`
//! 而序列化为 `null`，进而被 `jsonschema` 0.46.5 在 draft 7 meta-schema 严格校验下拒绝，
//! 抛出 `Schema compile error: null is not of types "boolean", "object"`。
//!
//! 复现条件：构造与 `stock_analysis_setup.rs::seed_workflow_template` 同形状的
//! `input_schema` / `output_schema` / `ToolDef::parameters`。
//!
//! 见 PR 描述（harness `workflow_types.rs`）：所有 `Option` 字段已加
//! `#[serde(default, skip_serializing_if = "Option::is_none")]`。

#[cfg(test)]
mod tests {
    use axagent_harness::workflow_types::{JsonSchema, JsonSchemaProperty, ToolDef};
    use std::collections::HashMap;

    /// 模拟 stock-analysis 的 input_schema：顶层所有可选字段为 None，properties 内部
    /// property 的 `default` / `enumValues` / `format` 也都是 None。
    fn build_input_schema_like_seed() -> JsonSchema {
        let mut props = HashMap::new();
        props.insert(
            "stock_code".to_string(),
            JsonSchemaProperty {
                schema_type: "string".to_string(),
                description: Some("股票代码，如 000001、600519".to_string()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        JsonSchema {
            schema_type: "object".to_string(),
            description: Some("股票分析运行时输入".to_string()),
            properties: Some(props),
            required: Some(vec!["stock_code".to_string()]),
            items: None,
        }
    }

    /// 模拟 stock-analysis 的 output_schema。
    fn build_output_schema_like_seed() -> JsonSchema {
        let mut props = HashMap::new();
        props.insert(
            "action".to_string(),
            JsonSchemaProperty {
                schema_type: "string".to_string(),
                description: Some("投资决策: 买入/增持/持有/减持/卖出".to_string()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        props.insert(
            "positionPct".to_string(),
            JsonSchemaProperty {
                schema_type: "number".to_string(),
                description: Some("建议仓位百分比 (0-100)".to_string()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        JsonSchema {
            schema_type: "object".to_string(),
            description: Some("股票分析最终决策输出".to_string()),
            properties: Some(props),
            required: Some(vec!["action".to_string(), "positionPct".to_string()]),
            items: None,
        }
    }

    /// 模拟 `ToolDef::parameters`：常用 stock tool 的 `parameters` 都是
    /// `Some(JsonSchema { items: None, description: None, properties: Some(...) })` 形状。
    fn build_tool_parameters_like_seed() -> JsonSchema {
        let mut props = HashMap::new();
        props.insert(
            "stock_code".to_string(),
            JsonSchemaProperty {
                schema_type: "string".to_string(),
                description: Some("6位股票代码，如 600519".to_string()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        JsonSchema {
            schema_type: "object".to_string(),
            description: None,
            properties: Some(props),
            required: Some(vec!["stock_code".to_string()]),
            items: None,
        }
    }

    #[test]
    fn input_schema_serialize_omits_none_top_level_fields() {
        let schema = build_input_schema_like_seed();
        let json = serde_json::to_value(&schema).expect("序列化应成功");

        // 顶层 Some 字段必须保留
        assert_eq!(json["type"], "object");
        assert_eq!(json["description"], "股票分析运行时输入");
        assert!(json["properties"].is_object());
        assert_eq!(json["required"], serde_json::json!(["stock_code"]));

        // 顶层 None 字段不应出现（防止下游 jsonschema 校验失败）
        assert!(json.get("items").is_none(), "items 字段为 None，序列化时不应出现该 key");
    }

    #[test]
    fn input_schema_serialize_omits_none_property_fields() {
        let schema = build_input_schema_like_seed();
        let prop = &schema
            .properties
            .as_ref()
            .expect("properties 应是 Some")
            .get("stock_code")
            .expect("stock_code 应存在");
        let prop_json = serde_json::to_value(prop).expect("property 序列化应成功");

        // property 自身的 None 字段也不应出现
        assert_eq!(prop_json["type"], "string");
        assert_eq!(prop_json["description"], "股票代码，如 000001、600519");
        assert!(prop_json.get("default").is_none(), "default 字段为 None，不应出现");
        assert!(prop_json.get("enumValues").is_none(), "enumValues 字段为 None，不应出现");
        assert!(prop_json.get("format").is_none(), "format 字段为 None，不应出现");
    }

    #[test]
    fn serialized_text_contains_no_null() {
        // input_schema
        let input_s = serde_json::to_string(&build_input_schema_like_seed()).unwrap();
        assert!(
            !input_s.contains(": null"),
            "input_schema 序列化结果不应包含 `: null`，实际：\n{input_s}"
        );

        // output_schema
        let output_s = serde_json::to_string(&build_output_schema_like_seed()).unwrap();
        assert!(
            !output_s.contains(": null"),
            "output_schema 序列化结果不应包含 `: null`，实际：\n{output_s}"
        );

        // ToolDef::parameters
        let tool_s = serde_json::to_string(&build_tool_parameters_like_seed()).unwrap();
        assert!(
            !tool_s.contains(": null"),
            "tool.parameters 序列化结果不应包含 `: null`，实际：\n{tool_s}"
        );
    }

    #[test]
    fn serialized_text_omits_items_key() {
        // 防止 `"items": null` 直接出现在 schema 顶层（这是 jsonschema draft 7
        // meta-schema 校验失败的关键触发点：items 必须是 boolean | object）。
        for schema in [
            build_input_schema_like_seed(),
            build_output_schema_like_seed(),
            build_tool_parameters_like_seed(),
        ] {
            let s = serde_json::to_string(&schema).unwrap();
            assert!(!s.contains("\"items\""), "schema 序列化结果中不应出现 items key，实际：\n{s}");
        }
    }

    #[test]
    fn deserialize_legacy_data_with_null_fields() {
        // 模拟旧版本序列化的 schema 数据（含显式 null 字段）。反序列化时应兼容：
        // `#[serde(default)]` 兜底把缺失/显式 null 的字段当 None 处理。
        let legacy = serde_json::json!({
            "type": "object",
            "description": "股票分析运行时输入",
            "properties": {
                "stock_code": {
                    "type": "string",
                    "description": "股票代码",
                    "default": null,
                    "enumValues": null,
                    "format": null
                }
            },
            "required": ["stock_code"],
            "items": null
        });
        let schema: JsonSchema = serde_json::from_value(legacy)
            .expect("旧数据（含 null 字段）应能反序列化，不应破坏向后兼容");

        assert_eq!(schema.schema_type, "object");
        assert_eq!(schema.description.as_deref(), Some("股票分析运行时输入"));
        assert!(schema.items.is_none(), "items 字段为 null 应反序列化为 None");

        let prop = schema
            .properties
            .as_ref()
            .expect("properties 应是 Some")
            .get("stock_code")
            .expect("stock_code 应存在");
        assert!(prop.default.is_none());
        assert!(prop.enum_values.is_none());
        assert!(prop.format.is_none());
    }

    #[test]
    fn tool_def_serialize_omits_none_fields() {
        // 完整 ToolDef 序列化：顶层 ToolDef 已有 skip_serializing_if，嵌套的
        // JsonSchema 也必须不含 null 字段。
        let tool = ToolDef {
            name: "get_stock_quote".to_string(),
            description: Some("获取股票实时行情".to_string()),
            parameters: Some(build_tool_parameters_like_seed()),
        };
        let s = serde_json::to_string(&tool).unwrap();
        assert!(!s.contains(": null"), "ToolDef 序列化结果不应包含 `: null`");
        assert!(!s.contains("\"items\""));
        // 验证 description 是 Some 时仍会出现
        assert!(s.contains("\"description\":\"获取股票实时行情\""));
    }

    #[test]
    fn tool_def_serialize_with_none_description_omits_description() {
        // 工具无描述时（description: None）—— ToolDef.description 已有
        // skip_serializing_if，验证不会输出 `"description": null`。
        let tool = ToolDef {
            name: "get_stock_quote".to_string(),
            description: None,
            parameters: Some(build_tool_parameters_like_seed()),
        };
        let s = serde_json::to_string(&tool).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert!(v.get("description").is_none(), "description 为 None 时不应出现该 key");
    }
}
