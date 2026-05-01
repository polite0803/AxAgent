use axagent_agent::react_engine::ReActEngine;

#[tokio::test]
async fn test_create_react_engine() {
    let engine = ReActEngine::new();
    // Just verify it doesn't panic
    let _ = engine.subscribe();
}

#[tokio::test]
async fn test_react_engine_with_config() {
    let config = axagent_agent::reasoning_state::ReActConfig {
        max_iterations: 10,
        ..Default::default()
    };
    let engine = ReActEngine::new().with_config(config);
    let _ = engine.subscribe();
}

#[tokio::test]
async fn test_react_run_basic() {
    let mut engine = ReActEngine::new();
    let result = engine.run("Hello").await;
    assert!(!result.final_response.is_empty() || result.error.is_some());
}

#[tokio::test]
async fn test_react_run_empty_input() {
    let mut engine = ReActEngine::new();
    let result = engine.run("").await;
    assert!(result.iterations > 0);
}

#[tokio::test]
async fn test_react_run_with_max_iterations_constraint() {
    let config = axagent_agent::reasoning_state::ReActConfig {
        max_iterations: 1,
        ..Default::default()
    };
    let mut engine = ReActEngine::new().with_config(config);
    let result = engine.run("Do something").await;
    assert!(result.iterations > 0);
}

#[tokio::test]
async fn test_react_result_contains_context() {
    let mut engine = ReActEngine::new();
    let result = engine.run("Test").await;
    // thought_chain should at least exist
    let _ = &result.thought_chain;
}
