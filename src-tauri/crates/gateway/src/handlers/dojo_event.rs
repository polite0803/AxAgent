// SPDX-License-Identifier: AGPL-3.0-only
//! G7 dojo.v2 Typed Event 协议
//!
//! 在 OpenAI 兼容 SSE chunk 上附加 `dojo_event` 字段，为客户端提供
//! 结构化事件元数据，便于前端做实时状态管理（如 phase 切换、tool call
//! 进度、token 用量等）。
//!
//! ## 事件类型（12 种 typed event）
//!
//! | type             | 触发时机                                       |
//! |------------------|-----------------------------------------------|
//! | `phase`          | 阶段切换（如 planning → executing → summarizing） |
//! | `delta`          | 增量文本输出                                   |
//! | `think_start`    | 推理开始                                       |
//! | `think_delta`    | 推理增量                                       |
//! | `think_end`      | 推理结束                                       |
//! | `retry`          | 重试 provider 调用                             |
//! | `tool_start`     | 工具调用开始                                   |
//! | `tool_result`    | 工具调用结果                                   |
//! | `eval_hint`      | 评估提示（前端可选展示）                        |
//! | `token_usage`    | token 用量推送                                 |
//! | `context_compacted` | 上下文压缩通知                              |
//! | `done`           | 流结束                                         |
//! | `error`          | 错误事件                                       |
//!
//! ## 元数据
//!
//! 每个事件携带 `(run_id, seq, call_id)` 三元组：
//! - `run_id`：单次 chat completion 请求的唯一标识（ULID）
//! - `seq`：事件序号，从 0 单调递增
//! - `call_id`：工具调用 ID（仅 tool_* 事件需要，其他为 null）

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// dojo.v2 typed event 类型标签
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DojoEventType {
    Phase,
    Delta,
    ThinkStart,
    ThinkDelta,
    ThinkEnd,
    Retry,
    ToolStart,
    ToolResult,
    EvalHint,
    TokenUsage,
    ContextCompacted,
    Done,
    Error,
}

impl DojoEventType {
    /// 序列化为字符串（用于 JSON `"type"` 字段）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Phase => "phase",
            Self::Delta => "delta",
            Self::ThinkStart => "think_start",
            Self::ThinkDelta => "think_delta",
            Self::ThinkEnd => "think_end",
            Self::Retry => "retry",
            Self::ToolStart => "tool_start",
            Self::ToolResult => "tool_result",
            Self::EvalHint => "eval_hint",
            Self::TokenUsage => "token_usage",
            Self::ContextCompacted => "context_compacted",
            Self::Done => "done",
            Self::Error => "error",
        }
    }
}

/// dojo_event 载荷：附加在 SSE chunk 上的结构化事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DojoEvent {
    /// 事件类型
    #[serde(rename = "type")]
    pub event_type: String,
    /// 单次 chat completion 请求的唯一标识
    pub run_id: String,
    /// 事件序号（从 0 单调递增）
    pub seq: u64,
    /// 工具调用 ID（仅 tool_* 事件需要，其他为 null）
    pub call_id: Option<String>,
    /// 事件特有数据（如 delta 文本、tool 名称、token usage 等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl DojoEvent {
    /// 创建一个新事件，自动填充 run_id 和 seq
    pub fn new(event_type: DojoEventType, run_id: &str, seq: u64) -> Self {
        Self {
            event_type: event_type.as_str().to_string(),
            run_id: run_id.to_string(),
            seq,
            call_id: None,
            data: None,
        }
    }

    /// 设置 call_id（用于 tool_* 事件）
    pub fn with_call_id(mut self, call_id: impl Into<String>) -> Self {
        self.call_id = Some(call_id.into());
        self
    }

    /// 设置 data 字段
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// 序列化为 JSON Value（用于附加到 SSE chunk）
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({ "type": "error" }))
    }
}

/// 维护单次 stream 的 dojo_event 状态
#[derive(Debug, Clone)]
pub struct DojoEventSequencer {
    /// 当前 run_id
    run_id: String,
    /// 下一个事件序号
    next_seq: u64,
    /// 是否处于推理段（用于区分 think_start/think_delta/think_end）
    in_thinking: bool,
}

impl DojoEventSequencer {
    /// 创建新的 sequencer，run_id 使用传入值
    pub fn new(run_id: impl Into<String>) -> Self {
        Self { run_id: run_id.into(), next_seq: 0, in_thinking: false }
    }

    /// 生成 run_id（基于时间戳的简单版本，避免引入 ulid 依赖）
    pub fn generate_run_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        format!("run_{nanos:x}")
    }

    /// 当前 run_id
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// 分配下一个序号
    fn next_seq(&mut self) -> u64 {
        let s = self.next_seq;
        self.next_seq += 1;
        s
    }

    /// 根据流式 chunk 内容生成对应的 dojo_event 列表
    ///
    /// 一个 chunk 可能触发多个事件（如同时包含 thinking 和 content，
    /// 会产生 think_delta + delta 两个事件）。
    pub fn events_for_chunk(
        &mut self,
        chunk: &axagent_harness::types::ChatStreamChunk,
    ) -> Vec<DojoEvent> {
        let mut events = Vec::new();
        let run_id = self.run_id.clone();

        // 推理段处理
        if let Some(reasoning) = chunk.thinking.as_deref().filter(|v| !v.is_empty()) {
            if !self.in_thinking {
                self.in_thinking = true;
                let seq = self.next_seq();
                events.push(DojoEvent::new(DojoEventType::ThinkStart, &run_id, seq));
            }
            let seq = self.next_seq();
            events.push(
                DojoEvent::new(DojoEventType::ThinkDelta, &run_id, seq)
                    .with_data(json!({ "text": reasoning })),
            );
        } else if self.in_thinking {
            // 之前在推理段，当前 chunk 无 thinking → 推理结束
            self.in_thinking = false;
            let seq = self.next_seq();
            events.push(DojoEvent::new(DojoEventType::ThinkEnd, &run_id, seq));
        }

        // 内容增量
        if let Some(content) = chunk.content.as_deref().filter(|v| !v.is_empty()) {
            let seq = self.next_seq();
            events.push(
                DojoEvent::new(DojoEventType::Delta, &run_id, seq)
                    .with_data(json!({ "text": content })),
            );
        }

        // 工具调用
        if let Some(tool_calls) = chunk.tool_calls.as_ref() {
            for tc in tool_calls {
                let seq = self.next_seq();
                events.push(
                    DojoEvent::new(DojoEventType::ToolStart, &run_id, seq)
                        .with_call_id(&tc.id)
                        .with_data(json!({
                            "name": tc.function.name,
                            "arguments": tc.function.arguments,
                        })),
                );
            }
        }

        // token 用量（在 done 之前的 usage chunk）
        if let Some(usage) = chunk.usage.as_ref() {
            let seq = self.next_seq();
            events.push(DojoEvent::new(DojoEventType::TokenUsage, &run_id, seq).with_data(json!({
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
                "cache_read_input_tokens": usage.cache_read_input_tokens,
                "cache_creation_input_tokens": usage.cache_creation_input_tokens,
            })));
        }

        events
    }

    /// 生成流结束事件（done）
    ///
    /// 注意：调用方应在调用此方法前先调用 `close_thinking_if_open()`
    /// 以确保推理段被正确关闭（如果存在）。
    pub fn done_event(&mut self) -> DojoEvent {
        let run_id = self.run_id.clone();
        let seq = self.next_seq();
        DojoEvent::new(DojoEventType::Done, &run_id, seq)
    }

    /// 生成错误事件
    pub fn error_event(&mut self, message: impl Into<String>) -> DojoEvent {
        let run_id = self.run_id.clone();
        let seq = self.next_seq();
        DojoEvent::new(DojoEventType::Error, &run_id, seq)
            .with_data(json!({ "message": message.into() }))
    }

    /// 如果当前仍在推理段，关闭它并返回 think_end 事件
    pub fn close_thinking_if_open(&mut self) -> Option<DojoEvent> {
        if self.in_thinking {
            self.in_thinking = false;
            let run_id = self.run_id.clone();
            let seq = self.next_seq();
            Some(DojoEvent::new(DojoEventType::ThinkEnd, &run_id, seq))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::types::{ChatStreamChunk, ToolCall, ToolCallFunction};

    #[test]
    fn test_event_type_serialization() {
        assert_eq!(DojoEventType::Phase.as_str(), "phase");
        assert_eq!(DojoEventType::ThinkStart.as_str(), "think_start");
        assert_eq!(DojoEventType::ToolResult.as_str(), "tool_result");
        assert_eq!(DojoEventType::ContextCompacted.as_str(), "context_compacted");
    }

    #[test]
    fn test_sequencer_basic_delta() {
        let mut seq = DojoEventSequencer::new("run_test");
        let chunk = ChatStreamChunk {
            content: Some("hello".to_string()),
            thinking: None,
            done: false,
            is_final: None,
            usage: None,
            tool_calls: None,
        };
        let events = seq.events_for_chunk(&chunk);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "delta");
        assert_eq!(events[0].seq, 0);
        assert_eq!(events[0].run_id, "run_test");
    }

    #[test]
    fn test_sequencer_thinking_lifecycle() {
        let mut seq = DojoEventSequencer::new("run_test");

        // 第一个 chunk 有 thinking → think_start + think_delta
        let chunk1 = ChatStreamChunk {
            content: None,
            thinking: Some("reasoning".to_string()),
            done: false,
            is_final: None,
            usage: None,
            tool_calls: None,
        };
        let events1 = seq.events_for_chunk(&chunk1);
        assert_eq!(events1.len(), 2);
        assert_eq!(events1[0].event_type, "think_start");
        assert_eq!(events1[1].event_type, "think_delta");

        // 第二个 chunk 只有 content → think_end + delta
        let chunk2 = ChatStreamChunk {
            content: Some("answer".to_string()),
            thinking: None,
            done: false,
            is_final: None,
            usage: None,
            tool_calls: None,
        };
        let events2 = seq.events_for_chunk(&chunk2);
        assert_eq!(events2.len(), 2);
        assert_eq!(events2[0].event_type, "think_end");
        assert_eq!(events2[1].event_type, "delta");
    }

    #[test]
    fn test_sequencer_tool_call() {
        let mut seq = DojoEventSequencer::new("run_test");
        let chunk = ChatStreamChunk {
            content: None,
            thinking: None,
            done: false,
            is_final: None,
            usage: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_abc".to_string(),
                call_type: "function".to_string(),
                function: ToolCallFunction {
                    name: "get_stock_quote".to_string(),
                    arguments: "{\"stock_code\":\"600519\"}".to_string(),
                },
            }]),
        };
        let events = seq.events_for_chunk(&chunk);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "tool_start");
        assert_eq!(events[0].call_id.as_deref(), Some("call_abc"));
    }

    #[test]
    fn test_sequencer_done() {
        let mut seq = DojoEventSequencer::new("run_test");
        let done = seq.done_event();
        assert_eq!(done.event_type, "done");
        assert_eq!(done.run_id, "run_test");
    }

    #[test]
    fn test_close_thinking_if_open() {
        let mut seq = DojoEventSequencer::new("run_test");

        // 打开推理段
        let chunk = ChatStreamChunk {
            content: None,
            thinking: Some("thinking...".to_string()),
            done: false,
            is_final: None,
            usage: None,
            tool_calls: None,
        };
        let _ = seq.events_for_chunk(&chunk);

        // 关闭推理段
        let close_event = seq.close_thinking_if_open();
        assert!(close_event.is_some());
        assert_eq!(close_event.unwrap().event_type, "think_end");

        // 再次关闭应返回 None
        assert!(seq.close_thinking_if_open().is_none());
    }

    #[test]
    fn test_generate_run_id_unique() {
        let id1 = DojoEventSequencer::generate_run_id();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = DojoEventSequencer::generate_run_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("run_"));
    }
}
