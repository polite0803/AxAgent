// SPDX-License-Identifier: AGPL-3.0-only

//! AxAgent 托管式工具调用适配器（ManagedToolAdapter）
//!
//! 包装任意 `ProviderAdapter`，把原生 tools 从请求中拔出，渲染为系统提示词注入，
//! 再把模型返回的 `<|{prefix}|tool_calls>...</|{prefix}|tool_calls>` XML 块
//! 解析成结构化的 `ToolCall`。
//!
//! marker_prefix 可配置（默认 `"CHAT2API"`），未来可为非 Chat2API 网关指定不同前缀。

use std::collections::{HashSet, VecDeque};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::Stream;

use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::*;
use axagent_harness::{ProviderAdapter, ProviderRequestContext};

// ── Prompt Template ──────────────────────────────────────────
// 占位符 {rendered_tools} 在运行时替换；CHAT2API 硬编码在模板中，
// 注入提示词时统一替换为实际 prefix。
const TOOL_PROMPT_TEMPLATE: &str = "\n\n## 可用工具 (Available Tools)\n\
你可以调用以下开发者工具。工具名称区分大小写，只能使用下面列出的确切名称，\
不要重命名、翻译、缩写或臆造工具名。\n\n\
{rendered_tools}\n\n\
需要调用工具时，只输出如下 XML 块，不要附加任何解释：\n\n\
<|CHAT2API|tool_calls><|CHAT2API|invoke name=\"tool_name\">\
<|CHAT2API|parameter name=\"arg_name\"><![CDATA[arg_value]]>\
</|CHAT2API|parameter></|CHAT2API|invoke></|CHAT2API|tool_calls>\n\n\
工具结果会以下列格式回传给你：\n\n\
<|CHAT2API|tool_result tool_call_id=\"call_0\"><![CDATA[结果内容]]></|CHAT2API|tool_result>\n\n\
如果不需要调用工具，直接用自然语言回答。\n";

// ── Marker 构建辅助 ──────────────────────────────────────────
// 格式：
//   <|{prefix}|tool_calls>            (open / close 块)
//   <|{prefix}|invoke name="..."      (调用标记)
//   <|{prefix}|parameter name="..."   (参数标记)
//   <|{prefix}|tool_result ...        (工具结果标记)
//
// 为流式热路径预计算完整标记字符串，避免运行时 format!。

/// 预计算的一组标记字符串，在 `ManagedToolStream` 中用于流式解析。
struct StreamMarkers {
    block_open: String,
    block_close: String,
}

/// 请求侧与解析函数用的标记格式器（构建完整的 XML 块）。
struct PromptMarkers {
    prefix: String,
    invoke_open: String,
    invoke_close: String,
    param_open: String,
    param_close: String,
    block_open: String,
    block_close: String,
}

impl PromptMarkers {
    fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            block_open: format!("<|{}|tool_calls>", prefix),
            block_close: format!("</|{}|tool_calls>", prefix),
            invoke_open: format!("<|{}|invoke name=\"", prefix),
            invoke_close: format!("</|{}|invoke>", prefix),
            param_open: format!("<|{}|parameter name=\"", prefix),
            param_close: format!("</|{}|parameter>", prefix),
        }
    }

    fn format_invoke(&self, name: &str, params_inner: &str) -> String {
        format!("{}{}{}{}{}", self.invoke_open, name, "\">", params_inner, self.invoke_close)
    }

    fn format_param(&self, name: &str, cdata_value: &str) -> String {
        format!(
            "{}{}{}{}]]>{}",
            self.param_open, name, "\"><![CDATA[", cdata_value, self.param_close
        )
    }

    fn format_tool_result(&self, tool_call_id: &str, content: &str) -> String {
        format!(
            "<|{}|tool_result tool_call_id=\"{}\"><![CDATA[{}]]></|{}|tool_result>",
            self.prefix, tool_call_id, content, self.prefix
        )
    }

    fn format_block(&self, inner: &str) -> String {
        format!("{}{}{}", self.block_open, inner, self.block_close)
    }

    fn render_prompt(&self, rendered_tools: &str) -> String {
        // 替换模板中的 CHAT2API 为用户指定的 prefix
        TOOL_PROMPT_TEMPLATE
            .replace("{rendered_tools}", rendered_tools)
            .replace("CHAT2API", &self.prefix)
    }
}

// ═══════════════════════════════════════════════════════════════
//  ManagedToolAdapter — 装饰器
// ═══════════════════════════════════════════════════════════════

pub struct ManagedToolAdapter {
    inner: Arc<dyn ProviderAdapter>,
    marker_prefix: String,
}

impl ManagedToolAdapter {
    /// 创建适配器，使用默认的 `CHAT2API` 标记前缀（兼容 Chat2API 协议）。
    pub fn new(inner: Arc<dyn ProviderAdapter>) -> Self {
        Self { inner, marker_prefix: "CHAT2API".to_string() }
    }

    /// 创建适配器，指定自定义 marker prefix。
    /// 格式：`<|{prefix}|tool_calls>...</|{prefix}|tool_calls>`。
    pub fn with_prefix(inner: Arc<dyn ProviderAdapter>, prefix: impl Into<String>) -> Self {
        Self { inner, marker_prefix: prefix.into() }
    }

    fn p(&self) -> PromptMarkers {
        PromptMarkers::new(&self.marker_prefix)
    }

    fn s(&self) -> StreamMarkers {
        StreamMarkers {
            block_open: format!("<|{}|tool_calls>", self.marker_prefix),
            block_close: format!("</|{}|tool_calls>", self.marker_prefix),
        }
    }

    /// 请求侧变换：拔出 tools → 注入系统提示词 → 翻译 tool 历史
    fn transform_request(&self, mut req: ChatRequest) -> (ChatRequest, Option<Vec<ChatTool>>) {
        let tools = req.tools.take();
        if let Some(ref tools) = tools {
            let pm = self.p();
            inject_tool_prompt(&mut req.messages, tools, &pm);
            translate_tool_history(&mut req.messages, &pm);
        }
        (req, tools)
    }
}

#[async_trait]
impl ProviderAdapter for ManagedToolAdapter {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: Arc<ChatRequest>,
    ) -> Result<ChatResponse> {
        let (transformed, tools) = self.transform_request((*request).clone());
        let mut response = self.inner.chat(ctx, Arc::new(transformed)).await?;

        // 非流式响应：从 content 中解析 tool_calls
        if let Some(ref tools) = tools {
            let pm = self.p();
            if let (Some(calls), cleaned) = parse_and_clean_content(&response.content, tools, &pm) {
                response.tool_calls = Some(calls);
                response.content = cleaned;
            }
        }

        Ok(response)
    }

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        cancel_token: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        let (transformed, tools) = self.transform_request(request);
        let inner_stream = self.inner.chat_stream(ctx, transformed, cancel_token);

        let allowed: HashSet<String> = tools
            .as_ref()
            .map(|t| t.iter().map(|ct| ct.function.name.clone()).collect())
            .unwrap_or_default();

        Box::pin(ManagedToolStream::new(inner_stream, allowed, self.s()))
    }

    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        self.inner.list_models(ctx).await
    }

    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse> {
        self.inner.embed(ctx, request).await
    }

    async fn validate_key(&self, ctx: &ProviderRequestContext) -> Result<bool> {
        self.inner.validate_key(ctx).await
    }

    async fn realtime_config(
        &self,
        ctx: &ProviderRequestContext,
        model: &str,
    ) -> Result<axagent_harness::RealtimeProviderConfig> {
        self.inner.realtime_config(ctx, model).await
    }
}

// ═══════════════════════════════════════════════════════════════
//  ManagedToolStream — 流式解析
// ═══════════════════════════════════════════════════════════════

struct ManagedToolStream {
    inner: Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>>,
    buffer: String,
    pending: VecDeque<ChatStreamChunk>,
    allowed_tool_names: HashSet<String>,
    block_open: String,
    block_close: String,
}

impl ManagedToolStream {
    fn new(
        inner: Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>>,
        allowed_tool_names: HashSet<String>,
        markers: StreamMarkers,
    ) -> Self {
        Self {
            inner,
            buffer: String::new(),
            pending: VecDeque::new(),
            allowed_tool_names,
            block_open: markers.block_open,
            block_close: markers.block_close,
        }
    }

    fn pm(&self) -> PromptMarkers {
        // 从 block_open 回推 prefix
        let prefix =
            self.block_open.trim_start_matches("<|").split('|').next().unwrap_or("CHAT2API");
        PromptMarkers::new(prefix)
    }

    /// 扫描 buffer 中已完整的工具调用块，提取为 text/tool_calls chunk。
    fn process_buffer(&mut self) {
        loop {
            let Some(open_pos) = self.buffer.find(&self.block_open) else {
                if !self.buffer.is_empty() {
                    let text = std::mem::take(&mut self.buffer);
                    self.pending.push_back(ChatStreamChunk {
                        content: Some(text),
                        thinking: None,
                        done: false,
                        is_final: None,
                        usage: None,
                        tool_calls: None,
                    });
                }
                break;
            };

            if open_pos > 0 {
                let before = self.buffer[..open_pos].to_string();
                self.pending.push_back(ChatStreamChunk {
                    content: Some(before),
                    thinking: None,
                    done: false,
                    is_final: None,
                    usage: None,
                    tool_calls: None,
                });
            }

            let after_open = &self.buffer[open_pos..];
            if let Some(close_pos) = after_open.find(&self.block_close) {
                let block = &after_open[..close_pos + self.block_close.len()];
                let pm = self.pm();
                if let Ok(calls) = parse_tool_calls_block(block, &self.allowed_tool_names, &pm)
                    && !calls.is_empty()
                {
                    self.pending.push_back(ChatStreamChunk {
                        content: None,
                        thinking: None,
                        done: false,
                        is_final: None,
                        usage: None,
                        tool_calls: Some(calls),
                    });
                }
                let block_end = open_pos + close_pos + self.block_close.len();
                self.buffer = self.buffer[block_end..].to_string();
            } else {
                self.buffer = after_open.to_string();
                break;
            }
        }
    }

    fn flush_text(&mut self) -> Option<ChatStreamChunk> {
        if self.buffer.contains(&self.block_open) {
            self.buffer.clear();
            None
        } else if !self.buffer.is_empty() {
            let text = std::mem::take(&mut self.buffer);
            Some(ChatStreamChunk {
                content: Some(text),
                thinking: None,
                done: false,
                is_final: None,
                usage: None,
                tool_calls: None,
            })
        } else {
            None
        }
    }
}

impl Stream for ManagedToolStream {
    type Item = Result<ChatStreamChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            if let Some(chunk) = this.pending.pop_front() {
                return Poll::Ready(Some(Ok(chunk)));
            }

            match this.inner.as_mut().poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    if let Some(chunk) = this.flush_text() {
                        this.pending.push_back(chunk);
                        continue;
                    }
                    return Poll::Ready(None);
                },
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(Some(Ok(mut chunk))) => {
                    let thinking_text = chunk.thinking.clone();
                    if let Some(ref t) = thinking_text
                        && !t.is_empty()
                    {
                        this.pending.push_back(ChatStreamChunk {
                            content: None,
                            thinking: thinking_text,
                            done: false,
                            is_final: None,
                            usage: None,
                            tool_calls: None,
                        });
                    }
                    chunk.thinking = None;

                    if let Some(content) = chunk.content.take()
                        && !content.is_empty()
                    {
                        this.buffer.push_str(&content);
                    }

                    this.process_buffer();

                    if chunk.done {
                        if let Some(text_chunk) = this.flush_text() {
                            this.pending.push_back(text_chunk);
                        }
                        let has_done = this.pending.iter().any(|c| c.done);
                        if !has_done {
                            this.pending.push_back(ChatStreamChunk {
                                content: None,
                                thinking: None,
                                done: true,
                                is_final: chunk.is_final,
                                usage: chunk.usage,
                                tool_calls: None,
                            });
                        }
                    }
                },
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  请求侧变换函数
// ═══════════════════════════════════════════════════════════════

fn inject_tool_prompt(messages: &mut Vec<ChatMessage>, tools: &[ChatTool], pm: &PromptMarkers) {
    let rendered = render_tool_list(tools);
    let prompt = pm.render_prompt(&rendered);

    for msg in messages.iter_mut() {
        if msg.role == "system"
            && let ChatContent::Text(ref mut text) = msg.content
        {
            text.push_str(&prompt);
            return;
        }
    }
    messages.insert(
        0,
        ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text(prompt),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        },
    );
}

fn render_tool_list(tools: &[ChatTool]) -> String {
    tools
        .iter()
        .map(|t| {
            let params = t
                .function
                .parameters
                .as_ref()
                .and_then(|p| serde_json::to_string(p).ok())
                .unwrap_or_default();
            format!(
                "Tool `{}`: {}\n参数 JSON Schema: {}",
                t.function.name,
                t.function.description.as_deref().unwrap_or(""),
                params
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn translate_tool_history(messages: &mut [ChatMessage], pm: &PromptMarkers) {
    for msg in messages.iter_mut() {
        if msg.role == "tool" {
            let content = crate::extract_text_content(&msg.content);
            let tool_call_id = msg.tool_call_id.as_deref().unwrap_or("call_0");
            let result_block = pm.format_tool_result(tool_call_id, &content);
            msg.role = "user".to_string();
            msg.content = ChatContent::Text(result_block);
            msg.tool_calls = None;
            msg.tool_call_id = None;
            msg.thinking = None;
        } else if msg.role == "assistant"
            && let Some(ref calls) = msg.tool_calls
            && !calls.is_empty()
        {
            let xml = format_tool_calls_xml(calls, pm);
            let text = crate::extract_text_content(&msg.content);
            let new_content = if text.is_empty() {
                xml
            } else {
                format!("{text}\n{xml}")
            };
            msg.content = ChatContent::Text(new_content);
            msg.tool_calls = None;
        }
    }
}

fn format_tool_calls_xml(calls: &[ToolCall], pm: &PromptMarkers) -> String {
    let inner: String = calls
        .iter()
        .map(|call| {
            let params = &call.function.arguments;
            let param_xml = if let Ok(json) = serde_json::from_str::<serde_json::Value>(params) {
                match json {
                    serde_json::Value::Object(map) => map
                        .iter()
                        .map(|(k, v)| {
                            let val_str = serde_json::to_string(v).unwrap_or_default();
                            let inner_val = if let Some(s) = v.as_str() {
                                s.to_string()
                            } else {
                                val_str
                            };
                            pm.format_param(k, &inner_val)
                        })
                        .collect::<Vec<_>>()
                        .join(""),
                    _ => pm.format_param("value", params),
                }
            } else {
                pm.format_param("value", params)
            };
            pm.format_invoke(&call.function.name, &param_xml)
        })
        .collect();
    pm.format_block(&inner)
}

// ═══════════════════════════════════════════════════════════════
//  响应侧解析
// ═══════════════════════════════════════════════════════════════

fn parse_and_clean_content(
    content: &str,
    tools: &[ChatTool],
    pm: &PromptMarkers,
) -> (Option<Vec<ToolCall>>, String) {
    let allowed: HashSet<String> = tools.iter().map(|t| t.function.name.clone()).collect();

    if let Some(block_start) = content.find(&pm.block_open) {
        let after_open = &content[block_start..];
        if let Some(close_pos) = after_open.find(&pm.block_close) {
            let block = &after_open[..close_pos + pm.block_close.len()];
            let block_end = block_start + close_pos + pm.block_close.len();

            let cleaned = format!("{}{}", &content[..block_start], &content[block_end..]);
            let trimmed = cleaned.trim().to_string();

            match parse_tool_calls_block(block, &allowed, pm) {
                Ok(calls) if !calls.is_empty() => (Some(calls), trimmed),
                _ => (None, content.to_string()),
            }
        } else {
            (None, content.to_string())
        }
    } else {
        (None, content.to_string())
    }
}

fn parse_tool_calls_block(
    block: &str,
    allowed: &HashSet<String>,
    pm: &PromptMarkers,
) -> Result<Vec<ToolCall>> {
    let mut calls = Vec::new();
    let mut pos = 0;
    let mut call_index = 0usize;

    while let Some(invoke_start) = block[pos..].find(&pm.invoke_open) {
        let abs_start = pos + invoke_start;
        let after_tag = &block[abs_start + pm.invoke_open.len()..];

        let name_end = after_tag.find('"').ok_or_else(|| {
            AxAgentError::Provider("Malformed invoke: missing closing quote on name".into())
        })?;
        let name = after_tag[..name_end].to_string();

        let body_from = &after_tag[name_end..];
        let body_open_end = body_from.find('>').ok_or_else(|| {
            AxAgentError::Provider("Malformed invoke: missing > after name".into())
        })?;
        let body = &body_from[body_open_end + 1..];

        let invoke_close_pos = body
            .find(&pm.invoke_close)
            .ok_or_else(|| AxAgentError::Provider("Malformed invoke: missing close tag".into()))?;

        let inner = &body[..invoke_close_pos];

        if !allowed.is_empty() && !allowed.contains(&name) {
            tracing::warn!(
                "[ManagedToolAdapter] Dropping tool call '{}': not in allowed list",
                name
            );
            pos = abs_start
                + pm.invoke_open.len()
                + name_end
                + 1
                + body_open_end
                + 1
                + invoke_close_pos
                + pm.invoke_close.len();
            continue;
        }

        let args = parse_params(inner, pm);
        let arguments = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());

        calls.push(ToolCall {
            id: format!("call_{call_index}"),
            call_type: "function".to_string(),
            function: ToolCallFunction { name, arguments },
        });
        call_index += 1;

        pos = abs_start
            + pm.invoke_open.len()
            + name_end
            + 1
            + body_open_end
            + 1
            + invoke_close_pos
            + pm.invoke_close.len();
    }

    Ok(calls)
}

fn parse_params(inner: &str, pm: &PromptMarkers) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    let mut pos = 0;

    while let Some(param_start) = inner[pos..].find(&pm.param_open) {
        let abs_start = pos + param_start;
        let after_tag = &inner[abs_start + pm.param_open.len()..];

        let name_end = after_tag.find('"').unwrap_or(0);
        if name_end == 0 {
            break;
        }
        let param_name = after_tag[..name_end].to_string();

        let content_open = after_tag[name_end..].find('>');
        let Some(content_from) = content_open else { break };
        let body_start = name_end + content_from + 1;

        let close_pos = after_tag[body_start..].find(&pm.param_close).unwrap_or(usize::MAX);
        if close_pos == usize::MAX {
            break;
        }
        let raw_value = &after_tag[body_start..body_start + close_pos];

        let unwrapped = unwrap_cdata(raw_value);

        let value = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&unwrapped) {
            v
        } else {
            serde_json::Value::String(unwrapped)
        };

        map.insert(param_name, value);

        let advance = body_start + close_pos + pm.param_close.len();
        pos = abs_start + pm.param_open.len() + advance;
    }

    serde_json::Value::Object(map)
}

fn unwrap_cdata(s: &str) -> String {
    if s.starts_with("<![CDATA[") && s.ends_with("]]>") {
        s[9..s.len() - 3].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_pm() -> PromptMarkers {
        PromptMarkers::new("CHAT2API")
    }

    fn custom_pm(prefix: &str) -> PromptMarkers {
        PromptMarkers::new(prefix)
    }

    #[test]
    fn test_unwrap_cdata() {
        assert_eq!(unwrap_cdata("<![CDATA[hello]]>"), "hello");
        assert_eq!(unwrap_cdata("hello"), "hello");
        assert_eq!(unwrap_cdata("<![CDATA[]]>"), "");
    }

    #[test]
    fn test_parse_tool_calls_block_simple() {
        let pm = default_pm();
        let block = format!(
            "{}<|CHAT2API|invoke name=\"read_file\">\
             <|CHAT2API|parameter name=\"path\">\
             <![CDATA[/tmp/test.txt]]></|CHAT2API|parameter>\
             </|CHAT2API|invoke>{}",
            pm.block_open, pm.block_close
        );
        let allowed: HashSet<String> = ["read_file"].iter().map(|s| s.to_string()).collect();
        let calls = parse_tool_calls_block(&block, &allowed, &pm).expect("测试应成功");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "read_file");
        assert_eq!(calls[0].id, "call_0");
        let args: serde_json::Value =
            serde_json::from_str(&calls[0].function.arguments).expect("测试：JSON反序列化应成功");
        assert_eq!(args["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_parse_tool_calls_block_drop_unauthorized() {
        let pm = default_pm();
        let block = format!(
            "{}<|CHAT2API|invoke name=\"bad_tool\">\
             <|CHAT2API|parameter name=\"x\"><![CDATA[1]]></|CHAT2API|parameter>\
             </|CHAT2API|invoke>{}",
            pm.block_open, pm.block_close
        );
        let allowed: HashSet<String> = ["good_tool"].iter().map(|s| s.to_string()).collect();
        let calls = parse_tool_calls_block(&block, &allowed, &pm).expect("测试应成功");
        assert!(calls.is_empty());
    }

    #[test]
    fn test_format_and_parse_roundtrip() {
        let pm = default_pm();
        let calls = vec![ToolCall {
            id: "call_0".to_string(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: "search".to_string(),
                arguments: r#"{"q":"hello"}"#.to_string(),
            },
        }];
        let xml = format_tool_calls_xml(&calls, &pm);
        assert!(xml.contains("search"));
        assert!(xml.contains("CDATA"));

        let allowed: HashSet<String> = ["search"].iter().map(|s| s.to_string()).collect();
        let parsed = parse_tool_calls_block(&xml, &allowed, &pm).expect("测试应成功");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].function.name, "search");
        let args: serde_json::Value =
            serde_json::from_str(&parsed[0].function.arguments).expect("测试：JSON反序列化应成功");
        assert_eq!(args["q"], "hello");
    }

    #[test]
    fn test_render_tool_list() {
        let tools = vec![ChatTool {
            r#type: "function".to_string(),
            function: ChatToolFunction {
                name: "my_tool".to_string(),
                description: Some("A tool for testing".to_string()),
                parameters: Some(
                    serde_json::json!({"type":"object","properties":{"x":{"type":"string"}}}),
                ),
            },
        }];
        let rendered = render_tool_list(&tools);
        assert!(rendered.contains("my_tool"));
        assert!(rendered.contains("A tool for testing"));
        assert!(rendered.contains("x"));
    }

    #[test]
    fn test_parse_params_multiple() {
        let pm = default_pm();
        // Actually simpler: construct directly
        let inner = r#"<|CHAT2API|parameter name="a"><![CDATA[1]]></|CHAT2API|parameter>\
               <|CHAT2API|parameter name="b"><![CDATA["hello"]]></|CHAT2API|parameter>"#;
        let value = parse_params(inner, &pm);
        assert_eq!(value["a"], 1);
        assert_eq!(value["b"], "hello");
    }

    #[test]
    fn test_parse_params_cdata_json_object() {
        let pm = default_pm();
        let inner = r#"<|CHAT2API|parameter name="config"><![CDATA[{"key":"value"}]]></|CHAT2API|parameter>"#;
        let value = parse_params(inner, &pm);
        assert_eq!(value["config"]["key"], "value");
    }

    #[test]
    fn test_transform_request_no_tools_unchanged() {
        let adapter = ManagedToolAdapter::new(Arc::new(TestAdapter));
        let req = ChatRequest {
            model: "test".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: ChatContent::Text("hello".into()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            }],
            stream: false,
            temperature: None,
            top_p: None,
            max_tokens: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
            response_format: None,
        };
        let (transformed, tools) = adapter.transform_request(req);
        assert!(tools.is_none());
        assert_eq!(transformed.messages.len(), 1);
    }

    #[test]
    fn test_transform_request_injects_prompt() {
        let adapter = ManagedToolAdapter::new(Arc::new(TestAdapter));
        let tools = vec![ChatTool {
            r#type: "function".into(),
            function: ChatToolFunction {
                name: "test_tool".into(),
                description: Some("A test".into()),
                parameters: None,
            },
        }];
        let req = ChatRequest {
            model: "test".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: ChatContent::Text("hello".into()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            }],
            stream: false,
            temperature: None,
            top_p: None,
            max_tokens: None,
            tools: Some(tools),
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
            response_format: None,
        };
        let (transformed, tools_out) = adapter.transform_request(req);
        assert!(tools_out.is_some());
        assert!(
            transformed.messages[0].role == "system"
                || transformed.messages.iter().any(|m| m.role == "system"
                    && crate::extract_text_content(&m.content).contains("test_tool"))
        );
    }

    #[test]
    fn test_translate_tool_history_tool_result() {
        let pm = default_pm();
        let mut msgs = vec![ChatMessage {
            role: "tool".into(),
            content: ChatContent::Text("result data".into()),
            tool_calls: None,
            tool_call_id: Some("call_1".into()),
            thinking: None,
        }];
        translate_tool_history(&mut msgs, &pm);
        assert_eq!(msgs[0].role, "user");
        assert!(crate::extract_text_content(&msgs[0].content).contains("tool_result"));
        assert!(crate::extract_text_content(&msgs[0].content).contains("result data"));
    }

    #[test]
    fn test_custom_prefix_parse() {
        let pm = custom_pm("MYGATE");
        let block = "<|MYGATE|tool_calls><|MYGATE|invoke name=\"my_tool\">\
             <|MYGATE|parameter name=\"x\"><![CDATA[42]]></|MYGATE|parameter>\
             </|MYGATE|invoke></|MYGATE|tool_calls>";
        let allowed: HashSet<String> = ["my_tool"].iter().map(|s| s.to_string()).collect();
        let calls = parse_tool_calls_block(block, &allowed, &pm).expect("测试应成功");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "my_tool");
    }

    #[test]
    fn test_custom_prefix_prompt() {
        let pm = custom_pm("MYGATE");
        let prompt = pm.render_prompt("Tool `foo`: bar");
        // prompt should use MYGATE markers instead of CHAT2API
        assert!(prompt.contains("|MYGATE|tool_calls>"));
        assert!(!prompt.contains("CHAT2API"));
    }

    #[test]
    fn test_custom_prefix_format_roundtrip() {
        let pm_custom = custom_pm("MYGATE");

        let calls = vec![ToolCall {
            id: "call_0".to_string(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: "my_tool".to_string(),
                arguments: r#"{"x":1}"#.to_string(),
            },
        }];
        let xml = format_tool_calls_xml(&calls, &pm_custom);
        assert!(xml.contains("|MYGATE|"));
        assert!(!xml.contains("CHAT2API"));

        let allowed: HashSet<String> = ["my_tool"].iter().map(|s| s.to_string()).collect();
        let parsed = parse_tool_calls_block(&xml, &allowed, &pm_custom).expect("测试应成功");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].function.name, "my_tool");
    }

    // ── Test double ──
    struct TestAdapter;

    #[async_trait]
    impl ProviderAdapter for TestAdapter {
        async fn chat(
            &self,
            _ctx: &ProviderRequestContext,
            _req: Arc<ChatRequest>,
        ) -> Result<ChatResponse> {
            Ok(ChatResponse::default())
        }

        fn chat_stream(
            &self,
            _ctx: &ProviderRequestContext,
            _req: ChatRequest,
            _cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
        ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
            Box::pin(futures::stream::empty())
        }

        async fn list_models(&self, _ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
            Ok(vec![])
        }

        async fn embed(
            &self,
            _ctx: &ProviderRequestContext,
            _req: EmbedRequest,
        ) -> Result<EmbedResponse> {
            Ok(EmbedResponse { embeddings: vec![], dimensions: 0 })
        }
    }
}
