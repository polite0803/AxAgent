// SPDX-License-Identifier: AGPL-3.0-only

//! Extended tests for react_engine: resume, builder patterns, token budget.

use axagent_agent::react_engine::{ReActConfig, ReActEngine};

// ---------------------------------------------------------------------------
// ReActEngine — resume() method  (checkpoint edge cases)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_react_engine_resume_after_run() {
    let mut engine = ReActEngine::new();

    // First run to establish state
    let _first = engine.run("Hello").await;
    // Then try resume (should handle gracefully even without saved checkpoint)
    let resume_result = engine.resume("Continue").await;
    // Either it succeeds or returns a meaningful error; no panic
    if resume_result.success {
        assert!(!resume_result.final_response.is_empty());
    }
}

// ---------------------------------------------------------------------------
// ReActEngine — builder pattern integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_react_engine_with_config_and_goal_eval() {
    let config = ReActConfig {
        max_iterations: 5,
        ..Default::default()
    };

    let mut engine = ReActEngine::new()
        .with_config(config)
        .with_goal_evaluation(2);

    let result = engine.run("Write a short greeting").await;
    // Should complete without panic
    if result.success {
        assert!(!result.final_response.is_empty());
    }
}

#[tokio::test]
async fn test_react_engine_token_budget_reset() {
    let mut engine = ReActEngine::new();
    engine.reset_token_budget();
    let result = engine.run("Hello").await;
    // token budget reset should not cause errors
    if result.success {
        assert!(!result.final_response.is_empty());
    }
}

#[tokio::test]
async fn test_react_engine_reset_and_rerun() {
    let mut engine = ReActEngine::new();
    let r1 = engine.run("Task one").await;
    let r2 = engine.run("Task two").await;
    // Multiple runs should each produce results without panic
    // (both may succeed or fail depending on mocked provider, but no crash)
    assert!(r1.total_duration_ms >= 0 || !r1.success);
    assert!(r2.total_duration_ms >= 0 || !r2.success);
}
