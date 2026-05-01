//! 多层压缩阈值系统
//!
//! 提供四层渐进式阈值管理，替代简单的二元压缩判断：
//! 1. Warning — 警告用户上下文即将用尽
//! 2. AutoCompact — 自动触发压缩
//! 3. Error — 阻止新用户消息，强制压缩
//! 4. BlockingLimit — 硬限制，即使压缩也无法恢复
//!
//! 同时包含熔断器，防止连续压缩失败导致无限重试。
//! 移植自 claude-code-main 的 compact 阈值管理。

use crate::compact::{estimate_session_tokens, CompactionConfig};
use crate::session::Session;

// ---------------------------------------------------------------------------
// 阈值常量（token 数）
// ---------------------------------------------------------------------------

/// 自动压缩阈值与有效上下文窗口之间的缓冲 token 数
pub const AUTOCOMPACT_BUFFER_TOKENS: u64 = 13_000;
/// 警告阈值与自动压缩阈值之间的缓冲
pub const WARNING_THRESHOLD_BUFFER_TOKENS: u64 = 20_000;
/// 错误/阻止阈值与有效窗口之间的缓冲
pub const ERROR_THRESHOLD_BUFFER_TOKENS: u64 = 20_000;
/// 手动压缩缓冲（硬阻止前最小的窗口空间）
pub const MANUAL_COMPACT_BUFFER_TOKENS: u64 = 3_000;

/// 熔断器触发前允许的最大连续自动压缩失败次数
pub const MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES: u32 = 3;

// ---------------------------------------------------------------------------
// 阈值状态
// ---------------------------------------------------------------------------

/// 根据当前 token 使用量计算的多层阈值状态。
#[derive(Debug, Clone, Copy)]
pub struct CompactThresholdState {
    /// 有效上下文窗口中剩余百分比
    pub percent_left: u32,
    /// 是否超过警告阈值（应显示黄色警告）
    pub is_above_warning_threshold: bool,
    /// 是否超过错误阈值（应阻止新用户消息直到压缩完成）
    pub is_above_error_threshold: bool,
    /// 是否超过自动压缩阈值（应自动触发压缩）
    pub is_above_auto_compact_threshold: bool,
    /// 是否达到硬阻止限制（即使压缩也可能无法恢复）
    pub is_at_blocking_limit: bool,
    /// 当前估算 token 数
    pub estimated_tokens: u64,
    /// 有效上下文窗口大小
    pub effective_window: u64,
}

impl CompactThresholdState {
    /// 根据当前会话和有效上下文窗口计算阈值状态。
    ///
    /// # 参数
    /// - `session`: 当前会话
    /// - `effective_window`: 模型的有效上下文窗口大小（token 数）
    pub fn compute(session: &Session, effective_window: u64) -> Self {
        let estimated = estimate_session_tokens(session) as u64;

        let auto_compact_threshold = effective_window.saturating_sub(AUTOCOMPACT_BUFFER_TOKENS);
        let warning_threshold =
            auto_compact_threshold.saturating_sub(WARNING_THRESHOLD_BUFFER_TOKENS);
        let error_threshold = effective_window.saturating_sub(ERROR_THRESHOLD_BUFFER_TOKENS);
        let blocking_limit = effective_window.saturating_sub(MANUAL_COMPACT_BUFFER_TOKENS);

        let percent_left = if effective_window > 0 {
            let used_pct = ((estimated as f64 / effective_window as f64) * 100.0).round() as u32;
            100u32.saturating_sub(used_pct)
        } else {
            0
        };

        Self {
            percent_left,
            is_above_warning_threshold: estimated >= warning_threshold,
            is_above_auto_compact_threshold: estimated >= auto_compact_threshold,
            is_above_error_threshold: estimated >= error_threshold,
            is_at_blocking_limit: estimated >= blocking_limit,
            estimated_tokens: estimated,
            effective_window,
        }
    }

    /// 是否需要立即执行压缩。
    pub fn needs_compaction(&self) -> bool {
        self.is_above_auto_compact_threshold && !self.is_at_blocking_limit
    }

    /// 是否应该向用户显示警告。
    pub fn should_warn(&self) -> bool {
        self.is_above_warning_threshold && !self.is_above_error_threshold
    }

    /// 是否应该阻止新的用户输入。
    pub fn should_block_input(&self) -> bool {
        self.is_above_error_threshold
    }
}

// ---------------------------------------------------------------------------
// 自动压缩跟踪（熔断器）
// ---------------------------------------------------------------------------

/// 跟踪自动压缩的执行状态，实现熔断器模式。
#[derive(Debug, Clone)]
pub struct AutoCompactTracking {
    /// 当前轮次是否已执行压缩
    pub compacted: bool,
    /// 自上次检查以来的轮次计数
    pub turn_counter: u32,
    /// 唯一轮次标识符
    pub turn_id: String,
    /// 连续失败次数（用于熔断器）
    pub consecutive_failures: u32,
}

impl Default for AutoCompactTracking {
    fn default() -> Self {
        Self {
            compacted: false,
            turn_counter: 0,
            turn_id: String::new(),
            consecutive_failures: 0,
        }
    }
}

impl AutoCompactTracking {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次压缩成功。
    pub fn record_success(&mut self) {
        self.compacted = true;
        self.consecutive_failures = 0;
    }

    /// 记录一次压缩失败。
    pub fn record_failure(&mut self) {
        self.compacted = false;
        self.consecutive_failures += 1;
    }

    /// 检查熔断器是否已跳闸（连续失败次数达到上限）。
    pub fn is_circuit_breaker_tripped(&self) -> bool {
        self.consecutive_failures >= MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES
    }

    /// 开始新一轮。
    pub fn begin_turn(&mut self) {
        self.compacted = false;
        self.turn_counter += 1;
        self.turn_id = uuid::Uuid::new_v4().to_string();
    }

    /// 重置跟踪状态。
    pub fn reset(&mut self) {
        self.compacted = false;
        self.turn_counter = 0;
        self.consecutive_failures = 0;
        self.turn_id.clear();
    }
}

// ---------------------------------------------------------------------------
// 阈值辅助函数
// ---------------------------------------------------------------------------

/// 判断会话是否应该进行自动压缩（使用阈值系统替代二元判断）。
///
/// 返回 `true` 当：
/// - 超过自动压缩阈值但未达到硬阻止限制
/// - 且熔断器未跳闸
pub fn should_auto_compact(
    session: &Session,
    effective_window: u64,
    tracking: &AutoCompactTracking,
) -> bool {
    if tracking.is_circuit_breaker_tripped() {
        return false;
    }

    let state = CompactThresholdState::compute(session, effective_window);
    state.needs_compaction()
}

/// 判断会话是否应该触发响应式压缩。
///
/// 响应式压缩在超过错误阈值但未达到硬阻止限制时触发，
/// 使用更激进的压缩参数。
pub fn should_reactive_compact(session: &Session, effective_window: u64) -> bool {
    let state = CompactThresholdState::compute(session, effective_window);
    // 响应式压缩在超过错误阈值但尚未硬阻止时触发
    state.is_above_error_threshold && !state.is_at_blocking_limit
}

/// 获取推荐的压缩配置，基于当前阈值状态。
///
/// 越接近硬阻止限制，配置越激进。
pub fn recommended_compaction_config(
    session: &Session,
    effective_window: u64,
) -> CompactionConfig {
    let state = CompactThresholdState::compute(session, effective_window);

    if state.is_at_blocking_limit {
        // 最激进：只保留 2 条最近消息
        CompactionConfig {
            preserve_recent_messages: 2,
            max_estimated_tokens: 10_000,
            ..CompactionConfig::default()
        }
    } else if state.is_above_error_threshold {
        // 激进：保留 4 条
        CompactionConfig {
            preserve_recent_messages: 4,
            max_estimated_tokens: 30_000,
            ..CompactionConfig::default()
        }
    } else if state.is_above_auto_compact_threshold {
        // 适度
        CompactionConfig {
            preserve_recent_messages: 8,
            max_estimated_tokens: 50_000,
            ..CompactionConfig::default()
        }
    } else {
        // 默认
        CompactionConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{ContentBlock, ConversationMessage, Session};

    fn make_session_with_token_estimate(approx_tokens: usize) -> Session {
        let mut session = Session::new();
        // 每个字符大约 0.25 tokens，所以需要 approx_tokens * 4 个字符
        let chars_per_msg = approx_tokens * 4 / 10; // 10 条消息
        for i in 0..10 {
            let text = format!("msg{} {}", i, "x".repeat(chars_per_msg.max(20)));
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
    fn test_below_all_thresholds() {
        let session = make_session_with_token_estimate(1_000);
        let state = CompactThresholdState::compute(&session, 200_000);
        assert!(!state.is_above_warning_threshold);
        assert!(!state.is_above_auto_compact_threshold);
        assert!(!state.is_above_error_threshold);
        assert!(!state.is_at_blocking_limit);
    }

    #[test]
    fn test_above_warning_threshold() {
        // 创建接近自动压缩阈值的会话
        // 200_000 - 13_000 = 187_000 (auto compact threshold)
        // 187_000 - 20_000 = 167_000 (warning threshold)
        let session = make_session_with_token_estimate(170_000);
        let state = CompactThresholdState::compute(&session, 200_000);
        assert!(state.is_above_warning_threshold);
        assert!(!state.is_above_auto_compact_threshold);
    }

    #[test]
    fn test_above_auto_compact_threshold() {
        let session = make_session_with_token_estimate(190_000);
        let state = CompactThresholdState::compute(&session, 200_000);
        assert!(state.is_above_auto_compact_threshold);
        assert!(state.needs_compaction());
    }

    #[test]
    fn test_circuit_breaker() {
        let mut tracking = AutoCompactTracking::new();
        assert!(!tracking.is_circuit_breaker_tripped());

        tracking.record_failure();
        tracking.record_failure();
        tracking.record_failure();
        assert!(tracking.is_circuit_breaker_tripped());

        tracking.record_success();
        assert!(!tracking.is_circuit_breaker_tripped());
    }

    #[test]
    fn test_recommended_config_gets_more_aggressive() {
        let small_session = make_session_with_token_estimate(1_000);
        let medium_session = make_session_with_token_estimate(180_000);
        let large_session = make_session_with_token_estimate(195_000);

        let config1 = recommended_compaction_config(&small_session, 200_000);
        let config2 = recommended_compaction_config(&medium_session, 200_000);
        let config3 = recommended_compaction_config(&large_session, 200_000);

        // 越接近限制，preserve_recent_messages 越少
        assert!(config3.preserve_recent_messages <= config2.preserve_recent_messages);
        assert!(config3.max_estimated_tokens <= config2.max_estimated_tokens);
    }

    #[test]
    fn test_percent_left() {
        let session = make_session_with_token_estimate(50_000);
        let state = CompactThresholdState::compute(&session, 200_000);
        assert!(state.percent_left >= 70); // ~75% left
    }
}
