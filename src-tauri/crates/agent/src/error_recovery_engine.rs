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
            }
            RecoveryStrategy::AdjustAndRetry {
                max_attempts,
                adjustments,
            } => {
                self.adjust_and_retry(f, *max_attempts, adjustments, start)
                    .await
            }
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
            }
            RecoveryStrategy::SkipTask => {
                RecoveryResult::skipped(start.elapsed().as_millis() as u64)
            }
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
                        }
                        Err(e) => {
                            last_error = e;
                            self.emit(RecoveryEvent::AttemptCompleted {
                                attempt,
                                success: false,
                            });
                        }
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
            }
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
                }
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
                }
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
                    }
                    Err(_e) => {
                        self.emit(RecoveryEvent::AttemptCompleted {
                            attempt: attempts,
                            success: false,
                        });

                        if current_adjustment_idx < adjustments.len() - 1 {
                            current_adjustment_idx += 1;
                        }
                    }
                },
                Err(_) => {
                    self.emit(RecoveryEvent::AttemptCompleted {
                        attempt: attempts,
                        success: false,
                    });
                }
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
