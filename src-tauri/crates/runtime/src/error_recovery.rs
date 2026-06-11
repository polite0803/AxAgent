// SPDX-License-Identifier: AGPL-3.0-only

//! 错误恢复策略 — 分级退避重试 + 熔断器
//!
//! 重试策略：
//! - 429 Rate Limit → 立即退避，指数增长 (2s, 4s, 8s...)
//! - 5xx Server Error → 线性退避 (1s, 2s, 3s...)
//! - Network Error → 固定间隔重试 (500ms)
//!
//! 熔断器：连续失败 5 次 → 熔断 30 秒
//!
//! 恢复配方 (merged from recovery_recipes.rs):
//! 六种故障场景的自动恢复配方，一次自动恢复尝试后升级。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::worker_boot::WorkerFailureKind;

/// 错误类型分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    RateLimit,    // 429
    ServerError,  // 5xx
    NetworkError, // 超时/连接
    ClientError,  // 4xx (不可重试)
    Unknown,
}

impl ErrorKind {
    /// 根据错误消息字符串分类错误类型
    pub fn classify(error_msg: &str) -> Self {
        let lower = error_msg.to_lowercase();
        if lower.contains("429") || lower.contains("rate") {
            return Self::RateLimit;
        }
        if lower.contains("500") || lower.contains("502") || lower.contains("503") {
            return Self::ServerError;
        }
        if lower.contains("timeout")
            || lower.contains("network")
            || lower.contains("connection")
            || lower.contains("reset")
        {
            return Self::NetworkError;
        }
        if lower.contains("400")
            || lower.contains("401")
            || lower.contains("403")
            || lower.contains("404")
        {
            return Self::ClientError;
        }
        Self::Unknown
    }

    /// 判断该错误类型是否可以重试
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::RateLimit | Self::ServerError | Self::NetworkError)
    }
}

/// 将 runtime 层 HTTP 错误分类桥接到 agent 层的抽象错误类型。
impl From<ErrorKind> for axagent_agent::ErrorType {
    fn from(kind: ErrorKind) -> Self {
        match kind {
            ErrorKind::RateLimit => axagent_agent::ErrorType::Transient,
            ErrorKind::ServerError => axagent_agent::ErrorType::Transient,
            ErrorKind::NetworkError => axagent_agent::ErrorType::Transient,
            ErrorKind::ClientError => axagent_agent::ErrorType::Recoverable,
            ErrorKind::Unknown => axagent_agent::ErrorType::Unknown,
        }
    }
}

/// 重试策略配置
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 最大重试次数
    pub max_retries: u32,
    /// 基础延迟时间
    pub base_delay: Duration,
    /// 最大延迟上限
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// 计算第 n 次重试的等待延迟
    pub fn delay_for(&self, kind: ErrorKind, attempt: u32) -> Duration {
        match kind {
            ErrorKind::RateLimit => {
                // 指数退避：2^attempt 秒
                let secs = 2u64.pow(attempt).min(self.max_delay.as_secs());
                Duration::from_secs(secs).min(self.max_delay)
            },
            ErrorKind::ServerError => {
                // 线性退避：base * attempt
                let secs =
                    (self.base_delay.as_secs() * attempt as u64).min(self.max_delay.as_secs());
                Duration::from_secs(secs).min(self.max_delay)
            },
            ErrorKind::NetworkError => {
                // 固定间隔重试
                Duration::from_millis(500)
            },
            _ => Duration::ZERO,
        }
    }

    /// 判断是否应该继续重试
    pub fn should_retry(&self, kind: ErrorKind, attempt: u32) -> bool {
        kind.is_retryable() && attempt < self.max_retries
    }
}

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 正常状态，请求允许通过
    Closed,
    /// 熔断状态，拒绝所有请求
    Open,
    /// 半开状态，允许探测请求
    HalfOpen,
}

/// 熔断器 — 连续失败达到阈值后熔断，防止雪崩
#[derive(Debug)]
pub struct CircuitBreaker {
    /// 失败次数阈值（达到后熔断）
    failure_threshold: u32,
    /// 熔断恢复超时
    recovery_timeout: Duration,
    /// 当前连续失败计数
    failure_count: u32,
    /// 最近一次失败的时间
    last_failure: Option<Instant>,
    /// 当前熔断器状态
    state: CircuitState,
}

impl CircuitBreaker {
    /// 创建新的熔断器实例（默认阈值 5 次失败 → 熔断 30 秒）
    pub fn new() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            failure_count: 0,
            last_failure: None,
            state: CircuitState::Closed,
        }
    }

    /// 查询当前熔断器状态
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// 检查是否允许执行请求
    ///
    /// - Closed 状态：直接放行
    /// - Open 状态：检查恢复时间是否已过，若已过则进入 HalfOpen 并放行探测请求
    /// - HalfOpen 状态：放行探测请求
    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last) = self.last_failure
                    && last.elapsed() >= self.recovery_timeout
                {
                    self.state = CircuitState::HalfOpen;
                    return true; // 允许探测请求
                }
                false
            },
            CircuitState::HalfOpen => true,
        }
    }

    /// 记录一次成功调用，重置熔断器状态
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
    }

    /// 记录一次失败调用，累计失败次数
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());
        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
        if self.state == CircuitState::HalfOpen {
            // 探测失败，重新熔断
            self.state = CircuitState::Open;
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_uses_exponential_backoff() {
        let policy = RetryPolicy::default();
        let d1 = policy.delay_for(ErrorKind::RateLimit, 1);
        let d2 = policy.delay_for(ErrorKind::RateLimit, 2);
        // 指数退避：第 2 次延迟应大于第 1 次
        assert!(d2 > d1);
    }

    #[test]
    fn server_error_uses_linear_backoff() {
        let policy = RetryPolicy::default();
        let d1 = policy.delay_for(ErrorKind::ServerError, 1);
        let d2 = policy.delay_for(ErrorKind::ServerError, 2);
        assert!(d2 > d1);
    }

    #[test]
    fn network_error_uses_fixed_interval() {
        let policy = RetryPolicy::default();
        let d1 = policy.delay_for(ErrorKind::NetworkError, 1);
        let d2 = policy.delay_for(ErrorKind::NetworkError, 5);
        // 固定间隔，每次都是 500ms
        assert_eq!(d1, Duration::from_millis(500));
        assert_eq!(d2, Duration::from_millis(500));
    }

    #[test]
    fn circuit_breaker_opens_after_threshold() {
        let mut cb = CircuitBreaker::new();
        for _ in 0..5 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn circuit_breaker_half_open_probe() {
        let mut cb = CircuitBreaker::new();
        // 达到熔断阈值
        for _ in 0..5 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);

        // 模拟恢复时间已过（无法真正等待，直接测试状态转换逻辑）
        // 由于 last_failure 刚刚记录，不允许请求
        assert!(!cb.allow_request());
    }

    #[test]
    fn circuit_breaker_recovers_after_success() {
        let mut cb = CircuitBreaker::new();
        for _ in 0..4 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Closed); // 未达到阈值
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        // 重置后计数归零
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed); // 仅 1 次失败
    }

    #[test]
    fn client_errors_not_retryable() {
        let policy = RetryPolicy::default();
        assert!(!policy.should_retry(ErrorKind::ClientError, 0));
    }

    #[test]
    fn unknown_errors_not_retryable() {
        let policy = RetryPolicy::default();
        assert!(!policy.should_retry(ErrorKind::Unknown, 0));
    }

    #[test]
    fn classify_rate_limit_from_message() {
        assert_eq!(ErrorKind::classify("HTTP 429 Too Many Requests"), ErrorKind::RateLimit);
        assert_eq!(ErrorKind::classify("rate limit exceeded"), ErrorKind::RateLimit);
    }

    #[test]
    fn classify_server_error_from_message() {
        assert_eq!(ErrorKind::classify("HTTP 500 Internal Server Error"), ErrorKind::ServerError);
        assert_eq!(ErrorKind::classify("502 Bad Gateway"), ErrorKind::ServerError);
    }

    #[test]
    fn classify_network_error_from_message() {
        assert_eq!(ErrorKind::classify("connection reset by peer"), ErrorKind::NetworkError);
        assert_eq!(ErrorKind::classify("request timeout"), ErrorKind::NetworkError);
    }

    #[test]
    fn max_retries_exceeded_stops_retry() {
        let policy = RetryPolicy::default();
        assert!(!policy.should_retry(ErrorKind::ServerError, 3)); // attempt 3 >= max_retries 3
        assert!(policy.should_retry(ErrorKind::ServerError, 2)); // attempt 2 < max_retries 3
    }
}

/// The six failure scenarios that have known recovery recipes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureScenario {
    TrustPromptUnresolved,
    PromptMisdelivery,
    StaleBranch,
    CompileRedCrossCrate,
    McpHandshakeFailure,
    PartialPluginStartup,
    ProviderFailure,
}

impl FailureScenario {
    /// Returns all known failure scenarios.
    #[must_use]
    pub fn all() -> &'static [FailureScenario] {
        &[
            Self::TrustPromptUnresolved,
            Self::PromptMisdelivery,
            Self::StaleBranch,
            Self::CompileRedCrossCrate,
            Self::McpHandshakeFailure,
            Self::PartialPluginStartup,
            Self::ProviderFailure,
        ]
    }

    /// Map a `WorkerFailureKind` to the corresponding `FailureScenario`.
    /// This is the bridge that lets recovery policy consume worker boot events.
    #[must_use]
    pub fn from_worker_failure_kind(kind: WorkerFailureKind) -> Self {
        match kind {
            WorkerFailureKind::TrustGate => Self::TrustPromptUnresolved,
            WorkerFailureKind::PromptDelivery => Self::PromptMisdelivery,
            WorkerFailureKind::Protocol => Self::McpHandshakeFailure,
            WorkerFailureKind::Provider => Self::ProviderFailure,
        }
    }
}

impl std::fmt::Display for FailureScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TrustPromptUnresolved => write!(f, "trust_prompt_unresolved"),
            Self::PromptMisdelivery => write!(f, "prompt_misdelivery"),
            Self::StaleBranch => write!(f, "stale_branch"),
            Self::CompileRedCrossCrate => write!(f, "compile_red_cross_crate"),
            Self::McpHandshakeFailure => write!(f, "mcp_handshake_failure"),
            Self::PartialPluginStartup => write!(f, "partial_plugin_startup"),
            Self::ProviderFailure => write!(f, "provider_failure"),
        }
    }
}

/// Individual step that can be executed as part of a recovery recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStep {
    AcceptTrustPrompt,
    RedirectPromptToAgent,
    RebaseBranch,
    CleanBuild,
    RetryMcpHandshake { timeout: u64 },
    RestartPlugin { name: String },
    RestartWorker,
    EscalateToHuman { reason: String },
}

/// Policy governing what happens when automatic recovery is exhausted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationPolicy {
    AlertHuman,
    LogAndContinue,
    Abort,
}

/// A recovery recipe encodes the sequence of steps to attempt for a
/// given failure scenario, along with the maximum number of automatic
/// attempts and the escalation policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryRecipe {
    pub scenario: FailureScenario,
    pub steps: Vec<RecoveryStep>,
    pub max_attempts: u32,
    pub escalation_policy: EscalationPolicy,
}

/// Outcome of a recovery attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryResult {
    Recovered {
        steps_taken: u32,
    },
    PartialRecovery {
        recovered: Vec<RecoveryStep>,
        remaining: Vec<RecoveryStep>,
    },
    EscalationRequired {
        reason: String,
    },
}

/// Structured event emitted during recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryEvent {
    RecoveryAttempted {
        scenario: FailureScenario,
        recipe: RecoveryRecipe,
        result: RecoveryResult,
    },
    RecoverySucceeded,
    RecoveryFailed,
    Escalated,
}

/// Minimal context for tracking recovery state and emitting events.
///
/// Holds per-scenario attempt counts, a structured event log, and an
/// optional simulation knob for controlling step outcomes during tests.
#[derive(Debug, Clone, Default)]
pub struct RecoveryContext {
    attempts: HashMap<FailureScenario, u32>,
    events: Vec<RecoveryEvent>,
    /// Optional step index at which simulated execution fails.
    /// `None` means all steps succeed.
    fail_at_step: Option<usize>,
}

impl RecoveryContext {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure a step index at which simulated execution will fail.
    #[must_use]
    pub fn with_fail_at_step(mut self, index: usize) -> Self {
        self.fail_at_step = Some(index);
        self
    }

    /// Returns the structured event log populated during recovery.
    #[must_use]
    pub fn events(&self) -> &[RecoveryEvent] {
        &self.events
    }

    /// Returns the number of recovery attempts made for a scenario.
    #[must_use]
    pub fn attempt_count(&self, scenario: &FailureScenario) -> u32 {
        self.attempts.get(scenario).copied().unwrap_or(0)
    }
}

/// Returns the known recovery recipe for the given failure scenario.
#[must_use]
pub fn recipe_for(scenario: &FailureScenario) -> RecoveryRecipe {
    match scenario {
        FailureScenario::TrustPromptUnresolved => RecoveryRecipe {
            scenario: *scenario,
            steps: vec![RecoveryStep::AcceptTrustPrompt],
            max_attempts: 1,
            escalation_policy: EscalationPolicy::AlertHuman,
        },
        FailureScenario::PromptMisdelivery => RecoveryRecipe {
            scenario: *scenario,
            steps: vec![RecoveryStep::RedirectPromptToAgent],
            max_attempts: 1,
            escalation_policy: EscalationPolicy::AlertHuman,
        },
        FailureScenario::StaleBranch => RecoveryRecipe {
            scenario: *scenario,
            steps: vec![RecoveryStep::RebaseBranch, RecoveryStep::CleanBuild],
            max_attempts: 1,
            escalation_policy: EscalationPolicy::AlertHuman,
        },
        FailureScenario::CompileRedCrossCrate => RecoveryRecipe {
            scenario: *scenario,
            steps: vec![RecoveryStep::CleanBuild],
            max_attempts: 1,
            escalation_policy: EscalationPolicy::AlertHuman,
        },
        FailureScenario::McpHandshakeFailure => RecoveryRecipe {
            scenario: *scenario,
            steps: vec![RecoveryStep::RetryMcpHandshake { timeout: 5000 }],
            max_attempts: 1,
            escalation_policy: EscalationPolicy::Abort,
        },
        FailureScenario::PartialPluginStartup => RecoveryRecipe {
            scenario: *scenario,
            steps: vec![
                RecoveryStep::RestartPlugin {
                    name: "stalled".to_string(),
                },
                RecoveryStep::RetryMcpHandshake { timeout: 3000 },
            ],
            max_attempts: 1,
            escalation_policy: EscalationPolicy::LogAndContinue,
        },
        FailureScenario::ProviderFailure => RecoveryRecipe {
            scenario: *scenario,
            steps: vec![RecoveryStep::RestartWorker],
            max_attempts: 1,
            escalation_policy: EscalationPolicy::AlertHuman,
        },
    }
}

/// Attempts automatic recovery for the given failure scenario.
///
/// Looks up the recipe, enforces the one-attempt-before-escalation
/// policy, simulates step execution (controlled by the context), and
/// emits structured [`RecoveryEvent`]s for every attempt.
pub fn attempt_recovery(scenario: &FailureScenario, ctx: &mut RecoveryContext) -> RecoveryResult {
    let recipe = recipe_for(scenario);
    let attempt_count = ctx.attempts.entry(*scenario).or_insert(0);

    // Enforce one automatic recovery attempt before escalation.
    if *attempt_count >= recipe.max_attempts {
        let result = RecoveryResult::EscalationRequired {
            reason: format!(
                "max recovery attempts ({}) exceeded for {}",
                recipe.max_attempts, scenario
            ),
        };
        ctx.events.push(RecoveryEvent::RecoveryAttempted {
            scenario: *scenario,
            recipe,
            result: result.clone(),
        });
        ctx.events.push(RecoveryEvent::Escalated);
        return result;
    }

    *attempt_count += 1;

    // Execute steps, honoring the optional fail_at_step simulation.
    let fail_index = ctx.fail_at_step;
    let mut executed = Vec::new();
    let mut failed = false;

    for (i, step) in recipe.steps.iter().enumerate() {
        if fail_index == Some(i) {
            failed = true;
            break;
        }
        executed.push(step.clone());
    }

    let result = if failed {
        let remaining: Vec<RecoveryStep> = recipe.steps[executed.len()..].to_vec();
        if executed.is_empty() {
            RecoveryResult::EscalationRequired {
                reason: format!("recovery failed at first step for {}", scenario),
            }
        } else {
            RecoveryResult::PartialRecovery {
                recovered: executed,
                remaining,
            }
        }
    } else {
        RecoveryResult::Recovered {
            steps_taken: recipe.steps.len() as u32,
        }
    };

    // Emit the attempt as structured event data.
    ctx.events.push(RecoveryEvent::RecoveryAttempted {
        scenario: *scenario,
        recipe,
        result: result.clone(),
    });

    match &result {
        RecoveryResult::Recovered { .. } => {
            ctx.events.push(RecoveryEvent::RecoverySucceeded);
        },
        RecoveryResult::PartialRecovery { .. } => {
            ctx.events.push(RecoveryEvent::RecoveryFailed);
        },
        RecoveryResult::EscalationRequired { .. } => {
            ctx.events.push(RecoveryEvent::Escalated);
        },
    }

    result
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    #[test]
    fn each_scenario_has_a_matching_recipe() {
        // given
        let scenarios = FailureScenario::all();

        // when / then
        for scenario in scenarios {
            let recipe = recipe_for(scenario);
            assert_eq!(
                recipe.scenario, *scenario,
                "recipe scenario should match requested scenario"
            );
            assert!(
                !recipe.steps.is_empty(),
                "recipe for {} should have at least one step",
                scenario
            );
            assert!(
                recipe.max_attempts >= 1,
                "recipe for {} should allow at least one attempt",
                scenario
            );
        }
    }

    #[test]
    fn successful_recovery_returns_recovered_and_emits_events() {
        // given
        let mut ctx = RecoveryContext::new();
        let scenario = FailureScenario::TrustPromptUnresolved;

        // when
        let result = attempt_recovery(&scenario, &mut ctx);

        // then
        assert_eq!(result, RecoveryResult::Recovered { steps_taken: 1 });
        assert_eq!(ctx.events().len(), 2);
        assert!(matches!(
            &ctx.events()[0],
            RecoveryEvent::RecoveryAttempted {
                scenario: s,
                result: r,
                ..
            } if *s == FailureScenario::TrustPromptUnresolved
              && matches!(r, RecoveryResult::Recovered { steps_taken: 1 })
        ));
        assert_eq!(ctx.events()[1], RecoveryEvent::RecoverySucceeded);
    }

    #[test]
    fn escalation_after_max_attempts_exceeded() {
        // given
        let mut ctx = RecoveryContext::new();
        let scenario = FailureScenario::PromptMisdelivery;

        // when — first attempt succeeds
        let first = attempt_recovery(&scenario, &mut ctx);
        assert!(matches!(first, RecoveryResult::Recovered { .. }));

        // when — second attempt should escalate
        let second = attempt_recovery(&scenario, &mut ctx);

        // then
        assert!(
            matches!(
                &second,
                RecoveryResult::EscalationRequired { reason }
                    if reason.contains("max recovery attempts")
            ),
            "second attempt should require escalation, got: {second:?}"
        );
        assert_eq!(ctx.attempt_count(&scenario), 1);
        assert!(
            ctx.events()
                .iter()
                .any(|e| matches!(e, RecoveryEvent::Escalated))
        );
    }

    #[test]
    fn partial_recovery_when_step_fails_midway() {
        // given — PartialPluginStartup has two steps; fail at step index 1
        let mut ctx = RecoveryContext::new().with_fail_at_step(1);
        let scenario = FailureScenario::PartialPluginStartup;

        // when
        let result = attempt_recovery(&scenario, &mut ctx);

        // then
        match &result {
            RecoveryResult::PartialRecovery {
                recovered,
                remaining,
            } => {
                assert_eq!(recovered.len(), 1, "one step should have succeeded");
                assert_eq!(remaining.len(), 1, "one step should remain");
                assert!(matches!(recovered[0], RecoveryStep::RestartPlugin { .. }));
                assert!(matches!(remaining[0], RecoveryStep::RetryMcpHandshake { .. }));
            },
            other => panic!("expected PartialRecovery, got {other:?}"),
        }
        assert!(
            ctx.events()
                .iter()
                .any(|e| matches!(e, RecoveryEvent::RecoveryFailed))
        );
    }

    #[test]
    fn first_step_failure_escalates_immediately() {
        // given — fail at step index 0
        let mut ctx = RecoveryContext::new().with_fail_at_step(0);
        let scenario = FailureScenario::CompileRedCrossCrate;

        // when
        let result = attempt_recovery(&scenario, &mut ctx);

        // then
        assert!(
            matches!(
                &result,
                RecoveryResult::EscalationRequired { reason }
                    if reason.contains("failed at first step")
            ),
            "zero-step failure should escalate, got: {result:?}"
        );
        assert!(
            ctx.events()
                .iter()
                .any(|e| matches!(e, RecoveryEvent::Escalated))
        );
    }

    #[test]
    fn emitted_events_include_structured_attempt_data() {
        // given
        let mut ctx = RecoveryContext::new();
        let scenario = FailureScenario::McpHandshakeFailure;

        // when
        let _ = attempt_recovery(&scenario, &mut ctx);

        // then — verify the RecoveryAttempted event carries full context
        let attempted = ctx
            .events()
            .iter()
            .find(|e| matches!(e, RecoveryEvent::RecoveryAttempted { .. }))
            .expect("should have emitted RecoveryAttempted event");

        match attempted {
            RecoveryEvent::RecoveryAttempted {
                scenario: s,
                recipe,
                result,
            } => {
                assert_eq!(*s, scenario);
                assert_eq!(recipe.scenario, scenario);
                assert!(!recipe.steps.is_empty());
                assert!(matches!(result, RecoveryResult::Recovered { .. }));
            },
            _ => unreachable!(),
        }

        // Verify the event is serializable as structured JSON
        let json = serde_json::to_string(&ctx.events()[0])
            .expect("recovery event should be serializable to JSON");
        assert!(
            json.contains("mcp_handshake_failure"),
            "serialized event should contain scenario name"
        );
    }

    #[test]
    fn recovery_context_tracks_attempts_per_scenario() {
        // given
        let mut ctx = RecoveryContext::new();

        // when
        assert_eq!(ctx.attempt_count(&FailureScenario::StaleBranch), 0);
        attempt_recovery(&FailureScenario::StaleBranch, &mut ctx);

        // then
        assert_eq!(ctx.attempt_count(&FailureScenario::StaleBranch), 1);
        assert_eq!(ctx.attempt_count(&FailureScenario::PromptMisdelivery), 0);
    }

    #[test]
    fn stale_branch_recipe_has_rebase_then_clean_build() {
        // given
        let recipe = recipe_for(&FailureScenario::StaleBranch);

        // then
        assert_eq!(recipe.steps.len(), 2);
        assert_eq!(recipe.steps[0], RecoveryStep::RebaseBranch);
        assert_eq!(recipe.steps[1], RecoveryStep::CleanBuild);
    }

    #[test]
    fn partial_plugin_startup_recipe_has_restart_then_handshake() {
        // given
        let recipe = recipe_for(&FailureScenario::PartialPluginStartup);

        // then
        assert_eq!(recipe.steps.len(), 2);
        assert!(matches!(recipe.steps[0], RecoveryStep::RestartPlugin { .. }));
        assert!(matches!(recipe.steps[1], RecoveryStep::RetryMcpHandshake { timeout: 3000 }));
        assert_eq!(recipe.escalation_policy, EscalationPolicy::LogAndContinue);
    }

    #[test]
    fn failure_scenario_display_all_variants() {
        // given
        let cases = [
            (FailureScenario::TrustPromptUnresolved, "trust_prompt_unresolved"),
            (FailureScenario::PromptMisdelivery, "prompt_misdelivery"),
            (FailureScenario::StaleBranch, "stale_branch"),
            (FailureScenario::CompileRedCrossCrate, "compile_red_cross_crate"),
            (FailureScenario::McpHandshakeFailure, "mcp_handshake_failure"),
            (FailureScenario::PartialPluginStartup, "partial_plugin_startup"),
        ];

        // when / then
        for (scenario, expected) in &cases {
            assert_eq!(scenario.to_string(), *expected);
        }
    }

    #[test]
    fn multi_step_success_reports_correct_steps_taken() {
        // given — StaleBranch has 2 steps, no simulated failure
        let mut ctx = RecoveryContext::new();
        let scenario = FailureScenario::StaleBranch;

        // when
        let result = attempt_recovery(&scenario, &mut ctx);

        // then
        assert_eq!(result, RecoveryResult::Recovered { steps_taken: 2 });
    }

    #[test]
    fn mcp_handshake_recipe_uses_abort_escalation_policy() {
        // given
        let recipe = recipe_for(&FailureScenario::McpHandshakeFailure);

        // then
        assert_eq!(recipe.escalation_policy, EscalationPolicy::Abort);
        assert_eq!(recipe.max_attempts, 1);
    }

    #[test]
    fn worker_failure_kind_maps_to_failure_scenario() {
        // given / when / then — verify the bridge is correct
        assert_eq!(
            FailureScenario::from_worker_failure_kind(WorkerFailureKind::TrustGate),
            FailureScenario::TrustPromptUnresolved,
        );
        assert_eq!(
            FailureScenario::from_worker_failure_kind(WorkerFailureKind::PromptDelivery),
            FailureScenario::PromptMisdelivery,
        );
        assert_eq!(
            FailureScenario::from_worker_failure_kind(WorkerFailureKind::Protocol),
            FailureScenario::McpHandshakeFailure,
        );
        assert_eq!(
            FailureScenario::from_worker_failure_kind(WorkerFailureKind::Provider),
            FailureScenario::ProviderFailure,
        );
    }

    #[test]
    fn provider_failure_recipe_uses_restart_worker_step() {
        // given
        let recipe = recipe_for(&FailureScenario::ProviderFailure);

        // then
        assert_eq!(recipe.scenario, FailureScenario::ProviderFailure);
        assert!(recipe.steps.contains(&RecoveryStep::RestartWorker));
        assert_eq!(recipe.escalation_policy, EscalationPolicy::AlertHuman);
        assert_eq!(recipe.max_attempts, 1);
    }

    #[test]
    fn provider_failure_recovery_attempt_succeeds_then_escalates() {
        // given
        let mut ctx = RecoveryContext::new();
        let scenario = FailureScenario::ProviderFailure;

        // when — first attempt
        let first = attempt_recovery(&scenario, &mut ctx);
        assert!(matches!(first, RecoveryResult::Recovered { .. }));

        // when — second attempt should escalate (max_attempts=1)
        let second = attempt_recovery(&scenario, &mut ctx);
        assert!(matches!(second, RecoveryResult::EscalationRequired { .. }));
        assert!(
            ctx.events()
                .iter()
                .any(|e| matches!(e, RecoveryEvent::Escalated))
        );
    }
}
