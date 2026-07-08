// SPDX-License-Identifier: AGPL-3.0-only

//! `IrRenderer` trait 的实现 — 将中间表示（`Vec<ContentBlock>`）渲染为清爽的纯文本。
//!
//! 遵循「harness 定义 trait → agent 实现 → LlmCallConfig 注入」架构。

use axagent_harness::ir_renderer::IrRenderer;
use axagent_harness::types::ContentBlock;

/// 默认的 IR 渲染器实现。
///
/// ## 渲染规则
/// - `ContentBlock::Text` → 直接拼接，段落间空行
/// - `ContentBlock::ToolUse` → 跳过（不暴露给最终用户）
/// - `ContentBlock::ToolResult` → 翻译为自然语言描述
/// - 最终正则清理：去多余空行、特殊占位符、重复标点
pub struct DefaultIrRenderer;

#[async_trait::async_trait]
impl IrRenderer for DefaultIrRenderer {
    async fn render(&self, blocks: &[ContentBlock]) -> String {
        let mut parts: Vec<String> = Vec::new();

        for block in blocks {
            match block {
                ContentBlock::Text { text } => {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        parts.push(trimmed.to_string());
                    }
                },
                ContentBlock::ToolUse { .. } => {
                    // 工具调用本身不暴露给用户
                },
                ContentBlock::ToolResult {
                    tool_name,
                    output,
                    is_error,
                    ..
                } => {
                    let translated = translate_tool_result(tool_name, output, *is_error);
                    parts.push(translated);
                },
            }
        }

        let raw = parts.join("\n\n");
        clean_output(&raw)
    }
}

/// 翻译 `ToolResult` 为自然语言描述。
fn translate_tool_result(tool_name: &str, output: &str, is_error: bool) -> String {
    let prefix = if is_error {
        format!("[工具 {tool_name} 执行出错]")
    } else {
        format!("[工具 {tool_name} 执行结果]")
    };

    let trimmed = output.trim();
    if trimmed.is_empty() {
        return format!("{prefix}: （空结果）");
    }

    // 尝试 JSON 展开
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let natural = json_to_natural(&val);
        if !natural.is_empty() {
            return format!("{prefix}: {natural}");
        }
    }

    let truncated = if trimmed.len() > 2000 {
        format!("{}…（共 {} 字符）", &trimmed[..2000], trimmed.len())
    } else {
        trimmed.to_string()
    };

    format!("{prefix}: {truncated}")
}

/// 将 JSON Value 展开为自然语言描述。
fn json_to_natural(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| {
                    let v_str = match v {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Null => "空".to_string(),
                        other => serde_json::to_string(other).unwrap_or_default(),
                    };
                    format!("{k}: {v_str}")
                })
                .collect();
            items.join("，")
        },
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(json_to_natural)
                .filter(|s| !s.is_empty())
                .collect();
            items.join("；")
        },
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
    }
}

/// 最终文本清理：剔除多余空行、特殊占位符、重复标点（委托到 runtime-core）。
pub fn clean_output(text: &str) -> String {
    axagent_runtime_core::clean_output(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn renders_plain_text() {
        let renderer = DefaultIrRenderer;
        let blocks = vec![ContentBlock::Text {
            text: "你好，世界！".to_string(),
        }];
        let output = renderer.render(&blocks).await;
        assert_eq!(output, "你好，世界！");
    }

    #[tokio::test]
    async fn skips_tool_use() {
        let renderer = DefaultIrRenderer;
        let blocks = vec![
            ContentBlock::Text {
                text: "我来查天气。".to_string(),
            },
            ContentBlock::ToolUse {
                id: "call_1".to_string(),
                name: "get_weather".to_string(),
                input: "{\"city\": \"北京\"}".to_string(),
            },
            ContentBlock::Text {
                text: "完成。".to_string(),
            },
        ];
        let output = renderer.render(&blocks).await;
        assert_eq!(output, "我来查天气。\n\n完成。");
        assert!(!output.contains("get_weather"));
    }

    #[tokio::test]
    async fn translates_tool_result_json() {
        let renderer = DefaultIrRenderer;
        let blocks = vec![
            ContentBlock::Text {
                text: "查询结果：".to_string(),
            },
            ContentBlock::ToolResult {
                tool_use_id: "call_1".to_string(),
                tool_name: "get_weather".to_string(),
                output: "{\"temp\": 26, \"condition\": \"晴\"}".to_string(),
                is_error: false,
            },
        ];
        let output = renderer.render(&blocks).await;
        assert!(output.contains("get_weather"));
        assert!(output.contains("temp: 26"));
        assert!(output.contains("condition: 晴"));
    }

    #[tokio::test]
    async fn handles_error_tool_result() {
        let renderer = DefaultIrRenderer;
        let blocks = vec![ContentBlock::ToolResult {
            tool_use_id: "call_err".to_string(),
            tool_name: "bash".to_string(),
            output: "command not found".to_string(),
            is_error: true,
        }];
        let output = renderer.render(&blocks).await;
        assert!(output.contains("执行出错"));
        assert!(output.contains("bash"));
    }

    #[tokio::test]
    async fn cleans_special_placeholders() {
        let renderer = DefaultIrRenderer;
        let blocks = vec![ContentBlock::Text {
            text: "你好<|endoftext|>世界".to_string(),
        }];
        let output = renderer.render(&blocks).await;
        assert_eq!(output, "你好世界");
    }

    #[tokio::test]
    async fn consolidates_repeated_blank_lines() {
        let renderer = DefaultIrRenderer;
        let blocks = vec![ContentBlock::Text {
            text: "段落1\n\n\n\n\n段落2".to_string(),
        }];
        let output = renderer.render(&blocks).await;
        assert_eq!(output, "段落1\n\n段落2");
    }

    #[tokio::test]
    async fn handles_empty_blocks() {
        let renderer = DefaultIrRenderer;
        let blocks: Vec<ContentBlock> = vec![];
        let output = renderer.render(&blocks).await;
        assert_eq!(output, "");
    }
}
