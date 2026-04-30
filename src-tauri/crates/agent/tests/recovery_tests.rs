use axagent_agent::error_classifier::ErrorType;
use axagent_agent::recovery_strategies::RecoveryStrategy;
use std::time::Duration;

#[test]
fn test_transient_error_maps_to_retry() {
    let strategy = RecoveryStrategy::for_error_type(ErrorType::Transient);
    assert!(matches!(strategy, RecoveryStrategy::Retry { .. }));
    if let RecoveryStrategy::Retry {
        max_attempts,
        exponential_backoff,
        ..
    } = strategy
    {
        assert!(max_attempts > 0);
        assert!(exponential_backoff);
    }
}

#[test]
fn test_recoverable_error_maps_to_adjust_and_retry() {
    let strategy = RecoveryStrategy::for_error_type(ErrorType::Recoverable);
    assert!(matches!(strategy, RecoveryStrategy::AdjustAndRetry { .. }));
    if let RecoveryStrategy::AdjustAndRetry { adjustments, .. } = strategy {
        assert!(!adjustments.is_empty());
    }
}

#[test]
fn test_unrecoverable_error_maps_to_fail() {
    let strategy = RecoveryStrategy::for_error_type(ErrorType::Unrecoverable);
    assert!(matches!(strategy, RecoveryStrategy::Fail));
}

#[test]
fn test_unknown_error_maps_to_retry() {
    let strategy = RecoveryStrategy::for_error_type(ErrorType::Unknown);
    assert!(matches!(strategy, RecoveryStrategy::Retry { .. }));
}

#[test]
fn test_retry_strategy_defaults() {
    let strategy = RecoveryStrategy::for_error_type(ErrorType::Transient);
    if let RecoveryStrategy::Retry {
        max_attempts,
        base_delay_ms,
        max_delay_ms,
        exponential_backoff,
    } = strategy
    {
        assert_eq!(max_attempts, 3);
        assert_eq!(base_delay_ms, 1000);
        assert_eq!(max_delay_ms, 10000);
        assert!(exponential_backoff);
    }
}

#[test]
fn test_all_error_types_have_strategy() {
    let types = [
        ErrorType::Transient,
        ErrorType::Recoverable,
        ErrorType::Unrecoverable,
        ErrorType::Unknown,
    ];
    for error_type in &types {
        let strategy = RecoveryStrategy::for_error_type(*error_type);
        let is_valid = matches!(
            strategy,
            RecoveryStrategy::Retry { .. }
                | RecoveryStrategy::AdjustAndRetry { .. }
                | RecoveryStrategy::Fallback { .. }
                | RecoveryStrategy::SkipTask
                | RecoveryStrategy::Fail
        );
        assert!(
            is_valid,
            "ErrorType {:?} should map to a valid recovery strategy",
            error_type
        );
    }
}

#[test]
fn test_fallback_strategy() {
    let fallback = RecoveryStrategy::Fallback {
        fallback_value: "default response".to_string(),
    };
    assert!(matches!(fallback, RecoveryStrategy::Fallback { .. }));
}

#[test]
fn test_skip_task_strategy() {
    let strategy = RecoveryStrategy::SkipTask;
    assert!(matches!(strategy, RecoveryStrategy::SkipTask));
}

#[test]
fn test_fail_strategy() {
    let strategy = RecoveryStrategy::Fail;
    assert!(matches!(strategy, RecoveryStrategy::Fail));
}

#[test]
fn test_recovery_adjustments() {
    use axagent_agent::recovery_strategies::RecoveryAdjustment;

    let adjustments = vec![
        RecoveryAdjustment::ReduceConcurrency,
        RecoveryAdjustment::IncreaseTimeout(Duration::from_secs(30)),
        RecoveryAdjustment::UseCache,
        RecoveryAdjustment::SimplifyRequest,
        RecoveryAdjustment::RetryWithDifferentModel,
    ];
    assert_eq!(adjustments.len(), 5);
}
