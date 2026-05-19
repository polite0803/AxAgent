use crate::recovery_strategies::{ErrorClassifier, ErrorType};
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
            },
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
            },
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

    #[test]
    fn test_retry_policy_new() {
        let policy = RetryPolicy::new(3);
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay, Duration::from_secs(1));
        assert_eq!(policy.max_delay, Duration::from_secs(60));
        assert!(policy.exponential_backoff);
        assert!(policy.jitter);
        assert_eq!(policy.retry_on, vec![ErrorType::Transient, ErrorType::Unknown]);
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
    }

    #[test]
    fn test_with_base_delay() {
        let policy = RetryPolicy::new(3).with_base_delay(Duration::from_millis(500));
        assert_eq!(policy.base_delay, Duration::from_millis(500));
    }

    #[test]
    fn test_with_max_delay() {
        let policy = RetryPolicy::new(3).with_max_delay(Duration::from_secs(120));
        assert_eq!(policy.max_delay, Duration::from_secs(120));
    }

    #[test]
    fn test_with_exponential_backoff_disabled() {
        let policy = RetryPolicy::new(5)
            .with_base_delay(Duration::from_secs(2))
            .with_exponential_backoff(false)
            .with_jitter(false);

        assert_eq!(policy.next_delay(0), Duration::from_secs(2));
        assert_eq!(policy.next_delay(1), Duration::from_secs(2));
        assert_eq!(policy.next_delay(2), Duration::from_secs(2));
    }

    #[test]
    fn test_with_jitter_disabled() {
        let policy = RetryPolicy::new(5)
            .with_base_delay(Duration::from_secs(1))
            .with_exponential_backoff(true)
            .with_jitter(false);

        assert_eq!(policy.next_delay(0), Duration::from_secs(1));
        assert_eq!(policy.next_delay(1), Duration::from_secs(2));
        assert_eq!(policy.next_delay(2), Duration::from_secs(4));
    }

    #[test]
    fn test_should_retry_within_limit() {
        let policy = RetryPolicy::new(3);
        assert!(policy.should_retry(0, ErrorType::Transient));
        assert!(policy.should_retry(1, ErrorType::Transient));
        assert!(policy.should_retry(2, ErrorType::Unknown));
    }

    #[test]
    fn test_should_retry_at_max() {
        let policy = RetryPolicy::new(3);
        assert!(!policy.should_retry(3, ErrorType::Transient));
    }

    #[test]
    fn test_should_retry_wrong_error_type() {
        let policy = RetryPolicy::new(3);
        assert!(!policy.should_retry(0, ErrorType::Unrecoverable));
        assert!(!policy.should_retry(0, ErrorType::Recoverable));
    }

    #[test]
    fn test_should_retry_transient_and_unknown_only() {
        let policy = RetryPolicy::new(3);
        assert!(policy.should_retry(0, ErrorType::Transient));
        assert!(policy.should_retry(0, ErrorType::Unknown));
        assert!(!policy.should_retry(0, ErrorType::Recoverable));
        assert!(!policy.should_retry(0, ErrorType::Unrecoverable));
    }

    #[test]
    fn test_next_delay_with_jitter_in_range() {
        let policy = RetryPolicy::new(5)
            .with_base_delay(Duration::from_secs(1))
            .with_exponential_backoff(false)
            .with_jitter(true);

        for _ in 0..100 {
            let delay = policy.next_delay(0);
            let millis = delay.as_millis() as f64;
            assert!((900.0..=1100.0).contains(&millis));
        }
    }

    #[test]
    fn test_total_timeout() {
        let policy = RetryPolicy::new(3)
            .with_base_delay(Duration::from_secs(1))
            .with_exponential_backoff(false)
            .with_jitter(false);

        let total = policy.total_timeout();
        assert_eq!(total, Duration::from_secs(3));
    }

    #[test]
    fn test_total_timeout_with_exponential() {
        let policy = RetryPolicy::new(3)
            .with_base_delay(Duration::from_secs(1))
            .with_exponential_backoff(true)
            .with_jitter(false);

        let total = policy.total_timeout();
        assert_eq!(total, Duration::from_secs(7));
    }

    #[test]
    fn test_retry_state_new() {
        let state = RetryState::new();
        assert_eq!(state.current_attempt, 0);
        assert_eq!(state.total_delay_ms, 0);
        assert!(state.errors.is_empty());
    }

    #[test]
    fn test_retry_state_default() {
        let state = RetryState::default();
        assert_eq!(state.current_attempt, 0);
    }

    #[test]
    fn test_retry_state_increment() {
        let mut state = RetryState::new();
        state.increment("error1".to_string(), 100);
        assert_eq!(state.current_attempt, 1);
        assert_eq!(state.total_delay_ms, 100);
        assert_eq!(state.errors.len(), 1);
        assert_eq!(state.errors[0], "error1");
    }

    #[test]
    fn test_retry_state_increment_multiple() {
        let mut state = RetryState::new();
        state.increment("error1".to_string(), 100);
        state.increment("error2".to_string(), 200);
        assert_eq!(state.current_attempt, 2);
        assert_eq!(state.total_delay_ms, 300);
        assert_eq!(state.errors.len(), 2);
    }

    #[test]
    fn test_retry_state_reset() {
        let mut state = RetryState::new();
        state.increment("error1".to_string(), 100);
        state.increment("error2".to_string(), 200);
        state.reset();
        assert_eq!(state.current_attempt, 0);
        assert_eq!(state.total_delay_ms, 0);
        assert!(state.errors.is_empty());
    }

    #[test]
    fn test_retry_state_can_continue() {
        let mut state = RetryState::new();
        assert!(state.can_continue(3));
        state.increment("e".to_string(), 10);
        assert!(state.can_continue(3));
        state.increment("e".to_string(), 10);
        assert!(state.can_continue(3));
        state.increment("e".to_string(), 10);
        assert!(!state.can_continue(3));
    }

    #[test]
    fn test_retry_policy_serialization() {
        let policy = RetryPolicy::new(5)
            .with_base_delay(Duration::from_millis(500))
            .with_max_delay(Duration::from_secs(30))
            .with_exponential_backoff(true)
            .with_jitter(false);

        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: RetryPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_attempts, 5);
        assert_eq!(deserialized.base_delay, Duration::from_millis(500));
        assert_eq!(deserialized.max_delay, Duration::from_secs(30));
        assert!(deserialized.exponential_backoff);
        assert!(!deserialized.jitter);
    }

    #[tokio::test]
    async fn test_with_retry_success() {
        let policy = RetryPolicy::new(3)
            .with_base_delay(Duration::from_millis(1))
            .with_jitter(false);
        let result = with_retry(&policy, || async { Ok::<i32, String>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_retry_eventual_success() {
        let policy = RetryPolicy::new(3)
            .with_base_delay(Duration::from_millis(1))
            .with_jitter(false);
        let mut count = 0;
        let result = with_retry(&policy, || {
            count += 1;
            async move {
                if count < 2 {
                    Err("timeout".to_string())
                } else {
                    Ok::<i32, String>(99)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 99);
    }

    #[tokio::test]
    async fn test_with_retry_exhausted() {
        let policy = RetryPolicy::new(2)
            .with_base_delay(Duration::from_millis(1))
            .with_jitter(false);
        let result =
            with_retry(&policy, || async { Err::<i32, String>("always fails".to_string()) }).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            RetryError::Exhausted { attempts, .. } => assert!(attempts > 0),
            _ => panic!("Expected Exhausted"),
        }
    }

    #[tokio::test]
    async fn test_with_retry_unrecoverable_stops() {
        let policy = RetryPolicy::new(5)
            .with_base_delay(Duration::from_millis(1))
            .with_jitter(false);
        let result =
            with_retry(&policy, || async { Err::<i32, String>("syntax error".to_string()) }).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_retry_error_exhausted_display() {
        let err = RetryError::Exhausted {
            errors: vec!["e1".to_string()],
            attempts: 3,
            last_error: "fail".to_string(),
            elapsed: Duration::from_secs(5),
        };
        assert!(err.to_string().contains("3"));
    }

    #[test]
    fn test_retry_error_cancelled_display() {
        let err = RetryError::Cancelled;
        assert!(err.to_string().contains("cancelled"));
    }

    #[test]
    fn test_retry_error_timeout_display() {
        let err = RetryError::Timeout(Duration::from_secs(10));
        assert!(err.to_string().contains("10"));
    }

    #[test]
    fn test_next_delay_zero_attempt() {
        let policy = RetryPolicy::new(3)
            .with_base_delay(Duration::from_secs(1))
            .with_exponential_backoff(true)
            .with_jitter(false);
        assert_eq!(policy.next_delay(0), Duration::from_secs(1));
    }

    #[test]
    fn test_next_delay_large_attempt_capped() {
        let policy = RetryPolicy::new(10)
            .with_base_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(60))
            .with_exponential_backoff(true)
            .with_jitter(false);
        let delay = policy.next_delay(10);
        assert_eq!(delay, Duration::from_secs(60));
    }
}
