use axagent_agent::error_recovery_engine::{ErrorRecoveryEngine, RecoveryConfig, RecoveryContext};
use axagent_agent::error_classifier::ErrorType;
use axagent_agent::recovery_strategies::RecoveryStrategy;

#[test]
fn test_create_recovery_engine() {
    let engine = ErrorRecoveryEngine::new();
    let subscriber = engine.subscribe();
    drop(subscriber);
}

#[test]
fn test_recovery_engine_with_config() {
    let config = RecoveryConfig {
        max_total_attempts: 3,
        enable_fallback: false,
        enable_adjustments: true,
        timeout_per_attempt: std::time::Duration::from_secs(10),
    };
    let engine = ErrorRecoveryEngine::new().with_config(config);
    let _ = engine.subscribe();
}

#[test]
fn test_classify_transient_error() {
    let engine = ErrorRecoveryEngine::new();
    let classified = engine.classify_error("connection timeout occurred");
    assert_eq!(classified.error_type, ErrorType::Transient);
}

#[test]
fn test_classify_transient_network_error() {
    let engine = ErrorRecoveryEngine::new();
    let classified = engine.classify_error("network error: 503 service unavailable");
    assert_eq!(classified.error_type, ErrorType::Transient);
}

#[test]
fn test_classify_recoverable_rate_limit() {
    let engine = ErrorRecoveryEngine::new();
    let classified = engine.classify_error("insufficient quota exceeded for resource");
    assert_eq!(classified.error_type, ErrorType::Recoverable);
}

#[test]
fn test_classify_recoverable_not_found() {
    let engine = ErrorRecoveryEngine::new();
    let classified = engine.classify_error("resource not found");
    assert_eq!(classified.error_type, ErrorType::Recoverable);
}

#[test]
fn test_classify_unrecoverable_parse_error() {
    let engine = ErrorRecoveryEngine::new();
    let classified = engine.classify_error("parse error: invalid syntax in configuration");
    assert_eq!(classified.error_type, ErrorType::Unrecoverable);
}

#[test]
fn test_classify_unrecoverable_internal() {
    let engine = ErrorRecoveryEngine::new();
    let classified = engine.classify_error("500 internal error: null pointer dereference");
    assert_eq!(classified.error_type, ErrorType::Unrecoverable);
}

#[test]
fn test_get_recovery_strategy_for_transient() {
    let engine = ErrorRecoveryEngine::new();
    let strategy = engine.get_recovery_strategy(ErrorType::Transient);
    assert!(matches!(strategy, RecoveryStrategy::Retry { .. }));
}

#[test]
fn test_get_recovery_strategy_for_recoverable() {
    let engine = ErrorRecoveryEngine::new();
    let strategy = engine.get_recovery_strategy(ErrorType::Recoverable);
    assert!(matches!(strategy, RecoveryStrategy::AdjustAndRetry { .. }));
}

#[test]
fn test_get_recovery_strategy_for_unrecoverable() {
    let engine = ErrorRecoveryEngine::new();
    let strategy = engine.get_recovery_strategy(ErrorType::Unrecoverable);
    assert!(matches!(strategy, RecoveryStrategy::Fail));
}

#[test]
fn test_recovery_config_default() {
    let config = RecoveryConfig::default();
    assert_eq!(config.max_total_attempts, 5);
    assert!(config.enable_fallback);
    assert!(config.enable_adjustments);
    assert_eq!(config.timeout_per_attempt, std::time::Duration::from_secs(30));
}

#[test]
fn test_recovery_context_builder() {
    let ctx = RecoveryContext::new()
        .with_task_id("task_42".to_string())
        .with_error("test error".to_string())
        .build();

    assert_eq!(ctx.task_id, Some("task_42".to_string()));
    assert_eq!(ctx.original_error, Some("test error".to_string()));
}

#[test]
fn test_recovery_context_default() {
    let ctx = RecoveryContext::new().build();
    assert_eq!(ctx.task_id, None);
    assert_eq!(ctx.original_error, None);
    assert_eq!(ctx.attempts, 0);
}

#[test]
fn test_classify_empty_error_unknown() {
    let engine = ErrorRecoveryEngine::new();
    let classified = engine.classify_error("");
    assert_eq!(classified.error_type, ErrorType::Unknown);
}

#[test]
fn test_classify_generic_message_unknown() {
    let engine = ErrorRecoveryEngine::new();
    let classified = engine.classify_error("something went wrong");
    assert_eq!(classified.error_type, ErrorType::Unknown);
}
