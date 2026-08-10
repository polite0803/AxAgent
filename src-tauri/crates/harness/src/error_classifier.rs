// SPDX-License-Identifier: AGPL-3.0-only

//! 错误分类与故障转移系统 (P0-5)
//!
//! 借鉴 Hermes Agent 的 error_classifier.py：
//! - 结构化三级分类（HTTP 状态码 → 错误码 → 消息正则）
//! - FailoverReason 枚举供前端精确提示
//! - ClassifiedError 结构供故障转移使用
//!
//! **注意**：本模块仅定义共享 DTO，不包含业务逻辑实现。
//! ErrorClassifier 的具体实现位于 agent crate。

use std::time::Duration;

use serde::{Deserialize, Serialize};

// ── 三级错误分类 ─────────────────────────────────────────────────

/// 错误类型 - 三级分类体系
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    /// 瞬态错误 - 重试可能恢复
    Transient,
    /// 可恢复错误 - 调整参数可恢复
    Recoverable,
    /// 不可恢复错误 - 应直接降级
    Unrecoverable,
    /// 未知错误
    Unknown,
}

impl ErrorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorType::Transient => "transient",
            ErrorType::Recoverable => "recoverable",
            ErrorType::Unrecoverable => "unrecoverable",
            ErrorType::Unknown => "unknown",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ErrorType::Transient => "Temporary error - retry may resolve",
            ErrorType::Recoverable => "Recoverable error - can be fixed with adjustment",
            ErrorType::Unrecoverable => "Unrecoverable error - should fail",
            ErrorType::Unknown => "Unknown error type",
        }
    }
}

// ── 故障转移原因 ─────────────────────────────────────────────────

/// 故障转移原因枚举（合并了两套定义的优点）
///
/// 三级分类: Transient(可重试) / Recoverable(可调整) / Unrecoverable(直接降级)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailoverReason {
    /// 网络超时/连接失败 — 重试
    NetworkTimeout,
    /// 限流 (429/rate limit) — 退避重试
    RateLimit,
    /// 提供商故障 (5xx) — 切换 provider
    ProviderOutage,
    /// 认证失败 - API key 无效或过期 (401/403)
    AuthFailed,
    /// 余额不足 - 账户额度耗尽
    QuotaExceeded,
    /// 上下文长度超出限制
    ContextLength,
    /// 模型不存在 - 模型 ID 无效
    ModelNotFound,
    /// 参数错误 - 请求参数无效
    InvalidParameters,
    /// 上游提供商错误 - OpenRouter 等聚合商的上游错误
    UpstreamProviderError,
    /// 内容安全拦截 — 改写 prompt
    ContentBlocked,
    /// 成本超限 — 降级到更便宜的模型
    CostLimit,
    /// 未知错误 — 默认降级
    UnknownError,
}

impl FailoverReason {
    /// 映射到 ErrorType (三级分类)
    pub fn to_error_type(&self) -> ErrorType {
        match self {
            // Transient — 重试即可恢复
            FailoverReason::NetworkTimeout
            | FailoverReason::RateLimit
            | FailoverReason::ProviderOutage => ErrorType::Transient,
            // Recoverable — 调整参数即可恢复
            FailoverReason::ContextLength
            | FailoverReason::ModelNotFound
            | FailoverReason::CostLimit
            | FailoverReason::ContentBlocked
            | FailoverReason::InvalidParameters => ErrorType::Recoverable,
            // Unrecoverable — 直接降级
            FailoverReason::AuthFailed
            | FailoverReason::QuotaExceeded
            | FailoverReason::UnknownError => ErrorType::Unrecoverable,
            // UpstreamProviderError 视为瞬态，可重试或切换
            FailoverReason::UpstreamProviderError => ErrorType::Transient,
        }
    }

    /// 是否可通过重试恢复
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            FailoverReason::NetworkTimeout
                | FailoverReason::RateLimit
                | FailoverReason::ProviderOutage
                | FailoverReason::UpstreamProviderError
        )
    }

    /// 是否需要用户干预
    pub fn requires_user_action(&self) -> bool {
        matches!(
            self,
            FailoverReason::AuthFailed
                | FailoverReason::QuotaExceeded
                | FailoverReason::ModelNotFound
                | FailoverReason::InvalidParameters
        )
    }

    /// 是否应该切换 provider (跨 provider 降级)
    pub fn should_switch_provider(&self) -> bool {
        matches!(
            self,
            FailoverReason::ProviderOutage
                | FailoverReason::ModelNotFound
                | FailoverReason::CostLimit
        )
    }

    /// 是否应该降级到更便宜的模型
    pub fn should_downgrade_tier(&self) -> bool {
        matches!(self, FailoverReason::CostLimit)
    }

    /// 是否应该直接失败 (不尝试恢复)
    pub fn should_fail_fast(&self) -> bool {
        matches!(self, FailoverReason::AuthFailed | FailoverReason::QuotaExceeded)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FailoverReason::NetworkTimeout => "network_timeout",
            FailoverReason::RateLimit => "rate_limit",
            FailoverReason::ProviderOutage => "provider_outage",
            FailoverReason::AuthFailed => "auth_failed",
            FailoverReason::QuotaExceeded => "quota_exceeded",
            FailoverReason::ContextLength => "context_length",
            FailoverReason::ModelNotFound => "model_not_found",
            FailoverReason::InvalidParameters => "invalid_parameters",
            FailoverReason::UpstreamProviderError => "upstream_provider_error",
            FailoverReason::ContentBlocked => "content_blocked",
            FailoverReason::CostLimit => "cost_limit",
            FailoverReason::UnknownError => "unknown_error",
        }
    }

    /// 获取人类可读的描述
    pub fn description(&self) -> &'static str {
        match self {
            FailoverReason::NetworkTimeout => "网络连接超时，请重试",
            FailoverReason::RateLimit => "请求过于频繁，请稍后重试",
            FailoverReason::ProviderOutage => "服务商暂时过载，请稍后重试或切换",
            FailoverReason::AuthFailed => "认证失败，请检查 API key 是否正确",
            FailoverReason::QuotaExceeded => "余额不足，请充值或更换账户",
            FailoverReason::ContextLength => {
                "上下文长度超出限制，请缩短对话或使用支持更长上下文的模型"
            },
            FailoverReason::ModelNotFound => "模型不存在，请检查模型 ID 是否正确",
            FailoverReason::InvalidParameters => "参数错误，请检查请求参数",
            FailoverReason::UpstreamProviderError => "上游服务商错误，可能正在维护",
            FailoverReason::ContentBlocked => "内容被安全策略拦截，请改写后重试",
            FailoverReason::CostLimit => "成本超限，建议降级到更便宜的模型",
            FailoverReason::UnknownError => "未知错误",
        }
    }
}

// ── 错误分析结果 ─────────────────────────────────────────────────

/// 分类后的错误（含 FailoverReason 精确原因）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedError {
    /// 错误类型（三级分类）
    pub error_type: ErrorType,
    /// 原始错误消息
    pub original_error: String,
    /// HTTP 状态码（如果有）
    pub http_status: Option<u16>,
    /// 提供商特定错误码（如果有）
    pub provider_error_code: Option<String>,
    /// 上下文信息
    pub context: Option<String>,
    /// 精确的故障转移原因
    pub failover_reason: Option<FailoverReason>,
}

// ── 建议的操作 ─────────────────────────────────────────────────

/// 建议的操作（供前端 UI 提示和自动恢复使用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedAction {
    /// 重试
    Retry,
    /// 故障转移到备用提供商
    Failover,
    /// 降级上下文
    TruncateContext,
    /// 更换模型
    SwitchModel,
    /// 需要用户干预
    UserIntervention,
    /// 直接返回错误
    ReturnError,
}

// ── 恢复策略 ─────────────────────────────────────────────────

/// 恢复策略枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// 带指数退避的重试
    Retry { max_attempts: usize, base_delay_ms: u64, max_delay_ms: u64, exponential_backoff: bool },
    /// 调整参数后重试
    AdjustAndRetry { max_attempts: usize, adjustments: Vec<RecoveryAdjustment> },
    /// 使用回退值
    Fallback { fallback_value: String },
    /// 跳过任务
    SkipTask,
    /// 直接失败
    Fail,
    /// 自动恢复（带检查点）
    AutoRecover { max_attempts: usize, checkpoint_interval_secs: u64 },
}

/// 恢复调整项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAdjustment {
    ReduceConcurrency,
    IncreaseTimeout(Duration),
    UseCache,
    SimplifyRequest,
    RetryWithDifferentModel,
}

// ── 恢复结果 ─────────────────────────────────────────────────

/// 恢复执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    pub success: bool,
    pub recovered: bool,
    pub strategy_used: String,
    pub attempts_made: usize,
    pub final_error: Option<String>,
    pub recovery_time_ms: u64,
}

/// 恢复尝试记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAttempt {
    pub attempt_number: usize,
    pub error: String,
    pub strategy: RecoveryStrategy,
    pub delay_ms: Option<u64>,
    pub success: bool,
    pub message: Option<String>,
}

impl FailoverReason {
    /// 映射到 ErrorType (三级分类) - 别名用于向后兼容
    pub fn to_error(&self) -> ErrorType {
        self.to_error_type()
    }

    /// 获取建议的恢复策略
    pub fn suggested_strategy(&self) -> RecoveryStrategy {
        match self {
            FailoverReason::NetworkTimeout => RecoveryStrategy::Retry {
                max_attempts: 3,
                base_delay_ms: 1000,
                max_delay_ms: 10_000,
                exponential_backoff: true,
            },
            FailoverReason::RateLimit => RecoveryStrategy::Retry {
                max_attempts: 5,
                base_delay_ms: 2000,
                max_delay_ms: 30_000,
                exponential_backoff: true,
            },
            FailoverReason::ProviderOutage => {
                RecoveryStrategy::Fallback { fallback_value: String::new() }
            },
            FailoverReason::ContextLength => RecoveryStrategy::AdjustAndRetry {
                max_attempts: 2,
                adjustments: vec![
                    RecoveryAdjustment::SimplifyRequest,
                    RecoveryAdjustment::ReduceConcurrency,
                ],
            },
            FailoverReason::ModelNotFound => {
                RecoveryStrategy::Fallback { fallback_value: String::new() }
            },
            FailoverReason::AuthFailed | FailoverReason::QuotaExceeded => RecoveryStrategy::Fail,
            FailoverReason::ContentBlocked => RecoveryStrategy::AdjustAndRetry {
                max_attempts: 1,
                adjustments: vec![RecoveryAdjustment::SimplifyRequest],
            },
            FailoverReason::CostLimit => {
                RecoveryStrategy::Fallback { fallback_value: String::new() }
            },
            FailoverReason::InvalidParameters => RecoveryStrategy::Fail,
            FailoverReason::UpstreamProviderError => RecoveryStrategy::Retry {
                max_attempts: 2,
                base_delay_ms: 3000,
                max_delay_ms: 15_000,
                exponential_backoff: true,
            },
            FailoverReason::UnknownError => RecoveryStrategy::Retry {
                max_attempts: 1,
                base_delay_ms: 500,
                max_delay_ms: 2_000,
                exponential_backoff: false,
            },
        }
    }
}

impl RecoveryStrategy {
    pub fn for_error_type(error_type: ErrorType) -> Self {
        match error_type {
            ErrorType::Transient => RecoveryStrategy::Retry {
                max_attempts: 3,
                base_delay_ms: 1000,
                max_delay_ms: 10000,
                exponential_backoff: true,
            },
            ErrorType::Recoverable => RecoveryStrategy::AdjustAndRetry {
                max_attempts: 2,
                adjustments: vec![
                    RecoveryAdjustment::IncreaseTimeout(Duration::from_secs(30)),
                    RecoveryAdjustment::ReduceConcurrency,
                ],
            },
            ErrorType::Unrecoverable => RecoveryStrategy::Fail,
            ErrorType::Unknown => RecoveryStrategy::Retry {
                max_attempts: 1,
                base_delay_ms: 500,
                max_delay_ms: 2000,
                exponential_backoff: false,
            },
        }
    }

    /// 从 ClassifiedError 获取精确的恢复策略
    pub fn for_classified_error(error: &ClassifiedError) -> Self {
        if let Some(reason) = error.failover_reason {
            return reason.suggested_strategy();
        }
        Self::for_error_type(error.error_type)
    }

    /// 从 FailoverReason 直接获取策略
    pub fn for_failover_reason(reason: FailoverReason) -> Self {
        reason.suggested_strategy()
    }

    pub fn should_retry(&self) -> bool {
        match self {
            RecoveryStrategy::Retry { max_attempts, .. } => *max_attempts > 0,
            RecoveryStrategy::AdjustAndRetry { max_attempts, .. } => *max_attempts > 0,
            RecoveryStrategy::Fallback { .. } => true,
            RecoveryStrategy::SkipTask => false,
            RecoveryStrategy::Fail => false,
            RecoveryStrategy::AutoRecover { max_attempts, .. } => *max_attempts > 0,
        }
    }

    pub fn max_attempts(&self) -> usize {
        match self {
            RecoveryStrategy::Retry { max_attempts, .. } => *max_attempts,
            RecoveryStrategy::AdjustAndRetry { max_attempts, .. } => *max_attempts,
            RecoveryStrategy::Fallback { .. } => 1,
            RecoveryStrategy::SkipTask => 0,
            RecoveryStrategy::Fail => 0,
            RecoveryStrategy::AutoRecover { max_attempts, .. } => *max_attempts,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            RecoveryStrategy::Retry { .. } => "Retry with exponential backoff",
            RecoveryStrategy::AdjustAndRetry { .. } => "Adjust parameters and retry",
            RecoveryStrategy::Fallback { .. } => "Use fallback value",
            RecoveryStrategy::SkipTask => "Skip this task",
            RecoveryStrategy::Fail => "Fail immediately",
            RecoveryStrategy::AutoRecover { .. } => "Auto-recover with checkpointing",
        }
    }

    pub fn for_interrupt() -> Self {
        RecoveryStrategy::AutoRecover { max_attempts: 3, checkpoint_interval_secs: 30 }
    }
}

impl RecoveryResult {
    pub fn success(attempts: usize, recovery_time_ms: u64) -> Self {
        Self {
            success: true,
            recovered: true,
            strategy_used: String::new(),
            attempts_made: attempts,
            final_error: None,
            recovery_time_ms,
        }
    }

    pub fn failure(strategy: &str, attempts: usize, error: String, recovery_time_ms: u64) -> Self {
        Self {
            success: false,
            recovered: false,
            strategy_used: strategy.to_string(),
            attempts_made: attempts,
            final_error: Some(error),
            recovery_time_ms,
        }
    }

    pub fn skipped(recovery_time_ms: u64) -> Self {
        Self {
            success: true,
            recovered: false,
            strategy_used: "SkipTask".to_string(),
            attempts_made: 0,
            final_error: None,
            recovery_time_ms,
        }
    }
}

impl RecoveryAttempt {
    pub fn new(attempt_number: usize, error: String, strategy: RecoveryStrategy) -> Self {
        Self { attempt_number, error, strategy, delay_ms: None, success: false, message: None }
    }

    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = Some(delay_ms);
        self
    }

    pub fn with_success(mut self, message: String) -> Self {
        self.success = true;
        self.message = Some(message);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_type_as_str() {
        assert_eq!(ErrorType::Transient.as_str(), "transient");
        assert_eq!(ErrorType::Recoverable.as_str(), "recoverable");
        assert_eq!(ErrorType::Unrecoverable.as_str(), "unrecoverable");
        assert_eq!(ErrorType::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_failover_reason_to_error_type() {
        // Transient
        assert_eq!(FailoverReason::NetworkTimeout.to_error_type(), ErrorType::Transient);
        assert_eq!(FailoverReason::RateLimit.to_error_type(), ErrorType::Transient);
        assert_eq!(FailoverReason::ProviderOutage.to_error_type(), ErrorType::Transient);
        assert_eq!(FailoverReason::UpstreamProviderError.to_error_type(), ErrorType::Transient);

        // Recoverable
        assert_eq!(FailoverReason::ContextLength.to_error_type(), ErrorType::Recoverable);
        assert_eq!(FailoverReason::ModelNotFound.to_error_type(), ErrorType::Recoverable);
        assert_eq!(FailoverReason::CostLimit.to_error_type(), ErrorType::Recoverable);
        assert_eq!(FailoverReason::ContentBlocked.to_error_type(), ErrorType::Recoverable);
        assert_eq!(FailoverReason::InvalidParameters.to_error_type(), ErrorType::Recoverable);

        // Unrecoverable
        assert_eq!(FailoverReason::AuthFailed.to_error_type(), ErrorType::Unrecoverable);
        assert_eq!(FailoverReason::QuotaExceeded.to_error_type(), ErrorType::Unrecoverable);
        assert_eq!(FailoverReason::UnknownError.to_error_type(), ErrorType::Unrecoverable);
    }

    #[test]
    fn test_failover_reason_retryable() {
        assert!(FailoverReason::NetworkTimeout.is_retryable());
        assert!(FailoverReason::RateLimit.is_retryable());
        assert!(FailoverReason::ProviderOutage.is_retryable());
        assert!(FailoverReason::UpstreamProviderError.is_retryable());
        assert!(!FailoverReason::AuthFailed.is_retryable());
        assert!(!FailoverReason::QuotaExceeded.is_retryable());
        assert!(!FailoverReason::ContextLength.is_retryable());
    }

    #[test]
    fn test_failover_reason_requires_action() {
        assert!(FailoverReason::AuthFailed.requires_user_action());
        assert!(FailoverReason::QuotaExceeded.requires_user_action());
        assert!(FailoverReason::ModelNotFound.requires_user_action());
        assert!(FailoverReason::InvalidParameters.requires_user_action());
        assert!(!FailoverReason::NetworkTimeout.requires_user_action());
        assert!(!FailoverReason::RateLimit.requires_user_action());
    }

    #[test]
    fn test_failover_reason_switch_provider() {
        assert!(FailoverReason::ProviderOutage.should_switch_provider());
        assert!(FailoverReason::ModelNotFound.should_switch_provider());
        assert!(FailoverReason::CostLimit.should_switch_provider());
        assert!(!FailoverReason::NetworkTimeout.should_switch_provider());
    }

    #[test]
    fn test_failover_reason_fail_fast() {
        assert!(FailoverReason::AuthFailed.should_fail_fast());
        assert!(FailoverReason::QuotaExceeded.should_fail_fast());
        assert!(!FailoverReason::NetworkTimeout.should_fail_fast());
    }

    #[test]
    fn test_failover_reason_as_str() {
        assert_eq!(FailoverReason::NetworkTimeout.as_str(), "network_timeout");
        assert_eq!(FailoverReason::AuthFailed.as_str(), "auth_failed");
        assert_eq!(FailoverReason::QuotaExceeded.as_str(), "quota_exceeded");
    }

    #[test]
    fn test_failover_reason_description() {
        assert!(!FailoverReason::AuthFailed.description().is_empty());
        assert!(!FailoverReason::UnknownError.description().is_empty());
    }

    #[test]
    fn test_classified_error_structure() {
        let error = ClassifiedError {
            error_type: ErrorType::Transient,
            original_error: "timeout".to_string(),
            http_status: Some(504),
            provider_error_code: None,
            context: Some("streaming".to_string()),
            failover_reason: Some(FailoverReason::NetworkTimeout),
        };
        assert_eq!(error.error_type, ErrorType::Transient);
        assert_eq!(error.failover_reason, Some(FailoverReason::NetworkTimeout));
    }

    #[test]
    fn test_recovery_strategy_variants() {
        let retry = RecoveryStrategy::Retry {
            max_attempts: 3,
            base_delay_ms: 1000,
            max_delay_ms: 10000,
            exponential_backoff: true,
        };
        assert!(matches!(retry, RecoveryStrategy::Retry { .. }));

        let fallback = RecoveryStrategy::Fallback { fallback_value: "default".to_string() };
        assert!(matches!(fallback, RecoveryStrategy::Fallback { .. }));

        assert!(matches!(RecoveryStrategy::SkipTask, RecoveryStrategy::SkipTask));
        assert!(matches!(RecoveryStrategy::Fail, RecoveryStrategy::Fail));
    }

    #[test]
    fn test_recovery_adjustment_variants() {
        let adjustments = [
            RecoveryAdjustment::ReduceConcurrency,
            RecoveryAdjustment::IncreaseTimeout(Duration::from_secs(30)),
            RecoveryAdjustment::UseCache,
            RecoveryAdjustment::SimplifyRequest,
            RecoveryAdjustment::RetryWithDifferentModel,
        ];
        assert_eq!(adjustments.len(), 5);
    }

    #[test]
    fn test_recovery_result_structure() {
        let result = RecoveryResult {
            success: true,
            recovered: true,
            strategy_used: "Retry".to_string(),
            attempts_made: 3,
            final_error: None,
            recovery_time_ms: 150,
        };
        assert!(result.success);
        assert!(result.recovered);
    }

    #[test]
    fn test_recovery_attempt_structure() {
        let strategy = RecoveryStrategy::Fail;
        let attempt = RecoveryAttempt {
            attempt_number: 1,
            error: "test error".to_string(),
            strategy,
            delay_ms: None,
            success: false,
            message: None,
        };
        assert_eq!(attempt.attempt_number, 1);
        assert!(!attempt.success);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let strategy = RecoveryStrategy::Retry {
            max_attempts: 3,
            base_delay_ms: 1000,
            max_delay_ms: 10000,
            exponential_backoff: true,
        };
        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: RecoveryStrategy = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, RecoveryStrategy::Retry { .. }));
    }

    #[test]
    fn test_failover_reason_serialization() {
        let reason = FailoverReason::AuthFailed;
        let json = serde_json::to_string(&reason).unwrap();
        let deserialized: FailoverReason = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, reason);
    }

    #[test]
    fn test_classified_error_serialization() {
        let error = ClassifiedError {
            error_type: ErrorType::Recoverable,
            original_error: "context too long".to_string(),
            http_status: Some(400),
            provider_error_code: Some("context_length_exceeded".to_string()),
            context: None,
            failover_reason: Some(FailoverReason::ContextLength),
        };
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: ClassifiedError = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.error_type, ErrorType::Recoverable);
        assert_eq!(deserialized.failover_reason, Some(FailoverReason::ContextLength));
    }
}
