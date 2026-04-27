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
        }
    }

    pub fn max_attempts(&self) -> usize {
        match self {
            RecoveryStrategy::Retry { max_attempts, .. } => *max_attempts,
            RecoveryStrategy::AdjustAndRetry { max_attempts, .. } => *max_attempts,
            RecoveryStrategy::Fallback { .. } => 1,
            RecoveryStrategy::SkipTask => 0,
            RecoveryStrategy::Fail => 0,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            RecoveryStrategy::Retry { .. } => "Retry with exponential backoff",
            RecoveryStrategy::AdjustAndRetry { .. } => "Adjust parameters and retry",
            RecoveryStrategy::Fallback { .. } => "Use fallback value",
            RecoveryStrategy::SkipTask => "Skip this task",
            RecoveryStrategy::Fail => "Fail immediately",
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
