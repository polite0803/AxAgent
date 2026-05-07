use crate::error_classifier::ErrorType;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    Retry {
        max_attempts: usize,
        base_delay_ms: u64,
        max_delay_ms: u64,
        exponential_backoff: bool,
    },
    AdjustAndRetry {
        max_attempts: usize,
        adjustments: Vec<RecoveryAdjustment>,
    },
    Fallback {
        fallback_value: String,
    },
    SkipTask,
    Fail,
    AutoRecover {
        max_attempts: usize,
        checkpoint_interval_secs: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAdjustment {
    ReduceConcurrency,
    IncreaseTimeout(Duration),
    UseCache,
    SimplifyRequest,
    RetryWithDifferentModel,
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
        RecoveryStrategy::AutoRecover {
            max_attempts: 3,
            checkpoint_interval_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    pub success: bool,
    pub recovered: bool,
    pub strategy_used: String,
    pub attempts_made: usize,
    pub final_error: Option<String>,
    pub recovery_time_ms: u64,
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

#[derive(Debug, Clone)]
pub struct RecoveryAttempt {
    pub attempt_number: usize,
    pub error: String,
    pub strategy: RecoveryStrategy,
    pub delay_ms: Option<u64>,
    pub success: bool,
    pub message: Option<String>,
}

impl RecoveryAttempt {
    pub fn new(attempt_number: usize, error: String, strategy: RecoveryStrategy) -> Self {
        Self {
            attempt_number,
            error,
            strategy,
            delay_ms: None,
            success: false,
            message: None,
        }
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
            RecoveryStrategy::AdjustAndRetry {
                max_attempts,
                adjustments,
            } => {
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
    fn test_should_retry_retry() {
        let strategy = RecoveryStrategy::Retry {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 1000,
            exponential_backoff: true,
        };
        assert!(strategy.should_retry());
    }

    #[test]
    fn test_should_retry_fail() {
        let strategy = RecoveryStrategy::Fail;
        assert!(!strategy.should_retry());
    }

    #[test]
    fn test_should_retry_skip_task() {
        let strategy = RecoveryStrategy::SkipTask;
        assert!(!strategy.should_retry());
    }

    #[test]
    fn test_should_retry_fallback() {
        let strategy = RecoveryStrategy::Fallback {
            fallback_value: "default".to_string(),
        };
        assert!(strategy.should_retry());
    }

    #[test]
    fn test_should_retry_adjust_and_retry() {
        let strategy = RecoveryStrategy::AdjustAndRetry {
            max_attempts: 2,
            adjustments: vec![],
        };
        assert!(strategy.should_retry());
    }

    #[test]
    fn test_should_retry_auto_recover() {
        let strategy = RecoveryStrategy::AutoRecover {
            max_attempts: 3,
            checkpoint_interval_secs: 30,
        };
        assert!(strategy.should_retry());
    }

    #[test]
    fn test_should_retry_auto_recover_zero_attempts() {
        let strategy = RecoveryStrategy::AutoRecover {
            max_attempts: 0,
            checkpoint_interval_secs: 30,
        };
        assert!(!strategy.should_retry());
    }

    #[test]
    fn test_max_attempts() {
        assert_eq!(
            RecoveryStrategy::Retry {
                max_attempts: 5,
                base_delay_ms: 100,
                max_delay_ms: 1000,
                exponential_backoff: true
            }
            .max_attempts(),
            5
        );
        assert_eq!(
            RecoveryStrategy::AdjustAndRetry {
                max_attempts: 3,
                adjustments: vec![]
            }
            .max_attempts(),
            3
        );
        assert_eq!(
            RecoveryStrategy::Fallback {
                fallback_value: "x".to_string()
            }
            .max_attempts(),
            1
        );
        assert_eq!(RecoveryStrategy::SkipTask.max_attempts(), 0);
        assert_eq!(RecoveryStrategy::Fail.max_attempts(), 0);
        assert_eq!(
            RecoveryStrategy::AutoRecover {
                max_attempts: 4,
                checkpoint_interval_secs: 10
            }
            .max_attempts(),
            4
        );
    }

    #[test]
    fn test_description() {
        assert_eq!(
            RecoveryStrategy::Retry {
                max_attempts: 1,
                base_delay_ms: 100,
                max_delay_ms: 1000,
                exponential_backoff: false
            }
            .description(),
            "Retry with exponential backoff"
        );
        assert_eq!(
            RecoveryStrategy::AdjustAndRetry {
                max_attempts: 1,
                adjustments: vec![]
            }
            .description(),
            "Adjust parameters and retry"
        );
        assert_eq!(
            RecoveryStrategy::Fallback {
                fallback_value: "x".to_string()
            }
            .description(),
            "Use fallback value"
        );
        assert_eq!(RecoveryStrategy::SkipTask.description(), "Skip this task");
        assert_eq!(RecoveryStrategy::Fail.description(), "Fail immediately");
        assert_eq!(
            RecoveryStrategy::AutoRecover {
                max_attempts: 1,
                checkpoint_interval_secs: 10
            }
            .description(),
            "Auto-recover with checkpointing"
        );
    }

    #[test]
    fn test_for_interrupt() {
        let strategy = RecoveryStrategy::for_interrupt();
        match strategy {
            RecoveryStrategy::AutoRecover {
                max_attempts,
                checkpoint_interval_secs,
            } => {
                assert_eq!(max_attempts, 3);
                assert_eq!(checkpoint_interval_secs, 30);
            },
            _ => panic!("Expected AutoRecover for interrupt"),
        }
    }

    #[test]
    fn test_recovery_result_success() {
        let result = RecoveryResult::success(3, 150);
        assert!(result.success);
        assert!(result.recovered);
        assert_eq!(result.attempts_made, 3);
        assert_eq!(result.recovery_time_ms, 150);
        assert!(result.final_error.is_none());
    }

    #[test]
    fn test_recovery_result_failure() {
        let result = RecoveryResult::failure("Retry", 5, "timeout".to_string(), 300);
        assert!(!result.success);
        assert!(!result.recovered);
        assert_eq!(result.strategy_used, "Retry");
        assert_eq!(result.attempts_made, 5);
        assert_eq!(result.final_error, Some("timeout".to_string()));
        assert_eq!(result.recovery_time_ms, 300);
    }

    #[test]
    fn test_recovery_result_skipped() {
        let result = RecoveryResult::skipped(50);
        assert!(result.success);
        assert!(!result.recovered);
        assert_eq!(result.strategy_used, "SkipTask");
        assert_eq!(result.attempts_made, 0);
        assert!(result.final_error.is_none());
    }

    #[test]
    fn test_recovery_attempt_new() {
        let strategy = RecoveryStrategy::Retry {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 1000,
            exponential_backoff: true,
        };
        let attempt = RecoveryAttempt::new(2, "error msg".to_string(), strategy);
        assert_eq!(attempt.attempt_number, 2);
        assert_eq!(attempt.error, "error msg");
        assert!(attempt.delay_ms.is_none());
        assert!(!attempt.success);
        assert!(attempt.message.is_none());
    }

    #[test]
    fn test_recovery_attempt_with_delay() {
        let strategy = RecoveryStrategy::Fail;
        let attempt = RecoveryAttempt::new(1, "err".to_string(), strategy).with_delay(500);
        assert_eq!(attempt.delay_ms, Some(500));
    }

    #[test]
    fn test_recovery_attempt_with_success() {
        let strategy = RecoveryStrategy::Fail;
        let attempt =
            RecoveryAttempt::new(1, "err".to_string(), strategy).with_success("ok".to_string());
        assert!(attempt.success);
        assert_eq!(attempt.message, Some("ok".to_string()));
    }

    #[test]
    fn test_recovery_adjustment_variants() {
        let adjustments = vec![
            RecoveryAdjustment::ReduceConcurrency,
            RecoveryAdjustment::IncreaseTimeout(Duration::from_secs(30)),
            RecoveryAdjustment::UseCache,
            RecoveryAdjustment::SimplifyRequest,
            RecoveryAdjustment::RetryWithDifferentModel,
        ];
        assert_eq!(adjustments.len(), 5);
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
}
