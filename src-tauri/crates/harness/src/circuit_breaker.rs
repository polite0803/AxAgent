// SPDX-License-Identifier: AGPL-3.0-only
//! 熔断器契约
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}
impl CircuitState {
    pub fn as_str(&self) -> &'static str {
        match self {
            CircuitState::Closed => "closed",
            CircuitState::Open => "open",
            CircuitState::HalfOpen => "half_open",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub recovery_timeout_secs: u64,
    pub half_open_max_requests: u32,
}
impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self { failure_threshold: 5, recovery_timeout_secs: 30, half_open_max_requests: 1 }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerSnapshot {
    pub state: CircuitState,
    pub failure_count: u32,
    pub last_failure_secs_ago: Option<u64>,
    pub total_success: u64,
    pub total_failure: u64,
}

pub trait CircuitBreaker: Send + Sync {
    fn is_allowed(&self) -> bool;
    fn record_success(&self);
    fn record_failure(&self);
    fn reset(&self);
    fn snapshot(&self) -> CircuitBreakerSnapshot;
    fn config(&self) -> &CircuitBreakerConfig;
}
