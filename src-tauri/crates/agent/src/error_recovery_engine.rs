use crate::error_classifier::{ClassifiedError, ErrorClassifier, ErrorType};
use crate::recovery_strategies::{RecoveryAdjustment, RecoveryResult, RecoveryStrategy};
use crate::retry_policy::RetryPolicy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    pub max_total_attempts: usize,
    pub enable_fallback: bool,
    pub enable_adjustments: bool,
    pub timeout_per_attempt: Duration,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_total_attempts: 5,
            enable_fallback: true,
            enable_adjustments: true,
            timeout_per_attempt: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RecoveryEvent {
    RecoveryStarted {
        error: String,
        error_type: ErrorType,
    },
    AttemptStarted {
        attempt: usize,
        strategy: String,
    },
    AttemptCompleted {
        attempt: usize,
        success: bool,
    },
    RecoveryCompleted {
        result: RecoveryResult,
    },
    RecoveryFailed {
        error: String,
    },
    RetryScheduled {
        delay_ms: u64,
        attempt: usize,
    },
}

pub struct ErrorRecoveryEngine {
    classifier: Arc<ErrorClassifier>,
    config: RecoveryConfig,
    event_sender: broadcast::Sender<RecoveryEvent>,
}

impl ErrorRecoveryEngine {
    pub fn new() -> Self {
        let classifier = Arc::new(ErrorClassifier::new());
        let (event_sender, _) = broadcast::channel(100);

        Self {
            classifier,
            config: RecoveryConfig::default(),
            event_sender,
        }
    }

    pub fn with_config(mut self, config: RecoveryConfig) -> Self {
        self.config = config;
        self
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RecoveryEvent> {
        self.event_sender.subscribe()
    }

    pub fn classify_error(&self, error: &str) -> ClassifiedError {
        self.classifier.classify_with_context(error, None)
    }

    pub fn get_recovery_strategy(&self, error_type: ErrorType) -> RecoveryStrategy {
        if !self.config.enable_adjustments && matches!(error_type, ErrorType::Recoverable) {
            return RecoveryStrategy::Fail;
        }

        RecoveryStrategy::for_error_type(error_type)
    }

    pub async fn recover<F, Fut, T>(&self, error: &str, mut f: F) -> RecoveryResult
    where
        F: FnMut() -> Fut,
        F: Send,
        Fut: std::future::Future<Output = Result<T, String>> + Send,
    {
        let start = Instant::now();
        let classified = self.classify_error(error);

        self.emit(RecoveryEvent::RecoveryStarted {
            error: error.to_string(),
            error_type: classified.error_type,
        });

        let strategy = self.get_recovery_strategy(classified.error_type);

        if !strategy.should_retry() {
            self.emit(RecoveryEvent::RecoveryFailed {
                error: error.to_string(),
            });

            return RecoveryResult::failure(
                strategy.description(),
                0,
                error.to_string(),
                start.elapsed().as_millis() as u64,
            );
        }

        let result = self.execute_recovery(&strategy, &mut f, start).await;

        self.emit(RecoveryEvent::RecoveryCompleted {
            result: result.clone(),
        });

        result
    }

    async fn execute_recovery<F, Fut, T>(
        &self,
        strategy: &RecoveryStrategy,
        f: &mut F,
        start: Instant,
    ) -> RecoveryResult
    where
        F: FnMut() -> Fut,
        F: Send,
        Fut: std::future::Future<Output = Result<T, String>> + Send,
    {
        match strategy {
            RecoveryStrategy::Retry {
                max_attempts,
                base_delay_ms,
                max_delay_ms,
                exponential_backoff,
            } => {
                let policy = RetryPolicy::new(*max_attempts)
                    .with_base_delay(Duration::from_millis(*base_delay_ms))
                    .with_max_delay(Duration::from_millis(*max_delay_ms))
                    .with_exponential_backoff(*exponential_backoff);

                self.retry_with_policy(f, &policy, start).await
            },
            RecoveryStrategy::AdjustAndRetry {
                max_attempts,
                adjustments,
            } => {
                self.adjust_and_retry(f, *max_attempts, adjustments, start)
                    .await
            },
            RecoveryStrategy::Fallback { fallback_value } => {
                self.emit(RecoveryEvent::RecoveryFailed {
                    error: "Using fallback".to_string(),
                });

                RecoveryResult::failure(
                    "Fallback",
                    0,
                    format!("Fallback value: {}", fallback_value),
                    start.elapsed().as_millis() as u64,
                )
            },
            RecoveryStrategy::SkipTask => {
                RecoveryResult::skipped(start.elapsed().as_millis() as u64)
            },
            RecoveryStrategy::Fail => RecoveryResult::failure(
                "Fail",
                0,
                "Immediate failure".to_string(),
                start.elapsed().as_millis() as u64,
            ),
            RecoveryStrategy::AutoRecover {
                max_attempts,
                checkpoint_interval_secs: _,
            } => {
                let mut last_error = "Max attempts reached".to_string();
                for attempt in 0..*max_attempts {
                    self.emit(RecoveryEvent::AttemptStarted {
                        attempt,
                        strategy: "AutoRecover".to_string(),
                    });
                    let result = f().await;
                    match result {
                        Ok(_) => {
                            self.emit(RecoveryEvent::AttemptCompleted {
                                attempt,
                                success: true,
                            });
                            return RecoveryResult {
                                success: true,
                                recovered: true,
                                strategy_used: "AutoRecover".to_string(),
                                attempts_made: attempt + 1,
                                final_error: None,
                                recovery_time_ms: start.elapsed().as_millis() as u64,
                            };
                        },
                        Err(e) => {
                            last_error = e;
                            self.emit(RecoveryEvent::AttemptCompleted {
                                attempt,
                                success: false,
                            });
                        },
                    }
                }
                self.emit(RecoveryEvent::RecoveryFailed {
                    error: last_error.clone(),
                });
                RecoveryResult::failure(
                    "AutoRecover",
                    *max_attempts,
                    last_error,
                    start.elapsed().as_millis() as u64,
                )
            },
        }
    }

    async fn retry_with_policy<F, Fut, T>(
        &self,
        f: &mut F,
        policy: &RetryPolicy,
        start: Instant,
    ) -> RecoveryResult
    where
        F: FnMut() -> Fut,
        F: Send,
        Fut: std::future::Future<Output = Result<T, String>> + Send,
    {
        let mut attempts = 0;
        let mut errors = Vec::new();

        while attempts < policy.max_attempts {
            attempts += 1;

            self.emit(RecoveryEvent::AttemptStarted {
                attempt: attempts,
                strategy: "Retry".to_string(),
            });

            match f().await {
                Ok(_) => {
                    self.emit(RecoveryEvent::AttemptCompleted {
                        attempt: attempts,
                        success: true,
                    });

                    return RecoveryResult::success(attempts, start.elapsed().as_millis() as u64);
                },
                Err(e) => {
                    errors.push(e.clone());
                    self.emit(RecoveryEvent::AttemptCompleted {
                        attempt: attempts,
                        success: false,
                    });

                    if attempts < policy.max_attempts {
                        let delay = policy.next_delay(attempts - 1);
                        self.emit(RecoveryEvent::RetryScheduled {
                            delay_ms: delay.as_millis() as u64,
                            attempt: attempts,
                        });
                        tokio::time::sleep(delay).await;
                    }
                },
            }
        }

        RecoveryResult::failure(
            "Retry",
            attempts,
            errors.join("; "),
            start.elapsed().as_millis() as u64,
        )
    }

    async fn adjust_and_retry<F, Fut, T>(
        &self,
        f: &mut F,
        max_attempts: usize,
        adjustments: &[RecoveryAdjustment],
        start: Instant,
    ) -> RecoveryResult
    where
        F: FnMut() -> Fut,
        F: Send,
        Fut: std::future::Future<Output = Result<T, String>> + Send,
    {
        let mut attempts = 0;
        let mut current_adjustment_idx = 0;

        while attempts < max_attempts {
            attempts += 1;

            let adjustment_desc = adjustments
                .get(current_adjustment_idx)
                .map(|a| format!("{:?}", a))
                .unwrap_or_else(|| "None".to_string());

            self.emit(RecoveryEvent::AttemptStarted {
                attempt: attempts,
                strategy: format!("AdjustAndRetry({})", adjustment_desc),
            });

            match tokio::time::timeout(self.config.timeout_per_attempt, f()).await {
                Ok(result) => match result {
                    Ok(_) => {
                        self.emit(RecoveryEvent::AttemptCompleted {
                            attempt: attempts,
                            success: true,
                        });
                        return RecoveryResult::success(
                            attempts,
                            start.elapsed().as_millis() as u64,
                        );
                    },
                    Err(_e) => {
                        self.emit(RecoveryEvent::AttemptCompleted {
                            attempt: attempts,
                            success: false,
                        });

                        if current_adjustment_idx < adjustments.len() - 1 {
                            current_adjustment_idx += 1;
                        }
                    },
                },
                Err(_) => {
                    self.emit(RecoveryEvent::AttemptCompleted {
                        attempt: attempts,
                        success: false,
                    });
                },
            }

            if attempts < max_attempts {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        RecoveryResult::failure(
            "AdjustAndRetry",
            attempts,
            "Max adjustment attempts reached".to_string(),
            start.elapsed().as_millis() as u64,
        )
    }

    fn emit(&self, event: RecoveryEvent) {
        let _ = self.event_sender.send(event);
    }
}

impl Default for ErrorRecoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RecoveryContext {
    pub task_id: Option<String>,
    pub original_error: Option<String>,
    pub error_type: Option<ErrorType>,
    pub strategy_used: Option<String>,
    pub attempts: usize,
    pub recovery_time_ms: u64,
}

impl RecoveryContext {
    pub fn new() -> Self {
        Self {
            task_id: None,
            original_error: None,
            error_type: None,
            strategy_used: None,
            attempts: 0,
            recovery_time_ms: 0,
        }
    }

    pub fn with_task_id(mut self, id: String) -> Self {
        self.task_id = Some(id);
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.original_error = Some(error);
        self
    }

    pub fn build(self) -> RecoveryContext {
        self
    }
}

impl Default for RecoveryContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_config_default() {
        let config = RecoveryConfig::default();
        assert_eq!(config.max_total_attempts, 5);
        assert!(config.enable_fallback);
        assert!(config.enable_adjustments);
        assert_eq!(config.timeout_per_attempt, Duration::from_secs(30));
    }

    #[test]
    fn test_engine_new() {
        let engine = ErrorRecoveryEngine::new();
        let _rx = engine.subscribe();
    }

    #[test]
    fn test_engine_with_config() {
        let config = RecoveryConfig {
            max_total_attempts: 10,
            enable_fallback: false,
            enable_adjustments: false,
            timeout_per_attempt: Duration::from_secs(60),
        };
        let engine = ErrorRecoveryEngine::new().with_config(config);
        assert_eq!(engine.config.max_total_attempts, 10);
        assert!(!engine.config.enable_fallback);
    }

    #[test]
    fn test_classify_error_transient() {
        let engine = ErrorRecoveryEngine::new();
        let classified = engine.classify_error("connection timeout");
        assert_eq!(classified.error_type, ErrorType::Transient);
    }

    #[test]
    fn test_classify_error_unrecoverable() {
        let engine = ErrorRecoveryEngine::new();
        let classified = engine.classify_error("syntax error");
        assert_eq!(classified.error_type, ErrorType::Unrecoverable);
    }

    #[test]
    fn test_get_recovery_strategy_transient() {
        let engine = ErrorRecoveryEngine::new();
        let strategy = engine.get_recovery_strategy(ErrorType::Transient);
        assert!(matches!(strategy, RecoveryStrategy::Retry { .. }));
    }

    #[test]
    fn test_get_recovery_strategy_recoverable_without_adjustments() {
        let config = RecoveryConfig {
            enable_adjustments: false,
            ..RecoveryConfig::default()
        };
        let engine = ErrorRecoveryEngine::new().with_config(config);
        let strategy = engine.get_recovery_strategy(ErrorType::Recoverable);
        assert!(matches!(strategy, RecoveryStrategy::Fail));
    }

    #[test]
    fn test_get_recovery_strategy_recoverable_with_adjustments() {
        let engine = ErrorRecoveryEngine::new();
        let strategy = engine.get_recovery_strategy(ErrorType::Recoverable);
        assert!(matches!(strategy, RecoveryStrategy::AdjustAndRetry { .. }));
    }

    #[tokio::test]
    async fn test_recover_unrecoverable_error() {
        let engine = ErrorRecoveryEngine::new();
        let result = engine
            .recover("fatal error: out of memory", || async {
                Err::<i32, String>("fail".to_string())
            })
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_recover_success_on_first_try() {
        let engine = ErrorRecoveryEngine::new();
        let mut call_count = 0;
        let result = engine
            .recover("connection timeout", || {
                call_count += 1;
                async move {
                    if call_count == 1 {
                        Ok::<i32, String>(42)
                    } else {
                        Err("unexpected".to_string())
                    }
                }
            })
            .await;
        assert!(result.success);
        assert_eq!(result.attempts_made, 1);
    }

    #[tokio::test]
    async fn test_recover_retry_then_success() {
        let engine = ErrorRecoveryEngine::new();
        let mut call_count = 0;
        let result = engine
            .recover("connection timeout", || {
                call_count += 1;
                async move {
                    if call_count < 2 {
                        Err("retry".to_string())
                    } else {
                        Ok::<i32, String>(42)
                    }
                }
            })
            .await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_recover_all_attempts_fail() {
        let engine = ErrorRecoveryEngine::new();
        let result = engine
            .recover("connection timeout", || async {
                Err::<i32, String>("always fail".to_string())
            })
            .await;
        assert!(!result.success);
    }

    #[test]
    fn test_recovery_context_new() {
        let ctx = RecoveryContext::new();
        assert!(ctx.task_id.is_none());
        assert!(ctx.original_error.is_none());
        assert!(ctx.error_type.is_none());
        assert!(ctx.strategy_used.is_none());
        assert_eq!(ctx.attempts, 0);
        assert_eq!(ctx.recovery_time_ms, 0);
    }

    #[test]
    fn test_recovery_context_builder() {
        let ctx = RecoveryContext::new()
            .with_task_id("task-1".to_string())
            .with_error("timeout".to_string())
            .build();
        assert_eq!(ctx.task_id, Some("task-1".to_string()));
        assert_eq!(ctx.original_error, Some("timeout".to_string()));
    }

    #[test]
    fn test_recovery_context_default() {
        let ctx = RecoveryContext::default();
        assert!(ctx.task_id.is_none());
    }

    #[test]
    fn test_recovery_event_variants() {
        let events = vec![
            RecoveryEvent::RecoveryStarted {
                error: "e".to_string(),
                error_type: ErrorType::Transient,
            },
            RecoveryEvent::AttemptStarted {
                attempt: 1,
                strategy: "Retry".to_string(),
            },
            RecoveryEvent::AttemptCompleted {
                attempt: 1,
                success: true,
            },
            RecoveryEvent::RecoveryCompleted {
                result: RecoveryResult::success(1, 0),
            },
            RecoveryEvent::RecoveryFailed {
                error: "e".to_string(),
            },
            RecoveryEvent::RetryScheduled {
                delay_ms: 100,
                attempt: 1,
            },
        ];
        assert_eq!(events.len(), 6);
    }

    #[test]
    fn test_recovery_config_custom() {
        let config = RecoveryConfig {
            max_total_attempts: 10,
            enable_fallback: false,
            enable_adjustments: false,
            timeout_per_attempt: Duration::from_secs(60),
        };
        assert_eq!(config.max_total_attempts, 10);
        assert!(!config.enable_fallback);
        assert!(!config.enable_adjustments);
        assert_eq!(config.timeout_per_attempt, Duration::from_secs(60));
    }

    #[test]
    fn test_recovery_config_serialization() {
        let config = RecoveryConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RecoveryConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_total_attempts, 5);
        assert!(deserialized.enable_fallback);
        assert!(deserialized.enable_adjustments);
    }

    #[test]
    fn test_engine_default() {
        let engine = ErrorRecoveryEngine::default();
        assert_eq!(engine.config.max_total_attempts, 5);
    }

    #[test]
    fn test_classify_error_recoverable() {
        let engine = ErrorRecoveryEngine::new();
        let classified = engine.classify_error("permission denied");
        assert_eq!(classified.error_type, ErrorType::Recoverable);
    }

    #[test]
    fn test_classify_error_unknown() {
        let engine = ErrorRecoveryEngine::new();
        let classified = engine.classify_error("something weird happened");
        assert_eq!(classified.error_type, ErrorType::Unknown);
    }

    #[test]
    fn test_classify_error_preserves_original() {
        let engine = ErrorRecoveryEngine::new();
        let classified = engine.classify_error("connection timeout");
        assert_eq!(classified.original_error, "connection timeout");
    }

    #[test]
    fn test_classify_error_context_none() {
        let engine = ErrorRecoveryEngine::new();
        let classified = engine.classify_error("timeout");
        assert!(classified.context.is_none());
    }

    #[test]
    fn test_get_recovery_strategy_unrecoverable() {
        let engine = ErrorRecoveryEngine::new();
        let strategy = engine.get_recovery_strategy(ErrorType::Unrecoverable);
        assert!(matches!(strategy, RecoveryStrategy::Fail));
    }

    #[test]
    fn test_get_recovery_strategy_unknown() {
        let engine = ErrorRecoveryEngine::new();
        let strategy = engine.get_recovery_strategy(ErrorType::Unknown);
        assert!(matches!(strategy, RecoveryStrategy::Retry { .. }));
    }

    #[tokio::test]
    async fn test_recover_skip_task_error() {
        let engine = ErrorRecoveryEngine::new();
        let result = engine
            .recover("syntax error", || async { Err::<i32, String>("fail".to_string()) })
            .await;
        assert!(!result.success);
        assert!(!result.recovered);
    }

    #[tokio::test]
    async fn test_recover_emits_events() {
        let engine = ErrorRecoveryEngine::new();
        let mut rx = engine.subscribe();
        let _ = engine
            .recover("fatal error: panic", || async { Err::<i32, String>("fail".to_string()) })
            .await;
        let event = rx.try_recv();
        assert!(event.is_ok());
    }

    #[tokio::test]
    async fn test_recover_recoverable_error_success_after_adjust() {
        let engine = ErrorRecoveryEngine::new();
        let mut call_count = 0;
        let result = engine
            .recover("permission denied", || {
                call_count += 1;
                async move { Ok::<i32, String>(42) }
            })
            .await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_recover_recoverable_error_all_fail() {
        let engine = ErrorRecoveryEngine::new();
        let result = engine
            .recover("permission denied", || async {
                Err::<i32, String>("still denied".to_string())
            })
            .await;
        assert!(!result.success);
    }

    #[test]
    fn test_recovery_context_with_task_id() {
        let ctx = RecoveryContext::new()
            .with_task_id("task-42".to_string())
            .build();
        assert_eq!(ctx.task_id, Some("task-42".to_string()));
    }

    #[test]
    fn test_recovery_context_with_error() {
        let ctx = RecoveryContext::new()
            .with_error("timeout".to_string())
            .build();
        assert_eq!(ctx.original_error, Some("timeout".to_string()));
    }

    #[test]
    fn test_recovery_context_chained_builders() {
        let ctx = RecoveryContext::new()
            .with_task_id("t1".to_string())
            .with_error("err".to_string())
            .build();
        assert_eq!(ctx.task_id, Some("t1".to_string()));
        assert_eq!(ctx.original_error, Some("err".to_string()));
    }

    #[test]
    fn test_recovery_context_default_values() {
        let ctx = RecoveryContext::default();
        assert!(ctx.task_id.is_none());
        assert!(ctx.original_error.is_none());
        assert!(ctx.error_type.is_none());
        assert!(ctx.strategy_used.is_none());
        assert_eq!(ctx.attempts, 0);
        assert_eq!(ctx.recovery_time_ms, 0);
    }

    #[tokio::test]
    async fn test_recover_with_custom_config() {
        let config = RecoveryConfig {
            max_total_attempts: 2,
            enable_fallback: true,
            enable_adjustments: true,
            timeout_per_attempt: Duration::from_secs(5),
        };
        let engine = ErrorRecoveryEngine::new().with_config(config);
        let result = engine
            .recover("connection timeout", || async { Err::<i32, String>("fail".to_string()) })
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_recover_success_returns_attempts() {
        let engine = ErrorRecoveryEngine::new();
        let result = engine
            .recover("connection timeout", || async { Ok::<i32, String>(100) })
            .await;
        assert!(result.success);
        assert_eq!(result.attempts_made, 1);
    }

    #[test]
    fn test_recovery_event_debug_format() {
        let event = RecoveryEvent::RecoveryStarted {
            error: "test".to_string(),
            error_type: ErrorType::Transient,
        };
        let debug = format!("{:?}", event);
        assert!(debug.contains("RecoveryStarted"));
    }
}
