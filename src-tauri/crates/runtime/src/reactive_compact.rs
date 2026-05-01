//! 响应式压缩系统
//!
//! 当 API 返回上下文溢出错误（prompt_too_long / context_length_exceeded）时，
//! 触发紧急压缩并重试，而非直接返回硬错误。
//! 移植自 claude-code-main 的 reactiveCompact.ts。

use crate::compact::{compact_session, CompactionConfig, CompactionResult};
use crate::session::Session;

/// 响应式压缩尝试的结果。
#[derive(Debug, Clone)]
pub enum ReactiveCompactResult {
    /// 压缩成功，附带新的压缩结果
    Compacted {
        result: CompactionResult,
        trigger: ReactiveTrigger,
    },
    /// 压缩失败，附带失败原因
    Failed {
        reason: String,
    },
    /// 压缩被跳过（不适用或已禁用）
    Skipped,
}

/// 触发响应式压缩的事件类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactiveTrigger {
    /// API 返回 prompt_too_long / context_length_exceeded
    PromptTooLong,
    /// API 返回媒体/大小错误
    MediaSizeError,
    /// 手动触发
    Manual,
}

impl std::fmt::Display for ReactiveTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReactiveTrigger::PromptTooLong => write!(f, "prompt_too_long"),
            ReactiveTrigger::MediaSizeError => write!(f, "media_size_error"),
            ReactiveTrigger::Manual => write!(f, "manual"),
        }
    }
}

/// 响应式压缩使用的更激进的配置。
///
/// 相比默认配置，响应式压缩：
/// - 保留更少的最近消息（6 条 vs 12 条）
/// - 设置更低的 token 阈值
const REACTIVE_PRESERVE_RECENT: usize = 6;
const REACTIVE_MAX_TOKENS: usize = 50_000;

/// 尝试执行响应式压缩。
///
/// 当 API 返回上下文溢出错误时调用此函数。它使用更激进的压缩参数，
/// 尽可能释放上下文窗口空间以允许重试。
///
/// # 参数
/// - `session`: 要压缩的会话
/// - `base_config`: 基础压缩配置（用于回退）
/// - `trigger`: 触发压缩的事件类型
///
/// # 返回
/// - `Compacted`: 压缩成功，返回新会话
/// - `Failed`: 压缩失败（例如会话太小无法压缩）
/// - `Skipped`: 无法执行压缩（例如没有可压缩的消息）
pub fn try_reactive_compact(
    session: &Session,
    base_config: CompactionConfig,
    trigger: ReactiveTrigger,
) -> ReactiveCompactResult {
    // 如果会话消息太少，无法压缩
    if session.messages.len() <= REACTIVE_PRESERVE_RECENT {
        return ReactiveCompactResult::Skipped;
    }

    // 构建更激进的压缩配置
    let reactive_config = CompactionConfig {
        preserve_recent_messages: REACTIVE_PRESERVE_RECENT,
        max_estimated_tokens: REACTIVE_MAX_TOKENS,
        ..base_config
    };

    let result = compact_session(session, reactive_config);

    // 检查压缩是否有实际效果
    if result.removed_message_count == 0 {
        // 用更激进的参数再试一次
        let aggressive_config = CompactionConfig {
            preserve_recent_messages: 2,
            max_estimated_tokens: 10_000,
            ..base_config
        };
        let retry_result = compact_session(session, aggressive_config);

        if retry_result.removed_message_count == 0 {
            return ReactiveCompactResult::Failed {
                reason: "无法压缩：没有可移除的消息".to_string(),
            };
        }

        return ReactiveCompactResult::Compacted {
            result: retry_result,
            trigger,
        };
    }

    ReactiveCompactResult::Compacted {
        result,
        trigger,
    }
}

/// 从 API 错误消息中检测是否需要响应式压缩。
///
/// 检查错误文本中是否包含上下文溢出相关的关键词。
pub fn is_context_overflow_error(error_text: &str) -> bool {
    let lowered = error_text.to_lowercase();
    lowered.contains("prompt_too_long")
        || lowered.contains("context_length_exceeded")
        || lowered.contains("context length exceeded")
        || lowered.contains("maximum context length")
        || lowered.contains("too many tokens")
        || lowered.contains("token limit exceeded")
        || lowered.contains("input length too long")
}

/// 从 API 错误消息中检测是否为媒体/大小错误。
pub fn is_media_size_error(error_text: &str) -> bool {
    let lowered = error_text.to_lowercase();
    lowered.contains("image_too_large")
        || lowered.contains("media too large")
        || lowered.contains("file too large")
        || lowered.contains("attachment too large")
}

/// 根据错误文本推断触发类型。
pub fn classify_trigger(error_text: &str) -> Option<ReactiveTrigger> {
    if is_context_overflow_error(error_text) {
        Some(ReactiveTrigger::PromptTooLong)
    } else if is_media_size_error(error_text) {
        Some(ReactiveTrigger::MediaSizeError)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{ContentBlock, ConversationMessage, Session};

    fn make_large_session(message_count: usize) -> Session {
        let mut session = Session::new();
        for i in 0..message_count {
            // 创建足够大的消息（~2500 tokens per message）以确保超过压缩阈值
            let text = format!("message {} {}", i, "x".repeat(10_000));
            if i % 2 == 0 {
                session
                    .push_message(ConversationMessage::user_text(&text))
                    .unwrap();
            } else {
                session.push_message(ConversationMessage::assistant(vec![
                    ContentBlock::Text { text },
                ]))
                .unwrap();
            }
        }
        session
    }

    #[test]
    fn test_skips_small_session() {
        let session = make_large_session(4);
        let result = try_reactive_compact(
            &session,
            CompactionConfig::default(),
            ReactiveTrigger::PromptTooLong,
        );
        assert!(matches!(result, ReactiveCompactResult::Skipped));
    }

    #[test]
    fn test_compacts_large_session() {
        let session = make_large_session(30);
        let result = try_reactive_compact(
            &session,
            CompactionConfig::default(),
            ReactiveTrigger::PromptTooLong,
        );
        assert!(matches!(
            result,
            ReactiveCompactResult::Compacted { .. }
        ));
        if let ReactiveCompactResult::Compacted { result, .. } = result {
            assert!(result.removed_message_count > 0);
        }
    }

    #[test]
    fn test_context_overflow_detection() {
        assert!(is_context_overflow_error("prompt_too_long: input exceeds maximum length"));
        assert!(is_context_overflow_error("context_length_exceeded error"));
        assert!(is_context_overflow_error("exceeded maximum context length"));
        assert!(!is_context_overflow_error("network timeout"));
    }

    #[test]
    fn test_media_size_detection() {
        assert!(is_media_size_error("image_too_large: exceeds limit"));
        assert!(is_media_size_error("media too large for model"));
        assert!(!is_media_size_error("context overflow"));
    }

    #[test]
    fn test_classify_trigger() {
        assert_eq!(
            classify_trigger("prompt_too_long"),
            Some(ReactiveTrigger::PromptTooLong)
        );
        assert_eq!(
            classify_trigger("image_too_large"),
            Some(ReactiveTrigger::MediaSizeError)
        );
        assert_eq!(classify_trigger("unknown error"), None);
    }
}
