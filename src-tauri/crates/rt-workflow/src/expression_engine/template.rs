// SPDX-License-Identifier: AGPL-3.0-only

use super::ExpressionContext;
use regex::Regex;
use serde_json::Value;

/// 错误类型
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("表达式解析失败: {0}")]
    ParseError(String),
    #[error("表达式求值失败: {0}")]
    EvalError(String),
    #[error("表达式返回非字符串值")]
    NonStringValue,
}

/// 解析字符串模板中的 {{ expression }}，提取表达式，求值，拼接结果
///
/// 示例:
///   "Hello {{ $vars.name }}, total: {{ $vars.qty * $vars.price }}"
///   → 扫描到两个表达式，逐个求值后拼接
pub fn resolve_template(template: &str, ctx: &ExpressionContext) -> Result<String, TemplateError> {
    let re = Regex::new(r"\{\{(.+?)\}\}").map_err(|e| TemplateError::ParseError(e.to_string()))?;
    let mut result = String::new();
    let mut last_end = 0;

    for caps in re.captures_iter(template) {
        let full_match = caps
            .get(0)
            .ok_or_else(|| TemplateError::ParseError("missing capture group 0".into()))?;
        let expr = caps
            .get(1)
            .ok_or_else(|| TemplateError::ParseError("missing capture group 1".into()))?
            .as_str()
            .trim();

        // 添加模板片段
        result.push_str(&template[last_end..full_match.start()]);

        // 求值表达式
        let value = super::rhai_eval::resolve_expression(expr, ctx)
            .map_err(|e| TemplateError::EvalError(e.to_string()))?;

        // 转字符串拼接
        match value {
            Value::String(s) => result.push_str(&s),
            Value::Null => {}, // null 插入空字符串
            other => result.push_str(&other.to_string()),
        }

        last_end = full_match.end();
    }

    // 添加尾部片段
    result.push_str(&template[last_end..]);
    Ok(result)
}

/// 对 Value 递归解析模板
/// 遍历 JSON 树，对所有字符串值调用 resolve_template
pub fn resolve_value_templates(
    value: &Value,
    ctx: &ExpressionContext,
) -> Result<Value, TemplateError> {
    match value {
        Value::String(s) => {
            if s.contains("{{") {
                Ok(Value::String(resolve_template(s, ctx)?))
            } else {
                Ok(value.clone())
            }
        },
        Value::Array(arr) => {
            let resolved: Result<Vec<Value>, _> =
                arr.iter().map(|v| resolve_value_templates(v, ctx)).collect();
            Ok(Value::Array(resolved?))
        },
        Value::Object(obj) => {
            let resolved: Result<serde_json::Map<String, Value>, _> = obj
                .iter()
                .map(|(k, v)| Ok((k.clone(), resolve_value_templates(v, ctx)?)))
                .collect();
            Ok(Value::Object(resolved?))
        },
        _ => Ok(value.clone()),
    }
}
