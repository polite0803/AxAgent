// SPDX-License-Identifier: AGPL-3.0-only

//! 中心化超时/重试/降级策略
//!
//! 提供统一的 `RetryPolicy`，支持固定/线性/指数退避、超时控制、
//! 可重试状态码配置以及重试耗尽后的降级策略（直接失败、返回默认值、
//! 转人工、换模型）。

use std::time::Duration;

/// 退避策略
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// 固定延迟（每次等待 base_delay_ms）
    Fixed,
    /// 线性递增：base_delay_ms + (attempt-1) * increment_ms
    Linear { increment_ms: u64 },
    /// 指数退避：base_delay_ms * multiplier^(attempt-1)
    Exponential { multiplier: f64 },
}

/// 降级策略 — 重试耗尽后的行为
#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    /// 直接失败，返回错误
    Fail,
    /// 返回默认值
    ReturnDefault(serde_json::Value),
    /// 转人工处理
    EscalateToHuman,
    /// 换模型重试（需要 secondary_model）
    SwitchModel { secondary_model: String },
}

/// 重试错误 — `execute_with_retry` 的错误类型
///
/// 调用方可以通过 match 区分不同降级场景：
/// - `Exhausted`：重试耗尽（`FallbackStrategy::Fail`）
/// - `SwitchModelRequested`：请求切换模型（`FallbackStrategy::SwitchModel`）
/// - `EscalateToHuman`：需人工处理（`FallbackStrategy::EscalateToHuman`）
/// - `DefaultFallback`：降级为默认值（`FallbackStrategy::ReturnDefault`）
/// - `NonRetryable`：不可重试错误
///
/// 实现了 `Display`，调用方可用 `.map_err(|e| e.to_string())` 保持向后兼容。
#[derive(Debug, thiserror::Error)]
pub enum RetryError {
    /// 重试耗尽：`FallbackStrategy::Fail` 触发
    #[error("[RetryPolicy] 重试 {max_retries}/{max_retries} 次后失败: {last_error}")]
    Exhausted { max_retries: u32, last_error: String },

    /// 不可重试错误：错误不匹配 `retryable_status_codes` 和关键词
    #[error("[RetryPolicy] 不可重试错误: {0}")]
    NonRetryable(String),

    /// 请求切换模型：`FallbackStrategy::SwitchModel` 触发
    /// 调用方应感知此错误并执行模型切换（如通过 `ModelCascadeExecutor`）
    #[error("[RetryPolicy] 需切换至模型 '{secondary_model}': {original_error}")]
    SwitchModelRequested { secondary_model: String, original_error: String },

    /// 需人工处理：`FallbackStrategy::EscalateToHuman` 触发
    #[error("[RetryPolicy] 需人工处理: {0}")]
    EscalateToHuman(String),

    /// 降级为默认值：`FallbackStrategy::ReturnDefault` 触发
    #[error("[RetryPolicy] 降级为默认值（原始错误: {0}）")]
    DefaultFallback(String),
}

/// 重试策略
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 最大重试次数（不包含首次调用）
    pub max_retries: u32,
    /// 基础延迟（毫秒）
    pub base_delay_ms: u64,
    /// 退避策略
    pub backoff: BackoffStrategy,
    /// 单次调用超时（毫秒）
    pub timeout_ms: u64,
    /// 哪些 HTTP 状态码触发重试
    pub retryable_status_codes: Vec<u16>,
    /// 降级策略：重试耗尽后做什么
    pub fallback: FallbackStrategy,
}

impl RetryPolicy {
    /// 默认 LLM 重试策略：指数退避，3 次重试，60s 超时
    pub fn default_llm() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
            timeout_ms: 60000,
            retryable_status_codes: vec![429, 500, 502, 503, 504],
            fallback: FallbackStrategy::Fail,
        }
    }

    /// 执行带重试和超时的异步调用
    ///
    /// # 参数
    /// - `f`: 被包装的异步调用（闭包形式，每次重试重新调用）
    ///
    /// # 返回
    /// - `Ok(T)`：调用成功
    /// - `Err(RetryError)`：重试耗尽或不可重试错误
    ///
    /// 调用方可以通过 match `RetryError` 区分降级场景：
    /// - `RetryError::SwitchModelRequested` → 执行模型切换
    /// - `RetryError::Exhausted` → 彻底失败
    /// - 其他 → 对应降级策略
    pub async fn execute_with_retry<F, Fut, T, E>(&self, f: F) -> Result<T, RetryError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut last_error = String::new();

        for attempt in 0..=self.max_retries {
            // 重试等待（首次调用不等待）
            if attempt > 0 {
                let delay = self.compute_delay(attempt);
                tracing::warn!(
                    "[RetryPolicy] 第 {}/{} 次重试，等待 {}ms",
                    attempt,
                    self.max_retries,
                    delay
                );
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            // 带超时的调用
            match tokio::time::timeout(Duration::from_millis(self.timeout_ms), f()).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) => {
                    let err_str = e.to_string();
                    // 检查是否可重试
                    if !self.is_retryable(&err_str) {
                        return Err(RetryError::NonRetryable(err_str));
                    }
                    last_error = err_str;
                    tracing::warn!("[RetryPolicy] 第 {} 次失败: {}", attempt + 1, last_error);
                },
                Err(_timeout_elapsed) => {
                    last_error = format!("超时 ({}ms)", self.timeout_ms);
                    tracing::warn!("[RetryPolicy] 第 {} 次超时", attempt + 1);
                },
            }
        }

        // 重试耗尽，执行降级
        self.handle_fallback(&last_error)
    }

    /// 处理降级 — 根据 `FallbackStrategy` 返回对应的 `RetryError`
    ///
    /// 调用方可以 match `RetryError` 来感知降级意图：
    /// - `SwitchModelRequested` → 调用 `ModelCascadeExecutor` 执行模型切换
    /// - `EscalateToHuman` → 通知前端转人工
    /// - `DefaultFallback` → 返回预配置的默认值
    /// - `Exhausted` → 彻底失败
    fn handle_fallback<T>(&self, last_error: &str) -> Result<T, RetryError> {
        match &self.fallback {
            FallbackStrategy::Fail => {
                let err = format!(
                    "[RetryPolicy] 重试 {}/{} 次后失败: {}",
                    self.max_retries, self.max_retries, last_error
                );
                tracing::error!("{err}");
                Err(RetryError::Exhausted {
                    max_retries: self.max_retries,
                    last_error: last_error.to_string(),
                })
            },
            FallbackStrategy::ReturnDefault(_val) => {
                tracing::warn!("[RetryPolicy] 降级为默认值（原始错误: {last_error}）");
                Err(RetryError::DefaultFallback(last_error.to_string()))
            },
            FallbackStrategy::EscalateToHuman => {
                tracing::warn!("[RetryPolicy] 升级到人工处理: {last_error}");
                Err(RetryError::EscalateToHuman(last_error.to_string()))
            },
            FallbackStrategy::SwitchModel { secondary_model } => {
                tracing::warn!("[RetryPolicy] 请求切换模型至 '{secondary_model}': {last_error}");
                Err(RetryError::SwitchModelRequested {
                    secondary_model: secondary_model.clone(),
                    original_error: last_error.to_string(),
                })
            },
        }
    }

    /// 计算第 attempt 次重试的等待延迟（毫秒）
    fn compute_delay(&self, attempt: u32) -> u64 {
        match self.backoff {
            BackoffStrategy::Fixed => self.base_delay_ms,
            BackoffStrategy::Linear { increment_ms } => {
                self.base_delay_ms + (attempt as u64 - 1) * increment_ms
            },
            BackoffStrategy::Exponential { multiplier } => {
                (self.base_delay_ms as f64 * multiplier.powi(attempt as i32 - 1)) as u64
            },
        }
    }

    /// 判断错误是否可重试
    fn is_retryable(&self, err: &str) -> bool {
        let lower = err.to_lowercase();
        // 检查状态码
        if self.retryable_status_codes.iter().any(|code| lower.contains(&code.to_string())) {
            return true;
        }
        // 检查常见重试关键词
        lower.contains("timeout")
            || lower.contains("rate limit")
            || lower.contains("throttl")
            || lower.contains("too many")
            || lower.contains("temporarily")
            || lower.contains("server error")
            || lower.contains("service unavailable")
            || lower.contains("bad gateway")
    }
}

/// 为 RetryPolicy 提供 Default 实现（与 default_llm() 相同）
impl Default for RetryPolicy {
    fn default() -> Self {
        Self::default_llm()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_llm_config() {
        let policy = RetryPolicy::default_llm();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.base_delay_ms, 1000);
        assert_eq!(policy.timeout_ms, 60000);
        assert!(matches!(policy.backoff, BackoffStrategy::Exponential { multiplier: 2.0 }));
    }

    #[test]
    fn test_compute_delay_fixed() {
        let policy = RetryPolicy {
            base_delay_ms: 500,
            backoff: BackoffStrategy::Fixed,
            ..Default::default()
        };
        assert_eq!(policy.compute_delay(1), 500);
        assert_eq!(policy.compute_delay(3), 500);
    }

    #[test]
    fn test_compute_delay_linear() {
        let policy = RetryPolicy {
            base_delay_ms: 100,
            backoff: BackoffStrategy::Linear { increment_ms: 200 },
            ..Default::default()
        };
        assert_eq!(policy.compute_delay(1), 100);
        assert_eq!(policy.compute_delay(2), 300);
        assert_eq!(policy.compute_delay(3), 500);
    }

    #[test]
    fn test_compute_delay_exponential() {
        let policy = RetryPolicy {
            base_delay_ms: 1000,
            backoff: BackoffStrategy::Exponential { multiplier: 2.0 },
            ..Default::default()
        };
        assert_eq!(policy.compute_delay(1), 1000);
        assert_eq!(policy.compute_delay(2), 2000);
        assert_eq!(policy.compute_delay(3), 4000);
        assert_eq!(policy.compute_delay(4), 8000);
    }

    #[test]
    fn test_is_retryable_by_status() {
        let policy = RetryPolicy { retryable_status_codes: vec![429, 500], ..Default::default() };
        assert!(policy.is_retryable("status 429"));
        assert!(policy.is_retryable("HTTP 500 error"));
        assert!(!policy.is_retryable("HTTP 400 bad request"));
    }

    #[test]
    fn test_is_retryable_by_keyword() {
        let policy = RetryPolicy::default_llm();
        assert!(policy.is_retryable("timeout occurred"));
        assert!(policy.is_retryable("rate limit exceeded"));
        assert!(policy.is_retryable("throttling detected"));
        assert!(policy.is_retryable("too many requests"));
        assert!(policy.is_retryable("temporarily unavailable"));
        assert!(!policy.is_retryable("invalid input"));
        assert!(!policy.is_retryable("permission denied"));
    }

    #[test]
    fn test_handle_fallback_fail() {
        let policy = RetryPolicy { fallback: FallbackStrategy::Fail, ..Default::default() };
        let result: Result<String, RetryError> = policy.handle_fallback("server error");
        assert!(result.is_err());
        match result.unwrap_err() {
            RetryError::Exhausted { max_retries, last_error } => {
                assert_eq!(max_retries, 3);
                assert!(last_error.contains("server error"));
            },
            other => panic!("预期 Exhausted，实际: {other:?}"),
        }
    }

    #[test]
    fn test_handle_fallback_escalate() {
        let policy =
            RetryPolicy { fallback: FallbackStrategy::EscalateToHuman, ..Default::default() };
        let result: Result<String, RetryError> = policy.handle_fallback("complex case");
        assert!(result.is_err());
        match result.unwrap_err() {
            RetryError::EscalateToHuman(msg) => {
                assert!(msg.contains("complex case"));
            },
            other => panic!("预期 EscalateToHuman，实际: {other:?}"),
        }
    }

    #[test]
    fn test_handle_fallback_switch_model() {
        let policy = RetryPolicy {
            fallback: FallbackStrategy::SwitchModel { secondary_model: "gpt-4".into() },
            ..Default::default()
        };
        let result: Result<String, RetryError> = policy.handle_fallback("model overloaded");
        assert!(result.is_err());
        match result.unwrap_err() {
            RetryError::SwitchModelRequested { secondary_model, original_error } => {
                assert_eq!(secondary_model, "gpt-4");
                assert!(original_error.contains("model overloaded"));
            },
            other => panic!("预期 SwitchModelRequested，实际: {other:?}"),
        }
    }

    #[test]
    fn test_handle_fallback_default_value() {
        let policy = RetryPolicy {
            fallback: FallbackStrategy::ReturnDefault(serde_json::Value::Null),
            ..Default::default()
        };
        let result: Result<String, RetryError> = policy.handle_fallback("fallback triggered");
        assert!(result.is_err());
        match result.unwrap_err() {
            RetryError::DefaultFallback(msg) => {
                assert!(msg.contains("fallback triggered"));
            },
            other => panic!("预期 DefaultFallback，实际: {other:?}"),
        }
    }

    #[test]
    fn test_retry_error_display() {
        let err = RetryError::Exhausted { max_retries: 3, last_error: "timeout".to_string() };
        let s = err.to_string();
        assert!(s.contains("重试 3/3 次后失败"));
        assert!(s.contains("timeout"));

        let err = RetryError::SwitchModelRequested {
            secondary_model: "gpt-4o".to_string(),
            original_error: "overloaded".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("gpt-4o"));
        assert!(s.contains("overloaded"));

        let err = RetryError::NonRetryable("bad request".to_string());
        assert!(err.to_string().contains("不可重试"));

        let err = RetryError::EscalateToHuman("complex".to_string());
        assert!(err.to_string().contains("人工处理"));

        let err = RetryError::DefaultFallback("orig".to_string());
        assert!(err.to_string().contains("默认值"));
    }

    #[tokio::test]
    async fn test_execute_with_retry_success_first_try() {
        let policy = RetryPolicy::default_llm();
        let result: Result<i32, RetryError> =
            policy.execute_with_retry(|| async { Ok::<i32, String>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_execute_with_retry_non_retryable() {
        let policy = RetryPolicy::default_llm();
        let result: Result<i32, RetryError> = policy
            .execute_with_retry(|| async { Err::<i32, String>("HTTP 400 bad request".to_string()) })
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            RetryError::NonRetryable(msg) => assert!(msg.contains("400")),
            other => panic!("预期 NonRetryable，实际: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_execute_with_retry_switch_model_fallback() {
        let policy = RetryPolicy {
            max_retries: 0, // 不重试，直接降级
            base_delay_ms: 1,
            timeout_ms: 1000,
            fallback: FallbackStrategy::SwitchModel { secondary_model: "gpt-4o".into() },
            ..Default::default()
        };
        let result: Result<i32, RetryError> =
            policy.execute_with_retry(|| async { Err::<i32, String>("timeout".to_string()) }).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            RetryError::SwitchModelRequested { secondary_model, .. } => {
                assert_eq!(secondary_model, "gpt-4o");
            },
            other => panic!("预期 SwitchModelRequested，实际: {other:?}"),
        }
    }
}
