// SPDX-License-Identifier: AGPL-3.0-only

//! 工具输入校验服务
//!
//! 从 `Tool::validate()` trait 默认方法提取为独立结构体，
//! 便于单元测试和依赖注入，不污染 trait 契约。

use crate::error::ToolError;
use serde_json::Value;

/// 工具参数 JSON Schema 校验器。
///
/// 支持校验：required / type / enum / minimum / maximum / minLength / maxLength。
/// 从原 `Tool::validate()` trait 默认方法提取。
#[derive(Debug, Clone, Default)]
pub struct ToolValidator;

impl ToolValidator {
    /// 根据 JSON Schema 校验输入参数。
    ///
    /// 参数：
    /// - `input`: 实际输入值
    /// - `schema`: 工具声明的 JSON Schema（来自 `Tool::input_schema()`）
    ///
    /// 返回 `Ok(())` 或 `Err(ToolError)`。
    pub fn validate(&self, input: &Value, schema: &Value) -> Result<(), ToolError> {
        // 必填字段检查
        if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
            for field in required {
                let key = field.as_str().unwrap_or("");
                if input.get(key).is_none() || input.get(key) == Some(&Value::Null) {
                    return Err(ToolError::invalid_input(format!("缺少必需参数: {key}")));
                }
            }
        }

        // 校验 properties 中每个字段的类型/格式/枚举值/范围
        if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
            for (prop_name, prop_schema) in properties {
                let val = match input.get(prop_name) {
                    Some(v) if !v.is_null() => v,
                    _ => continue, // 可选参数且未提供，跳过
                };

                // 类型校验
                if let Some(expected_type) = prop_schema.get("type").and_then(|t| t.as_str()) {
                    let type_ok = match expected_type {
                        "string" => val.is_string(),
                        "number" | "integer" => val.is_number(),
                        "boolean" => matches!(val, Value::Bool(_)),
                        "array" => val.is_array(),
                        "object" => val.is_object(),
                        _ => true,
                    };
                    if !type_ok {
                        return Err(ToolError::invalid_input(format!(
                            "参数 '{prop_name}' 应为 {expected_type} 类型"
                        )));
                    }
                    // 对 integer 额外检查必须是整数
                    if expected_type == "integer" && !val.as_f64().is_some_and(|f| f.fract() == 0.0)
                    {
                        return Err(ToolError::invalid_input(format!(
                            "参数 '{prop_name}' 应为整数"
                        )));
                    }
                }

                // 枚举值校验
                if let Some(enum_vals) = prop_schema.get("enum").and_then(|e| e.as_array())
                    && !enum_vals.contains(val)
                {
                    return Err(ToolError::invalid_input(format!(
                        "参数 '{prop_name}' 值不在允许范围内: {:?}",
                        enum_vals
                    )));
                }

                // 最小值/最大值校验（数值）
                if let Some(min) = prop_schema.get("minimum").and_then(|m| m.as_f64())
                    && let Some(n) = val.as_f64()
                    && n < min
                {
                    return Err(ToolError::invalid_input(format!(
                        "参数 '{prop_name}' 不能小于 {min}"
                    )));
                }
                if let Some(max) = prop_schema.get("maximum").and_then(|m| m.as_f64())
                    && let Some(n) = val.as_f64()
                    && n > max
                {
                    return Err(ToolError::invalid_input(format!(
                        "参数 '{prop_name}' 不能大于 {max}"
                    )));
                }

                // 最小长度/最大长度校验（字符串）
                if let Some(min_len) = prop_schema.get("minLength").and_then(|m| m.as_u64())
                    && let Some(s) = val.as_str()
                    && (s.len() as u64) < min_len
                {
                    return Err(ToolError::invalid_input(format!(
                        "参数 '{prop_name}' 长度不能少于 {min_len}"
                    )));
                }
                if let Some(max_len) = prop_schema.get("maxLength").and_then(|m| m.as_u64())
                    && let Some(s) = val.as_str()
                    && (s.len() as u64) > max_len
                {
                    return Err(ToolError::invalid_input(format!(
                        "参数 '{prop_name}' 长度不能超过 {max_len}"
                    )));
                }
            }
        }

        Ok(())
    }
}
