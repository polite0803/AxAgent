//! 错误恢复策略 — 分级退避重试 + 熔断器
//!
//! 重试策略：
//! - 429 Rate Limit → 立即退避，指数增长 (2s, 4s, 8s...)
//! - 5xx Server Error → 线性退避 (1s, 2s, 3s...)
//! - Network Error → 固定间隔重试 (500ms)
//!
//! 熔断器：连续失败 5 次 → 熔断 30 秒

use std::time::{Duration, Instant};

/// 错误类型分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    RateLimit,      // 429
    ServerError,    // 5xx
    NetworkError,   // 超时/连接
    ClientError,    // 4xx (不可重试)
    Unknown,
}

impl ErrorKind {
    /// 根据错误消息字符串分类错误类型
    pub fn classify(error_msg: &str) -> Self {
        let lower = error_msg.to_lowercase();
        if lower.contains("429") || lower.contains("rate") {
            return Self::RateLimit;
        }
        if lower.contains("500") || lower.contains("502") || lower.contains("503") {
            return Self::ServerError;
        }
        if lower.contains("timeout")
            || lower.contains("network")
            || lower.contains("connection")
            || lower.contains("reset")
        {
            return Self::NetworkError;
        }
        if lower.contains("400") || lower.contains("401") || lower.contains("403") || lower.contains("404") {
            return Self::ClientError;
        }
        Self::Unknown
    }

    /// 判断该错误类型是否可以重试
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::RateLimit | Self::ServerError | Self::NetworkError)
    }
}

/// 重试策略配置
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 最大重试次数
    pub max_retries: u32,
    /// 基础延迟时间
    pub base_delay: Duration,
    /// 最大延迟上限
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// 计算第 n 次重试的等待延迟
    pub fn delay_for(&self, kind: ErrorKind, attempt: u32) -> Duration {
        match kind {
            ErrorKind::RateLimit => {
                // 指数退避：2^attempt 秒
                let secs = 2u64.pow(attempt).min(self.max_delay.as_secs());
                Duration::from_secs(secs).min(self.max_delay)
            }
            ErrorKind::ServerError => {
                // 线性退避：base * attempt
                let secs =
                    (self.base_delay.as_secs() * attempt as u64).min(self.max_delay.as_secs());
                Duration::from_secs(secs).min(self.max_delay)
            }
            ErrorKind::NetworkError => {
                // 固定间隔重试
                Duration::from_millis(500)
            }
            _ => Duration::ZERO,
        }
    }

    /// 判断是否应该继续重试
    pub fn should_retry(&self, kind: ErrorKind, attempt: u32) -> bool {
        kind.is_retryable() && attempt < self.max_retries
    }
}

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 正常状态，请求允许通过
    Closed,
    /// 熔断状态，拒绝所有请求
    Open,
    /// 半开状态，允许探测请求
    HalfOpen,
}

/// 熔断器 — 连续失败达到阈值后熔断，防止雪崩
#[derive(Debug)]
pub struct CircuitBreaker {
    /// 失败次数阈值（达到后熔断）
    failure_threshold: u32,
    /// 熔断恢复超时
    recovery_timeout: Duration,
    /// 当前连续失败计数
    failure_count: u32,
    /// 最近一次失败的时间
    last_failure: Option<Instant>,
    /// 当前熔断器状态
    state: CircuitState,
}

impl CircuitBreaker {
    /// 创建新的熔断器实例（默认阈值 5 次失败 → 熔断 30 秒）
    pub fn new() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            failure_count: 0,
            last_failure: None,
            state: CircuitState::Closed,
        }
    }

    /// 查询当前熔断器状态
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// 检查是否允许执行请求
    ///
    /// - Closed 状态：直接放行
    /// - Open 状态：检查恢复时间是否已过，若已过则进入 HalfOpen 并放行探测请求
    /// - HalfOpen 状态：放行探测请求
    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last) = self.last_failure {
                    if last.elapsed() >= self.recovery_timeout {
                        self.state = CircuitState::HalfOpen;
                        return true; // 允许探测请求
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// 记录一次成功调用，重置熔断器状态
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
    }

    /// 记录一次失败调用，累计失败次数
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());
        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
        if self.state == CircuitState::HalfOpen {
            // 探测失败，重新熔断
            self.state = CircuitState::Open;
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_uses_exponential_backoff() {
        let policy = RetryPolicy::default();
        let d1 = policy.delay_for(ErrorKind::RateLimit, 1);
        let d2 = policy.delay_for(ErrorKind::RateLimit, 2);
        // 指数退避：第 2 次延迟应大于第 1 次
        assert!(d2 > d1);
    }

    #[test]
    fn server_error_uses_linear_backoff() {
        let policy = RetryPolicy::default();
        let d1 = policy.delay_for(ErrorKind::ServerError, 1);
        let d2 = policy.delay_for(ErrorKind::ServerError, 2);
        assert!(d2 > d1);
    }

    #[test]
    fn network_error_uses_fixed_interval() {
        let policy = RetryPolicy::default();
        let d1 = policy.delay_for(ErrorKind::NetworkError, 1);
        let d2 = policy.delay_for(ErrorKind::NetworkError, 5);
        // 固定间隔，每次都是 500ms
        assert_eq!(d1, Duration::from_millis(500));
        assert_eq!(d2, Duration::from_millis(500));
    }

    #[test]
    fn circuit_breaker_opens_after_threshold() {
        let mut cb = CircuitBreaker::new();
        for _ in 0..5 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn circuit_breaker_half_open_probe() {
        let mut cb = CircuitBreaker::new();
        // 达到熔断阈值
        for _ in 0..5 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);

        // 模拟恢复时间已过（无法真正等待，直接测试状态转换逻辑）
        // 由于 last_failure 刚刚记录，不允许请求
        assert!(!cb.allow_request());
    }

    #[test]
    fn circuit_breaker_recovers_after_success() {
        let mut cb = CircuitBreaker::new();
        for _ in 0..4 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Closed); // 未达到阈值
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        // 重置后计数归零
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed); // 仅 1 次失败
    }

    #[test]
    fn client_errors_not_retryable() {
        let policy = RetryPolicy::default();
        assert!(!policy.should_retry(ErrorKind::ClientError, 0));
    }

    #[test]
    fn unknown_errors_not_retryable() {
        let policy = RetryPolicy::default();
        assert!(!policy.should_retry(ErrorKind::Unknown, 0));
    }

    #[test]
    fn classify_rate_limit_from_message() {
        assert_eq!(
            ErrorKind::classify("HTTP 429 Too Many Requests"),
            ErrorKind::RateLimit
        );
        assert_eq!(
            ErrorKind::classify("rate limit exceeded"),
            ErrorKind::RateLimit
        );
    }

    #[test]
    fn classify_server_error_from_message() {
        assert_eq!(
            ErrorKind::classify("HTTP 500 Internal Server Error"),
            ErrorKind::ServerError
        );
        assert_eq!(ErrorKind::classify("502 Bad Gateway"), ErrorKind::ServerError);
    }

    #[test]
    fn classify_network_error_from_message() {
        assert_eq!(ErrorKind::classify("connection reset by peer"), ErrorKind::NetworkError);
        assert_eq!(ErrorKind::classify("request timeout"), ErrorKind::NetworkError);
    }

    #[test]
    fn max_retries_exceeded_stops_retry() {
        let policy = RetryPolicy::default();
        assert!(!policy.should_retry(ErrorKind::ServerError, 3)); // attempt 3 >= max_retries 3
        assert!(policy.should_retry(ErrorKind::ServerError, 2));  // attempt 2 < max_retries 3
    }
}
