//! OPC 领域工作流 — 输出格式强约束公共模板
//!
//! 本模块提供统一的输出格式约束模板，用于确保所有行业工作流的 Agent 节点
//! 输出遵循一致的 JSON Schema 格式，便于下游节点解析和验证。

/// 生成输出格式强约束提示词
///
/// 将此函数返回的字符串追加到 Agent 步骤的 prompt 末尾，强制 LLM 输出
/// 符合指定 Schema 的 JSON 格式。
///
/// # 参数
/// - `schema_hint`: 数据结构的 JSON Schema 提示（如 `{"endpoints":[],...}`）
/// - `empty_fallback`: 数据不可用时的空结构回退（如 `{"endpoints":[],"error":"原因"}`）
///
/// # 返回
/// 格式化的约束提示词字符串
pub fn wrap_json_output(schema_hint: &str, empty_fallback: &str) -> String {
    format!(
        "============== 输出格式强约束（必须严格遵守） ==============\n\
         1. 回复必须且只能包含一个代码块，开头三个反引号紧跟 tool_json。\n\
         2. 代码块内容为单一 JSON 对象：{{\"name\": \"submit_result\", \"arguments\": <数据>}}\n\
         3. <数据> 结构：{schema_hint}\n\
         4. 代码块外禁止任何文字：不要写\"以下是...\"、注释、解释。\n\
         5. 若数据不可用，返回合法空结构：{empty_fallback}，禁止自然语言拒绝。\n\
         ============================================================"
    )
}

/// 生成标准的"空数据降级"提示
///
/// 用于 Agent 步骤的 on_error 或 prompt 末尾，指示当上游无有效数据时
/// 应返回空结构而非自然语言拒绝。
pub fn empty_data_fallback(description: &str) -> String {
    format!(
        "\n\n[空数据降级] 若上游无有效{description}数据，请返回空结构 JSON（{{\"empty\":true,\"reason\":\"无数据\"}}），\n\
         禁止以自然语言拒绝或编造数据。"
    )
}

/// 生成工具使用约束提示
///
/// 指定 Agent 节点可使用的工具列表，并要求优先使用工具获取信息。
pub fn tool_usage_hint(tools: &[String]) -> String {
    if tools.is_empty() {
        return String::new();
    }
    let tools_list = tools.iter().map(|t| format!("- {t}")).collect::<Vec<_>>().join("\n");
    format!(
        "\n\n[工具使用] 你可以使用以下工具来完成任务：\n{tools_list}\n\
         优先使用工具获取信息，基于工具返回的结果进行分析和决策。"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_json_output() {
        let result =
            wrap_json_output(r#"{"name":"test","value":0}"#, r#"{"name":"test","value":null}"#);
        assert!(result.contains("输出格式强约束"));
        assert!(result.contains("tool_json"));
        assert!(result.contains("submit_result"));
        assert!(result.contains(r#"{"name":"test","value":0}"#));
        assert!(result.contains(r#"{"name":"test","value":null}"#));
    }

    #[test]
    fn test_empty_data_fallback() {
        let result = empty_data_fallback("需求");
        assert!(result.contains("空数据降级"));
        assert!(result.contains("需求"));
        assert!(result.contains(r#"{"empty":true,"reason":"无数据"}"#));
    }

    #[test]
    fn test_tool_usage_hint_empty() {
        let result = tool_usage_hint(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_tool_usage_hint_with_tools() {
        let result = tool_usage_hint(&["FileRead".to_string(), "Grep".to_string()]);
        assert!(result.contains("工具使用"));
        assert!(result.contains("FileRead"));
        assert!(result.contains("Grep"));
    }
}
