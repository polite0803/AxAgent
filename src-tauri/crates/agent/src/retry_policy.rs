use crate::error_classifier::{ErrorClassifier, ErrorType};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: usize,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub exponential_backoff: bool,
    pub jitter: bool,
    pub retry_on: Vec<ErrorType>,
}

impl RetryPolicy {
    pub fn new(max_attempts: usize) -> Self {
        Self {
            max_attempts,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            exponential_backoff: true,
            jitter: true,
            retry_on: vec![ErrorType::Transient, ErrorType::Unknown],
        }
    }

    pub fn with_base_delay(mut self, delay: Duration) -> Self {
        self.base_delay = delay;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    pub fn with_exponential_backoff(mut self, enabled: bool) -> Self {
        self.exponential_backoff = enabled;
        self
    }

    pub fn with_jitter(mut self, enabled: bool) -> Self {
        self.jitter = enabled;
        self
    }

    pub fn should_retry(&self, attempt: usize, error_type: ErrorType) -> bool {
        if attempt >= self.max_attempts {
            return false;
        }

        self.retry_on.contains(&error_type)
    }

    pub fn next_delay(&self, attempt: usize) -> Duration {
        let base = if self.exponential_backoff {
            self.base_delay * 2u32.pow(attempt as u32)
        } else {
            self.base_delay
        };

        let delay = base.min(self.max_delay);

        if self.jitter {
            let jitter_range = delay.as_millis() as f64 * 0.1;
            let jitter = (fastrand::f64() - 0.5) * 2.0 * jitter_range;
            let millis = delay.as_millis() as i64 + jitter as i64;
            Duration::from_millis(millis.max(0) as u64)
        } else {
            delay
        }
    }

    pub fn total_timeout(&self) -> Duration {
        let mut total = Duration::ZERO;
        for i in 0..self.max_attempts {
            total += self.next_delay(i);
        }
        total
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(3)
    }
}

pub struct RetryState {
    pub current_attempt: usize,
    pub total_delay_ms: u64,
    pub errors: Vec<String>,
}

impl RetryState {
    pub fn new() -> Self {
        Self {
            current_attempt: 0,
            total_delay_ms: 0,
            errors: Vec::new(),
        }
    }

    pub fn increment(&mut self, error: String, delay_ms: u64) {
        self.current_attempt += 1;
        self.total_delay_ms += delay_ms;
        self.errors.push(error);
    }

    pub fn reset(&mut self) {
        self.current_attempt = 0;
        self.total_delay_ms = 0;
        self.errors.clear();
    }

    pub fn can_continue(&self, max_attempts: usize) -> bool {
        self.current_attempt < max_attempts
    }
}

impl Default for RetryState {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn with_retry<F, Fut, T, E>(policy: &RetryPolicy, mut f: F) -> Result<T, RetryError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let classifier = ErrorClassifier::new();
    let mut state = RetryState::new();
    let start = std::time::Instant::now();

    loop {
        match f().await {
            Ok(result) => {
                return Ok(result);
            }
            Err(error) => {
                let error_str = error.to_string();
                let error_type = classifier.classify(&error_str);

                if !policy.should_retry(state.current_attempt, error_type) {
                    return Err(RetryError::Exhausted {
                        errors: state.errors,
                        attempts: state.current_attempt,
                        last_error: error_str,
                        elapsed: start.elapsed(),
                    });
                }

                let delay = policy.next_delay(state.current_attempt);
                state.increment(error_str.clone(), delay.as_millis() as u64);

                if state.current_attempt >= policy.max_attempts {
                    return Err(RetryError::Exhausted {
                        errors: state.errors,
                        attempts: state.current_attempt,
                        last_error: error_str,
                        elapsed: start.elapsed(),
                    });
                }

                tokio::time::sleep(delay).await;
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RetryError {
    #[error("Retry exhausted after {attempts} attempts")]
    Exhausted {
        errors: Vec<String>,
        attempts: usize,
        last_error: String,
        elapsed: Duration,
    },

    #[error("Retry cancelled")]
    Cancelled,

    #[error("Retry timeout after {0:?}")]
    Timeout(Duration),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let policy = RetryPolicy::new(5)
            .with_base_delay(Duration::from_secs(1))
            .with_exponential_backoff(true)
            .with_jitter(false);

        assert_eq!(policy.next_delay(0), Duration::from_secs(1));
        assert_eq!(policy.next_delay(1), Duration::from_secs(2));
        assert_eq!(policy.next_delay(2), Duration::from_secs(4));
        assert_eq!(policy.next_delay(3), Duration::from_secs(8));
    }

    #[test]
    fn test_max_delay() {
        let policy = RetryPolicy::new(5)
            .with_base_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(5))
            .with_exponential_backoff(true)
            .with_jitter(false);

        assert_eq!(policy.next_delay(0), Duration::from_secs(1));
        assert_eq!(policy.next_delay(1), Duration::from_secs(2));
        assert_eq!(policy.next_delay(2), Duration::from_secs(4));
        assert_eq!(policy.next_delay(3), Duration::from_secs(5)); // capped
    }
}
