// SPDX-License-Identifier: AGPL-3.0-only

//! 错误分类与恢复策略实现
//!
//! 类型权威定义在 `axagent-harness::error_classifier`，本模块：
//! 1. 通过 `pub use` 重导出 harness 中的共享类型
//! 2. 提供 ErrorClassifier 的具体实现（业务逻辑）
//! 3. 提供 RecoveryStrategy 的实现方法

// ── 从 harness 重导出共享类型 ──────────────────────────────────

pub use axagent_harness::error_classifier::{
    ClassifiedError, ErrorType, FailoverReason, RecoveryAdjustment, RecoveryAttempt,
    RecoveryResult, RecoveryStrategy, SuggestedAction,
};

// ── ErrorClassifier 实现 ─────────────────────────────────────────

/// 错误分类器 - 基于 HTTP 状态码和消息内容进行精确分类
pub struct ErrorClassifier;

impl ErrorClassifier {
    pub fn new() -> Self {
        Self
    }

    /// 精确分类: 先判断 FailoverReason, 再映射到 ErrorType
    pub fn classify(&self, error: &str) -> ErrorType {
        let failover = self.classify_with_reason(error);
        failover.error_type
    }

    /// 带 FailoverReason 的精确分类
    pub fn classify_with_reason(&self, error: &str) -> ClassifiedError {
        let failover_reason = self.detect_failover_reason(error);
        let error_type = failover_reason
            .map(|r| r.to_error_type())
            .unwrap_or_else(|| self.classify_error_type(error));

        ClassifiedError {
            error_type,
            original_error: error.to_string(),
            http_status: None,
            provider_error_code: Self::extract_error_code(error),
            context: None,
            failover_reason,
        }
    }

    pub fn classify_with_context(&self, error: &str, context: Option<String>) -> ClassifiedError {
        let mut result = self.classify_with_reason(error);
        result.context = context;
        result
    }

    /// 基于 HTTP 状态码分类
    pub fn classify_http_error(&self, status: u16, message: &str) -> ClassifiedError {
        let failover_reason = self.classify_by_http_status(status, message);
        let error_type = failover_reason.to_error();

        ClassifiedError {
            error_type,
            original_error: message.to_string(),
            http_status: Some(status),
            provider_error_code: None,
            context: None,
            failover_reason: Some(failover_reason),
        }
    }

    /// 检测精确的故障转移原因
    pub fn detect_failover_reason(&self, error: &str) -> Option<FailoverReason> {
        let lower = error.to_lowercase();

        // 按优先级检测
        if Self::is_content_blocked(&lower) {
            return Some(FailoverReason::ContentBlocked);
        }
        if Self::is_context_length_error(&lower) {
            return Some(FailoverReason::ContextLength);
        }
        if Self::is_auth_failed(&lower) {
            return Some(FailoverReason::AuthFailed);
        }
        if Self::is_quota_exceeded(&lower) {
            return Some(FailoverReason::QuotaExceeded);
        }
        if Self::is_cost_limit(&lower) {
            return Some(FailoverReason::CostLimit);
        }
        if Self::is_model_not_found(&lower) {
            return Some(FailoverReason::ModelNotFound);
        }
        if Self::is_invalid_parameters(&lower) {
            return Some(FailoverReason::InvalidParameters);
        }
        if Self::is_upstream_provider_error(&lower) {
            return Some(FailoverReason::UpstreamProviderError);
        }
        if Self::is_rate_limit(&lower) {
            return Some(FailoverReason::RateLimit);
        }
        if Self::is_provider_outage(&lower) {
            return Some(FailoverReason::ProviderOutage);
        }
        if Self::is_network_timeout(&lower) {
            return Some(FailoverReason::NetworkTimeout);
        }

        None
    }

    /// 按 HTTP 状态码分类
    fn classify_by_http_status(&self, status: u16, message: &str) -> FailoverReason {
        let lower = message.to_lowercase();

        match status {
            401 | 403 => {
                if lower.contains("quota")
                    || lower.contains("balance")
                    || lower.contains("insufficient")
                {
                    FailoverReason::QuotaExceeded
                } else {
                    FailoverReason::AuthFailed
                }
            },
            429 => FailoverReason::RateLimit,
            408 | 504 => FailoverReason::NetworkTimeout,
            500 | 502 | 503 => {
                if lower.contains("overloaded") || lower.contains("capacity") {
                    FailoverReason::ProviderOutage
                } else {
                    FailoverReason::UpstreamProviderError
                }
            },
            400 => {
                if lower.contains("context length")
                    || lower.contains("max tokens")
                    || lower.contains("token limit")
                {
                    FailoverReason::ContextLength
                } else if lower.contains("model") && lower.contains("not found") {
                    FailoverReason::ModelNotFound
                } else if lower.contains("content") || lower.contains("safety") {
                    FailoverReason::ContentBlocked
                } else {
                    FailoverReason::InvalidParameters
                }
            },
            404 => FailoverReason::ModelNotFound,
            _ => {
                if let Some(reason) = self.detect_failover_reason(message) {
                    reason
                } else {
                    FailoverReason::UnknownError
                }
            },
        }
    }

    /// 基础 ErrorType 分类 (不带 FailoverReason)
    fn classify_error_type(&self, error: &str) -> ErrorType {
        let lower = error.to_lowercase();

        if Self::is_transient(&lower) {
            ErrorType::Transient
        } else if Self::is_recoverable(&lower) {
            ErrorType::Recoverable
        } else if Self::is_unrecoverable(&lower) {
            ErrorType::Unrecoverable
        } else {
            ErrorType::Unknown
        }
    }

    // ── FailoverReason 检测模式 ────────────────────────────────────

    fn is_context_length_error(error: &str) -> bool {
        let patterns = [
            "context length",
            "context window",
            "max context",
            "token limit",
            "max tokens",
            "too many tokens",
            "input too long",
            "prompt too long",
            "max_input_tokens",
            "context_length_exceeded",
            "too large for model",
            "input length exceeds",
        ];
        patterns.iter().any(|p| error.contains(p))
    }

    fn is_content_blocked(error: &str) -> bool {
        let patterns = [
            "content_policy",
            "content policy",
            "safety",
            "blocked",
            "inappropriate",
            "harmful",
            "unsafe",
            "explicit",
            "violates",
            "content_filter",
            "content_filtered",
            "modr",
            "moderation",
            "rejected.*content",
        ];
        patterns.iter().any(|p| error.contains(p))
    }

    fn is_auth_failed(error: &str) -> bool {
        let patterns = [
            "401",
            "403",
            "unauthorized",
            "authentication",
            "invalid api key",
            "invalid token",
            "api key",
            "forbidden",
        ];
        patterns.iter().any(|p| error.contains(p))
    }

    fn is_quota_exceeded(error: &str) -> bool {
        let patterns = [
            "quota exceeded",
            "insufficient quota",
            "billing",
            "out of credits",
            "payment required",
            "account suspended",
            "balance.",
            "credits exhausted",
            "spend limit",
        ];
        patterns.iter().any(|p| error.contains(p))
    }

    fn is_cost_limit(error: &str) -> bool {
        let patterns = [
            "cost limit",
            "budget exceeded",
            "max cost",
            "token limit.*billing",
            "daily limit",
            "monthly limit",
        ];
        patterns.iter().any(|p| error.contains(p))
    }

    fn is_model_not_found(error: &str) -> bool {
        let patterns = [
            "model not found",
            "model.*not.*exist",
            "no such model",
            "unsupported model",
            "invalid model",
            "model unavailable",
            "does not exist",
            "not available",
        ];
        patterns.iter().any(|p| error.contains(p))
    }

    fn is_invalid_parameters(error: &str) -> bool {
        let patterns = [
            "invalid parameters",
            "invalid request",
            "bad request",
            "missing parameter",
            "invalid argument",
            "400.*error",
        ];
        patterns.iter().any(|p| error.contains(p))
    }

    fn is_upstream_provider_error(error: &str) -> bool {
        let patterns = ["upstream", "provider error", "bad gateway", "502", "503", "504"];
        patterns.iter().any(|p| error.contains(p))
    }

    fn is_rate_limit(error: &str) -> bool {
        let patterns = [
            "429",
            "rate limit",
            "too many requests",
            "requests per minute",
            "rpm limit",
            "tpm limit",
            "tokens per minute",
            "concurrent requests",
        ];
        patterns.iter().any(|p| error.contains(p))
    }

    fn is_provider_outage(error: &str) -> bool {
        let patterns = [
            "500",
            "internal server error",
            "service unavailable",
            "server error",
            "overloaded",
            "capacity",
        ];
        let has_provider_pattern = patterns.iter().any(|p| error.contains(p));

        // 排除明确是应用 bug 的情况（如 "null pointer dereference"），
        // 这些应该由 is_unrecoverable 处理而非被误判为 ProviderOutage
        let bug_patterns = [
            "null pointer",
            "panic",
            "assertion",
            "invariant",
            "not implemented",
            "unsupported",
            "illegal",
            "malformed",
            "syntax error",
            "parse error",
            "invalid syntax",
            "invalid format",
            "type mismatch",
            "cast error",
        ];
        let is_bug = bug_patterns.iter().any(|p| error.contains(p));

        has_provider_pattern && !is_bug
    }

    fn is_network_timeout(error: &str) -> bool {
        let patterns = [
            "timeout",
            "timed out",
            "connection",
            "connection refused",
            "unreachable",
            "reset by peer",
            "broken pipe",
            "econnreset",
            "econnrefused",
            "etimedout",
            "enotfound",
            "network error",
            "tls",
            "ssl error",
            "dns resolution",
        ];
        patterns.iter().any(|p| error.contains(p))
    }

    // ── 基础分类 (向后兼容) ────────────────────────────────────────

    fn is_transient(error: &str) -> bool {
        let transient_patterns = [
            "timeout",
            "timed out",
            "network",
            "connection",
            "refused",
            "unreachable",
            "temporarily unavailable",
            "service unavailable",
            "503",
            "502",
            "504",
            "429",
            "rate limit",
            "too many requests",
            "reset by peer",
            "broken pipe",
            "econnreset",
            "econnrefused",
            "etimedout",
            "enotfound",
        ];
        transient_patterns.iter().any(|p| error.contains(p))
    }

    fn is_recoverable(error: &str) -> bool {
        let recoverable_patterns = [
            "permission denied",
            "access denied",
            "unauthorized",
            "forbidden",
            "resource exhausted",
            "out of memory",
            "disk full",
            "quota exceeded",
            "limit exceeded",
            "capacity",
            "insufficient",
            "not found",
            "invalid state",
            "conflict",
            "409",
            "413",
            "401",
            "403",
        ];
        recoverable_patterns.iter().any(|p| error.contains(p))
    }

    fn is_unrecoverable(error: &str) -> bool {
        let unrecoverable_patterns = [
            "syntax error",
            "parse error",
            "invalid syntax",
            "illegal",
            "malformed",
            "unsupported",
            "not implemented",
            "invalid format",
            "type mismatch",
            "cast error",
            "null pointer",
            "panic",
            "assertion",
            "invariant",
            "500",
            "internal error",
        ];
        unrecoverable_patterns.iter().any(|p| error.contains(p))
    }

    fn extract_error_code(error: &str) -> Option<String> {
        if let Some(caps) = regex_lite::Regex::new(r"(?i)error[_\s]?code[:\s]+(\d+)")
            .ok()
            .and_then(|r| r.captures(error))
        {
            return caps.get(1).map(|m| m.as_str().to_string());
        }

        if let Some(caps) =
            regex_lite::Regex::new(r"\b(4\d{2}|5\d{2})\b").ok().and_then(|r| r.captures(error))
        {
            return caps.get(1).map(|m| m.as_str().to_string());
        }

        None
    }
}

impl Default for ErrorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

// ── RecoveryStrategy 实现 ─────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_error_type_transient() {
        let strategy = RecoveryStrategy::for_error_type(ErrorType::Transient);
        match strategy {
            RecoveryStrategy::Retry {
                max_attempts,
                base_delay_ms,
                max_delay_ms,
                exponential_backoff,
            } => {
                assert_eq!(max_attempts, 3);
                assert_eq!(base_delay_ms, 1000);
                assert_eq!(max_delay_ms, 10000);
                assert!(exponential_backoff);
            },
            _ => panic!("Expected Retry strategy for Transient"),
        }
    }

    #[test]
    fn test_for_error_type_recoverable() {
        let strategy = RecoveryStrategy::for_error_type(ErrorType::Recoverable);
        match strategy {
            RecoveryStrategy::AdjustAndRetry { max_attempts, adjustments } => {
                assert_eq!(max_attempts, 2);
                assert_eq!(adjustments.len(), 2);
            },
            _ => panic!("Expected AdjustAndRetry strategy for Recoverable"),
        }
    }

    #[test]
    fn test_for_error_type_unrecoverable() {
        let strategy = RecoveryStrategy::for_error_type(ErrorType::Unrecoverable);
        assert!(matches!(strategy, RecoveryStrategy::Fail));
    }

    #[test]
    fn test_for_error_type_unknown() {
        let strategy = RecoveryStrategy::for_error_type(ErrorType::Unknown);
        match strategy {
            RecoveryStrategy::Retry {
                max_attempts,
                base_delay_ms,
                max_delay_ms,
                exponential_backoff,
            } => {
                assert_eq!(max_attempts, 1);
                assert_eq!(base_delay_ms, 500);
                assert_eq!(max_delay_ms, 2000);
                assert!(!exponential_backoff);
            },
            _ => panic!("Expected Retry strategy for Unknown"),
        }
    }

    #[test]
    fn test_transient_errors() {
        let classifier = ErrorClassifier::new();
        assert_eq!(classifier.classify("connection timeout"), ErrorType::Transient);
        assert_eq!(classifier.classify("network error: 503"), ErrorType::Transient);
        assert_eq!(classifier.classify("rate limit exceeded"), ErrorType::Transient);
    }

    #[test]
    fn test_recoverable_errors() {
        let classifier = ErrorClassifier::new();
        assert_eq!(classifier.classify("permission denied"), ErrorType::Recoverable);
        assert_eq!(classifier.classify("resource exhausted"), ErrorType::Recoverable);
        // 401 认证失败归为 Unrecoverable（需用户干预，非自动恢复）
        assert_eq!(classifier.classify("401 unauthorized"), ErrorType::Unrecoverable);
    }

    #[test]
    fn test_unrecoverable_errors() {
        let classifier = ErrorClassifier::new();
        assert_eq!(classifier.classify("syntax error"), ErrorType::Unrecoverable);
        assert_eq!(classifier.classify("invalid format"), ErrorType::Unrecoverable);
        // 500 服务端错误归为 Transient（可重试/切换 provider）
        assert_eq!(classifier.classify("internal server error: 500"), ErrorType::Transient);
    }

    #[test]
    fn test_detect_failover_reason() {
        let classifier = ErrorClassifier::new();

        assert_eq!(
            classifier.detect_failover_reason("rate limit exceeded"),
            Some(FailoverReason::RateLimit)
        );
        assert_eq!(
            classifier.detect_failover_reason("context length exceeded"),
            Some(FailoverReason::ContextLength)
        );
        assert_eq!(
            classifier.detect_failover_reason("invalid api key"),
            Some(FailoverReason::AuthFailed)
        );
        assert_eq!(
            classifier.detect_failover_reason("insufficient quota"),
            Some(FailoverReason::QuotaExceeded)
        );
    }

    #[test]
    fn test_classify_http_error() {
        let classifier = ErrorClassifier::new();

        let result = classifier.classify_http_error(429, "Too many requests");
        assert_eq!(result.failover_reason, Some(FailoverReason::RateLimit));
        assert_eq!(result.http_status, Some(429));

        let result = classifier.classify_http_error(401, "Unauthorized");
        assert_eq!(result.failover_reason, Some(FailoverReason::AuthFailed));

        let result = classifier.classify_http_error(500, "Internal server error");
        assert_eq!(result.failover_reason, Some(FailoverReason::UpstreamProviderError));
    }

    #[test]
    fn test_recovery_strategy_for_classified_error() {
        let error = ClassifiedError {
            error_type: ErrorType::Transient,
            original_error: "timeout".to_string(),
            http_status: None,
            provider_error_code: None,
            context: None,
            failover_reason: Some(FailoverReason::NetworkTimeout),
        };
        let strategy = RecoveryStrategy::for_classified_error(&error);
        assert!(strategy.should_retry());
    }

    #[test]
    fn test_recovery_result_methods() {
        let result = RecoveryResult::success(3, 150);
        assert!(result.success);
        assert!(result.recovered);

        let result = RecoveryResult::failure("Retry", 5, "timeout".to_string(), 300);
        assert!(!result.success);

        let result = RecoveryResult::skipped(50);
        assert!(result.success);
        assert!(!result.recovered);
    }

    #[test]
    fn test_recovery_attempt_methods() {
        let strategy = RecoveryStrategy::Fail;
        let attempt = RecoveryAttempt::new(1, "err".to_string(), strategy).with_delay(500);
        assert_eq!(attempt.delay_ms, Some(500));

        let strategy = RecoveryStrategy::Fail;
        let attempt =
            RecoveryAttempt::new(1, "err".to_string(), strategy).with_success("ok".to_string());
        assert!(attempt.success);
    }

    #[test]
    fn test_failover_reason_suggested_strategy() {
        let strategy = FailoverReason::NetworkTimeout.suggested_strategy();
        assert!(strategy.should_retry());

        let strategy = FailoverReason::AuthFailed.suggested_strategy();
        assert!(!strategy.should_retry());
    }
}
