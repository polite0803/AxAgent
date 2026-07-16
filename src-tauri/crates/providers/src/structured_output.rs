// SPDX-License-Identifier: AGPL-3.0-only

//! Structured Output 降级工具
//!
//! 改进6（M2-a）为 OpenAI / Gemini / Anthropic 实现了原生 Structured Output。
//! 但所有国内 OpenAI 兼容 provider（DeepSeek / Kimi / Qwen / GLM / 豆包 /
//! SiliconFlow / MiniMax / NVIDIA）共用 `OpenAIAdapter`，其中部分 provider
//! 不支持 `response_format: json_schema` 严格模式，会被静默忽略或返回 400。
//!
//! 本模块提供：
//! - `supports_json_schema_strict`：按 base_url 识别 provider 是否支持严格模式
//! - `inject_schema_constraint`：在 system prompt 注入 JSON Schema 约束（fallback）
//!
//! 当 provider 不支持 `json_schema` 时，OpenAIAdapter 会：
//! 1. 将 `ResponseFormat::JsonSchema { .. }` 降级为 `ResponseFormat::JsonObject`
//! 2. 在 system prompt 追加 schema 约束（复用 Anthropic 的 prompt 注入思路）
//!
//! 这样可避免 400 报错，并尽量引导输出符合 schema（非严格保证）。

use axagent_harness::types::ResponseFormat;

/// 判断 base_url 对应的 provider 是否支持 `response_format: json_schema` 严格模式
///
/// 返回 `false` 的 provider 会触发降级路径：
/// - DeepSeek（api.deepseek.com）：不支持 json_schema 严格模式
/// - Kimi / Moonshot（api.moonshot.cn）：不支持
/// - MiniMax（api.minimax.io）：不支持
///
/// 返回 `true` 的 provider：
/// - OpenAI 官方（api.openai.com / 默认）
/// - 硅基流动 SiliconFlow（api.siliconflow.cn）：按底层模型判断，保守起见返回 true
/// - 通义千问 Qwen（dashscope.aliyuncs.com）：部分模型支持，保守返回 true
/// - 智谱 GLM（open.bigmodel.cn）：保守返回 true
/// - 豆包 Doubao（ark.cn-beijing.volces.com）：保守返回 true
/// - NVIDIA（integrate.api.nvidia.com）：保守返回 true
/// - Ollama（localhost / 127.0.0.1）：OpenAI 兼容端点支持
/// - 其它自定义端点：保守返回 true（让 provider 自行决定，触发 400 再降级）
pub fn supports_json_schema_strict(base_url: &str) -> bool {
    // 已知不支持的 provider
    let known_unsupported = ["api.deepseek.com", "api.moonshot.cn", "api.minimax.io"];

    // base_url 可能带 https:// 前缀或路径后缀，用 contains 匹配
    for host in &known_unsupported {
        if base_url.contains(host) {
            return false;
        }
    }

    true
}

/// 构造 system prompt 注入约束文本
///
/// 用于在 OpenAIAdapter 降级路径中，将 schema 约束追加到 system message。
/// 逻辑与 `anthropic.rs::inject_structured_output_constraint` 一致，确保
/// 所有 provider 的 fallback 行为统一。
pub fn build_schema_constraint(response_format: &ResponseFormat) -> String {
    match response_format {
        ResponseFormat::JsonObject => {
            "\n\nYou must respond with a valid JSON object. Do not include any text outside the JSON."
                .to_string()
        },
        ResponseFormat::JsonSchema { schema, .. } => {
            format!(
                "\n\nYou must respond with a valid JSON object that conforms to the following JSON Schema. \
                Do not include any text outside the JSON.\n\n```json\n{}\n```",
                serde_json::to_string_pretty(schema).unwrap_or_else(|_| schema.to_string())
            )
        },
    }
}

/// 把 schema 约束追加到 messages 的 system 消息（如果有的话）
///
/// OpenAI 协议的 system 消息是 `messages[0]` 且 role == "system"。
/// 如果没有 system 消息，则插入一条新的 system 消息到 messages 开头。
///
/// 返回新的 messages（原 messages 不变）。
pub fn inject_constraint_into_messages(
    mut messages: Vec<axagent_harness::types::ChatMessage>,
    constraint: &str,
) -> Vec<axagent_harness::types::ChatMessage> {
    use axagent_harness::types::{ChatContent, ChatMessage};

    // 找到第一条 system 消息的索引
    let system_idx = messages.iter().position(|m| m.role == "system");

    if let Some(idx) = system_idx {
        // 已有 system 消息：追加约束
        let existing = match &messages[idx].content {
            ChatContent::Text(text) => text.clone(),
            ChatContent::Multipart(parts) => {
                // 拼接所有 text part
                parts.iter().filter_map(|p| p.text.as_ref()).cloned().collect::<Vec<_>>().join("\n")
            },
        };
        let combined = format!("{existing}{constraint}");
        messages[idx].content = ChatContent::Text(combined);
    } else {
        // 无 system 消息：插入一条到开头
        messages.insert(
            0,
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(constraint.trim_start().to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        );
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::types::ResponseFormat;

    #[test]
    fn test_supports_json_schema_strict_openai() {
        assert!(supports_json_schema_strict("https://api.openai.com/v1"));
    }

    #[test]
    fn test_supports_json_schema_strict_deepseek() {
        assert!(!supports_json_schema_strict("https://api.deepseek.com"));
        assert!(!supports_json_schema_strict("https://api.deepseek.com/v1"));
    }

    #[test]
    fn test_supports_json_schema_strict_kimi() {
        assert!(!supports_json_schema_strict("https://api.moonshot.cn/v1"));
    }

    #[test]
    fn test_supports_json_schema_strict_minimax() {
        assert!(!supports_json_schema_strict("https://api.minimax.io"));
    }

    #[test]
    fn test_supports_json_schema_strict_qwen() {
        // 保守返回 true
        assert!(supports_json_schema_strict("https://dashscope.aliyuncs.com/compatible-mode/v1"));
    }

    #[test]
    fn test_supports_json_schema_strict_siliconflow() {
        // 保守返回 true
        assert!(supports_json_schema_strict("https://api.siliconflow.cn/v1"));
    }

    #[test]
    fn test_supports_json_schema_strict_ollama() {
        assert!(supports_json_schema_strict("http://localhost:11434/v1"));
        assert!(supports_json_schema_strict("http://127.0.0.1:11434/v1"));
    }

    #[test]
    fn test_build_schema_constraint_json_object() {
        let fmt = ResponseFormat::JsonObject;
        let constraint = build_schema_constraint(&fmt);
        assert!(constraint.contains("valid JSON object"));
        assert!(constraint.contains("Do not include any text outside the JSON"));
    }

    #[test]
    fn test_build_schema_constraint_json_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });
        let fmt = ResponseFormat::JsonSchema {
            name: "test_schema".to_string(),
            schema,
            strict: Some(true),
        };
        let constraint = build_schema_constraint(&fmt);
        assert!(constraint.contains("JSON Schema"));
        assert!(constraint.contains("\"name\""));
        assert!(constraint.contains("\"required\""));
    }

    #[test]
    fn test_inject_constraint_with_existing_system() {
        use axagent_harness::types::{ChatContent, ChatMessage};

        let messages = vec![ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text("You are helpful.".to_string()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        }];

        let result = inject_constraint_into_messages(messages, "\n\nBe nice.");
        assert_eq!(result.len(), 1);
        if let ChatContent::Text(text) = &result[0].content {
            assert!(text.contains("You are helpful."));
            assert!(text.contains("Be nice."));
        } else {
            panic!("expected Text content");
        }
    }

    #[test]
    fn test_inject_constraint_without_system() {
        use axagent_harness::types::{ChatContent, ChatMessage};

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text("hi".to_string()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        }];

        let result = inject_constraint_into_messages(messages, "\n\nBe nice.");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        if let ChatContent::Text(text) = &result[0].content {
            assert!(text.contains("Be nice."));
        } else {
            panic!("expected Text content");
        }
    }

    #[test]
    fn test_inject_constraint_empty_messages() {
        let result = inject_constraint_into_messages(Vec::new(), "\n\nBe nice.");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "system");
    }
}
