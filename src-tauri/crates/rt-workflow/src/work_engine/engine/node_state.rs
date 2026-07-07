// SPDX-License-Identifier: AGPL-3.0-only

//! Internal tracking types: circuit breaker, backoff computation, node result.

use axagent_harness::workflow_types::{BackoffType, WorkflowNode};

use crate::work_engine::{NodeError, NodeOutput};

// ── 内部追踪类型 ──

/// 断路器状态（按节点追踪）
#[derive(Debug, Clone)]
pub(crate) struct NodeCircuitBreaker {
    failure_count: u32,
    failure_threshold: u32,
    reset_timeout_ms: u64,
    opened_at: Option<u64>,
}

impl NodeCircuitBreaker {
    pub(crate) fn new() -> Self {
        Self {
            failure_count: 0,
            failure_threshold: 3,
            reset_timeout_ms: 60_000,
            opened_at: None,
        }
    }

    pub(crate) fn is_open(&self, now_ms: u64) -> bool {
        if let Some(opened_at) = self.opened_at {
            now_ms < opened_at + self.reset_timeout_ms
        } else {
            false
        }
    }

    pub(crate) fn record_success(&mut self) {
        self.failure_count = 0;
        self.opened_at = None;
    }

    pub(crate) fn record_failure(&mut self, now_ms: u64) {
        self.failure_count += 1;
        if self.failure_count >= self.failure_threshold {
            self.opened_at = Some(now_ms);
        }
    }
}

pub(crate) fn compute_backoff(
    backoff_type: BackoffType,
    base_delay_ms: u64,
    max_delay_ms: u64,
    attempt: u32,
) -> u64 {
    let delay = match backoff_type {
        BackoffType::Fixed => base_delay_ms,
        BackoffType::Linear => base_delay_ms.saturating_mul(attempt as u64),
        BackoffType::Exponential => {
            let exp = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
            base_delay_ms.saturating_mul(exp)
        },
    };
    delay.min(max_delay_ms)
}

pub(crate) struct NodeResult {
    pub(crate) node_id: String,
    pub(crate) node: WorkflowNode,
    pub(crate) input_snapshot: serde_json::Value,
    pub(crate) started_at: i64,
    pub(crate) elapsed_ms: u64,
    pub(crate) dispatch_result: Result<Result<NodeOutput, NodeError>, tokio::time::error::Elapsed>,
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaker_default_is_not_open() {
        let cb = NodeCircuitBreaker::new();
        assert!(!cb.is_open(0));
    }

    #[test]
    fn breaker_opens_after_threshold() {
        let mut cb = NodeCircuitBreaker::new();
        let mut now = 1000;
        cb.record_failure(now); // 1
        cb.record_failure(now); // 2
        cb.record_failure(now); // 3 → opens
        now += 1;
        assert!(cb.is_open(now));
    }

    #[test]
    fn breaker_resets_after_success() {
        let mut cb = NodeCircuitBreaker::new();
        let now = 1000;
        cb.record_failure(now);
        cb.record_failure(now);
        cb.record_failure(now); // opens
        assert!(cb.is_open(now + 1));
        cb.record_success();
        assert!(!cb.is_open(now + 1));
    }

    #[test]
    fn breaker_half_open_after_timeout() {
        let mut cb = NodeCircuitBreaker::new();
        let open_time = 1000;
        cb.record_failure(open_time);
        cb.record_failure(open_time);
        cb.record_failure(open_time); // opens at open_time
        // Still open right after
        assert!(cb.is_open(open_time + 1000));
        // Closed after reset timeout (60_000 ms)
        assert!(!cb.is_open(open_time + 61_000));
    }

    #[test]
    fn backoff_fixed() {
        assert_eq!(compute_backoff(BackoffType::Fixed, 1000, 10_000, 1), 1000);
        assert_eq!(compute_backoff(BackoffType::Fixed, 1000, 10_000, 5), 1000);
    }

    #[test]
    fn backoff_linear() {
        assert_eq!(compute_backoff(BackoffType::Linear, 1000, 10_000, 1), 1000);
        assert_eq!(compute_backoff(BackoffType::Linear, 1000, 10_000, 3), 3000);
        assert_eq!(compute_backoff(BackoffType::Linear, 1000, 10_000, 20), 10_000); // capped
    }

    #[test]
    fn backoff_exponential() {
        assert_eq!(compute_backoff(BackoffType::Exponential, 1000, 10_000, 0), 1000); // 2^0 = 1
        assert_eq!(compute_backoff(BackoffType::Exponential, 1000, 10_000, 1), 2000); // 2^1 = 2
        assert_eq!(compute_backoff(BackoffType::Exponential, 1000, 10_000, 2), 4000); // 2^2 = 4
        assert_eq!(compute_backoff(BackoffType::Exponential, 1000, 10_000, 4), 10_000); // 2^4=16 → 16000 capped
    }
}
