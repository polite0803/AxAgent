//! 压缩警告系统
//!
//! 在上下文即将用尽时向用户发出渐进式警告，而非静默压缩。
//! 实现抑制状态机以防止警告疲劳：用户可抑制警告，抑制在 TTL 后自动过期。
//! 移植自 claude-code-main 的 compactWarningHook.ts。

use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// 抑制 TTL 常量
// ---------------------------------------------------------------------------

/// 警告抑制的默认过期时间（5 分钟）
pub const DEFAULT_SUPPRESSION_TTL_SECS: u64 = 300;

/// 两次警告之间的最小间隔（防止刷屏）
pub const MIN_WARNING_INTERVAL_SECS: u64 = 30;

// ---------------------------------------------------------------------------
// 警告状态
// ---------------------------------------------------------------------------

/// 跟踪压缩警告的抑制状态。
///
/// 用户可以通过抑制来暂时隐藏警告，抑制会在设定的 TTL 后自动过期。
/// 同时跟踪上次警告时间以防止刷屏。
#[derive(Debug, Clone)]
pub struct CompactWarningState {
    /// 当前是否抑制警告
    pub suppressed: bool,
    /// 抑制设置的时间（用于自动过期）
    pub suppressed_at: Option<Instant>,
    /// 抑制的自动过期时长
    pub suppression_ttl: Duration,
    /// 上次显示警告的时间（用于防刷屏）
    pub last_warning_at: Option<Instant>,
    /// 两次警告之间的最小间隔
    pub min_interval: Duration,
}

impl Default for CompactWarningState {
    fn default() -> Self {
        Self {
            suppressed: false,
            suppressed_at: None,
            suppression_ttl: Duration::from_secs(DEFAULT_SUPPRESSION_TTL_SECS),
            last_warning_at: None,
            min_interval: Duration::from_secs(MIN_WARNING_INTERVAL_SECS),
        }
    }
}

impl CompactWarningState {
    /// 创建一个新的警告状态。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置抑制过期时间。
    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.suppression_ttl = Duration::from_secs(ttl_secs);
        self
    }

    /// 抑制警告（用户请求隐藏）。
    pub fn suppress(&mut self) {
        self.suppressed = true;
        self.suppressed_at = Some(Instant::now());
    }

    /// 清除抑制状态。
    pub fn clear(&mut self) {
        self.suppressed = false;
        self.suppressed_at = None;
    }

    /// 检查抑制是否已过期。如果过期则自动清除。
    pub fn check_expiry(&mut self) {
        if !self.suppressed {
            return;
        }
        if let Some(at) = self.suppressed_at {
            if at.elapsed() >= self.suppression_ttl {
                self.clear();
            }
        }
    }

    /// 判断当前是否应该显示警告。
    ///
    /// # 参数
    /// - `token_usage`: 当前 token 使用量
    /// - `warning_threshold`: 警告阈值
    ///
    /// # 返回
    /// - `true`: 应该显示警告（未抑制、未过期、不在冷却期）
    /// - `false`: 不应显示警告
    pub fn should_warn(&mut self, token_usage: u64, warning_threshold: u64) -> bool {
        // 先检查抑制是否过期
        self.check_expiry();

        // 被抑制 → 不显示
        if self.suppressed {
            return false;
        }

        // token 未达阈值 → 不显示
        if token_usage < warning_threshold {
            return false;
        }

        // 检查冷却期
        if let Some(last) = self.last_warning_at {
            if last.elapsed() < self.min_interval {
                return false;
            }
        }

        // 记录本次警告时间
        self.last_warning_at = Some(Instant::now());
        true
    }

    /// 仅检查是否需要警告（不更新状态，不记录时间）。
    /// 用于预判断，不影响实际警告状态。
    pub fn peek(&mut self, token_usage: u64, warning_threshold: u64) -> bool {
        self.check_expiry();

        if self.suppressed {
            return false;
        }

        if token_usage < warning_threshold {
            return false;
        }

        if let Some(last) = self.last_warning_at {
            if last.elapsed() < self.min_interval {
                return false;
            }
        }

        true
    }

    /// 重置所有状态（新会话开始时调用）。
    pub fn reset(&mut self) {
        self.suppressed = false;
        self.suppressed_at = None;
        self.last_warning_at = None;
    }
}

// ---------------------------------------------------------------------------
// 警告级别
// ---------------------------------------------------------------------------

/// 压缩警告的严重级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningLevel {
    /// 信息：接近但未达到压缩阈值
    Info,
    /// 警告：超过警告阈值，即将自动压缩
    Warning,
    /// 严重：即将达到硬阻止限制
    Critical,
}

impl std::fmt::Display for WarningLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WarningLevel::Info => write!(f, "info"),
            WarningLevel::Warning => write!(f, "warning"),
            WarningLevel::Critical => write!(f, "critical"),
        }
    }
}

/// 压缩警告消息。
#[derive(Debug, Clone)]
pub struct CompactWarning {
    /// 警告级别
    pub level: WarningLevel,
    /// 警告消息文本
    pub message: String,
    /// 已使用 token 占比（百分比）
    pub pct_used: u32,
    /// 当前估算 token 数
    pub estimated_tokens: u64,
    /// 有效上下文窗口大小
    pub effective_window: u64,
    /// 在自动压缩前还有多少 token 可用
    pub tokens_before_compact: i64,
}

impl CompactWarning {
    pub fn new(
        level: WarningLevel,
        message: impl Into<String>,
        pct_used: u32,
        estimated_tokens: u64,
        effective_window: u64,
        tokens_before_compact: i64,
    ) -> Self {
        Self {
            level,
            message: message.into(),
            pct_used,
            estimated_tokens,
            effective_window,
            tokens_before_compact,
        }
    }

    /// 构建警告消息文本。
    pub fn format(&self) -> String {
        let level_prefix = match self.level {
            WarningLevel::Info => "ℹ",
            WarningLevel::Warning => "⚠",
            WarningLevel::Critical => "🔴",
        };
        format!(
            "{} {} ({}% of {} tokens used, {} tokens before auto-compact)",
            level_prefix,
            self.message,
            self.pct_used,
            self.effective_window,
            if self.tokens_before_compact > 0 {
                self.tokens_before_compact
            } else {
                0
            },
        )
    }
}

/// 根据 token 使用量计算警告级别。
///
/// # 阈值
/// - < 70%: 无需警告
/// - 70-80%: Info
/// - 80-90%: Warning
/// - >= 90%: Critical
pub fn compute_warning_level(pct_used: u32) -> Option<WarningLevel> {
    match pct_used {
        0..=69 => None,
        70..=79 => Some(WarningLevel::Info),
        80..=89 => Some(WarningLevel::Warning),
        _ => Some(WarningLevel::Critical),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suppress_and_clear() {
        let mut state = CompactWarningState::new();
        assert!(!state.suppressed);

        state.suppress();
        assert!(state.suppressed);
        assert!(state.suppressed_at.is_some());

        state.clear();
        assert!(!state.suppressed);
        assert!(state.suppressed_at.is_none());
    }

    #[test]
    fn test_should_warn_below_threshold() {
        let mut state = CompactWarningState::new();
        assert!(!state.should_warn(5000, 10000));
    }

    #[test]
    fn test_should_warn_above_threshold() {
        let mut state = CompactWarningState::new();
        assert!(state.should_warn(15000, 10000));
    }

    #[test]
    fn test_suppressed_prevents_warning() {
        let mut state = CompactWarningState::new();
        state.suppress();
        assert!(!state.should_warn(15000, 10000));
    }

    #[test]
    fn test_ttl_expiry() {
        let mut state = CompactWarningState::new().with_ttl(0); // 立即过期
        state.suppress();
        state.check_expiry();
        assert!(!state.suppressed);
    }

    #[test]
    fn test_min_interval_prevents_spam() {
        let mut state = CompactWarningState::new();
        // 第一次触发
        assert!(state.should_warn(15000, 10000));
        // 立即再次检查 — 应在冷却期
        assert!(!state.should_warn(15000, 10000));
    }

    #[test]
    fn test_warning_levels() {
        assert_eq!(compute_warning_level(50), None);
        assert_eq!(compute_warning_level(75), Some(WarningLevel::Info));
        assert_eq!(compute_warning_level(85), Some(WarningLevel::Warning));
        assert_eq!(compute_warning_level(95), Some(WarningLevel::Critical));
    }

    #[test]
    fn test_warning_format() {
        let warning = CompactWarning::new(
            WarningLevel::Warning,
            "上下文即将用尽",
            85,
            170000,
            200000,
            17000,
        );
        let formatted = warning.format();
        assert!(formatted.contains("⚠"));
        assert!(formatted.contains("85%"));
        assert!(formatted.contains("200000"));
    }

    #[test]
    fn test_reset() {
        let mut state = CompactWarningState::new();
        state.suppress();
        assert!(state.should_warn(15000, 10000) == false); // should NOT warn when suppressed
        state.reset();
        assert!(!state.suppressed);
        assert!(state.last_warning_at.is_none());
    }
}
