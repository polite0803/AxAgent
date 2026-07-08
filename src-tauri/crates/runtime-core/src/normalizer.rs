// SPDX-License-Identifier: AGPL-3.0-only

//! `ResponseNormalizer` trait 的实现 — 将 LLM 响应的原生格式规范化为中间表示。
//!
//! 遵循「harness 定义 trait → runtime-core 实现 → LlmCallConfig 注入」架构。

use axagent_harness::response_normalizer::ResponseNormalizer;
use axagent_harness::types::{ChatResponse, ContentBlock, ToolCall};

/// 默认的响应规范化器实现。
///
/// ## 规范化规则
/// 1. 从 `response.content` 中提取 ` ```tool_json``` / ` ```json``` / ` ```tool``` ` 代码块
/// 2. 尝试将代码块解析为 `{name, arguments}` → `ContentBlock::ToolUse`
/// 3. 合并 `response.tool_calls`（结构化字段）转为 `ToolUse`（去重）
/// 4. 剩余纯文本 → `ContentBlock::Text`
pub struct DefaultResponseNormalizer;

#[async_trait::async_trait]
impl ResponseNormalizer for DefaultResponseNormalizer {
    async fn normalize(&self, response: &ChatResponse) -> Vec<ContentBlock> {
        let mut blocks: Vec<ContentBlock> = Vec::new();
        let mut text_buffer = String::new();

        // ── 1. 从 content 中提取 Markdown 代码块 ──
        let mut remaining = response.content.as_str();

        while let Some(start) = remaining.find("```") {
            // 代码块之前的纯文本
            let before = &remaining[..start];
            text_buffer.push_str(before);

            // 提取代码块
            let after_start = &remaining[start + 3..];
            let (lang, content_after_lang) = split_lang(after_start);

            if let Some(end) = content_after_lang.find("```") {
                let code_content = &content_after_lang[..end];

                if let Some(tool_call) = try_parse_code_block(lang, code_content) {
                    flush_text(&mut text_buffer, &mut blocks);
                    blocks.push(tool_call);
                } else {
                    // 无法解析 → 原文保留
                    text_buffer.push_str("```");
                    text_buffer.push_str(lang);
                    text_buffer.push('\n');
                    text_buffer.push_str(code_content);
                    text_buffer.push_str("\n```");
                }

                remaining = &content_after_lang[end + 3..];
            } else {
                text_buffer.push_str(&remaining[start..]);
                remaining = "";
                break;
            }
        }

        if !remaining.is_empty() {
            text_buffer.push_str(remaining);
        }
        flush_text(&mut text_buffer, &mut blocks);

        // ── 2. 合并 response.tool_calls（结构化字段） ──
        if let Some(ref tool_calls) = response.tool_calls {
            for tc in tool_calls {
                let already_present = blocks.iter().any(|b| {
                    if let ContentBlock::ToolUse { id, name, .. } = b {
                        id == &tc.id && name == &tc.function.name
                    } else {
                        false
                    }
                });
                if !already_present {
                    add_tool_use(&mut blocks, tc);
                }
            }
        }

        // ── 3. 保底：没有任何 block 时返回纯文本 ──
        if blocks.is_empty() && !response.content.is_empty() {
            blocks.push(ContentBlock::Text {
                text: response.content.clone(),
            });
        }

        blocks
    }
}

// ── 内部辅助函数 ──

fn split_lang(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    if let Some(newline) = s.find('\n') {
        let lang = &s[..newline].trim();
        let content = &s[newline + 1..];
        (lang, content)
    } else {
        ("", "")
    }
}

fn try_parse_code_block(lang: &str, content: &str) -> Option<ContentBlock> {
    let is_tool_block = lang.eq_ignore_ascii_case("tool_json")
        || lang.eq_ignore_ascii_case("tool")
        || lang.eq_ignore_ascii_case("json");
    if !is_tool_block {
        return None;
    }

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let name = val
            .get("name")
            .and_then(|v| v.as_str())
            .or_else(|| {
                val.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("unknown_tool");

        let arguments = val
            .get("arguments")
            .or_else(|| val.get("input"))
            .or_else(|| val.get("parameters"))
            .cloned()
            .map(|v| {
                if v.is_string() {
                    v.as_str().unwrap_or("{}").to_string()
                } else {
                    serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string())
                }
            })
            .unwrap_or_else(|| trimmed.to_string());

        let id = val
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("auto")
            .to_string();

        return Some(ContentBlock::ToolUse {
            id,
            name: name.to_string(),
            input: arguments,
        });
    }

    None
}

fn add_tool_use(blocks: &mut Vec<ContentBlock>, tc: &ToolCall) {
    let id = tc.id.clone();
    let name = tc.function.name.clone();
    let input = tc.function.arguments.clone();
    blocks.push(ContentBlock::ToolUse { id, name, input });
}

fn flush_text(buffer: &mut String, blocks: &mut Vec<ContentBlock>) {
    let text = std::mem::take(buffer);
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        blocks.push(ContentBlock::Text {
            text: trimmed.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::types::{TokenUsage, ToolCallFunction};

    fn make_text_response(content: &str) -> ChatResponse {
        ChatResponse {
            id: "test".into(),
            model: "test".into(),
            content: content.to_string(),
            thinking: None,
            usage: TokenUsage::default(),
            tool_calls: None,
        }
    }

    fn make_tool_call_response(content: &str, tool_calls: Vec<ToolCall>) -> ChatResponse {
        ChatResponse {
            id: "test".into(),
            model: "test".into(),
            content: content.to_string(),
            thinking: None,
            usage: TokenUsage::default(),
            tool_calls: Some(tool_calls),
        }
    }

    #[tokio::test]
    async fn normalizes_plain_text() {
        let normalizer = DefaultResponseNormalizer;
        let response = make_text_response("你好，世界！");
        let blocks = normalizer.normalize(&response).await;
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0],
            ContentBlock::Text {
                text: "你好，世界！".to_string()
            }
        );
    }

    #[tokio::test]
    async fn extracts_tool_json_block() {
        let normalizer = DefaultResponseNormalizer;
        let response = make_text_response(
            "我需要查一下天气。\n\n```tool_json\n{\"name\": \"get_weather\", \"arguments\": {\"city\": \"北京\"}}\n```\n\n这是结果。",
        );
        let blocks = normalizer.normalize(&response).await;
        assert_eq!(blocks.len(), 3);
        assert_eq!(
            blocks[0],
            ContentBlock::Text {
                text: "我需要查一下天气。".to_string()
            }
        );
        assert_eq!(
            blocks[1],
            ContentBlock::ToolUse {
                id: "auto".to_string(),
                name: "get_weather".to_string(),
                input: "{\"city\":\"北京\"}".to_string(),
            }
        );
        assert_eq!(
            blocks[2],
            ContentBlock::Text {
                text: "这是结果。".to_string()
            }
        );
    }

    #[tokio::test]
    async fn merges_structured_tool_calls() {
        let normalizer = DefaultResponseNormalizer;
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: "search".to_string(),
                arguments: "{\"q\": \"Rust\"}".to_string(),
            },
        };
        let response = make_tool_call_response("我来搜索一下。", vec![tool_call]);
        let blocks = normalizer.normalize(&response).await;
        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[1],
            ContentBlock::ToolUse {
                id: "call_123".to_string(),
                name: "search".to_string(),
                input: "{\"q\": \"Rust\"}".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn deduplicates_tool_calls() {
        let normalizer = DefaultResponseNormalizer;
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: "get_weather".to_string(),
                arguments: "{\"city\": \"北京\"}".to_string(),
            },
        };
        let response = make_tool_call_response(
            "```tool_json\n{\"id\": \"call_123\", \"name\": \"get_weather\", \"arguments\": {\"city\": \"北京\"}}\n```",
            vec![tool_call],
        );
        let blocks = normalizer.normalize(&response).await;
        let tool_uses: Vec<_> = blocks
            .iter()
            .filter(|b| matches!(b, ContentBlock::ToolUse { .. }))
            .collect();
        assert_eq!(tool_uses.len(), 1);
    }

    #[tokio::test]
    async fn handles_empty_content() {
        let normalizer = DefaultResponseNormalizer;
        let response = make_text_response("");
        let blocks = normalizer.normalize(&response).await;
        assert!(blocks.is_empty());
    }

    #[tokio::test]
    async fn ignores_regular_code_blocks() {
        let normalizer = DefaultResponseNormalizer;
        let response = make_text_response("以下是代码：\n```rust\nfn main() {}\n```\n结束。");
        let blocks = normalizer.normalize(&response).await;
        assert_eq!(blocks.len(), 1);
    }
}
